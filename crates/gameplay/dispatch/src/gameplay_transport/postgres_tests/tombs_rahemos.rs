//! The real world USE caller handles both of the Oasis lever's source outcomes.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::{overlay::TilePos, view::{self, MapFacts}};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64};
use sha2::{Digest, Sha256};

const LEVER: Position = Position { x: 33124, y: 32765, floor: -14 };
const HAT: TilePos = TilePos { x: 33123, y: 32765, floor: -14 };
const DOOR: TilePos = TilePos { x: 33122, y: 32765, floor: -14 };

#[test]
fn tombs_rahemos_oasis_win_opens_door_and_expires_carrot() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()); };
    run("qengine-rewards-rahemos-oasis-win")
}

#[test]
fn tombs_rahemos_oasis_loss_applies_physical_damage() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()); };
    run("qengine-rewards-rahemos-oasis-loss")
}

fn run(case: &'static str) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(case, || {
        super::quest_world_callbacks::scenario(case, LEVER, false)
    })
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    win: bool,
) -> TestResult {
    let map = owner.bundle_map().ok_or("Rahemos map")?;
    let actor = admitted.runtime_actor.ok_or("Rahemos actor")?;
    let session = admitted.game_session_id;
    let placement_key = view::placement_key(TilePos { x: LEVER.x, y: LEVER.y, floor: LEVER.floor }, 1)
        .ok_or("Rahemos lever placement")?;
    let root = {
        let runtime = owner.runtime.lock().await;
        GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest())
    };
    let command = (1_u64..=128).find(|command| {
        let hash = Sha256::new()
            .chain_update(b"oteryn:rahemos-oasis:v1")
            .chain_update(session.as_bytes())
            .chain_update(command.to_be_bytes())
            .chain_update(placement_key.to_be_bytes())
            .finalize();
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&hash[..16]);
        deterministic_decision_u64(&root, DecisionOccurrenceId::from_bytes(occurrence), "quest.rahemos.hat", 0)
            .is_ok_and(|draw| (crate::spell::uniform_draw(draw, 1, 3) == 1) == win)
    }).ok_or("no matching Rahemos roll")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&placement_key.to_be_bytes());
    let reply = super::quest_world_callbacks::wire(owner, admitted, command, 2, encode_use_intent(&WorldObjectTarget {
        placement, expected_revision: 0,
    }).map_err(debug)?).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
    let has_appearance = |position, appearance| {
        view::compose_tile(&map.overlay, &map.facts.pinned(), position)
            .is_ok_and(|tile| tile.is_some_and(|tile| tile.entries.iter().any(|entry| entry.facts.appearance_id == appearance)))
    };
    assert_eq!(has_appearance(HAT, 3595), win);
    assert_eq!(has_appearance(DOOR, 1663), win);
    assert_eq!(map.facts.object_revision(placement_key), u64::from(win));
    if win {
        let due = map.facts.next_movement_decay_time().ok_or("Rahemos reset deadline")?;
        assert_eq!(map.facts.expire_movement_items(due - 1), 0);
        assert_eq!(map.facts.expire_movement_items(due), 2);
        assert!(!has_appearance(HAT, 3595));
        assert!(has_appearance(DOOR, 1663));
        assert_eq!(map.facts.object_revision(placement_key), 2);
    } else {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert!(states.is_dead(actor) || states.get(&runtime, actor, session)
            .is_some_and(|state| state.vitals().health < state.vitals().max_health));
    }
    Ok(())
}
