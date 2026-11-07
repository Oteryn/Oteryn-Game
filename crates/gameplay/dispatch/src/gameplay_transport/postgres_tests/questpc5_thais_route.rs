//! Source Thais puzzle accepts creatures and restores its pressure projection on every end path.
use super::TestResult;

#[test]
fn questpc5_thais_complete_source_route() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_thais_route",
            false,
            false,
        )
        .await?;
        eprintln!(
            "POSTGRES_CASE_STARTED thais_creature_occupancy {:?}",
            started.elapsed()
        );
        let doc: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/quests/shared-access-callbacks.json"
        )))?;
        let mut plate = doc["callbacks"]
            .as_array()
            .ok_or("callbacks")?
            .iter()
            .find(|c| c["key"] == "oteryn:quest-callback/thais-lighthouse-plate")
            .ok_or("source plate")?
            .clone();
        plate["occupancy_driven"] = true.into();
        let _: oteryn_gameplay_quests::quest::world_callbacks::Callback =
            serde_json::from_value(plate)?;
        let startup: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/quest-startup-bindings.json"
        )))?;
        for uid in [6014, 6015] {
            assert!(
                startup["bindings"]
                    .as_array()
                    .ok_or("startup bindings")?
                    .iter()
                    .any(|b| b["attribute"] == "unique" && b["value"] == uid),
                "source lighthouse treasure selector {uid} is absent"
            );
        }
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED thais_source_occupancy_registration {:?}",
            started.elapsed()
        );
        Ok(())
    })?;
    super::quest_catalogue_chests::thais_route_cohort()
}

use super::fixture::debug;
use super::quest_catalogue_chests::relocate_source_actor as relocate;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
fn tile(p: Position) -> crate::map::overlay::TilePos {
    crate::map::overlay::TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
fn pos(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}
async fn use_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    at: Position,
) -> TestResult<UseDisposition> {
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, 1)
        .ok_or("source placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
async fn south(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
    command: u64,
    expected: Position,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
        encode_step_intent,
    };
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::South),
    )
    .await?;
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
        super::quest_world_callbacks::local(expected)
    );
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &super::fixture::Harness,
    actor: ExactActorRef,
) -> TestResult {
    let started = std::time::Instant::now();
    let fence = admitted.item_fence.ok_or("fence")?;
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let receipts_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
    let map = owner.bundle_map().ok_or("map")?;
    let appearance = |p: Position| -> TestResult<u16> {
        let source = map
            .overlay
            .base()
            .tile(p.x, p.y, p.floor)
            .ok_or("source tile")?;
        let id = *source
            .ids()
            .iter()
            .zip(source.depths())
            .filter(|(_, d)| **d == 0)
            .nth(1)
            .ok_or("source ordinal")?
            .0;
        Ok(map
            .facts
            .base_entry(tile(p), 1, id)
            .ok_or("source object")?
            .appearance_id)
    };
    relocate(owner, admitted, actor, pos(32226, 32278, -8)).await?;
    assert_eq!(
        use_at(owner, admitted, 3, pos(32227, 32278, -8)).await?,
        UseDisposition::Committed
    );
    assert_eq!(appearance(pos(32225, 32276, -8))?, 7767);
    relocate(owner, admitted, actor, pos(32225, 32275, -8)).await?;
    south(owner, admitted, actor, 4, pos(32225, 32276, -9)).await?;
    let creature = owner
        .runtime
        .lock()
        .await
        .admit_pinned_test_creature(super::quest_world_callbacks::local(pos(32225, 32268, -9)))
        .map_err(debug)?;
    relocate(owner, admitted, actor, pos(32225, 32281, -9)).await?;
    south(owner, admitted, actor, 5, pos(32225, 32282, -10)).await?;
    assert_eq!(appearance(pos(32225, 32268, -9))?, 430);
    assert_eq!(appearance(pos(32225, 32282, -9))?, 7767);
    owner
        .runtime
        .lock()
        .await
        .remove_test_actor(creature)
        .map_err(debug)?;
    relocate(owner, admitted, actor, pos(32224, 32285, -10)).await?;
    assert_eq!(
        use_at(owner, admitted, 6, pos(32225, 32285, -10)).await?,
        UseDisposition::Committed
    );
    for (at, destination) in [
        (pos(32232, 32276, -9), pos(32225, 32274, -10)),
        (pos(32225, 32276, -10), pos(32230, 32276, -9)),
    ] {
        assert_eq!(
            map.facts
                .quest_portal_destination(tile(at), owner.owner_now().get()),
            Some(tile(destination))
        );
        assert_eq!(map.facts.movement_items(tile(at)).len(), 1);
    }
    assert_eq!(appearance(pos(32225, 32268, -9))?, 431);
    assert_eq!(appearance(pos(32225, 32282, -9))?, 429);
    relocate(owner, admitted, actor, pos(32232, 32275, -9)).await?;
    south(owner, admitted, actor, 7, pos(32225, 32274, -10)).await?;
    relocate(owner, admitted, actor, pos(32225, 32275, -10)).await?;
    south(owner, admitted, actor, 8, pos(32230, 32276, -9)).await?;
    relocate(owner, admitted, actor, pos(32224, 32285, -10)).await?;
    assert_eq!(
        use_at(owner, admitted, 9, pos(32225, 32285, -10)).await?,
        UseDisposition::Committed
    );
    for at in [pos(32232, 32276, -9), pos(32225, 32276, -10)] {
        assert!(
            map.facts
                .quest_portal_destination(tile(at), owner.owner_now().get())
                .is_none()
        );
        assert!(map.facts.movement_items(tile(at)).is_empty());
    }
    // Repeated switch closes the exact original source ladder as well.
    relocate(owner, admitted, actor, pos(32226, 32278, -8)).await?;
    assert_eq!(
        use_at(owner, admitted, 10, pos(32227, 32278, -8)).await?,
        UseDisposition::Committed
    );
    assert_eq!(appearance(pos(32225, 32276, -8))?, 351);
    assert_eq!(
        owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?,
        before
    );
    let receipts_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        receipts_after, receipts_before,
        "all puzzle operations are read-only for durable quest state"
    );
    relocate(owner, admitted, actor, pos(32225, 32265, -10)).await?;
    assert_eq!(
        use_at(owner, admitted, 11, pos(32226, 32265, -10)).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_at(owner, admitted, 12, pos(32226, 32265, -10)).await?,
        UseDisposition::NothingToUse
    );
    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    assert_eq!(bag.entries.len(), 2);
    for key in ["oteryn:item.tibia.i3305", "oteryn:item.tibia.i3421"] {
        let matching: Vec<_> = bag
            .entries
            .iter()
            .filter(|e| e.item.definition.production_key == key)
            .collect();
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].item.quantity, 1);
    }
    let claims:Vec<(String,i64)>=sqlx::query_as("SELECT claim_production_key,count(*) FROM game_reward_claim_mint_receipts GROUP BY claim_production_key ORDER BY claim_production_key").fetch_all(&h.pool).await?;
    assert_eq!(
        claims,
        [
            ("oteryn:reward-claim.kv.lighthouse1".into(), 1),
            ("oteryn:reward-claim.kv.lighthouse2".into(), 1)
        ]
    );
    eprintln!(
        "POSTGRES_CASE_OBSERVED thais_same_character_ladder_native_descent_creature_plate_restore_both_portals_both_once_rewards {:?}",
        started.elapsed()
    );
    Ok(())
}
