//! L4: native lethal events, durable occurrence replay and threshold achievements.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{CharacterLease, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::sync::Arc;

const TRACK: &str = "oteryn:quest-progress/stampede/kills";
const CREATURE: &str = "oteryn:creature.terrified_elephant";

#[test]
fn quest_l4_stampede_native_five_kills_achievement_and_occurrence_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUEST_L4_POSTGRES_CASE_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "L4 occurrence scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    let started = std::time::Instant::now();
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "quest_l4_stampede", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content_and_capacity(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            7, // one player plus six deaths, including the saturation probe
        )?;
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: 7,
                        y: 0,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
        }
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        let session = admitted.game_session_id;
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
                deps.actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        // A dialogue/USE command cannot substitute for a native death occurrence.
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let forged = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                crate::durability::quest_state::QuestTransitionRequest {
                    transition_key: "oteryn:quest-transition/stampede/kill".into(),
                    cause: crate::durability::quest_state::QuestCause::Use(CommandRef::new(
                        session,
                        CommandId::new(90).map_err(debug)?,
                    )),
                },
                Arc::clone(owner.quest_catalogue.as_ref().ok_or("quest catalogue")?),
            )
            .await;
        assert!(matches!(
            forged,
            Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)
        ));
        drop(slot);
        for ordinal in 1..=6 {
            let queue = owner.states.attack.lock().await.kills().clone();
            let observed = owner.owner_now();
            let now = observed.get() / 1000;
            {
                let mut runtime = owner.runtime.lock().await;
                let at = runtime
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position();
                let policy = runtime.companion_policy(CREATURE).map_err(debug)?;
                let target = runtime
                    .admit_encounter_actor(
                        CREATURE,
                        policy.maximum_health,
                        MovementLocalPosition { x: at.x + 1, ..at },
                    )
                    .map_err(debug)?;
                runtime
                    .install_creature_policy(target, CREATURE)
                    .map_err(debug)?;
                let lease = runtime
                    .bound_attacker_lease(deps.actor, session)
                    .map_err(debug)?;
                runtime
                    .borrow_exact_actor_commit_at(now)
                    .commit_damage_for_bound_attacker(
                        target,
                        deps.actor,
                        CommandRef::new(session, CommandId::new(100 + ordinal).map_err(debug)?),
                        0,
                        crate::foundation::OwnerDamageCommand {
                            target: CREATURE.as_bytes(),
                            occurrence: session.as_bytes(),
                            binding: b"quest-l4-native-death",
                            damage: policy.maximum_health,
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
                            actor: deps.actor,
                        },
                        now,
                        Some(observed),
                    );
                }
            }
            assert!(
                queue.has_session(session),
                "the lethal event must enter the real queue"
            );
            let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) =
                queue.take(session, fence.character_lease_generation)
            else {
                return Err("captured native occurrence".into());
            };
            let replay = taken.entry().ok_or("native entry")?.clone();
            drop(taken);
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            eprintln!(
                "QUEST_L4_STAMPEDE POSTGRES_ASSERTION_REACHED kill={ordinal} duration_ms={}",
                started.elapsed().as_millis()
            );
            assert!(
                crate::gameplay_transport::quest_kills::settle(&owner, &replay).await,
                "durable occurrence replay completes without a second increment"
            );
            let mut stale = replay.clone();
            stale.facts.principal.lease_generation += 1;
            assert!(
                crate::gameplay_transport::quest_kills::settle(&owner, &stale).await,
                "a stale captured lease is terminal without a write"
            );
            let copy = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(
                copy.tracks().get(TRACK).copied().unwrap_or(0),
                ordinal.min(5) as i64,
                "one native occurrence, one increment, capped at five"
            );
            let achievements: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/trail_of_the_ape_god'").fetch_one(&h.pool).await?;
            assert_eq!(
                achievements,
                i64::from(ordinal >= 5),
                "achievement commits at exactly the fifth kill"
            );
        }
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/stampede/kill'").fetch_one(&h.pool).await?;
        assert_eq!(receipts, 5);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })?;
    eprintln!(
        "QUEST_L4_STAMPEDE POSTGRES_CASE_COMPLETED duration_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
