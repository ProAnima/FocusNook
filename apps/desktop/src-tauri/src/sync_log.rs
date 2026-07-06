use crate::reminders::{civil_from_days, parse_trigger_millis};
use rusqlite::{params, Connection, Transaction};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// Раздел 9/14 ТЗ, Iteration 2, первый локальный шаг: operation log + HLC.
// Ни провайдер, ни сеть, ни OAuth здесь не появляются — см. аннотацию у
// раздела 14 архитектурного документа.

// Раздел 9 ТЗ: HLC как строка "2026-07-04T12:10:22.003Z-0004-device".
// Порядок вывода полей struct важен: derive(PartialOrd, Ord) сравнивает
// лексикографически по millis, потом counter, потом device_id — то же самое
// сравнение, которого потребует будущий движок синхронизации при сортировке
// операций по HLC. device_id — строковый суффикс, а не отдельное поле вне
// сравнения: сравнение struct и сравнение отформатированной строки должны
// совпадать, иначе порядок незаметно разойдётся при сериализации в JSON.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hlc {
    millis: i64,
    counter: u32,
    device_id: String,
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn millis_to_iso8601(millis: i64) -> String {
    let days = millis.div_euclid(86_400_000);
    let ms_of_day = millis.rem_euclid(86_400_000);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        ms_of_day / 3_600_000,
        (ms_of_day % 3_600_000) / 60_000,
        (ms_of_day % 60_000) / 1000,
        ms_of_day % 1000
    )
}

impl Hlc {
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn to_string_repr(&self) -> String {
        format!(
            "{}-{:04}-{}",
            millis_to_iso8601(self.millis),
            self.counter,
            self.device_id
        )
    }

    // counter — фиксированная ширина 4 цифры: именно это делает разбор
    // однозначным, даже когда device_id сам содержит дефисы (например, UUID)
    // — граница между counter и device_id всегда на одной и той же позиции
    // байтов, а не по счёту дефисов слева направо.
    //
    // Пока не вызывается из прод-кода (нечего сравнивать/сливать без второго
    // устройства) — понадобится будущему движку синхронизации для разбора
    // HLC из чужого operation log. Покрыта тестами уже сейчас, а не отложена
    // вместе с остальным будущим функционалом, потому что to_string_repr без
    // проверенного обратного разбора — риск незамеченной асимметрии формата.
    #[allow(dead_code)]
    pub fn parse(input: &str) -> Option<Hlc> {
        let bytes = input.as_bytes();
        if input.len() < 31 || bytes.get(24) != Some(&b'-') || bytes.get(29) != Some(&b'-') {
            return None;
        }
        let millis = parse_trigger_millis(input.get(0..24)?)?;
        let counter: u32 = input.get(25..29)?.parse().ok()?;
        let device_id = input.get(30..)?.to_string();
        if device_id.is_empty() {
            return None;
        }
        Some(Hlc {
            millis,
            counter,
            device_id,
        })
    }
}

// Живёt в Tauri managed state как HlcClockState(Mutex<HlcClock>), по образцу
// db::Db(Mutex<Connection>). Внутреннего второго Mutex в самом HlcClock не
// заводили: он и так целиком за внешним Mutex (это нужно для переключения
// профиля — см. lib.rs::switch_vault, который обязан пересоздать HlcClock для
// нового профиля, а не только подменить Connection), второй уровень
// блокировки был бы лишним.
pub struct HlcClock {
    device_id: String,
    last_millis: i64,
    last_counter: u32,
}

pub struct HlcClockState(pub Mutex<HlcClock>);

