use crate::config::ProviderCredentials;
use crate::sync_tokens;
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::url::Url;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
#[cfg(target_os = "android")]
use tauri_plugin_google_auth::{GoogleAuthExt, TokenRequest};
use tauri_plugin_opener::OpenerExt;

// Раздел 14 ТЗ, sync — только два провайдера ожидаются за весь MVP (плюс,
// возможно, VDS позже — раздел 22), а OpenClaw-режим sync уже помечен в
// разделе 14 как "под вопросом после спайка". Enum + match, а не trait object:
// (1) async fn в trait не dyn-compatible без крейта async_trait — четвёртая
// зависимость ради полиморфизма, которым больше нигде в проекте не пользуются
// (plan_items.rs/notes.rs/reminders.rs — свободные функции, не trait objects);
// (2) match на call site даёт компилятору проверку "оба варианта обработаны",
// которую trait object не даёт.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    GoogleDrive,
    YandexDisk,
}

impl ProviderId {
    fn auth_url(self) -> &'static str {
        match self {
            ProviderId::GoogleDrive => "https://accounts.google.com/o/oauth2/v2/auth",
            ProviderId::YandexDisk => "https://oauth.yandex.ru/authorize",
        }
    }

    fn token_url(self) -> &'static str {
        match self {
            ProviderId::GoogleDrive => "https://oauth2.googleapis.com/token",
            ProviderId::YandexDisk => "https://oauth.yandex.ru/token",
        }
    }

    // appDataFolder / app_folder — песочница провайдера, не весь диск.
    pub(crate) fn scope(self) -> &'static str {
        match self {
            ProviderId::GoogleDrive => "https://www.googleapis.com/auth/drive.appdata",
            ProviderId::YandexDisk => "cloud_api:disk.app_folder",
        }
    }

    pub(crate) fn keyring_prefix(self) -> &'static str {
        match self {
            ProviderId::GoogleDrive => "google_drive",
            ProviderId::YandexDisk => "yandex_disk",
        }
    }
}

type ConfiguredClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

// auth_url/token_url — отдельные параметры, а не ProviderId напрямую: так
// функция тестируется против локального мок-сервера, не только против
// настоящих Google/Yandex хостов.
fn build_client(
    auth_url: &str,
    token_url: &str,
    client_id: &str,
    client_secret: Option<&str>,
    redirect_uri: &str,
) -> Result<ConfiguredClient, String> {
    let auth_url = AuthUrl::new(auth_url.to_string()).map_err(|e| e.to_string())?;
    let token_url = TokenUrl::new(token_url.to_string()).map_err(|e| e.to_string())?;
    let redirect_url = RedirectUrl::new(redirect_uri.to_string()).map_err(|e| e.to_string())?;

    let mut client = BasicClient::new(ClientId::new(client_id.to_string()))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect_url);
    if let Some(secret) = client_secret {
        client = client.set_client_secret(ClientSecret::new(secret.to_string()));
    }
    Ok(client)
}

// Разбор redirect-запроса на code/state — чистая функция, тестируется без
// реального HTTP-сервера. path_and_query — то, что отдаёт tiny_http::Request::url()
// (путь + query string, без схемы/хоста).
fn parse_callback(path_and_query: &str) -> Result<(String, String), String> {
    let full_url = format!("http://127.0.0.1{path_and_query}");
    let parsed = Url::parse(&full_url).map_err(|e| e.to_string())?;
    let mut code = None;
    let mut state = None;
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            _ => {}
        }
    }
    let code = code.ok_or_else(|| "в редиректе нет кода авторизации".to_string())?;
    let state = state.ok_or_else(|| "в редиректе нет state".to_string())?;
    Ok((code, state))
}

// CSRF-проверка — обязательна до обмена кода на токен, иначе вся защита PKCE
// от перехвата кода теряет смысл. Отдельная функция, а не инлайн в run_flow,
// чтобы это конкретное свойство было прямо протестировано, а не только
// подразумевалось happy-path тестами.
fn verify_state(received: &str, expected: &CsrfToken) -> Result<(), String> {
    if received == expected.secret().as_str() {
        Ok(())
    } else {
        Err("state не совпадает — возможная CSRF-атака, авторизация отменена".to_string())
    }
}

async fn exchange_code_for_tokens(
    client: &ConfiguredClient,
    code: String,
    pkce_verifier: PkceCodeVerifier,
) -> Result<BasicTokenResponse, String> {
    let http_client = reqwest::Client::new();
    client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
        .map_err(|e| e.to_string())
}

