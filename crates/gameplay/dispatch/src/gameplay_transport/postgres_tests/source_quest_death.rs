//! Source creature callbacks must run at the ordinary native lethal boundary.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

const TRACK: &str = "oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/quest_door";
const PREFIX: &str = "oteryn:quest-transition/quest/u8_54/an_uneasy_alliance/quest_door/";
const CREATURE: &str = "oteryn:creature.renegade_orc";

#[test]
fn source_quest_death_renegade_active() -> TestResult {
    scenario("active")
}
#[test]
fn source_quest_death_renegade_replay() -> TestResult {
    scenario("replay")
}
#[test]
fn source_quest_death_renegade_inactive() -> TestResult {
    scenario("inactive")
}
#[test]
fn source_quest_death_renegade_late_start() -> TestResult {
    scenario("late-start")
}
#[test]
fn source_quest_death_renegade_restarted() -> TestResult {
    scenario("restarted")
}
fn scenario(case: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(case, move || {
        super::quest_world_callbacks::scenario(
            &format!("source-quest-death-{case}"),
            Position {
                x: 33000,
                y: 32000,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "SOURCE_QUEST_DEATH POSTGRES_CASE_{} case={case} duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}
pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    assert!(version.starts_with("PostgreSQL"));
    eprintln!("SOURCE_QUEST_DEATH actual_postgresql=true {version}");
    let session = admitted.game_session_id;
    let actor = admitted.runtime_actor.ok_or("player actor")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    if !name.ends_with("inactive") && !name.ends_with("late-start") {
        start_quest(owner, session, 7000).await?;
    }
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            session,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    if name.ends_with("-active") {
        // A valid UUIDv7 exercises the writer guard, not occurrence parsing.
        let mut occurrence = [0; 16];
        occurrence[6] = 0x70;
        occurrence[8] = 0x80;
        occurrence[15] = 251;
        let outcome=owner.revision_sequencer.acquire(fence.character_id).await.commit_quest_transition(owner.root,owner.character,owner.holder,fence,QuestTransitionRequest{transition_key:format!("{PREFIX}creature_event_1"),cause:QuestCause::CreatureDeath(crate::durability::character_progression::ExperienceRewardOccurrence::from_bytes(occurrence).map_err(debug)?)},owner.quest_catalogue.clone().ok_or("catalogue")?).await;
        assert!(matches!(outcome,Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)),"generic writer must not bypass captured source eligibility: {outcome:?}");
    }
    owner.refresh_quest_session(session, false).await;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let position = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let policy = runtime.companion_policy(CREATURE).map_err(debug)?;
        let target = runtime
            .admit_encounter_actor(
                CREATURE,
                policy.maximum_health,
                crate::foundation::MovementLocalPosition {
                    x: position.x + 1,
                    ..position
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, CREATURE)
            .map_err(debug)?;
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(session, CommandId::new(7001).map_err(debug)?),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: CREATURE.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"source-quest-death",
                    damage: policy.maximum_health,
                },
            )
            .map_err(debug)?;
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                crate::gameplay_transport::kill_reward::KillSink {
                    prey: None,
                    activity: &[],
                    queue: &queue,
                    rewards: native.reward_table(),
                    charms: None,
                },
                target,
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session,
                    actor,
                },
                now,
                Some(observed),
            );
        }
        target
    };
    if name.ends_with("late-start") || name.ends_with("restarted") {
        start_quest(owner, session, 7002).await?;
        owner.refresh_quest_session(session, false).await;
    }
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    if name.ends_with("replay") {
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            crate::gameplay_transport::kill_reward::DrainOutcome::Drained
        );
    }
    let stored:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(admitted.item_fence.ok_or("item fence")?.character_id.as_bytes().as_slice()).bind(TRACK).fetch_optional(&h.pool).await?;
    assert_eq!(
        stored,
        if name.ends_with("inactive") {
            None
        } else if name.ends_with("late-start") || name.ends_with("restarted") {
            Some(0)
        } else {
            Some(1)
        },
        "native damaging-player source callback"
    );
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2").bind(admitted.item_fence.ok_or("item fence")?.character_id.as_bytes().as_slice()).bind(format!("{PREFIX}creature_event_1")).fetch_one(&h.pool).await?;
    assert_eq!(
        receipts,
        if name.ends_with("-active") || name.ends_with("-replay") {
            1
        } else {
            0
        },
        "one source receipt per qualifying death, no delayed eligibility"
    );
    assert!(!owner.runtime.lock().await.corpse_projection_pinned(target));
    Ok(())
}

pub(super) async fn start_quest(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
    command: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: format!("{PREFIX}npc_1"),
                cause: QuestCause::Command(CommandRef::new(
                    session,
                    CommandId::new(command).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("catalogue")?,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "explicit source quest start: {outcome:?}"
    );
    Ok(())
}
