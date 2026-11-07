//! Both real source instructions, cold replay and all physical hoist/lever branches.
use super::TestResult;
use crate::map::view::MapFacts;

#[test]
fn questpc5_hoist_complete_source_route() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_hoist_route",
            false,
            false,
        )
        .await?;
        eprintln!(
            "POSTGRES_CASE_STARTED hoist_complete_route {:?}",
            started.elapsed()
        );
        let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
        for uid in [3059, 3060, 3061] {
            assert!(
                callbacks.iter().any(|c| c.selector.unique == Some(uid)),
                "source hoist callback {uid} is absent"
            );
        }
        assert!(
            callbacks
                .iter()
                .any(|c| c.key == "oteryn:quest-callback/formogar-hoist-direction-switch"),
            "source reversible hoist direction lever is absent"
        );
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED hoist_callback_registration {:?}",
            started.elapsed()
        );
        Ok(())
    })?;
    super::quest_catalogue_chests::hoist_route_cohort()
}
pub(super) const SKELETON_CLAIM: &str = "oteryn:reward-claim.source.formorgar_hoist_skeleton";
pub(super) const PLANS_TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_0/the_ice_islands/formorgar_mines_hoist_chest";
const SKELETON_TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_0/the_ice_islands/formorgar_mines_hoist_skeleton";

use super::fixture::debug;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

fn position(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

use super::quest_catalogue_chests::relocate_source_actor as relocate;

async fn snapshot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &super::fixture::Harness,
) -> TestResult<(crate::durability::quest_state::QuestStateCopy, i64)> {
    let fence = admitted.item_fence.ok_or("item fence")?;
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let receipts = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    Ok((state, receipts))
}

async fn use_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    at: Position,
    ordinal: u8,
) -> TestResult<oteryn_protocol_oteryn::world_object::UseDisposition> {
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, ordinal)
        .ok_or("source placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = oteryn_protocol_oteryn::world_object::WorldObjectTarget {
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

#[allow(clippy::too_many_arguments)]
async fn travel(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
    h: &super::fixture::Harness,
    command: u64,
    floor: i8,
    destination: Position,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
        encode_step_intent,
    };
    let before = snapshot(owner, admitted, h).await?;
    relocate(owner, admitted, actor, position(32156, 31126, floor)).await?;
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
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
        super::quest_world_callbacks::local(destination)
    );
    assert_eq!(
        snapshot(owner, admitted, h).await?,
        before,
        "travel must preserve tracks, receipts and Character revision"
    );
    eprintln!(
        "POSTGRES_CASE_OBSERVED hoist_travel source_floor={floor} destination={destination:?} no_quest_writes"
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &super::fixture::Harness,
    actor: ExactActorRef,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let started = std::time::Instant::now();
    let fence = admitted.item_fence.ok_or("item fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let (state, _) = snapshot(owner, admitted, h).await?;
    assert_eq!(state.tracks().get(PLANS_TRACK), Some(&1));
    assert_ne!(state.tracks().get(SKELETON_TRACK), Some(&1));
    travel(owner, admitted, actor, h, 3, -9, position(32156, 31126, -9)).await?;

    let skeleton = position(32211, 31273, -10);
    relocate(
        owner,
        admitted,
        actor,
        position(skeleton.x - 1, skeleton.y, skeleton.floor),
    )
    .await?;
    assert_eq!(
        use_at(owner, admitted, 4, skeleton, 1).await?,
        UseDisposition::Committed
    );
    let (state, _) = snapshot(owner, admitted, h).await?;
    assert_eq!(state.tracks().get(PLANS_TRACK), Some(&1));
    assert_eq!(state.tracks().get(SKELETON_TRACK), Some(&1));
    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    let mut keys: Vec<_> = bag
        .entries
        .iter()
        .map(|e| e.item.definition.production_key.as_str())
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["oteryn:item.tibia.i2820", "oteryn:item.tibia.i2822"]);
    let source_text: String = sqlx::query_scalar(
        "SELECT root_text FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1",
    )
    .bind(SKELETON_CLAIM)
    .fetch_one(&h.pool)
    .await?;
    let expected = "The map shows the original floor plan of this mine. You recognise your position and that the shaft to the south actually would reach much further.\n\n(In the lower right corner someone scribbled a note how to use the hoist on the first mine floor.)";
    assert_eq!(source_text, expected);
    let written: String = sqlx::query_scalar("SELECT t.text FROM game_item_texts t JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i2822'")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        written, expected,
        "the player receives the exact source instruction text"
    );
    let before = snapshot(owner, admitted, h).await?;
    assert_eq!(
        use_at(owner, admitted, 5, skeleton, 1).await?,
        UseDisposition::NothingToUse
    );
    assert_eq!(snapshot(owner, admitted, h).await?, before);
    let claims: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        claims, 2,
        "exactly one receipt per actual instruction acquisition"
    );

    travel(
        owner,
        admitted,
        actor,
        h,
        6,
        -9,
        position(32157, 31125, -10),
    )
    .await?;
    travel(
        owner,
        admitted,
        actor,
        h,
        7,
        -10,
        position(32157, 31125, -9),
    )
    .await?;
    let lever = position(32156, 31125, -10);
    relocate(owner, admitted, actor, position(32155, 31125, -10)).await?;
    let before = snapshot(owner, admitted, h).await?;
    assert_eq!(
        use_at(owner, admitted, 8, lever, 2).await?,
        UseDisposition::Committed
    );
    assert_eq!(snapshot(owner, admitted, h).await?, before);
    travel(
        owner,
        admitted,
        actor,
        h,
        9,
        -10,
        position(32157, 31125, -11),
    )
    .await?;
    relocate(owner, admitted, actor, position(32155, 31125, -10)).await?;
    assert_eq!(
        use_at(owner, admitted, 10, lever, 2).await?,
        UseDisposition::Committed
    );
    assert_eq!(snapshot(owner, admitted, h).await?, before);
    let key = oteryn_world_bundle::bundle::placement_key(lever.floor, lever.x, lever.y, 2)
        .ok_or("lever key")?;
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        2
    );
    assert_eq!(
        use_at(owner, admitted, 11, lever, 0).await?,
        UseDisposition::NothingToUse
    );
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        2,
        "ground cannot masquerade as the lever"
    );
    travel(
        owner,
        admitted,
        actor,
        h,
        12,
        -11,
        position(32157, 31125, -10),
    )
    .await?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED hoist_real_instructions_all_source_routes {:?}",
        started.elapsed()
    );
    Ok(())
}