// Раздел 14 ТЗ: полный loopback+PKCE флоу. tiny_http::Server::recv блокирует
// реальный OS-поток, поэтому идёт через spawn_blocking — тот же класс заботы
// об исполнителе Tokio, что и про std::sync::MutexGuard через await у
// sync_log::HlcClockState (здесь эта функция вообще не трогает db::Db/
// HlcClockState, поэтому то конкретное ограничение неприменимо напрямую, но
// принцип "не блокировать executor" тот же).
pub async fn run_flow(
    app: &tauri::AppHandle,
    provider: ProviderId,
    creds: &ProviderCredentials,
    profile_id: &str,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    if provider == ProviderId::GoogleDrive {
        let token = app
            .google_auth()
            .connect(TokenRequest {
                scope: provider.scope().to_string(),
            })
            .map_err(|e| e.to_string())?;
        if token.access_token.is_empty() {
            return Err("Google account connected but no access token was returned".to_string());
        }
        return Ok(());
    }

    let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = server
        .server_addr()
        .to_ip()
        .ok_or_else(|| "не удалось определить локальный порт".to_string())?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    let client = build_client(
        provider.auth_url(),
        provider.token_url(),
        &creds.client_id,
        creds.client_secret.as_deref(),
        &redirect_uri,
    )?;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    // access_type=offline + prompt=consent (Google-специфика, у Yandex это
    // просто игнорируемые лишние параметры): без них Google отдаёт
    // refresh-токен только при самой первой авторизации аккаунта под этим
    // client_id — при повторной (например, после переустановки) токен для
    // обновления не пришёл бы.
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(provider.scope().to_string()))
        .set_pkce_challenge(pkce_challenge)
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .url();

    app.opener()
        .open_url(auth_url.to_string(), None::<&str>)
        .map_err(|e| e.to_string())?;

    let (code, state) = tokio::task::spawn_blocking(move || -> Result<(String, String), String> {
        let request = server
            .recv_timeout(std::time::Duration::from_secs(300))
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "истекло время ожидания авторизации".to_string())?;
        let result = parse_callback(request.url());
        let _ = request.respond(tiny_http::Response::from_string(
            "Готово, можно закрыть эту вкладку и вернуться в FocusNook.",
        ));
        result
    })
    .await
    .map_err(|e| e.to_string())??;

    verify_state(&state, &csrf_token)?;

    let token_response = exchange_code_for_tokens(&client, code, pkce_verifier).await?;
    let refresh_token = token_response
        .refresh_token()
        .ok_or_else(|| "провайдер не выдал refresh-токен".to_string())?;
    sync_tokens::store_refresh_token(profile_id, provider, refresh_token.secret())?;

    Ok(())
}

// Обновляет access-токен через сохранённый refresh-токен при каждом
// использовании, без кеширования expiry — оптимизация имеет смысл только
// когда появится реальный повторяющийся push/pull-цикл (которого в этом шаге
// ещё нет), а раздел 20 ТЗ и так предполагает пакетную, не поштучную
// синхронизацию, так что цена лишнего round-trip на пакет амортизируется.
//
// ВАЖНО для будущего шага (push/pull): возвращает голую строку токена, без
// заголовка — Yandex ожидает "Authorization: OAuth <token>" (буквально слово
// "OAuth", не "Bearer"), в отличие от Google. Собирать заголовок — забота
// вызывающего кода, конкретного для провайдера.
// Отдельная функция от ensure_valid_token ниже — так же, как
// exchange_code_for_tokens отделена от run_flow: тестируется против мок
// token_url напрямую, без привязки к реальному Google/Yandex хосту.
async fn refresh_access_token(
    client: &ConfiguredClient,
    refresh_token: String,
) -> Result<String, String> {
    let http_client = reqwest::Client::new();
    let token_response = client
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token))
        .request_async(&http_client)
        .await
        .map_err(|e| e.to_string())?;
    Ok(token_response.access_token().secret().clone())
}

