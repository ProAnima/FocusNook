# Code structure

Карта кода FocusNook и правила, куда класть новый код. Правила проверок и бюджеты
размера — в [AGENTS.md](../AGENTS.md); статус релиза — в
[v1-release-plan.md](v1-release-plan.md). Обновляй этот файл, когда меняешь структуру папок.

```text
apps/
  desktop/            Tauri 2 + React/TypeScript; тот же проект собирает Android (src-tauri/gen/android)
    src/              фронтенд
    src-tauri/src/    Rust-ядро (SQLite, синхронизация, окна, команды)
  server/             self-hosted VDS-релей синхронизации (Rust, axum + sqlx + PostgreSQL)
plugins/              собственные Tauri-плагины (reminder-alarm, secure-storage; google-auth — запаркован)
docs/                 планы, контракты, материалы для магазинов
```

## Фронтенд (`apps/desktop/src`)

Поток зависимостей: **компоненты → хуки (`shared/use*.ts`) → `shared/commands/` → Tauri**.

| Путь | Назначение |
|---|---|
| `App.tsx`, `main.tsx` | Оркестрация shell: вкладки, настройки, слой окна. Без логики фич. |
| `components/*.tsx` | Экраны и крупные компоненты (`DayView`, `NotesView`, `RemindersView`, `SettingsPanel`, `AccountGate`, `ReminderAlert`, …). Файл верхнего уровня — тонкая точка входа. |
| `components/settings/` | Секции настроек: `PreferenceSections` (тема, язык, микрофон, папки заметок, автозапуск, горячая клавиша, диагностика), `SelectGroup` (общий выпадающий список), `SyncSection` (сборка блока), `SyncReadinessCard` + `useSyncReadiness`, `AccountSyncToggle`, `ServerAuthForm` / `ServerAccountCard` и хук `useServerAccount` (вход, регистрация, отключение, удаление аккаунта VDS). |
| `components/reminders/` | `ReminderComposer`, `ReminderCustomTimePicker`, `ReminderRow`, `TimeStepper` и чистая логика времени в `reminderTime.ts` (с тестами). |
| `components/notes/` | Части экрана заметок: `NoteFolders` (+ `DesktopFolderRail`, `MobileFolderSheet`, `FolderCreateForm`, `DropFolderButton`), `NoteRow`, `NoteEditor`, `NoteComposer`; модель папок — `folderModel.ts` (с тестами), хуки `useNoteFolderCreate`, `useMobileSheetLock`, `useFolderOptions`. |
| `shared/commands/` | Единственное место, где разрешены `invoke`/`listen`/store/window API. По файлу на область: `planItems`, `notes`, `reminders`, `profiles`, `settings`, `sync`, `serverSync`, `overlay`, `diagnostics`, `legal`; `types.ts` — общие типы; `safeListen.ts` — обёртка `listen`; `index.ts` собирает объект `commands`. |
| `shared/use*.ts` | Хуки состояния: `usePlanItems`, `useNotes`, `useReminders`, `useProfiles`, `useMicrophoneSettings` (на `useMicrophoneDevices`, `useMicrophoneTest`, `microphoneDevices.ts`, `microphoneLevel.ts`), `useReminderAlert`, `useSelectedMicrophone`, `useServerSyncWakeup`, … Общий хук подписки на события — `useEventSubscription`; меню у кнопки — `useAnchoredMenu`; загрузка аудио — `useAudioSource`. |
| `shared/*.ts(x)` без `use` | Чистые утилиты и контексты: локаль (`locale*`), тема (`theme*`, `themeCatalog`), даты (`dateKeys`, `calendarMarks`), напоминания (`reminderPresets`), звук (`playChime`), микрофон (`microphoneDevices`, `microphoneLevel`), координаты курсора. |
| `shared/translations/` | Словари: один файл на язык (`ru`, `en`, `es`, `de`, `fr`, `pt`, `zh`, `ja`, `ko`, `hi`), `index.ts` — `translate`, `LOCALES`; `types.ts`, `makeDictionary.ts` — тип словаря и подстановка английского для отсутствующих ключей. |
| `styles/*.css`, `App.css`, `theme.css` | `App.css` только подключает `theme.css` (токены темы) и файлы из `styles/` в исходном каскадном порядке (порядок важен). |
| `test/setup.ts` | Общая настройка Vitest. |

### Где что лежит при добавлении фичи

1. Команда Rust → метод в подходящем файле `shared/commands/<область>.ts` (типы — в `types.ts`).
2. Состояние/побочные эффекты → хук в `shared/use<Фича>.ts`; повторяющийся паттерн вынеси, а не копируй.
3. Разметка → `components/<фича>/`, тонкая точка входа остаётся в `components/<Фича>.tsx`.
4. Стили → подходящий файл в `styles/` (новый файл подключай в `App.css` в нужном месте каскада).
5. Строки → во все локали `shared/translations/*`.
6. Тесты рядом с кодом (`*.test.ts(x)`).

## Rust-ядро (`apps/desktop/src-tauri/src`)

Зеркало фронтенда: **команда Tauri → доменный модуль**. Обработчики не содержат
бизнес-логики (валидация → вызов домена → типизированный результат).

