//! A lost Ice Islands paint acknowledgement replays its exact selected-tool binding.
use super::engitem_seal_paint::{SEALS, position, predecessor, source_appearances};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    ice_islands_seals as source,
    world_callbacks::{Callback, Offering},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

pub(super) const NAME: &str = "engitem-native-seal-paint-lost-ack";

#[test]
fn lost_ack_ice_islands_painting_replay_keeps_selected_tool_binding() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e::scenario_thread::run("engitem_seal_paint_lost_ack", || {
            super::quest_world_callbacks::scenario(NAME, position(SEALS[0])?, false)
        });
    eprintln!(
        "ENGITEM_PG_EXECUTED seal_paint_lost_ack duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
) -> TestResult {
    use crate::foundation::{CharacterId, CharacterLease};
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    let target = position(SEALS[0])?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: target,
            item: "oteryn:item.tibia.i7253".into(),
            count: 1,
        },
        951,
        true,
    )
    .await?;
    predecessor(owner, admitted, h, callback).await?;
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i7253", "definition-r1"))
        .ok_or("paint reference")?;
    let binding = source::targets::target(SEALS[0]).ok_or("native seal")?;
    let key = oteryn_world_bundle::bundle::placement_key(
        target.floor,
        target.x,
        target.y,
        binding.ordinal,
    )
    .ok_or("seal key")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let payload = {
        let map = owner.bundle_map().ok_or("map")?;
        assert_eq!(source_appearances(map, target)?[2], 7178);
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("handle")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            })),
            use_with_position: None,
        })
        .map_err(debug)?
    };
    let command = admitted.continuity.next_command_id;
    owner
        .states
        .quest_callbacks
        .interrupt_after_commit
        .store(true, std::sync::atomic::Ordering::Release);
    assert!(
        super::item_craft::exchange(owner, *admitted, command, &payload, Some(reference))
            .await
            .is_err(),
        "the acknowledgement of the committed paint is lost"
    );
    assert!(owner.has_pending_quest_callback(admitted.game_session_id));
    assert_eq!(receipts(h).await?, 1, "the paint receipt committed durably");
    assert!(
        owner
            .reconcile_quest_callbacks(admitted.game_session_id)
            .await,
        "the original cause reconciles"
    );
    assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
    assert_eq!(
        source_appearances(owner.bundle_map().ok_or("map")?, target)?[2],
        7252,
        "the replay with the bound tool projects the committed paint"
    );
    let snapshot = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(snapshot.tracks().get(source::TARGETS), Some(&123));
    assert_eq!(receipts(h).await?, 1, "the replay writes no second receipt");
    eprintln!("ENGITEM_SEAL_PAINT_LOST_ACK_REPLAYED_SELECTED_TOOL");
    Ok(())
}

async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(source::QUEST)
            .fetch_one(&h.pool)
            .await?,
    )
}
