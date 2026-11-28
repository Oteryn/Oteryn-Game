//! W15c through real command-7 bytes, native NPC content, Premium and the quest writer.
use super::grizzly_npc_fixtures::scenario;

use super::TestResult;



#[path = "grizzly_npc/questpc8_campaign.rs"]
pub(super) mod questpc8_campaign;


#[test]
fn grizzly_npc_wire_join_choose_cancel_complete_replay_and_journal() -> TestResult {
    postgres_case(0)
}

#[test]
fn grizzly_npc_wire_rank_gate_and_pending_holy_icon_retry_once() -> TestResult {
    postgres_case(1)
}

#[test]
fn grizzly_npc_wire_rank_shop_native_gate_and_purchase_replay_once() -> TestResult {
    postgres_case(2)
}

#[test]
fn grizzly_npc_wire_partial_batch_refusal_refresh_and_unknown_resume() -> TestResult {
    postgres_case(3)
}

#[test]
fn grizzly_npc_wire_task_xp_crosses_level_50_and_unlocks_live_promotion() -> TestResult {
    postgres_case(4)
}

#[test]
fn grizzly_npc_wire_level_up_then_step_rereads_capacity_next_tick() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("grizzly-step-capacity", || scenario(8))
}

#[test]
fn cach1_grizzly_npc_wire_awards_confirmed_huntsman_and_elite_hunter() -> TestResult {
    postgres_case(7)
}

#[test]
fn questpc8_grizzly_production_placement_native_join_and_task() -> TestResult {
    let started = std::time::Instant::now();
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-grizzly-placement", || scenario(6))?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_grizzly_production_placement_native_join_and_task duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn postgres_case(case: u8) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("GRIZZLY_NPC_POSTGRES_CASE_SKIPPED no_admin_url");
        return Ok(());
    }
    let runner: fn() -> TestResult = match case {
        1 => || scenario(1),
        2 => || scenario(2),
        3 => || scenario(3),
        4 => || scenario(4),
        7 => || scenario(7),
        _ => || scenario(0),
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("grizzly-npc", runner)
}