impl HlcClock {
    // Если строки в sync_clock_state ещё нет (первый запуск этого профиля) —
    // (0, 0) корректный дефолт: защищать первый когда-либо выпущенный HLC не
    // от чего, предыдущего состояния попросту не существует.
    pub fn load(conn: &Connection, device_id: String) -> rusqlite::Result<Self> {
        let (last_millis, last_counter) = conn
            .query_row(
                "SELECT last_millis, last_counter FROM sync_clock_state WHERE id = 0",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or((0, 0));
        Ok(HlcClock {
            device_id,
            last_millis,
            last_counter,
        })
    }

    // Классический HLC-merge (Kulkarni et al.): если настенные часы реально
    // продвинулись вперёд относительно последнего выпущенного HLC — берём их
    // и обнуляем счётчик; если нет (та же миллисекунда или откат назад,
    // например NTP-коррекция/переход через летнее время/ручная правка часов)
    // — время остаётся прежним, а растёт счётчик. Именно это даёт
    // монотонность, которую голый SystemTime::now() сам по себе не даёт.
    //
    // &mut Transaction, не &Connection — состояние часов пишется той же
    // транзакцией, что и сама мутация и запись в sync_operations (см.
    // record_operation ниже и lib.rs). Если что-то из трёх не закоммитится —
    // не закоммитится ничего: счётчик не может разойтись с журналом.
    pub fn next(&mut self, tx: &Transaction) -> rusqlite::Result<Hlc> {
        let wall_now = now_millis();
        let (next_millis, next_counter) = if wall_now > self.last_millis {
            (wall_now, 0)
        } else {
            (self.last_millis, self.last_counter + 1)
        };
        self.last_millis = next_millis;
        self.last_counter = next_counter;
        // UPSERT, не UPDATE: на первом-в-жизни-профиля вызове строки с id=0
        // ещё не существует, обычный UPDATE тогда молча обновил бы ноль строк
        // и состояние никогда не сохранилось бы на диск.
        tx.execute(
            "INSERT INTO sync_clock_state (id, last_millis, last_counter) VALUES (0, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET last_millis = excluded.last_millis, last_counter = excluded.last_counter",
            params![next_millis, next_counter],
        )?;
        Ok(Hlc {
            millis: next_millis,
            counter: next_counter,
            device_id: self.device_id.clone(),
        })
    }
}

// Раздел 9 ТЗ: минимальный device_id — только чтобы у HLC был tie-breaker
// между устройствами. НЕ полноценный device linking раздела 16 (без pairing/
// QR/revocation/display name). На уровне профиля (vault), а не машины —
// см. аннотацию в архитектурном документе про открытый вопрос "per-profile
// vs per-installation", который сознательно не решается в этом шаге.
pub fn ensure_device_identity(conn: &Connection) -> Result<String, String> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT device_id FROM device_identity WHERE id = 0",
            [],
            |row| row.get(0),
        )
        .ok();
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO device_identity (id, device_id) VALUES (0, ?1)",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

