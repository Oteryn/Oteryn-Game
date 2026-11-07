//! Chosen archival vault programme uses native chambers and the existing quest/MINT owners.
use super::{TestResult, quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_magic_sword_two_and_four_native_demons_both_caches_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-magic-sword", || {
        quest_world_callbacks::scenario(
            "authored-magic-sword",
            Position {
                x: 32575,
                y: 32197,
                floor: -14,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_magic_sword_two_and_four_native_demons_both_caches_once duration={:?}",
        started.elapsed()
    );
    Ok(())
}

use super::fixture::{Harness, debug, id};
use super::questpc8_authored_programmes::{object_target, step, use_target};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[path = "questpc8_magic_sword/world.rs"]
mod fixture_world;
pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let active = controller.active().ok_or("active content")?;
    fixture_world::world(active.native_gameplay().ok_or("native world items")?)
}
const UPPER: &str = "oteryn:quest-death/authored-magic-sword-upper";
const LOWER: &str = "oteryn:quest-death/authored-magic-sword-lower";

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner.premium_coordinator = None;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let mut command = 1;
    raise_level(owner, admitted, 99, 80).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h, "stage").await?, -1);
    assert!(owner.quest_guardians_for_test(UPPER).await.is_empty());
    raise_level(owner, admitted, 100, 81).await?;
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(progress(h, "stage").await?, 1);
    assert_eq!(progress(h, "upper").await?, 0);
    assert!(owner.quest_guardians_for_test(LOWER).await.is_empty());
    clear_group(owner, admitted, UPPER, 2, command).await?;
    assert_eq!(progress(h, "upper").await?, 2);
    walk(owner, admitted, &mut command, 32575, 32198).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(
        position(owner, admitted).await?.floor,
        15,
        "real STEP descends the native432 hole"
    );
    walk(owner, admitted, &mut command, 32578, 32198).await?;
    let before_door = position(owner, admitted).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let blocked = quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::North),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&blocked).map_err(debug)?,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Blocked
    );
    command += 1;
    assert_eq!(
        position(owner, admitted).await?,
        before_door,
        "the native unresolved locked barrier is solid before its scoped passage callback"
    );
    let door = object_target(
        owner,
        TilePos {
            x: 32578,
            y: 32197,
            floor: -15,
        },
        1,
    )?;
    assert_eq!(
        use_target(owner, admitted, command, &door).await?,
        UseDisposition::Committed
    );
    command += 1;
    walk(owner, admitted, &mut command, 32578, 32196).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(progress(h, "stage").await?, 2);
    let north = TilePos {
        x: 32577,
        y: 32193,
        floor: -15,
    };
    let south = TilePos {
        x: 32578,
        y: 32193,
        floor: -15,
    };
    // The lower chamber has four live native actors; no manually created kill targets.
    assert_eq!(owner.quest_guardians_for_test(LOWER).await.len(), 4);
    walk(owner, admitted, &mut command, 32578, 32194).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, north).await?,
        UseDisposition::Rejected,
        "the first cache requires all four captured lower-chamber deaths"
    );
    assert_eq!(progress(h, "lower").await?, 0);
    clear_group(owner, admitted, LOWER, 4, command).await?;
    assert_eq!(progress(h, "lower").await?, 4);
    walk(owner, admitted, &mut command, 32578, 32194).await?;
    assert_eq!(
        claim(owner, admitted, &mut command, south).await?,
        UseDisposition::Rejected,
        "the second cache cannot pay first"
    );
    assert_eq!(
        claim(owner, admitted, &mut command, north).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h, "stage").await?, 3);
    for (item, count) in [(651, 1), (3388, 1), (3389, 1), (3288, 0)] {
        assert_eq!(quantity(h, item).await?, count);
    }
    assert_eq!(
        claim(owner, admitted, &mut command, north).await?,
        UseDisposition::Rejected
    );
    let target = object_target(owner, south, 1)?;
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    command += 1;
    assert_eq!(progress(h, "stage").await?, 4);
    assert_eq!(
        claim(owner, admitted, &mut command, south).await?,
        UseDisposition::Rejected
    );
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest cache")?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    assert_eq!(
        claim(owner, admitted, &mut command, south).await?,
        UseDisposition::Rejected
    );
    for (item, count) in [(651, 1), (3388, 2), (3389, 2), (3288, 1)] {
        assert_eq!(quantity(h, item).await?, count);
    }
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.magic_sword_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    Ok(())
}

async fn clear_group(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
    count: usize,
    command: u64,
) -> TestResult {
    let actors = owner.quest_guardians_for_test(key).await;
    assert_eq!(actors.len(), count);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        owner.quest_guardians_for_test(key).await,
        actors,
        "refresh cannot duplicate group slots"
    );
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
    for (index, actor) in actors.into_iter().enumerate() {
        super::grizzly_npc::questpc8_campaign::lethal(
            owner,
            admitted,
            actor,
            "oteryn:creature.demon",
            command,
            u16::try_from(index + 1)?,
        )
        .await?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert!(owner.quest_guardians_for_test(key).await.is_empty());
    Ok(())
}

pub(super) async fn raise_level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    level: u32,
    occurrence: u8,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression")?;
    let xp = binding
        .policy
        .thresholds
        .iter()
        .find(|t| t.level == level)
        .ok_or("level threshold")?
        .minimum_experience
        .get();
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let current = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("progression state")?;
    let amount = xp.checked_sub(current.total_experience.get()).ok_or("XP")?;
    if amount > 0 {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_experience(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(occurrence))
                        .map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(amount),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
                None,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, ExperienceCommitOutcome::Committed(_)));
    }
    Ok(())
}
async fn progress(h: &Harness, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/chosen/magic_sword/{suffix}")).fetch_one(&h.pool).await?)
}
async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 AND i.lifecycle=1").bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn claim(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    pos: TilePos,
) -> TestResult<UseDisposition> {
    let target = object_target(owner, pos, 1)?;
    let result = use_target(owner, admitted, *command, &target).await?;
    *command += 1;
    Ok(result)
}
async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<crate::foundation::MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position())
}
async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    x: i32,
    y: i32,
) -> TestResult {
    for _ in 0..64 {
        let at = position(owner, admitted).await?;
        let direction = if at.x < x {
            StepDirection::East
        } else if at.x > x {
            StepDirection::West
        } else if at.y < y {
            StepDirection::South
        } else if at.y > y {
            StepDirection::North
        } else {
            return Ok(());
        };
        step(owner, admitted, *command, direction).await?;
        *command += 1;
    }
    Err("route did not arrive".into())
}
