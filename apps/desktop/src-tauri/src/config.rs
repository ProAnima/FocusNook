use serde::Deserialize;
use std::fs;
use std::path::Path;

// Раздел 14 ТЗ, sync — OAuth client_id/secret регистрирует владелец продукта
// сам (Google Cloud Console / oauth.yandex.ru), не хардкод в исходниках и не
// в git (см. .gitignore). Отсутствие файла или отдельного провайдера в нём —
// нормальное состояние для всех, кто ещё не настроил sync, а не ошибка:
// load() поэтому возвращает пустую структуру, а не Result. Общий для всех
// профилей файл (не per-profile) — client_id/secret это регистрация OAuth-
// приложения, общая на инсталляцию, а не что-то по своей природе привязанное
// к конкретному профилю (в отличие от refresh-токенов, см. sync_tokens.rs).
const CONFIG_FILENAME: &str = "sync_providers.json";
pub const DEFAULT_SERVER_ENDPOINT: &str = "https://focus.proanima.net";

#[derive(Clone)]
pub struct ProviderCredentials {
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Default)]
pub struct SyncProvidersConfig {
    pub google: Option<ProviderCredentials>,
    pub server: Option<ServerSyncBootstrap>,
    pub yandex: Option<ProviderCredentials>,
}

#[derive(Clone)]
pub struct ServerSyncBootstrap {
    pub endpoint: String,
    pub user_token: Option<String>,
}

#[derive(Deserialize)]
struct RawCredentials {
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "clientSecret")]
    client_secret: Option<String>,
}

#[derive(Deserialize)]
struct RawServerSyncBootstrap {
    endpoint: Option<String>,
    #[serde(rename = "userToken")]
    user_token: Option<String>,
}

impl From<RawServerSyncBootstrap> for ServerSyncBootstrap {
    fn from(raw: RawServerSyncBootstrap) -> Self {
        ServerSyncBootstrap {
            endpoint: raw
                .endpoint
                .unwrap_or_else(|| DEFAULT_SERVER_ENDPOINT.to_string()),
            user_token: raw.user_token,
        }
    }
}

impl From<RawCredentials> for ProviderCredentials {
    fn from(raw: RawCredentials) -> Self {
        ProviderCredentials {
            client_id: raw.client_id,
            client_secret: raw.client_secret,
        }
    }
}

#[derive(Deserialize, Default)]
struct RawConfig {
    google: Option<RawCredentials>,
    server: Option<RawServerSyncBootstrap>,
    yandex: Option<RawCredentials>,
}

// Ни один из вариантов отказа (файла нет / JSON битый) не считается ошибкой,
// требующей всплытия наверх — это тот же принцип, что уже применялся в
// diagnostics.rs: не заполнять функциональность выдумкой, а честно вернуть
// "ничего не настроено" там, где это правда.
pub fn load(data_dir: &Path) -> SyncProvidersConfig {
    let Ok(raw) = fs::read_to_string(data_dir.join(CONFIG_FILENAME)) else {
        return SyncProvidersConfig::default();
    };
    let Ok(parsed) = serde_json::from_str::<RawConfig>(&raw) else {
        return SyncProvidersConfig::default();
    };
    SyncProvidersConfig {
        google: parsed.google.map(ProviderCredentials::from),
        server: parsed.server.map(ServerSyncBootstrap::from),
        yandex: parsed.yandex.map(ProviderCredentials::from),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::io::Write;

    fn temp_dir() -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("focusnook-config-test-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_config(dir: &Path, contents: &str) {
        let mut file = fs::File::create(dir.join(CONFIG_FILENAME)).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn missing_file_returns_empty_config() {
        let dir = temp_dir();
        let config = load(&dir);
        assert!(config.google.is_none());
        assert!(config.server.is_none());
        assert!(config.yandex.is_none());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn malformed_json_returns_empty_config_not_a_panic() {
        let dir = temp_dir();
        write_config(&dir, "not json at all");
        let config = load(&dir);
        assert!(config.google.is_none());
        assert!(config.yandex.is_none());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn partial_config_leaves_the_other_provider_unset() {
        let dir = temp_dir();
        write_config(
            &dir,
            r#"{"google": {"clientId": "abc", "clientSecret": "def"}}"#,
        );
        let config = load(&dir);
        assert!(config.google.is_some());
        assert!(config.server.is_none());
        assert!(config.yandex.is_none());
        assert_eq!(config.google.unwrap().client_id, "abc");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn client_secret_is_optional_within_a_configured_provider() {
        let dir = temp_dir();
        write_config(&dir, r#"{"yandex": {"clientId": "xyz"}}"#);
        let config = load(&dir);
        let yandex = config.yandex.unwrap();
        assert_eq!(yandex.client_id, "xyz");
        assert_eq!(yandex.client_secret, None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_server_sync_bootstrap_without_committing_it() {
        let dir = temp_dir();
        write_config(
            &dir,
            r#"{"server": {"endpoint": "https://focus.proanima.net", "userToken": "fnk_user_secret"}}"#,
        );
        let config = load(&dir);
        let server = config.server.unwrap();
        assert_eq!(server.endpoint, "https://focus.proanima.net");
        assert_eq!(server.user_token.as_deref(), Some("fnk_user_secret"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn server_sync_endpoint_has_a_product_default() {
        let dir = temp_dir();
        write_config(&dir, r#"{"server": {}}"#);
        let config = load(&dir);
        let server = config.server.unwrap();
        assert_eq!(server.endpoint, DEFAULT_SERVER_ENDPOINT);
        assert!(server.user_token.is_none());
        fs::remove_dir_all(&dir).unwrap();
    }
}
