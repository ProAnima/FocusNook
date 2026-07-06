use crate::audio_crypto;
use crate::sync_log::{self, HlcClock};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use std::fs;
use std::path::Path;

// Раздел 8 ТЗ: Note. audio — запись без транскрипта: старт/стоп через
// MediaRecorder на фронтенде, байты уходят в Rust base64-строкой и ложатся
// файлом на диск (audio_path хранит только имя файла, не полный путь).
// Полная сущность AudioAsset из раздела 8 (codec/durationMs/sizeBytes/
// contentHash/remoteBlobRef) урезана до одного поля — остальное имеет смысл
// только вместе с sync (Iteration 2) и явно не нужно раньше. transcript/
// audio_with_transcript всё ещё ждут speech-to-text спайка (Iteration 3).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteDto {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub kind: String,
    pub audio_path: Option<String>,
}

fn row_to_dto(row: &Row) -> rusqlite::Result<NoteDto> {
    Ok(NoteDto {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        kind: row.get(3)?,
        audio_path: row.get(4)?,
    })
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<NoteDto>> {
    let mut stmt = conn
        .prepare("SELECT id, title, body, kind, audio_path FROM notes ORDER BY created_at DESC")?;
    let notes = stmt.query_map([], row_to_dto)?.collect();
    notes
}

// Раздел 9 ТЗ, Iteration 2 (первый локальный шаг): мутация и запись в
// sync_operations идут одной транзакцией — см. plan_items.rs::toggle_done
// для развёрнутого объяснения, здесь тот же паттерн.
pub fn create(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    body: &str,
) -> rusqlite::Result<NoteDto> {
    let tx = conn.transaction()?;
    let id = uuid::Uuid::now_v7().to_string();
    tx.execute(
        "INSERT INTO notes (id, title, body, kind, created_at)
         VALUES (?1, NULL, ?2, 'text', datetime('now'))",
        params![id, body],
    )?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "note",
        &id,
        "create",
        &serde_json::json!({ "body": body, "kind": "text" }),
    )?;
    tx.commit()?;
    Ok(NoteDto {
        id,
        title: None,
        body: body.to_string(),
        kind: "text".to_string(),
        audio_path: None,
    })
}

// audio_dir — общая (не per-profile) папка на диске; заметки, которые на неё
// ссылаются, уже изолированы по профилю через свой vault, так что утечки
// данных между профилями нет — только неиспользуемые файлы могут накапливаться
// на диске, если профиль/заметку удалили. Достаточно для MVP, см. раздел 22.
//
// Запись файла на диск остаётся вне SQL-транзакции ниже (она и не была в ней
// раньше) — при крахе между fs::write и commit возможен осиротевший файл; это
// уже существовавший, а не новый этим шагом пробел.
// 10 МБ с запасом покрывает голосовую заметку до ~5 минут (см. лимит на
// фронтенде в useAudioRecorder.ts) — заведомо больше типичного размера, но
// не даёт откровенно раздутому payload’у лечь на диск без предупреждения.
const MAX_AUDIO_BYTES: usize = 10 * 1024 * 1024;

// audio_key: Some(vault_key) на десктопе — файл шифруется тем же секретом,
// что и сам vault (через доменное разделение, см. audio_crypto.rs); None на
// Android, где своего Keystore-эквивалента пока нет (раздел 26) — файл
// остаётся как есть, тот же честно объявленный уровень защиты, что и у
// самой БД там.
pub fn create_audio(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    audio_dir: &Path,
    audio_key: Option<&str>,
    base64_data: &str,
) -> Result<NoteDto, String> {
    let bytes = STANDARD.decode(base64_data).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_AUDIO_BYTES {
        return Err(format!(
            "аудиозапись слишком большая ({} МБ), максимум {} МБ",
            bytes.len() / (1024 * 1024),
            MAX_AUDIO_BYTES / (1024 * 1024)
        ));
    }
    let to_write = match audio_key {
        Some(key) => audio_crypto::encrypt(key, &bytes)?,
        None => bytes,
    };
    fs::create_dir_all(audio_dir).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::now_v7().to_string();
    let filename = format!("{id}.webm");
    fs::write(audio_dir.join(&filename), &to_write).map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO notes (id, title, body, kind, audio_path, created_at)
         VALUES (?1, NULL, '', 'audio', ?2, datetime('now'))",
        params![id, filename],
    )
    .map_err(|e| e.to_string())?;
    let hlc = clock.next(&tx).map_err(|e| e.to_string())?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "note",
        &id,
        "create",
        &serde_json::json!({ "kind": "audio", "audioPath": filename }),
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(NoteDto {
        id,
        title: None,
        body: String::new(),
        kind: "audio".to_string(),
        audio_path: Some(filename),
    })
}

