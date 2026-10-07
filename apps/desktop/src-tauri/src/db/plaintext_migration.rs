use rusqlite::Connection;
use std::path::{Path, PathBuf};

const SQLITE_PLAINTEXT_MAGIC: &[u8; 16] = b"SQLite format 3\0";

// Настоящий (незашифрованный) SQLite-файл всегда начинается с этой сигнатуры;
// у SQLCipher-зашифрованного файла первые байты неотличимы от случайных.
// Разбор ошибок чтения (файла нет / он короче 16 байт) как "не похоже на
// plaintext" — не ошибка сама по себе, migrate_plaintext_if_needed ниже и так
// отдельно проверяет существование файла до вызова этой функции.
pub(super) fn looks_like_plaintext_sqlite(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut header = [0u8; 16];
    if file.read_exact(&mut header).is_err() {
        return false;
    }
    &header == SQLITE_PLAINTEXT_MAGIC
}

pub(super) fn verify_encrypted_sqlite(path: &Path, key_hex: &str) -> Result<(), String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute_batch(&format!("PRAGMA key = \"x'{key_hex}'\";"))
        .map_err(|e| e.to_string())?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if integrity == "ok" {
        Ok(())
    } else {
        Err(format!(
            "encrypted vault integrity check failed: {integrity}"
        ))
    }
}

// P1 ревью + раздел 26 ТЗ: Iteration 0 (без шифрования) вышла раньше
// Iteration 1 (шифрование) — установки с того периода имеют настоящий
// plaintext vault.db. Открыть такой файл через голый PRAGMA key бессмысленно:
// SQLCipher примет любой ключ молча, но не сможет прочитать уже
// существующие данные как валидный формат. Конвертируем через официальный
// sqlcipher_export (не свою бинарную миграцию) — задокументированный,
// проверенный путь plaintext -> encrypted у самого SQLCipher.
//
// Оригинал не трогаем "на месте": переименовываем в .plaintext-backup ДО
// конвертации, так что при сбое sqlcipher_export исходные данные остаются
// целы. После проверки integrity_check plaintext-backup обязательно удаляется.
// Если предыдущая попытка миграции упала между этими двумя шагами (path
// уже нет, а .plaintext-backup ещё есть) — доводим её до конца с backup,
// а не заводим на его месте пустой новый vault.
pub(super) fn migrate_plaintext_if_needed(path: &Path, key_hex: &str) -> Result<(), String> {
    let backup_path = PathBuf::from(format!("{}.plaintext-backup", path.display()));

    let source = if path.exists() {
        if !looks_like_plaintext_sqlite(path) {
            match verify_encrypted_sqlite(path, key_hex) {
                Ok(()) => {
                    if backup_path.exists() {
                        std::fs::remove_file(&backup_path).map_err(|e| {
                            format!(
                                "encrypted vault is valid, but plaintext backup could not be removed: {e}"
                            )
                        })?;
                    }
                    return Ok(());
                }
                Err(_) if backup_path.exists() => {
                    std::fs::remove_file(path).map_err(|e| {
                        format!("failed to replace incomplete encrypted vault: {e}")
                    })?;
                    &backup_path
                }
                Err(error) => return Err(error),
            }
        } else {
            std::fs::rename(path, &backup_path).map_err(|e| e.to_string())?;
            &backup_path
        }
    } else if backup_path.exists() {
        &backup_path
    } else {
        return Ok(());
    };

    log::info!("vault: обнаружен незашифрованный vault, конвертирую в SQLCipher");
    let plain_conn = Connection::open(source).map_err(|e| e.to_string())?;
    let escaped_target = path.display().to_string().replace('\'', "''");
    plain_conn
        .execute_batch(&format!(
            "ATTACH DATABASE '{escaped_target}' AS encrypted KEY \"x'{key_hex}'\";
             SELECT sqlcipher_export('encrypted');
             DETACH DATABASE encrypted;"
        ))
        .map_err(|e| {
            format!(
                "не удалось сконвертировать старую базу в зашифрованный формат: {e}. \
                 Исходные данные сохранены в {}",
                source.display()
            )
        })?;
    drop(plain_conn);
    verify_encrypted_sqlite(path, key_hex)?;
    std::fs::remove_file(&backup_path)
        .map_err(|e| format!("encrypted vault is valid, but plaintext backup remains: {e}"))?;
    Ok(())
}
