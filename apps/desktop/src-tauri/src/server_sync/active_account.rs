//! Account-switch safety for server sync.
//!
//! A sync run captures the active account and a generation number when it starts. Switching
//! or creating an account bumps the generation twice (on start and on finish of the change),
//! so an odd generation means "a change is in progress". Before every write a run checks that
//! the account and the generation are still the ones it started with — otherwise a response
//! for account A could land in account B's vault after a switch.
use crate::profiles;
use std::sync::atomic::{AtomicU64, Ordering};

static ACTIVE_ACCOUNT_GENERATION: AtomicU64 = AtomicU64::new(0);

/// The account a sync run was started for.
pub(super) struct ActiveSyncContext<'a> {
    pub(super) profiles_state: &'a profiles::ProfilesState,
    pub(super) generation: u64,
    pub(super) local_profile_id: &'a str,
    pub(super) remote_profile_id: &'a str,
}

impl ActiveSyncContext<'_> {
    pub(super) fn ensure_active(&self) -> Result<(), String> {
        ensure_active_account(self.profiles_state, self.local_profile_id, self.generation)
    }
}

pub(super) fn current_generation() -> u64 {
    ACTIVE_ACCOUNT_GENERATION.load(Ordering::SeqCst)
}

pub(super) fn ensure_active_account(
    profiles_state: &profiles::ProfilesState,
    expected_profile_id: &str,
    expected_generation: u64,
) -> Result<(), String> {
    let current_generation = current_generation();
    let current_profile_id = profiles::active_profile_id(profiles_state)?;
    if !active_account_matches(
        expected_profile_id,
        expected_generation,
        &current_profile_id,
        current_generation,
    ) {
        return Err("active account changed during sync".to_string());
    }
    Ok(())
}

fn active_account_matches(
    expected_profile_id: &str,
    expected_generation: u64,
    current_profile_id: &str,
    current_generation: u64,
) -> bool {
    expected_generation.is_multiple_of(2)
        && expected_generation == current_generation
        && expected_profile_id == current_profile_id
}

/// Held while the active account changes; dropping it ends the change.
pub struct ActiveAccountChangeGuard;

impl Drop for ActiveAccountChangeGuard {
    fn drop(&mut self) {
        ACTIVE_ACCOUNT_GENERATION.fetch_add(1, Ordering::SeqCst);
        super::SYNC_RERUN_REQUESTED.store(true, Ordering::SeqCst);
    }
}

/// Invalidates every in-flight sync run before the vault changes and asks for a rerun.
pub fn begin_active_account_change() -> ActiveAccountChangeGuard {
    ACTIVE_ACCOUNT_GENERATION.fetch_add(1, Ordering::SeqCst);
    super::SYNC_RERUN_REQUESTED.store(true, Ordering::SeqCst);
    ActiveAccountChangeGuard
}

#[cfg(test)]
mod tests {
    use super::active_account_matches;

    #[test]
    fn sync_context_expires_when_the_active_account_changes() {
        assert!(active_account_matches("account-a", 2, "account-a", 2));
        assert!(!active_account_matches("account-a", 2, "account-b", 2));
        assert!(!active_account_matches("account-a", 2, "account-a", 4));
        assert!(!active_account_matches("account-a", 3, "account-a", 3));
    }
}