// Пока не вызывается из команд этого шага (нечего обновлять токеном без
// push/pull, которых здесь ещё нет) — понадобится следующему шагу перед
// каждым запросом к API провайдера. Покрыта тестом уже сейчас, а не отложена
// вместе с остальным будущим функционалом (тот же принцип, что и у
// sync_log.rs::Hlc::parse).
#[allow(dead_code)]
pub async fn ensure_valid_token(
    _app: &tauri::AppHandle,
    provider: ProviderId,
    profile_id: &str,
    creds: &ProviderCredentials,
) -> Result<String, String> {
    #[cfg(target_os = "android")]
    if provider == ProviderId::GoogleDrive {
        let token = _app
            .google_auth()
            .access_token(TokenRequest {
                scope: provider.scope().to_string(),
            })
            .map_err(|e| e.to_string())?;
        if token.access_token.is_empty() {
            return Err("Google account did not return an access token".to_string());
        }
        return Ok(token.access_token);
    }

    let Some(refresh_token) = sync_tokens::load_refresh_token(profile_id, provider)? else {
        return Err("провайдер не подключён".to_string());
    };
    let client = build_client(
        provider.auth_url(),
        provider.token_url(),
        &creds.client_id,
        creds.client_secret.as_deref(),
        // redirect_uri не используется для refresh-запроса, но типаж требует
        // валидный URL — тот же placeholder, реальный редирект здесь не участвует.
        "http://127.0.0.1:0/callback",
    )?;
    refresh_access_token(&client, refresh_token).await
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn provider_urls_and_scopes_are_distinct_and_correct() {
        assert_ne!(
            ProviderId::GoogleDrive.auth_url(),
            ProviderId::YandexDisk.auth_url()
        );
        assert_ne!(
            ProviderId::GoogleDrive.token_url(),
            ProviderId::YandexDisk.token_url()
        );
        assert!(ProviderId::GoogleDrive.auth_url().contains("google"));
        assert!(ProviderId::YandexDisk.auth_url().contains("yandex"));
        assert_eq!(
            ProviderId::GoogleDrive.scope(),
            "https://www.googleapis.com/auth/drive.appdata"
        );
        assert_eq!(ProviderId::YandexDisk.scope(), "cloud_api:disk.app_folder");
        assert_ne!(
            ProviderId::GoogleDrive.keyring_prefix(),
            ProviderId::YandexDisk.keyring_prefix()
        );
    }

    #[test]
    fn build_client_accepts_missing_client_secret() {
        assert!(build_client(
            "https://example.com/auth",
            "https://example.com/token",
            "id",
            None,
            "http://127.0.0.1:1/cb"
        )
        .is_ok());
    }

    #[test]
    fn parse_callback_extracts_code_and_state() {
        let (code, state) = parse_callback("/callback?code=abc123&state=xyz789").unwrap();
        assert_eq!(code, "abc123");
        assert_eq!(state, "xyz789");
    }

    #[test]
    fn parse_callback_rejects_missing_code() {
        assert!(parse_callback("/callback?state=xyz789").is_err());
    }

    #[test]
    fn parse_callback_rejects_missing_state() {
        assert!(parse_callback("/callback?code=abc123").is_err());
    }

    #[test]
    fn verify_state_accepts_matching_state() {
        let token = CsrfToken::new("matching-state".to_string());
        assert!(verify_state("matching-state", &token).is_ok());
    }

    // Это не просто happy-path — это единственный тест, напрямую проверяющий
    // защиту от CSRF/перехвата кода, ради которой state-параметр вообще
    // существует в OAuth2.
    #[test]
    fn verify_state_rejects_mismatched_state() {
        let token = CsrfToken::new("expected-state".to_string());
        assert!(verify_state("attacker-supplied-state", &token).is_err());
    }

    // Мок и authorization, и token эндпоинтов на локальном tiny_http —
    // проверяет обмен кода на токен без обращения к настоящим Google/Yandex
    // хостам. Сам браузер/redirect здесь не участвует (это run_flow целиком,
    // не тестируется без реального AppHandle) — только код+PKCE-verifier -> токен.
    #[tokio::test]
    async fn exchange_code_for_tokens_parses_a_realistic_response() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        let token_url = format!("http://127.0.0.1:{port}/token");

        let handle = std::thread::spawn(move || {
            let request = server.recv().unwrap();
            let body = r#"{"access_token":"mock-access","refresh_token":"mock-refresh","token_type":"bearer","expires_in":3600}"#;
            let response = tiny_http::Response::from_string(body).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap(),
            );
            request.respond(response).unwrap();
        });

        let client = build_client(
            "https://example.com/auth",
            &token_url,
            "id",
            Some("secret"),
            "http://127.0.0.1:1/cb",
        )
        .unwrap();
        let (_pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let token_response =
            exchange_code_for_tokens(&client, "mock-code".to_string(), pkce_verifier)
                .await
                .unwrap();

        assert_eq!(token_response.access_token().secret(), "mock-access");
        assert_eq!(
            token_response.refresh_token().unwrap().secret(),
            "mock-refresh"
        );
        assert_eq!(
            token_response.expires_in(),
            Some(std::time::Duration::from_secs(3600))
        );

        handle.join().unwrap();
    }

    #[tokio::test]
    async fn refresh_access_token_parses_a_realistic_response() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        let token_url = format!("http://127.0.0.1:{port}/token");

        let handle = std::thread::spawn(move || {
            let request = server.recv().unwrap();
            let body = r#"{"access_token":"fresh-access","token_type":"bearer","expires_in":3600}"#;
            let response = tiny_http::Response::from_string(body).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap(),
            );
            request.respond(response).unwrap();
        });

        let client = build_client(
            "https://example.com/auth",
            &token_url,
            "id",
            Some("secret"),
            "http://127.0.0.1:1/cb",
        )
        .unwrap();
        let access_token = refresh_access_token(&client, "stored-refresh".to_string())
            .await
            .unwrap();

        assert_eq!(access_token, "fresh-access");
        handle.join().unwrap();
    }
}
