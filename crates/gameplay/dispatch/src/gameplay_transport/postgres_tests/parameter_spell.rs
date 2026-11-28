//! PARAM-CAST-WIRE-1: admitted CHAT_V1 speech reaches the existing spell owner.
#![allow(clippy::expect_used)]
use super::parameter_spell_fixtures::scenario;

use super::TestResult;

#[test]
fn parameter_spell_online_exiva_real_session_postgres() -> TestResult {
    scenario(0)
}
#[test]
fn parameter_spell_offline_and_unknown_exiva_real_session_postgres() -> TestResult {
    scenario(1)
}
#[test]
fn parameter_spell_ordinary_cast_unchanged_real_session_postgres() -> TestResult {
    scenario(2)
}

#[test]
fn parameter_spell_protected_exiva_real_session_postgres() -> TestResult {
    scenario(3)
}

#[test]
fn parameter_spell_action_bar_named_target_real_session_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(255)
}

#[test]
fn parameter_spell_unsaved_party_preferences_real_dispatch_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(super::parameter_spell_fixtures::UNSAVED_PARTY_CASE)
}

#[path = "parameter_spell/exiva_settings.rs"]
mod exiva_settings;
