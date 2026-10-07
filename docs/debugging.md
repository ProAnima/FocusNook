# Разработка и отладка

Практическая шпаргалка: как запустить, где смотреть логи и данные, как проверить
синхронизацию. Правила качества — в [AGENTS.md](../AGENTS.md), карта кода — в
[code-structure.md](code-structure.md).

## Быстрый старт

| Что нужно | Команда |
|---|---|
| Все проверки одной командой | `scripts/check.ps1` (`-Only frontend`, `-Only desktop-rust,server`, `-KeepGoing`, `-Build`) |
| Отформатировать Rust | `cargo fmt` в `apps/desktop/src-tauri` и `apps/server` |
| Только фронтенд в браузере (без нативных функций) | `cd apps/desktop && npm run dev` → `http://localhost:1420` |
| Полное desktop-приложение | `cd apps/desktop && npm run tauri dev` |
| Тесты фронтенда в режиме наблюдения | `cd apps/desktop && npx vitest` |
| Один Rust-тест | `cd apps/desktop/src-tauri && cargo test <часть_имени>` |

Для сборки desktop-crate нужен полноценный Perl (SQLCipher + vendored OpenSSL).
`scripts/check.ps1` ищет его так: переменная `FOCUSNOOK_PERL` (каталог с `perl.exe`),
`C:\Strawberry\perl\bin`, `%USERPROFILE%\Tools\perl\perl\bin`, затем `perl` из `PATH`
(MSYS Perl из Git отбрасывается). Если Perl стоит в другом месте, задайте переменную
один раз: `setx FOCUSNOOK_PERL D:\Tools\perl\perl\bin` (и перезапустите терминал).
Для ручных `cargo`-команд добавьте этот каталог в `PATH` на время сессии.
Первая сборка занимает ~10+ минут (компиляция OpenSSL), дальше — инкрементально;
не делайте `cargo clean` без необходимости.

## Где что лежит на диске (Windows)

Данные приложения — `app_data_dir` Tauri: `%APPDATA%\com.proanima.focusnook\`.

- профили и их SQLite-хранилища (SQLCipher-шифрование), каталог аудио;
- `sync_providers.json` — необязательный локальный конфиг; для отладки на своём
  сервере: `{ "server": { "endpoint": "http://localhost:8080" } }`.
  Эндпоинт должен быть `https://`, исключение — `http://localhost` и
  `http://127.0.0.1` (для разработки);
- положение окна и настройки.

Чтобы получить «чистую» установку для проверки, переименуйте этот каталог (не
удаляйте — там данные профилей).

## Логи

- **Десктоп:** `log` + `tauri-plugin-log`. Пишется в stdout (при `npm run tauri dev`
  это консоль; на Android — Logcat) и в ротируемый файл `focusnook.log` в каталоге
  логов приложения (Windows: `%LOCALAPPDATA%\com.proanima.focusnook\logs`; текущий
  файл и до 5 старых по ~1 МБ). Цель записи — путь модуля (`desktop_lib::alerts`, …).
  Уровень по умолчанию Info (в debug-сборке Debug); поднять: запуск с
  `FOCUSNOOK_LOG=debug` (или `trace`; на Android переменная недоступна — действует
  уровень сборки). Шумные зависимости ограничены уровнем Warn.
  В лог попадают только идентификаторы, счётчики и длительности — не содержимое
  задач/заметок/напоминаний, не токены и не пароли. Текст ошибок (`{err}`) может
  содержать адрес сервера синхронизации; перед отправкой лога в поддержку
  просмотрите его.
- **Диагностика для обращения в поддержку:** *Настройки → Экспортировать
  диагностику* сохраняет JSON без пользовательского содержимого (версия,
  платформа, счётчики, хеш профиля, давность опроса планировщика).
- **Сервер:** `tracing`; уровень задаётся `RUST_LOG`
  (по умолчанию `focusnook_sync_server=info,tower_http=info`), логи сервера пишутся
  в формате JSON (`tracing_subscriber::fmt().json()`). В продакшне — `docker compose logs server`.
