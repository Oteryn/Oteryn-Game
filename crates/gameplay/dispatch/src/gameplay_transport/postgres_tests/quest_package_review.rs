use super::TestResult;
use super::inquisition_coffin_recovery_fixtures::{FOREIGN_GENERATION, RESTART};
use super::quest_world_callbacks_fixtures as world;
use super::srvenc_source_deaths_fixtures::scenario as death_scenario;

#[test]
fn inquisition_coffins_recover_published_counts_until_defeat() -> TestResult {
    for name in [RESTART, FOREIGN_GENERATION] {
        for variant in ["ordinary_published_restart", "weakened_published_restart"] {
            run_coffin(name, variant)?;
        }
    }
    Ok(())
}

#[test]
fn inquisition_coffins_refuse_fresh_use_after_delivery_delete_rolls_back_and_count_dies()
-> TestResult {
    for name in [RESTART, FOREIGN_GENERATION] {
        for variant in ["rolled_back_delete", "ordinary_rolled_back_delete"] {
            run_coffin(name, variant)?;
        }
    }
    Ok(())
}

#[test]
fn nomad_excludes_member_kicked_by_leader() -> TestResult {
    death_scenario(stringify!(nomad_excludes_member_kicked_by_leader), "nomad")
}

#[test]
fn nomad_respects_leader_disabling_shared_xp() -> TestResult {
    death_scenario(
        stringify!(nomad_respects_leader_disabling_shared_xp),
        "nomad",
    )
}

#[test]
fn sarina_vines_refuse_pz_locked_players_in_both_directions() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    for name in [
        "thieves-guild-sarina-vine-climb",
        "thieves-guild-sarina-vine-descend",
    ] {
        super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
            let callback = world::callback(name)?;
            let trigger = callback.selector.native.ok_or("Sarina vine")?.position;
            world::scenario(name, trigger, false)
        })?;
    }
    Ok(())
}

#[test]
fn serpentine_close_resolves_commit_after_receipt_read_is_unavailable() -> TestResult {
    run_serpentine("serpentine-tower-lever#unknown_commit")
}

#[test]
fn serpentine_close_superseded_recovery_keeps_the_scene_open() -> TestResult {
    run_serpentine("serpentine-tower-lever#superseded")
}

fn run_serpentine(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        world::scenario(name, super::serpentine_tower_fixtures::LEVER, false)
    })
}

fn run_coffin(name: &'static str, variant: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        let trigger = world::callback(name)?.selector.position.ok_or("coffin")?;
        world::scenario(&format!("{name}#{variant}"), trigger, false)
    })
}