// Единственная точка записи в sync_operations — все мутирующие функции
// plan_items/notes/reminders вызывают её той же транзакцией, что и саму
// мутацию (см. lib.rs). device_id берётся из hlc, а не отдельным параметром —
// Hlc уже самодостаточный источник "кто, когда, в каком логическом порядке".
pub fn record_operation(
    tx: &Transaction,
    hlc: &Hlc,
    profile_id: &str,
    entity_type: &str,
    entity_id: &str,
    op: &str,
    patch: &serde_json::Value,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO sync_operations
            (operation_id, profile_id, device_id, entity_type, entity_id, op, patch, hlc, schema_version, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, datetime('now'))",
        params![
            uuid::Uuid::now_v7().to_string(),
            profile_id,
            hlc.device_id(),
            entity_type,
            entity_id,
            op,
            patch.to_string(),
            hlc.to_string_repr(),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
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
        conn.execute(
            "CREATE TABLE device_identity (
                id INTEGER PRIMARY KEY CHECK (id = 0),
                device_id TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn hlc_next_increments_from_a_fresh_table() {
        let mut conn = setup_conn();
        let mut clock = HlcClock::load(&conn, "device-a".to_string()).unwrap();
        let tx = conn.transaction().unwrap();
        let first = clock.next(&tx).unwrap();
        let second = clock.next(&tx).unwrap();
        tx.commit().unwrap();
        assert!(second > first);
    }

    #[test]
    fn hlc_to_string_repr_matches_doc_format() {
        let hlc = Hlc {
            millis: parse_trigger_millis("2026-07-04T12:10:22.003Z").unwrap(),
            counter: 4,
            device_id: "device".to_string(),
        };
        assert_eq!(hlc.to_string_repr(), "2026-07-04T12:10:22.003Z-0004-device");
    }

    #[test]
    fn hlc_parse_is_inverse_of_to_string_repr() {
        let original = Hlc {
            millis: 1_751_630_000_123,
            counter: 42,
            device_id: "a1b2c3d4-e5f6-47a8-9b1c-000000000000".to_string(),
        };
        assert_eq!(Hlc::parse(&original.to_string_repr()), Some(original));
    }

    #[test]
    fn hlc_parse_rejects_malformed_input() {
        assert_eq!(Hlc::parse(""), None);
        assert_eq!(Hlc::parse("not-an-hlc"), None);
        assert_eq!(Hlc::parse("2026-07-04T12:10:22.003Z-0004-"), None); // пустой device_id
    }

    #[test]
    fn hlc_ordering_respects_millis_then_counter_then_device_id() {
        let base = Hlc {
            millis: 1000,
            counter: 0,
            device_id: "a".to_string(),
        };
        let later_millis = Hlc {
            millis: 2000,
            counter: 0,
            device_id: "a".to_string(),
        };
        let higher_counter = Hlc {
            millis: 1000,
            counter: 1,
            device_id: "a".to_string(),
        };
        let higher_device = Hlc {
            millis: 1000,
            counter: 0,
            device_id: "b".to_string(),
        };

        assert!(later_millis > base);
        assert!(higher_counter > base);
        assert!(higher_device > base);
    }

    #[test]
    fn hlc_clock_persists_and_restores_state_across_reload() {
        let mut conn = setup_conn();

        let mut clock = HlcClock::load(&conn, "device-a".to_string()).unwrap();
        let tx = conn.transaction().unwrap();
        let before_restart = clock.next(&tx).unwrap();
        tx.commit().unwrap();
        drop(clock);

        let mut reloaded = HlcClock::load(&conn, "device-a".to_string()).unwrap();
        let tx = conn.transaction().unwrap();
        let after_restart = reloaded.next(&tx).unwrap();
        tx.commit().unwrap();

        assert!(after_restart > before_restart);
    }

    // Смысл HLC против голого wall-clock: последнее выпущенное время могло
    // оказаться в будущем (сбой часов), а потом ОС скорректировала часы
    // назад — next() не должен выдать значение меньше уже выпущенного.
    #[test]
    fn hlc_clock_next_does_not_regress_if_wall_clock_moved_backward() {
        let mut conn = setup_conn();
        let far_future = now_millis() + 1_000_000_000;
        conn.execute(
            "INSERT INTO sync_clock_state VALUES (0, ?1, 5)",
            params![far_future],
        )
        .unwrap();

        let mut clock = HlcClock::load(&conn, "device-a".to_string()).unwrap();
        let tx = conn.transaction().unwrap();
        let hlc = clock.next(&tx).unwrap();
        tx.commit().unwrap();

        assert_eq!(hlc.millis, far_future);
        assert_eq!(hlc.counter, 6);
    }

    #[test]
    fn device_identity_is_created_once_and_stable_across_reopen() {
        let conn = setup_conn();
        let first = ensure_device_identity(&conn).unwrap();
        let second = ensure_device_identity(&conn).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn record_operation_inserts_a_row_with_expected_fields() {
        let mut conn = setup_conn();
        let mut clock = HlcClock::load(&conn, "device-a".to_string()).unwrap();
        let tx = conn.transaction().unwrap();
        let hlc = clock.next(&tx).unwrap();
        let patch = serde_json::json!({ "status": "done" });
        record_operation(
            &tx,
            &hlc,
            "profile-1",
            "plan_item",
            "item-1",
            "update",
            &patch,
        )
        .unwrap();
        tx.commit().unwrap();

        let (entity_type, entity_id, op, patch_text, device_id): (
            String,
            String,
            String,
            String,
            String,
        ) = conn
            .query_row(
                "SELECT entity_type, entity_id, op, patch, device_id FROM sync_operations",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(entity_type, "plan_item");
        assert_eq!(entity_id, "item-1");
        assert_eq!(op, "update");
        assert_eq!(device_id, "device-a");
        let parsed_patch: serde_json::Value = serde_json::from_str(&patch_text).unwrap();
        assert_eq!(parsed_patch, patch);
    }
}
