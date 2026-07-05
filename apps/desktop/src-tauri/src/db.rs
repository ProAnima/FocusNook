use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

// Раздел 9 ТЗ: локальная база на профиль. Пока нет multi-account — один файл
// на дефолтный профиль; per-profile пути придут вместе с Iteration 1.
//
// Шифрование (SQLCipher) временно отключено: ключ через OS keychain
// (crate `keyring`) не переживает перезапуск процесса в текущем окружении
// разработки (воспроизведено дважды, Windows Credential Manager не
// сохраняет запись между запусками) — см. AGENTS.md/архитектурный документ,
// раздел 26, для деталей и плана возврата к шифрованию.
pub struct Db(pub Mutex<Connection>);

const MIGRATIONS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS plan_items (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    progress_percent INTEGER,
    created_at TEXT NOT NULL
)",
    "CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    title TEXT,
    body TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'text',
    created_at TEXT NOT NULL
)",
    "CREATE TABLE IF NOT EXISTS reminders (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    trigger_at_utc TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'scheduled',
    created_at TEXT NOT NULL
)",
];

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    for migration in MIGRATIONS {
        conn.execute(migration, [])?;
    }
    Ok(conn)
}
