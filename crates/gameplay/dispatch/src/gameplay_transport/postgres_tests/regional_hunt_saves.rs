use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn regional_hunt_dispatch_reports_from_a_merge_base_save() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("regional-hunt-save", || {
        super::quest_world_callbacks_fixtures::scenario(
            "regional-hunt-save",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!("POSTGRES_CASE_COMPLETED regional-hunt-save");
    Ok(())
}

fn regional_hunt_dialogue(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(
            name,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!("POSTGRES_CASE_COMPLETED {name}");
    Ok(())
}

#[test]
fn regional_hunt_dialogue_daniel_confirms_pending_hunt() -> TestResult {
    regional_hunt_dialogue("regional-hunt-dialogue-daniel")
}

#[test]
fn regional_hunt_dialogue_raymond_outfit_retires_offer() -> TestResult {
    regional_hunt_dialogue("regional-hunt-dialogue-raymond")
}

#[test]
fn regional_hunt_dialogue_lugri_outfit_retires_offer() -> TestResult {
    regional_hunt_dialogue("regional-hunt-dialogue-lugri")
}

#[test]
fn regional_hunt_dialogue_raymond_mixed_keyword_claims_outfit() -> TestResult {
    regional_hunt_dialogue("regional-hunt-dialogue-raymond-mixed")
}
