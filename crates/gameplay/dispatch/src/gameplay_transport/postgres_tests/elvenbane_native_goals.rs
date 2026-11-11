//! Elvenbane's personal defender goals use source-native death occurrences.
use super::TestResult;
use super::fixture::{Harness, run};

#[test]
fn elvenbane_source_defender_goal_catalogue_is_closed() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "elvenbane_native_goals", false, false)
            .await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&h.pool)
            .await?;
        println!("ELVENBANE_NATIVE_GOALS POSTGRES_CASE_OBSERVED server_version_num={version}");
        let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        assert!(
            lowered
                .catalogue()
                .track("oteryn:quest-progress/elvenbane/native-defenders/dragon")
                .is_some(),
            "Elvenbane requires a bounded native defender goal, not a client-written completion flag"
        );
        h.cleanup().await?;
        Ok(())
    })?;
    println!(
        "ELVENBANE_NATIVE_GOALS POSTGRES_CASE_COMPLETED duration_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

use super::fixture::{debug, id};
use super::{AdmittedSession, ComposedFreshAdmission};
use crate::foundation::{
    CharacterLease, CommandId, CommandRef, ExactActorRef, MovementLocalPosition,
};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn elvenbane_native_defenders_capture_refuse_replay_and_cap_per_origin() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("elvenbane-native-goals", || {
        super::quest_world_callbacks::scenario(
            "elvenbane-native-goals",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 32598,
                y: 31641,
                floor: -8,
            },
            false,
        )
    })?;
    println!(
        "ELVENBANE_NATIVE_GOALS POSTGRES_CASE_COMPLETED duration_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

async fn value(owner: &ComposedFreshAdmission<'_, '_, '_>, name: &str) -> TestResult<i64> {
    let copy = owner
        .root
        .read_character_quest_state(
            owner.character,
            crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
        )
        .await
        .map_err(debug)?;
    Ok(copy
        .tracks()
        .get(&format!("oteryn:quest-progress/elvenbane/native-defenders/{name}"))
        .copied()
        .unwrap_or(if name == "phase" { -1 } else { 0 }))
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
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    println!("ELVENBANE_NATIVE_GOALS POSTGRES_CASE_OBSERVED server_version_num={version}");
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    for creature in [
        "dragon",
        "minotaur_mage",
        "orc_shaman",
        "elf_arcanist",
        "demon_skeleton",
    ] {
        native
            .reward_table()
            .row(&format!("oteryn:creature.{creature}"))
            .map_err(|reason| {
                format!(
                    "native defender ordinary settlement must be admitted: {creature} {reason:?}"
                )
            })?;
    }
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
    let origin = MovementLocalPosition {
        x: 32590,
        y: 31645,
        floor: 5,
    };
    let dragon = "oteryn:creature.dragon";
    let target = capture(owner, admitted, dragon, origin, 1000).await?;
    drain(owner, admitted, target).await?;
    assert_eq!(
        value(owner, "dragon").await?,
        0,
        "pre-activation death cannot advance the personal route"
    );
    super::desert_earned_route::walk_to(
        owner,
        admitted,
        oteryn_gameplay_quests::quest::world_callbacks::Position {
            x: 32598,
            y: 31641,
            floor: -8,
        },
    )
    .await?;
    assert_eq!(
        value(owner, "phase").await?,
        0,
        "literal source STEP activates the route once"
    );
    for (serial, creature, at) in [
        (1001, dragon, MovementLocalPosition { x: 32591, ..origin }),
        (1002, "oteryn:creature.elf_arcanist", origin),
    ] {
        let target = capture(owner, admitted, creature, at, serial).await?;
        drain(owner, admitted, target).await?;
        assert_eq!(
            value(owner, "dragon").await?,
            0,
            "wrong origin or species cannot advance a goal"
        );
    }
    let target = capture_origin(owner, admitted, dragon, origin, 1003, false).await?;
    drain(owner, admitted, target).await?;
    assert_eq!(
        value(owner, "dragon").await?,
        0,
        "an encounter copy cannot impersonate a native spawn"
    );
    let early = MovementLocalPosition {
        x: 32590,
        y: 31643,
        floor: 3,
    };
    let target = capture(owner, admitted, "oteryn:creature.elf_arcanist", early, 1004).await?;
    drain(owner, admitted, target).await?;
    assert_eq!(
        value(owner, "elf-arcanist").await?,
        0,
        "upper defender credit requires the mandatory Dragon first"
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
                transition_key: "oteryn:quest-transition/native-kill/elvenbane-dragon".into(),
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
        value(owner, "dragon").await?,
        0,
        "command causes cannot manufacture native deaths"
    );
    let targets = [
        ("dragon", dragon, origin),
        (
            "minotaur-mage",
            "oteryn:creature.minotaur_mage",
            MovementLocalPosition {
                x: 32592,
                y: 31643,
                floor: 4,
            },
        ),
        (
            "orc-shaman",
            "oteryn:creature.orc_shaman",
            MovementLocalPosition {
                x: 32590,
                y: 31647,
                floor: 4,
            },
        ),
        ("elf-arcanist", "oteryn:creature.elf_arcanist", early),
        (
            "demon-skeleton-east",
            "oteryn:creature.demon_skeleton",
            MovementLocalPosition {
                x: 32592,
                y: 31644,
                floor: 3,
            },
        ),
        (
            "demon-skeleton-west",
            "oteryn:creature.demon_skeleton",
            MovementLocalPosition {
                x: 32589,
                y: 31646,
                floor: 3,
            },
        ),
    ];
    for (index, (name, creature, at)) in targets.iter().copied().enumerate() {
        for repeat in 0..2 {
            let serial = 1020 + u16::try_from(index)? * 2 + repeat;
            let target = capture(owner, admitted, creature, at, serial).await?;
            drain(owner, admitted, target).await?;
            assert_eq!(
                value(owner, name).await?,
                1,
                "each exact origin is capped after its first death"
            );
            assert_eq!(
                owner.drain_kill_rewards(admitted.game_session_id).await,
                DrainOutcome::Drained
            );
            assert_eq!(
                value(owner, name).await?,
                1,
                "cancellation and repeated drains never duplicate progress"
            );
        }
        for (later, _, _) in &targets[index + 1..] {
            assert_eq!(
                value(owner, later).await?,
                0,
                "one defender never substitutes for another source origin"
            );
        }
    }
    let corpses: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key IN ('oteryn:item.tibia.i3990','oteryn:item.tibia.i4013','oteryn:item.tibia.i4025','oteryn:item.tibia.i4026','oteryn:item.tibia.i4027','oteryn:item.tibia.i4041','oteryn:item.tibia.i4042','oteryn:item.tibia.i4047','oteryn:item.tibia.i4048','oteryn:item.tibia.i4103','oteryn:item.tibia.i4104','oteryn:item.tibia.i4128','oteryn:item.tibia.i4147','oteryn:item.tibia.i4160','oteryn:item.tibia.i4161','oteryn:item.tibia.i5963','oteryn:item.tibia.i5973','oteryn:item.tibia.i5978','oteryn:item.tibia.i5981','oteryn:item.tibia.i6011')")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        corpses, 17,
        "each actual native death owns one physical corpse through its source decay chain; duplicate observations never mint twice"
    );
    println!(
        "ELVENBANE_NATIVE_GOALS six_exact_origins independent_caps frozen_principal source_step_activation predecessor_refusal cancellation_and_replay"
    );
    Ok(())
}
