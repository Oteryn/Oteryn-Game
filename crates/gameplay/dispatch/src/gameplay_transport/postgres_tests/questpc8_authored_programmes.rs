//! Chosen programmes use real movement, captured native deaths and literal USE.
use super::fixture::{Harness, debug, id};
use super::{TestResult, quest_world_callbacks};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

#[test]
fn questpc8_braindeath_exploration_native_death_coffin_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-braindeath", || {
        quest_world_callbacks::scenario(
            "authored-braindeath",
            Position {
                x: 32545,
                y: 31522,
                floor: -11,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_braindeath_exploration_native_death_coffin_and_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn questpc8_tinder_box_summit_and_once_reward() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-tinder", || {
        quest_world_callbacks::scenario(
            "authored-tinder-box",
            Position {
                x: 32084,
                y: 31025,
                floor: -1,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_tinder_box_summit_and_once_reward duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise_braindeath(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let premium = owner.premium_coordinator.take();
    step(owner, admitted, 1, StepDirection::East).await?;
    assert_eq!(
        progress(h, "stage").await?,
        -1,
        "absent native Premium authority cannot enter the programme"
    );
    owner.premium_coordinator = premium;
    owner.admit_premium(id(40), admitted.game_session_id);
    step(owner, admitted, 2, StepDirection::West).await?;
    step(owner, admitted, 3, StepDirection::East).await?;
    assert_eq!(progress(h, "stage").await?, 1);
    assert_eq!(coffin(owner, admitted, 4).await?, UseDisposition::Rejected);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    super::grizzly_npc::questpc8_campaign::hunt(
        owner,
        admitted,
        "oteryn:creature.braindeath",
        1,
        4,
    )
    .await?;
    assert_eq!(progress(h, "kills").await?, 1);
    let reward_target = coffin_target(owner)?;
    assert_eq!(
        use_target(owner, admitted, 5, &reward_target).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h, "stage").await?, 3);
    let quantity: i64 = sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3035' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(quantity, 13);
    let potions: i64 = sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i239' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(potions, 2);
    let bags: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i2867' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(bags, 1);
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        use_target(owner, admitted, 5, &reward_target).await?,
        UseDisposition::Committed
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(after, before);
    Ok(())
}

async fn progress(h: &Harness, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/chosen/braindeath/{suffix}")).fetch_one(&h.pool).await?)
}

async fn coffin(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
) -> TestResult<UseDisposition> {
    let target = coffin_target(owner)?;
    use_target(owner, admitted, command, &target).await
}

fn coffin_target(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("map")?;
    let key = crate::map::view::placement_key(
        TilePos {
            x: 32546,
            y: 31522,
            floor: -11,
        },
        1,
    )
    .ok_or("coffin key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

pub(super) async fn use_target(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let result = quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        encode_use_intent(target).map_err(debug)?,
    )
    .await?;
    decode_use_result(&result).map_err(debug)
}

pub(super) async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let result = quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result(&result).map_err(debug)?,
        StepDisposition::Moved,
        "command={command} direction={direction:?} position={:?}",
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
    );
    Ok(())
}

pub(super) async fn exercise_tinder_box(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let target = object_target(
        owner,
        TilePos {
            x: 32085,
            y: 31025,
            floor: -1,
        },
        1,
    )?;
    let premium = owner.premium_coordinator.take();
    step(owner, admitted, 1, StepDirection::East).await?;
    assert_eq!(
        use_target(owner, admitted, 2, &target).await?,
        UseDisposition::Rejected
    );
    owner.premium_coordinator = premium;
    owner.admit_premium(id(40), admitted.game_session_id);
    let stage: i64 = sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/chosen/tinder-box/stage'),-1)").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(stage, -1);
    step(owner, admitted, 3, StepDirection::West).await?;
    step(owner, admitted, 4, StepDirection::East).await?;
    assert_eq!(
        use_target(owner, admitted, 5, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_target(owner, admitted, 5, &target).await?,
        UseDisposition::Committed
    );
    let target = object_target(
        owner,
        TilePos {
            x: 32085,
            y: 31025,
            floor: -1,
        },
        1,
    )?;
    assert_eq!(
        use_target(owner, admitted, 6, &target).await?,
        UseDisposition::Rejected
    );
    let quantity: i64 = sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i20357' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(quantity, 1);
    let complete: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.tinder_box_quest_chyllfroest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(complete);
    Ok(())
}
pub(super) fn object_target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pos: TilePos,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("map")?;
    let key = crate::map::view::placement_key(pos, ordinal).ok_or("key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}
