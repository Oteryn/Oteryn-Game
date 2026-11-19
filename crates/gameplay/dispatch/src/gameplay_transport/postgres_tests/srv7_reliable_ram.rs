//! Real framed carried USE on the source Ram map object, with atomic PG state.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn srv7_quest_turnips_never_enter_the_unguarded_generic_taming_owner() {
    assert!(
        crate::content::native_gameplay::mount_taming::rules()
            .iter()
            .all(|rule| rule.item_key != "oteryn:item.tibia.i53073")
    );
}

#[test]
fn srv7_reliable_ram_real_map_use_guards_grants_and_replays() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("srv7_ram", || {
        super::quest_world_callbacks_fixtures::scenario(
            "srv7-reliable-ram",
            Position {
                x: 31926,
                y: 31896,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "SRV7_PG_EXECUTED reliable_ram duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    result
}

#[test]
fn engitem_targuna_consumes_the_selected_copy_and_replays_its_fenced_receipt() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() { return Ok(()); }
    let started=std::time::Instant::now();
    let result=super::kill_reward_e2e_fixtures::scenario_thread::run("engitem_selected_world_tool", || {
        super::quest_world_callbacks_fixtures::scenario("engitem-targuna-selected-tool",Position{x:31926,y:31896,floor:-7},false)
    });
    assert!(started.elapsed().as_secs_f64()>0.01);
    eprintln!("ENGITEM_PG_EXECUTED selected_world_tool duration={:?} success={}",started.elapsed(),result.is_ok());
    result
}