// Путь берётся из БД по id заметки, а не принимается напрямую от фронтенда —
// так имя файла на диске никогда не приходит как чужой ввод (без этого
// пришлось бы отдельно защищаться от path traversal в audio_path).
//
// decrypt_if_needed (не decrypt) — файлы, записанные до этого фикса, лежат
// как обычный webm без метки формата; audio_key может быть Some и для них
// (это не новые файлы, а старые на том же диске), так что различать
// "старый/новый" должна сама audio_crypto по метке, а не эта функция по
// внешним признакам.
pub fn read_audio(
    conn: &Connection,
    audio_dir: &Path,
    audio_key: Option<&str>,
    note_id: &str,
) -> Result<String, String> {
    let filename: String = conn
        .query_row(
            "SELECT audio_path FROM notes WHERE id = ?1 AND audio_path IS NOT NULL",
            params![note_id],
            |row| row.get(0),
        )
        .map_err(|_| "аудио для этой заметки не найдено".to_string())?;
    let raw = fs::read(audio_dir.join(filename)).map_err(|e| e.to_string())?;
    let bytes = match audio_key {
        Some(key) => audio_crypto::decrypt_if_needed(key, &raw)?,
        None => raw,
    };
    Ok(STANDARD.encode(bytes))
}

// audio_dir нужен, чтобы удалить файл с диска, а не только строку из БД
// (раздел P1 ревью — раньше удаление заметки оставляло .webm висеть на
// диске вечно). Файл убирается ПОСЛЕ commit и best-effort (ошибка не
// заваливает удаление самой заметки) — БД остаётся источником истины о том,
// существует ли заметка, а файл на диске уже не критичен для этого; тот же
// принцип, что и у fs::write вне транзакции при создании (см. create_audio).
pub fn delete(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    audio_dir: &Path,
    id: &str,
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let audio_path: Option<String> = tx
        .query_row(
            "SELECT audio_path FROM notes WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    tx.execute("DELETE FROM notes WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    let hlc = clock.next(&tx).map_err(|e| e.to_string())?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "note",
        id,
        "delete",
        &serde_json::json!({}),
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    if let Some(filename) = audio_path {
        if let Err(e) = fs::remove_file(audio_dir.join(&filename)) {
            eprintln!("notes: не удалось удалить аудиофайл {filename}: {e}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    // unwrap/expect в тестах — норма (паника здесь и есть правильный сигнал
    // "тест упал"), а не то, от чего защищает прод-lint в Cargo.toml.
    #![allow(clippy::unwrap_used)]
    use super::*;

    const PROFILE: &str = "profile-1";

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE notes (
                id TEXT PRIMARY KEY,
                title TEXT,
                body TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'text',
                audio_path TEXT,
                created_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE sync_operations (
                operation_id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                op TEXT NOT NULL,
                patch TEXT NOT NULL,
                hlc TEXT NOT NULL,
                schema_version INTEGER NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE sync_clock_state (
                id INTEGER PRIMARY KEY CHECK (id = 0),
                last_millis INTEGER NOT NULL,
                last_counter INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();
        conn
    }

    fn test_clock() -> HlcClock {
        let conn = Connection::open_in_memory().unwrap();
        HlcClock::load(&conn, "test-device".to_string()).unwrap()
    }

    fn temp_audio_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("focusnook-test-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn create_and_list_text_note() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let created = create(&mut conn, &mut clock, PROFILE, "hello").unwrap();
        assert_eq!(created.kind, "text");
        assert_eq!(created.audio_path, None);

        let notes = list(&conn).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].body, "hello");
    }

    #[test]
    fn create_writes_a_matching_operation_log_row() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let created = create(&mut conn, &mut clock, PROFILE, "hello").unwrap();

        let (op, patch): (String, String) = conn
            .query_row(
                "SELECT op, patch FROM sync_operations WHERE entity_id = ?1",
                params![created.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(op, "create");
        let patch: serde_json::Value = serde_json::from_str(&patch).unwrap();
        assert_eq!(patch["body"], "hello");
    }

    // Именно эту цепочку (base64 из фронтенда -> файл на диске -> base64
    // обратно) я не мог проверить живьём — в browser-preview нет доступа к
    // микрофону, а кликать внутри нативного Tauri-окна нечем. base64 не
    // различает "настоящее аудио" и любые другие байты, так что синтетические
    // байты здесь дают ту же гарантию, что и реальная запись с микрофона.
    const AUDIO_KEY: &str = "test-vault-key-hex";

    #[test]
    fn audio_roundtrip_through_disk_and_base64() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let dir = temp_audio_dir();
        let original = STANDARD.encode(b"fake audio bytes");

        let created = create_audio(
            &mut conn,
            &mut clock,
            PROFILE,
            &dir,
            Some(AUDIO_KEY),
            &original,
        )
        .unwrap();
        assert_eq!(created.kind, "audio");
        assert!(created.audio_path.is_some());

        let read_back = read_audio(&conn, &dir, Some(AUDIO_KEY), &created.id).unwrap();
        assert_eq!(read_back, original);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    // P1 ревью: аудио раньше писалось в открытую несмотря на зашифрованный
    // vault. Регрессионный тест на то, что байты на диске реально не
    // совпадают с исходным аудио, когда есть ключ.
    #[test]
    fn create_audio_actually_encrypts_the_file_on_disk() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let dir = temp_audio_dir();
        let plaintext = b"fake audio bytes, not a secret in this test but treated as one";
        let original = STANDARD.encode(plaintext);

        let created = create_audio(
            &mut conn,
            &mut clock,
            PROFILE,
            &dir,
            Some(AUDIO_KEY),
            &original,
        )
        .unwrap();
        let filename = created.audio_path.unwrap();
        let on_disk = std::fs::read(dir.join(filename)).unwrap();
        assert_ne!(
            on_disk, plaintext,
            "файл на диске не должен совпадать с исходным аудио"
        );
        assert!(
            !on_disk
                .windows(plaintext.len())
                .any(|window| window == plaintext.as_slice()),
            "исходные байты не должны встречаться в зашифрованном файле как есть"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    // Android (пока без Keystore-эквивалента, раздел 26): audio_key = None
    // должен вести себя как раньше — записывать байты как есть.
    #[test]
    fn create_audio_without_a_key_writes_plain_bytes() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let dir = temp_audio_dir();
        let original = STANDARD.encode(b"fake audio bytes");

        let created = create_audio(&mut conn, &mut clock, PROFILE, &dir, None, &original).unwrap();
        let read_back = read_audio(&conn, &dir, None, &created.id).unwrap();
        assert_eq!(read_back, original);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    // P1 ревью: файлы, записанные до этого фикса, — обычный webm без метки
    // формата. Регрессия на то, что они остаются читаемыми даже когда
    // audio_key теперь есть (ключ появляется у всех, файл — нет).
    #[test]
    fn read_audio_returns_legacy_plaintext_files_unchanged() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let dir = temp_audio_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let legacy_bytes = b"legacy plaintext webm, written before encryption existed";
        let created = create(&mut conn, &mut clock, PROFILE, "").unwrap();
        std::fs::write(dir.join("legacy.webm"), legacy_bytes).unwrap();
        conn.execute(
            "UPDATE notes SET kind = 'audio', audio_path = 'legacy.webm' WHERE id = ?1",
            params![created.id],
        )
        .unwrap();

        let read_back = read_audio(&conn, &dir, Some(AUDIO_KEY), &created.id).unwrap();
        assert_eq!(read_back, STANDARD.encode(legacy_bytes));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn create_audio_rejects_a_payload_over_the_size_limit() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let dir = temp_audio_dir();
        let too_big = STANDARD.encode(vec![0u8; MAX_AUDIO_BYTES + 1]);

        let result = create_audio(
            &mut conn,
            &mut clock,
            PROFILE,
            &dir,
            Some(AUDIO_KEY),
            &too_big,
        );
        assert!(result.is_err());
        assert!(
            !dir.exists(),
            "отклонённый payload не должен успевать создать audio_dir"
        );
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn read_audio_fails_for_a_text_note() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let created = create(&mut conn, &mut clock, PROFILE, "no audio here").unwrap();
        assert!(read_audio(&conn, &std::env::temp_dir(), Some(AUDIO_KEY), &created.id).is_err());
    }

    #[test]
    fn delete_removes_the_row() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let created = create(&mut conn, &mut clock, PROFILE, "temp").unwrap();
        delete(
            &mut conn,
            &mut clock,
            PROFILE,
            &std::env::temp_dir(),
            &created.id,
        )
        .unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }

    // P1 ревью: раньше удаление заметки чистило только строку в БД, .webm
    // оставался на диске навсегда. Основной регрессионный тест этого фикса.
    #[test]
    fn delete_removes_the_audio_file_from_disk() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let dir = temp_audio_dir();
        let original = STANDARD.encode(b"fake audio bytes");
        let created = create_audio(
            &mut conn,
            &mut clock,
            PROFILE,
            &dir,
            Some(AUDIO_KEY),
            &original,
        )
        .unwrap();
        let file_path = dir.join(created.audio_path.clone().unwrap());
        assert!(file_path.exists());

        delete(&mut conn, &mut clock, PROFILE, &dir, &created.id).unwrap();

        assert!(
            !file_path.exists(),
            "аудиофайл должен быть удалён вместе с заметкой"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // Удаление несуществующей заметки — идемпотентная операция (как и было
    // до этого фикса): не должно ни падать, ни пытаться трогать диск.
    #[test]
    fn delete_of_an_unknown_id_is_a_harmless_no_op() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        assert!(delete(
            &mut conn,
            &mut clock,
            PROFILE,
            &std::env::temp_dir(),
            "does-not-exist"
        )
        .is_ok());
    }
}