- **Фронтенд:** обычная консоль WebView. В dev-режиме откройте DevTools (в
  dev-сборке Tauri они доступны из контекстного меню окна). Вызовы в Rust видны как `invoke` с именем команды —
  имена перечислены в `apps/desktop/src/shared/commands/`.

### Полезные сообщения в логе

- `sync cycle: завершён за N мс` (debug) — цикл синхронизации прошёл;
  `sync exchange: отправлено операций X, получено Y` (debug) — раунды обмена.
- `blob upload: загружено …, отложено …` / `blob download: скачано …, недоступно …` —
  состояние передачи вложений; `attachment <id> is waiting for a media key` —
  ждём ключ медиа (нужен повторный вход, см. восстановление ключа медиа в настройках).
- `активный профиль переключён` — смена профиля.
- `best-effort sync failed: …` / `event listener failed: …` (warn) — первый сбой серии
  (например, пропала сеть); повторы той же серии пишутся как debug, а
  `…: восстановлено после N неудачных попыток` (info) отмечает конец серии.

## Как искать причину проблемы

1. Воспроизведите в браузерном превью (`npm run dev`) — если ошибка там же,
   причина во фронтенде/хуках; если нет — в нативной части.
2. Определите слой по симптому:
   - **UI не обновился после действия** — смотрите хук в `shared/use*.ts` и
     событие, на которое он подписан (`useEventSubscription`);
   - **команда вернула ошибку** — имя команды → файл в `shared/commands/` →
     одноимённая `#[tauri::command]` в Rust → доменный модуль;
   - **данные не доехали на другое устройство** — журнал операций
     (`sync_log.rs`), готовность синхронизации в настройках (число операций,
     хеш устройства), затем сервер.
3. Для проверки синхронизации без реальных устройств используйте два профиля
   или два каталога данных и локальный сервер (ниже). Автотест передачи аудио между двумя
   устройствами с разными ключами хранилища — `sync_blobs.rs`
   (`audio_round_trips_between_devices_with_distinct_local_keys`).

## Локальный сервер синхронизации

Нужны Docker и `openssl` (в Git Bash есть):

```bash
cd apps/server
FOCUSNOOK_LEGAL_NAME=dev FOCUSNOOK_SUPPORT_EMAIL=dev@localhost sh scripts/generate-env.sh localhost
docker compose --env-file .env up -d --build postgres server
```

`compose.yml` уже публикует сервер на `127.0.0.1:8080`; Caddy (порты 80/443) для
локальной отладки не нужен, поэтому запускаются только `postgres` и `server`.
Укажите клиенту `http://localhost:8080` через `sync_providers.json`. Без
`FOCUSNOOK_LEGAL_NAME` и `FOCUSNOOK_SUPPORT_EMAIL` регистрация аккаунтов на сервере
отключена (ответ 404). Эндпоинты `/healthz` и `/readyz` отвечают без авторизации.
Подробности и переменные окружения — в [apps/server/README.md](../apps/server/README.md).

Серверные тесты (`cargo test` в `apps/server`) не требуют живой базы.

## Типичные проблемы

| Симптом | Причина / решение |
|---|---|
| `Command 'perl' not found` / `IPC::Cmd` при сборке desktop | Нет полноценного Perl; Perl из Git for Windows не подходит. Поставьте Strawberry Perl. |
| `cargo-clippy.exe is not installed` / `cargo-fmt` не найден | `rustup component add clippy rustfmt` |
| Лог-файла нет | Проверьте каталог логов (см. «Логи»); файл создаётся при запуске собранного приложения. Лог-файл и Logcat пока не проверялись на живом приложении/устройстве. |
| `frontendDist ../dist doesn't exist` при `cargo` | Один раз выполните `npm run build` в `apps/desktop` (или создайте пустой `dist/index.html`). |
| Git показывает изменёнными снапшоты или `plugins/*/permissions` без реальных правок | Это окончания строк. `.gitattributes` фиксирует для них LF; если файлы были извлечены до него — удалите их и выполните `git checkout -- <путь>`. |
