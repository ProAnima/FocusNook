use crate::error::{AppError, AppResult};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const LOGIN_WINDOW: Duration = Duration::from_secs(60 * 10);
const REGISTER_WINDOW: Duration = Duration::from_secs(60 * 60);
const LOCKOUT: Duration = Duration::from_secs(60 * 15);
const MAX_ATTEMPTS: u32 = 8;
const MAX_REGISTRATIONS: u32 = 6;
const MIN_PASSWORD_LEN: usize = 10;
const MAX_PASSWORD_LEN: usize = 256;

#[derive(Default)]
pub struct AccountAuthState {
    attempts: Mutex<HashMap<String, LoginAttempt>>,
    registrations: Mutex<HashMap<String, LoginAttempt>>,
}

#[derive(Clone)]
struct LoginAttempt {
    count: u32,
    first_seen: Instant,
    locked_until: Option<Instant>,
}

impl Default for LoginAttempt {
    fn default() -> Self {
        Self {
            count: 0,
            first_seen: Instant::now(),
            locked_until: None,
        }
    }
}

impl AccountAuthState {
    pub fn ensure_not_locked(&self, ip: &str, email: &str) -> AppResult<()> {
        self.prune();
        let attempts = self
            .attempts
            .lock()
            .map_err(|_| AppError::Internal("account attempt lock failed".to_string()))?;
        let key = attempt_key(ip, email);
        if attempts
            .get(&key)
            .and_then(|attempt| attempt.locked_until)
            .is_some_and(|until| until > Instant::now())
        {
            Err(AppError::TooManyRequests(
                "too many sign-in attempts, try later".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn record_failure(&self, ip: &str, email: &str) -> AppResult<()> {
        let mut attempts = self
            .attempts
            .lock()
            .map_err(|_| AppError::Internal("account attempt lock failed".to_string()))?;
        let attempt = attempts.entry(attempt_key(ip, email)).or_default();
        if Instant::now().duration_since(attempt.first_seen) > LOGIN_WINDOW {
            *attempt = LoginAttempt::default();
        }
        attempt.count += 1;
        if attempt.count >= MAX_ATTEMPTS {
            attempt.locked_until = Some(Instant::now() + LOCKOUT);
        }
        Ok(())
    }

    pub fn record_registration(&self, ip: &str) -> AppResult<()> {
        self.prune();
        let mut registrations = self
            .registrations
            .lock()
            .map_err(|_| AppError::Internal("account registration lock failed".to_string()))?;
        let attempt = registrations.entry(ip.to_string()).or_default();
        if Instant::now().duration_since(attempt.first_seen) > REGISTER_WINDOW {
            *attempt = LoginAttempt::default();
        }
        if attempt
            .locked_until
            .is_some_and(|until| until > Instant::now())
        {
            return Err(AppError::TooManyRequests(
                "too many registrations, try later".to_string(),
            ));
        }
        attempt.count += 1;
        if attempt.count > MAX_REGISTRATIONS {
            attempt.locked_until = Some(Instant::now() + LOCKOUT);
            return Err(AppError::TooManyRequests(
                "too many registrations, try later".to_string(),
            ));
        }
        Ok(())
    }

    pub fn clear_failures(&self, ip: &str, email: &str) -> AppResult<()> {
        let mut attempts = self
            .attempts
            .lock()
            .map_err(|_| AppError::Internal("account attempt lock failed".to_string()))?;
        attempts.remove(&attempt_key(ip, email));
        Ok(())
    }

    fn prune(&self) {
        let now = Instant::now();
        if let Ok(mut attempts) = self.attempts.lock() {
            attempts.retain(|_, attempt| {
                attempt.locked_until.is_some_and(|until| until > now)
                    || now.duration_since(attempt.first_seen) <= LOGIN_WINDOW
            });
        }
        if let Ok(mut registrations) = self.registrations.lock() {
            registrations.retain(|_, attempt| {
                attempt.locked_until.is_some_and(|until| until > now)
                    || now.duration_since(attempt.first_seen) <= REGISTER_WINDOW
            });
        }
    }
}

pub fn normalize_email(raw: &str) -> AppResult<String> {
    let email = raw.trim().to_ascii_lowercase();
    let parts = email.split('@').collect::<Vec<_>>();
    if email.len() > 254
        || parts.len() != 2
        || parts[0].is_empty()
        || parts[1].is_empty()
        || !parts[1].contains('.')
        || email.chars().any(char::is_whitespace)
    {
        return Err(AppError::BadRequest("email is invalid".to_string()));
    }
    Ok(email)
}

pub fn validate_password(password: &str) -> AppResult<()> {
    let len = password.chars().count();
    if !(MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&len) {
        return Err(AppError::BadRequest(format!(
            "password must be {MIN_PASSWORD_LEN}-{MAX_PASSWORD_LEN} characters"
        )));
    }
    let has_letter = password.chars().any(char::is_alphabetic);
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !has_letter || !has_digit {
        return Err(AppError::BadRequest(
            "password must contain letters and digits".to_string(),
        ));
    }
    Ok(())
}

pub fn hash_password(password: &str) -> AppResult<String> {
    validate_password(password)?;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::Internal("password hashing failed".to_string()))
}

pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(hash)
        .map_err(|_| AppError::Internal("stored password hash is invalid".to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

fn attempt_key(ip: &str, email: &str) -> String {
    format!("{ip}|{email}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_email() -> AppResult<()> {
        assert_eq!(
            normalize_email(" User@Example.COM ")?.as_str(),
            "user@example.com"
        );
        assert!(normalize_email("not-an-email").is_err());
        Ok(())
    }

    #[test]
    fn password_round_trips() -> AppResult<()> {
        let hash = hash_password("StrongPass123")?;
        assert!(verify_password("StrongPass123", &hash)?);
        assert!(!verify_password("wrongStrong123", &hash)?);
        Ok(())
    }

    #[test]
    fn limits_registration_bursts() -> AppResult<()> {
        let state = AccountAuthState::default();
        for _ in 0..MAX_REGISTRATIONS {
            state.record_registration("127.0.0.1")?;
        }
        assert!(state.record_registration("127.0.0.1").is_err());
        assert!(state.record_registration("127.0.0.2").is_ok());
        Ok(())
    }
}
