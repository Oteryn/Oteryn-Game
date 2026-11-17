//! Personal one-shot goals consume native lethal occurrences, never NPC kill flags.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{
    CharacterLease, CommandId, CommandRef, ExactActorRef, MovementLocalPosition,
};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

const COUNTER: &str = "oteryn:quest-progress/fibula/cavern_kills";

#[test]
fn questpc6_fibula_native_cavern_goals_capture_replay_refusals_and_cap_eight() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-fibula-native-goals")
}

async fn value(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<i64> {
    let copy = owner
        .root
        .read_character_quest_state(
            owner.character,
            crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
        )
        .await
        .map_err(debug)?;
    Ok(copy.tracks().get(COUNTER).copied().unwrap_or(0))
}

async fn capture(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    position: MovementLocalPosition,
    serial: u16,
) -> TestResult<ExactActorRef> {
    capture_origin(owner, admitted, creature, position, serial, true).await
}

async fn capture_origin(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    position: MovementLocalPosition,
    serial: u16,
    static_origin: bool,
) -> TestResult<ExactActorRef> {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let map = owner.bundle_map().ok_or("qualified map")?;
    assert!(
        map.overlay
            .base()
            .tile(
                u16::try_from(position.x)?,
                u16::try_from(position.y)?,
                -i8::try_from(position.floor)?
            )
            .is_some(),
        "the spawn's physical tile must exist"
    );
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        let pin = runtime.content_pin().clone();
        let target = if static_origin {
            runtime
                .realize_native_qualification_spawn_for_source(serial, pin, creature, position)
                .map_err(debug)?
                .actor
        } else {
            let policy = runtime.companion_policy(creature).map_err(debug)?;
            let actor = runtime
                .admit_encounter_actor(creature, policy.maximum_health, position)
                .map_err(debug)?;
            runtime
                .install_creature_policy(actor, creature)
                .map_err(debug)?;
            actor
        };
        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(
                    session,
                    CommandId::new(10000 + u64::from(serial)).map_err(debug)?,
                ),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: creature.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"quest-native-goal",
                    damage: health,
                },
            )
            .map_err(debug)?;
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink {
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
    assert!(
        queue.has_session(session),
        "native death must enter the retained settlement queue"
    );
    let lease = owner
        .runtime
        .lock()
        .await
        .bound_attacker_lease(actor, session)
        .map_err(debug)?;
    let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) =
        queue.take(session, lease.generation())
    else {
        return Err("retained native occurrence".into());
    };
    drop(taken); // Cancellation returns the same frozen predecessor to the existing queue.
    Ok(target)
}

async fn drain(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: ExactActorRef,
) -> TestResult {
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    let mut runtime = owner.runtime.lock().await;
    assert!(
        !runtime.corpse_projection_pinned(target),
        "settlement must release the native corpse pin"
    );
    runtime.remove_test_actor(target).map_err(debug)?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let started = std::time::Instant::now();
    prepare(owner, admitted).await?;
    let origin = MovementLocalPosition {
        x: 32236,
        y: 32474,
        floor: 10,
    };
    let target = capture(owner, admitted, "oteryn:creature.dragon", origin, 1000).await?;
    drain(owner, admitted, target).await?;
    assert_eq!(
        value(owner).await?,
        0,
        "pre-activation native death grants no progress"
    );
    super::questpc6_fibula_purchase::exercise(
        owner,
        admitted,
        h,
        "qengine-rewards-fibula-key-coins",
    )
    .await?;
    active_goals(owner, admitted).await?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED fibula_native_goals duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn prepare(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                admitted.runtime_actor.ok_or("actor")?,
                admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    Ok(())
}

pub(super) async fn active_goals(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let origin = MovementLocalPosition {
        x: 32236,
        y: 32474,
        floor: 10,
    };
    for (serial, creature, position) in [
        (
            1001,
            "oteryn:creature.dragon",
            MovementLocalPosition { x: 32200, ..origin },
        ),
        (1002, "oteryn:creature.dragon_lord", origin),
    ] {
        let target = capture(owner, admitted, creature, position, serial).await?;
        drain(owner, admitted, target).await?;
        assert_eq!(
            value(owner).await?,
            0,
            "wrong source origin or species grants no progress"
        );
    }
    let target = capture_origin(
        owner,
        admitted,
        "oteryn:creature.dragon",
        origin,
        1003,
        false,
    )
    .await?;
    drain(owner, admitted, target).await?;
    assert_eq!(
        value(owner).await?,
        0,
        "an encounter copy at the exact coordinate is not a native source spawn"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let bypass = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key: "oteryn:quest-transition/native-kill/fibula-cavern".into(),
                cause: crate::durability::quest_state::QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(9000).map_err(debug)?,
                )),
            },
            std::sync::Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
        )
        .await;
    assert!(matches!(
        bypass,
        Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)
    ));
    assert_eq!(
        value(owner).await?,
        0,
        "a command cannot manufacture a native kill"
    );
    for count in 1..=9_u16 {
        let (creature, position) = if count % 2 == 0 {
            (
                "oteryn:creature.dragon_lord",
                MovementLocalPosition {
                    x: 32242,
                    y: 32487,
                    floor: 10,
                },
            )
        } else {
            ("oteryn:creature.dragon", origin)
        };
        let target = capture(owner, admitted, creature, position, 1010 + count).await?;
        drain(owner, admitted, target).await?;
        assert_eq!(value(owner).await?, i64::from(count.min(8)));
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Drained
        );
        assert_eq!(
            value(owner).await?,
            i64::from(count.min(8)),
            "replay never increments twice"
        );
    }
    Ok(())
}
