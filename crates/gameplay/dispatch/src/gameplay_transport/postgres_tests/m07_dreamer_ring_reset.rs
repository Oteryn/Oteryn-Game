//! Literal UID9234 on synthetic source geometry; real STEP restores only missing trees.
use super::TestResult;
use oteryn_gameplay_quests::quest::{dreamer_death_ring, world_callbacks::Position};
#[test]
fn m07_death_ring_real_step_restores_only_missing_trees_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m07-ring-reset", || {
        super::quest_world_callbacks::scenario(
            "dreamers-challenge-death-ring-reset",
            Position {
                x: 100,
                y: 100,
                floor: -11,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
    trigger: Position,
) -> TestResult {
    use super::fixture::debug;
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_spatial::{
        StepDirection, StepDisposition, decode_step_result, encode_step_intent,
    };
    let keys = dreamer_death_ring::TREES.map(|p| {
        oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1).ok_or("tree key")
    });
    let keys = [keys[0]?, keys[1]?, keys[2]?];
    let source = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("source key")?;
    let map = owner.bundle_map().ok_or("map")?;
    let pos = crate::map::overlay::TilePos {
        x: trigger.x,
        y: trigger.y,
        floor: trigger.floor,
    };
    let tile = map
        .overlay
        .base()
        .tile(pos.x, pos.y, pos.floor)
        .ok_or("source tile")?;
    let id = *tile.ids().get(1).ok_or("source entry")?;
    let source_facts = map.facts.base_entry(pos, 1, id).ok_or("source facts")?;
    // Missing trees are the test precondition; the reset itself runs through real ingress.
    assert!(map.facts.apply_quest_callback(
        source,
        0,
        source_facts,
        &[keys[0], keys[2]],
        &[],
        false,
        None
    ));
    assert!(
        map.facts.base_hidden(keys[0])
            && !map.facts.base_hidden(keys[1])
            && map.facts.base_hidden(keys[2])
    );
    let cursor = map
        .facts
        .movement_effects_since(0)
        .last()
        .map_or(0, |e| e.0);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let payload = encode_step_intent(StepDirection::East);
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pacing")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 1, payload.clone()).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(dreamer_death_ring::DESTINATION)
    );
    let map = owner.bundle_map().ok_or("map")?;
    for key in keys {
        assert!(!map.facts.base_hidden(key));
    }
    let cues = map.facts.movement_effects_since(cursor);
    let blue = cues
        .iter()
        .filter(|e| e.2 == 13)
        .map(|e| e.1)
        .collect::<Vec<_>>();
    assert_eq!(
        blue,
        vec![
            crate::map::overlay::TilePos {
                x: 32857,
                y: 32231,
                floor: -11
            },
            crate::map::overlay::TilePos {
                x: 32857,
                y: 32233,
                floor: -11
            }
        ]
    );
    assert!(cues.iter().any(|e| e.2 == 11
        && e.1
            == crate::map::overlay::TilePos {
                x: 32819,
                y: 32347,
                floor: -9
            }));
    let cursor = cues.last().map_or(cursor, |e| e.0);
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 1, payload).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(cursor)
            .is_empty()
    );
    assert!(dreamer_death_ring::shared_reset(callback));
    eprintln!(
        "M07 POSTGRES_CASE_COMPLETED Death Ring UID9234 real_STEP missing_only_tree_reset source_teleport replay"
    );
    Ok(())
}