| Модуль | Назначение |
|---|---|
| `lib.rs`, `main.rs` | Тонкий bootstrap: объявления модулей, `AppState`, инициализация логирования, `run()` с `setup()` и `generate_handler!`. |
| `commands/` | Обработчики `#[tauri::command]` по областям — как `shared/commands/` на фронтенде: `profiles`, `plan_items`, `notes`, `reminders`, `diagnostics`, `overlay`; `mod.rs` — общие хелперы (`trigger_server_sync`, `audio_dir`). Исключение — команды синхронизации: они живут в `server_sync/` рядом со своим протоколом. |
| `shell.rs` | Нативная оболочка окна: слой always-on-top, трей, глобальная горячая клавиша, слежение за положением окна. |
| `db/`, `profiles/` | SQLite-хранилища профилей, ключ vault в OS-хранилище, миграции, SQLCipher; реестр профилей и аккаунтов. |
| `plan_items.rs`, `notes.rs`, `reminders.rs` | Доменные операции задач дня, заметок и напоминаний. |
| `alerts.rs` | Планировщик и окно уведомления о напоминании. |
| `window_state.rs` | Положение окна, мульти-монитор, DPI. |
| `sync_log.rs` | Журнал операций, HLC-часы, идентичность устройства. |
| `sync_snapshot.rs`, `sync_status.rs` | Полный снимок для первичной выгрузки; статус готовности синхронизации. |
| `server_sync/` | Клиент VDS (см. ниже). |
| `sync_blobs.rs`, `blob_crypto.rs`, `audio_crypto.rs` | Очередь вложений (аудио), шифрование вложений и аудио на диске. |
| `android_vault_key.rs` | Ключ vault в Android Keystore (на десктопе — заглушка с той же сигнатурой). |
| `diagnostics.rs` | Экспорт диагностики без пользовательского содержимого. |
| `config.rs` | Локальный `sync_providers.json`: эндпоинт VDS (`server.endpoint`, по умолчанию `https://focus.proanima.net`); OAuth-учётки Google/Yandex читаются только фичей `cloud-providers`. |
| `alarms.rs` | Системные будильники напоминаний (плагин reminder-alarm): постановка, отмена, восстановление после перезагрузки на Android. |
| `cloud_sync.rs`, `oauth.rs`, `sync.rs`, `sync_tokens.rs` | **Запаркованы (пост-v1):** Google Drive / Yandex Disk. Подключаются только фичей `cloud-providers`; UI на них не ссылается. |

### `server_sync/`

| Файл | Назначение |
|---|---|
| `mod.rs` | Константы, статические флаги, реэкспорт по старым путям `crate::server_sync::…`. |
| `credentials.rs` | Учётные данные в keyring, нормализация эндпоинта/токена, `status_for_profile`. |
| `protocol.rs` | DTO запросов/ответов сервера и типы операций. |
| `transport.rs` | HTTP-клиент, обмен операциями, ожидание событий. |
| `account.rs` | Статус, подключение по токену устройства (регистрация устройства), отключение, удаление аккаунта. |
| `account_auth.rs` | Вход и регистрация по e-mail/паролю, включение синхронизации аккаунта (зависит от `account.rs`, не наоборот). |
| `journal.rs`, `payload.rs`, `patch.rs` | Курсор, очередь неотправленных операций, шифрование полезной нагрузки, разбор патчей. |
| `attachments.rs` | Загрузка/выгрузка вложений, статусы передачи. |
| `apply.rs`, `apply_entities.rs` | Применение удалённых операций к локальной БД (задачи, заметки, напоминания), разрешение конфликтов. |
| `engine.rs` | Цикл синхронизации, фоновые задачи, слушатель событий; `FailureStreak` — один `warn` на серию повторяющихся сбоев. |
| `test_support.rs`, `apply/tests.rs` | Только тесты: общая фикстура БД и тесты применения операций. |

### Логирование и отладка

Логи — через `log` + `tauri-plugin-log`: stdout (Logcat на Android) и ротируемый
файл `focusnook.log` в каталоге логов приложения. Уровень по умолчанию — Info
(Debug в debug-сборке), переопределение — переменная `FOCUSNOOK_LOG`. Правило:
в журнал — только идентификаторы и счётчики, не содержимое. Подробнее —
[debugging.md](debugging.md).

## Сервер (`apps/server/src`)

| Путь | Назначение |
|---|---|
| `main.rs`, `config.rs`, `state.rs`, `error.rs` | Запуск, конфигурация, общее состояние, типы ошибок. |
| `routes/mod.rs` | Сборка роутера, health/ready, публичные страницы (`/privacy`, `/terms`). |
| `routes/accounts.rs` | Регистрация, вход, удаление аккаунта, регистрация устройств и токены. |
| `routes/sync.rs`, `routes/events.rs`, `routes/operations.rs` | Обмен операциями (`/v1/sync/exchange`), long-poll пробуждения (`/v1/sync/events`), валидация и хеширование операций. |
| `routes/blobs.rs` | Загрузка/выгрузка вложений с серверной проверкой SHA-256 и защитой от path traversal. |
| `routes/admin.rs`, `admin_web.rs` | Админ-API и консоль мониторинга. |
| `routes/helpers.rs` | Валидация идентификаторов, `client_ip` (предпочитает `X-Real-IP`; без него — первый адрес `X-Forwarded-For`), учёт трафика. |
| `account_auth.rs`, `auth.rs`, `crypto.rs` | Argon2 (в `spawn_blocking`), HMAC-токены, шифрование данных «в покое». |
| `legal.rs` | Тексты политики конфиденциальности и соглашения. |
| `sync_events.rs` | Шина событий пробуждения устройств. |
| `migrations/` | SQL-миграции PostgreSQL. |
