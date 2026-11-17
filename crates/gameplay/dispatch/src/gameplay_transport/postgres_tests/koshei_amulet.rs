//! KOSHEI: framed production use-with commits the selected amulet and RewardDoor.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{koshei_amulet as source, world_callbacks};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[test]
fn koshei_validator_accepts_definition_route_and_refuses_appearance_only() -> TestResult {
    let callback = source::callback()?;
    world_callbacks::validate_callbacks(std::slice::from_ref(&callback))?;
    let mut appearance = callback.clone();
    appearance.selector.target_definition = None;
    appearance.selector.appearance = Some(7535);
    assert!(world_callbacks::validate_callbacks(&[appearance]).is_err());
    for invalid in 0..4 {
        let mut bad = callback.clone();
        match invalid {
            0 => bad.selector.target_definition = Some("oteryn:terrain.tibia.i7535".into()),
            1 => bad.trigger = world_callbacks::Trigger::StepIn,
            2 => bad.item_on_target.as_mut().ok_or("tool")?.selected_instance = false,
            _ => bad.selector.appearance = Some(7535),
        }
        assert!(world_callbacks::validate_callbacks(&[bad]).is_err());
    }
    Ok(())
}

#[test]
fn koshei_real_use_with_grants_storage_refuses_other_operands_and_replays() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    assert!(!admin.is_empty());
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("koshei-amulet", || {
        super::quest_world_callbacks::scenario("koshei-amulet", world_callbacks::Position {
            x: 32700, y: 31200, floor: -7,
        }, false)
    });
    eprintln!("KOSHEI_PG_EXECUTED duration={:?} success={}", started.elapsed(), result.is_ok());
    result?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    target: world_callbacks::Position,
) -> TestResult {
    use crate::foundation::{CharacterId, CharacterLease};
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|_| "fence")?.ok_or("fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner.runtime.lock().await.bind_attacker_lease(actor, admitted.game_session_id,
        CharacterLease::new(CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
            fence.character_lease_generation).map_err(debug)?).map_err(debug)?;
    admitted.continuity.selected_capabilities = super::super::capabilities::SelectedCapabilities::select(
        owner.offered_capabilities(), &[4, 6, 11, 15, 18]).ok_or("capabilities")?;
    for (ordinal, item) in [(901, "oteryn:item.tibia.i7532"), (902, "oteryn:item.tibia.i3031")] {
        super::quest_world_callbacks::input_item(owner, fence.item_fence(), &world_callbacks::Offering {
            position: target, item: item.into(), count: 1,
        }, ordinal, true).await?;
    }
    let map = owner.bundle_map().ok_or("map")?;
    let payload = |ordinal| -> TestResult<Vec<u8>> {
        let key = oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, ordinal).ok_or("key")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        Ok(encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("handle")?),
            use_with: None, use_with_position: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement, expected_revision: map.facts.object_revision(key),
            })),
        }).map_err(debug)?)
    };
    let wrong_target = payload(1)?;
    let corpse = payload(2)?;
    let index = owner.active_generation.and_then(|g| g.native_gameplay()).and_then(|n| n.item_index()).ok_or("index")?;
    let amulet = index.definition_ref("Item", "oteryn:item.tibia.i7532", "definition-r1").ok_or("amulet definition")?;
    let other = index.definition_ref("Item", "oteryn:item.tibia.i3031", "definition-r1").ok_or("other definition")?;
    for (bytes, reference) in [(&wrong_target, amulet), (&corpse, other)] {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let (next, result, _) = super::item_craft::exchange(owner, *admitted,
            admitted.continuity.next_command_id, bytes, Some(reference)).await?;
        *admitted = next;
        assert_ne!(result, UseDisposition::Committed);
        assert_eq!(snapshot(h).await?, (0, 1, 0, 1));
        assert!(!door_access(owner, admitted)?);
    }
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let command = admitted.continuity.next_command_id;
    let (next, result, actual) = super::item_craft::exchange(owner, *admitted, command, &corpse, Some(amulet)).await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(snapshot(h).await?, (1, 0, 1, 1));
    assert!(door_access(owner, admitted)?);
    let (_, replay, _) = super::item_craft::exchange(owner, *admitted, command, &actual, None).await?;
    assert_eq!(replay, UseDisposition::Committed);
    assert_eq!(snapshot(h).await?, (1, 0, 1, 1));
    assert!(door_access(owner, admitted)?);
    let state = owner.root.read_character_quest_state(owner.character, fence.character_id).await.map_err(debug)?;
    assert_eq!(state.tracks().get(source::TRACK), Some(&1));
    Ok(())
}

async fn snapshot(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE track_key=$1 AND value=1), (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i7532' AND lifecycle=1), (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$2), (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3031' AND lifecycle=1)")
        .bind(source::TRACK).bind(source::KEY).fetch_one(&h.pool).await?)
}

fn door_access(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession) -> TestResult<bool> {
    let sessions = owner.states.quest_sessions.lock().map_err(|_| "quest sessions")?;
    let copy = sessions.get(&admitted.game_session_id).and_then(|state| state.copy.as_ref()).ok_or("quest copy")?;
    Ok(copy.meets_door_storage(41351))
}
