//! Crystal 201189f8, the_order_of_lion_quest/creaturescripts_commander_kills.lua:8-50
//! and action_drume.lua:1-18. The fixture supplies a translated battlefield.
use super::TestResult;
use super::drume_battlefield_fixtures::ExitCase;
use super::engenc2_cults_fixtures::encounter_scenario;

#[test]
fn drume_final_usurper_death_spawns_boss_and_kesar_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    encounter_scenario(false, Some(ExitCase::Timeout))
}

#[test]
fn drume_final_lion_death_clears_squads_and_ejects_both_players() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    encounter_scenario(false, Some(ExitCase::Defeat))
}

#[test]
fn drume_defeat_ejects_both_players_after_blocked_exit_outlasts_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    encounter_scenario(false, Some(ExitCase::ObstructedDefeat))
}

#[test]
fn drume_lever_replaces_vacated_battlefield_before_timeout() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    encounter_scenario(true, Some(ExitCase::Timeout))
}

#[test]
fn drume_untagged_rope_starts_skirmish_and_scheduler_ejects_both_players() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    encounter_scenario(false, Some(ExitCase::ScheduledTimeout))
}
