//! Native kill bindings: source activation, fenced replay and death-only admission.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{CharacterLease, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::sync::Arc;

#[path = "quest_l4_counters/tutorial.rs"]
mod tutorial;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CounterMode {
    Normal,
    Burst,
    WithoutQuestCatalogue,
    Tutorial,
    TutorialBurst,
}

impl CounterMode {
    fn tutorial(self) -> bool {
        matches!(self, Self::Tutorial | Self::TutorialBurst)
    }
    fn burst(self) -> bool {
        matches!(self, Self::Burst | Self::TutorialBurst)
    }
}

fn run_counter(
    creature: &'static str,
    track: &'static str,
    transition: &'static str,
    start: &'static str,
    mode: CounterMode,
) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUEST_L4_COUNTERS_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            counter_scenario(creature, track, transition, start, mode).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native counters panicked")??;
    Ok(())
}

#[test]
fn quest_l4_first_dragon_native_counter_requires_death_and_deduplicates() -> TestResult {
    run_counter(
        "oteryn:creature.dragon",
        "oteryn:quest-progress/quest/u11_02/the_first_dragon/dragon_counter",
        "oteryn:quest-transition/native-kills/first-dragon",
        "oteryn:quest-transition/quest/u11_02/the_first_dragon/questline/npc_1",
        CounterMode::Normal,
    )
}

#[test]
fn quest_l4_beginning_native_deaths_cannot_substitute_for_leg_handin() -> TestResult {
    run_counter(
        "oteryn:creature.cockroach",
        "oteryn:quest-progress/native-kills/beginning/cockroaches",
        "oteryn:quest-transition/native-kills/beginning",
        "oteryn:quest-transition/quest/u8_2/the_beginning_quest/santiago_quest_log/npc_4",
        CounterMode::Normal,
    )
}

#[test]
fn quest_l4_threatened_native_counter_requires_death_and_deduplicates() -> TestResult {
    run_counter(
        "oteryn:creature.enfeebled_silencer",
        "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/enfeebled_count",
        "oteryn:quest-transition/native-kills/threatened-enfeebled",
        "oteryn:quest-transition/threatened-dreams/maelyrra-start",
        CounterMode::Normal,
    )
}

#[test]
fn quest_l4_three_queued_dragon_deaths_keep_the_original_mission_epoch() -> TestResult {
    run_counter(
        "oteryn:creature.dragon",
        "oteryn:quest-progress/quest/u11_02/the_first_dragon/dragon_counter",
        "oteryn:quest-transition/native-kills/first-dragon",
        "oteryn:quest-transition/quest/u11_02/the_first_dragon/questline/npc_1",
        CounterMode::Burst,
    )
}

#[test]
fn quest_l4_inactive_dragon_without_quest_catalogue_does_not_block_drain() -> TestResult {
    run_counter(
        "oteryn:creature.dragon",
        "oteryn:quest-progress/quest/u11_02/the_first_dragon/dragon_counter",
        "oteryn:quest-transition/native-kills/first-dragon",
        "oteryn:quest-transition/quest/u11_02/the_first_dragon/questline/npc_1",
        CounterMode::WithoutQuestCatalogue,
    )
}

fn counter_scenario(
    creature: &'static str,
    track: &'static str,
    transition: &'static str,
    start: &'static str,
    mode: CounterMode,
) -> TestResult {
    let started = std::time::Instant::now();
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "quest_l4_counters", false, false).await?;
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
            4, // one player plus three native deaths
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
        if mode.tutorial() {
            owner.quest_catalogue = Some(tutorial::catalogue(
                owner.quest_catalogue.as_deref().ok_or("catalogue")?,
            )?);
        }
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
        if mode != CounterMode::WithoutQuestCatalogue {
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let begun = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    crate::durability::quest_state::QuestTransitionRequest {
                        transition_key: start.into(),
                        cause: crate::durability::quest_state::QuestCause::Use(CommandRef::new(
                            session,
                            CommandId::new(89).map_err(debug)?,
                        )),
                    },
                    Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
                )
                .await
                .map_err(debug)?;
            assert!(matches!(
                begun,
                crate::durability::quest_state::QuestTransitionOutcome::Committed(_)
            ));
            drop(slot);
        }
        owner.refresh_quest_session(session, false).await;
        assert!(
            owner
                .refresh_source_party_presence(deps.actor, session)
                .await
        );
        if mode == CounterMode::WithoutQuestCatalogue {
            owner.quest_catalogue = None;
            assert!(owner.quest_catalogue.is_none());
        }
        for ordinal in 1..=3 {
            let queue = owner.states.attack.lock().await.kills().clone();
            let observed = owner.owner_now();
            let now = observed.get() / 1000;
            {
                let mut runtime = owner.runtime.lock().await;
                let at = runtime
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position();
                let policy = runtime.companion_policy(creature).map_err(debug)?;
                let target = runtime
                    .admit_encounter_actor(
                        creature,
                        policy.maximum_health,
                        MovementLocalPosition { x: at.x + 1, ..at },
                    )
                    .map_err(debug)?;
                runtime
                    .install_creature_policy(target, creature)
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
                            target: creature.as_bytes(),
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
            eprintln!(
                "QUEST_L4_COUNTERS POSTGRES_ASSERTION_REACHED creature={creature} phase=native_capture duration_ms={}",
                started.elapsed().as_millis()
            );
            // Santiago consumes three physical legs (santiago.lua:88-96).
            // A death without ordinary corpse admission cannot invent those Items
            // or complete the hand-in through a synthetic scalar counter.
            if creature == "oteryn:creature.cockroach" && !mode.tutorial() {
                // Once the creature has a native binding, an ineligible death
                // may enter the known-empty queue. It must drain without value.
                assert_eq!(
                    owner.drain_kill_rewards(session).await,
                    DrainOutcome::Drained
                );
                assert!(!queue.has_session(session));
                let copy = h
                    .root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                assert!(!copy.tracks().contains_key(track));
                assert_eq!(
                    copy.tracks().get(
                        "oteryn:quest-progress/quest/u8_2/the_beginning_quest/santiago_quest_log"
                    ),
                    Some(&4)
                );
                let legs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i7882' AND lifecycle=1")
                    .fetch_one(&h.pool).await?;
                assert_eq!(
                    legs, 0,
                    "counter admission cannot fabricate physical hand-in items"
                );
                continue;
            }
            assert!(
                queue.has_session(session),
                "the lethal event must enter the real queue"
            );
            // Area damage can enqueue several deaths from one mission snapshot.
            // Earlier counter writes must not be mistaken for a mission reset.
            if mode.burst() && ordinal < 3 {
                continue;
            }
            let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) =
                queue.take(session, fence.character_lease_generation)
            else {
                return Err("captured native occurrence".into());
            };
            let replay = taken.entry().ok_or("native entry")?.clone();
            drop(taken);
            if mode == CounterMode::WithoutQuestCatalogue {
                assert!(
                    replay
                        .grizzly
                        .as_ref()
                        .is_some_and(|frozen| frozen.native.is_empty()),
                    "an unstarted mission has no frozen eligible recipient"
                );
            }

            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            eprintln!(
                "QUEST_L4_COUNTERS POSTGRES_ASSERTION_REACHED kill={ordinal} duration_ms={}",
                started.elapsed().as_millis()
            );
            assert!(
                crate::gameplay_transport::quest_kills::settle(&owner, &replay).await,
                "durable occurrence replay completes without a second increment"
            );
            let mut stale = replay.clone();
            stale.facts.principal.lease_generation += 1;
            stale.facts.occurrence = id(99);
            if let Some(frozen) = &mut stale.grizzly {
                for recipient in &mut Arc::make_mut(frozen).native {
                    recipient.lease_generation += 1;
                }
            }
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
                copy.tracks().get(track).copied().unwrap_or(0),
                if mode == CounterMode::WithoutQuestCatalogue {
                    0
                } else {
                    ordinal as i64
                },
                "one native occurrence, one increment"
            );
            if mode.tutorial() {
                tutorial::assert_goal(copy.tracks(), ordinal);
            }
        }
        if mode != CounterMode::WithoutQuestCatalogue {
            // A dialogue/USE command cannot substitute for a native death occurrence.
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let forged = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    crate::durability::quest_state::QuestTransitionRequest {
                        transition_key: transition.into(),
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
        }
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind(transition)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            receipts,
            if (creature == "oteryn:creature.cockroach" && !mode.tutorial())
                || mode == CounterMode::WithoutQuestCatalogue
            {
                0
            } else {
                3
            }
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })?;
    eprintln!(
        "QUEST_L4_COUNTERS POSTGRES_CASE_COMPLETED creature={creature} mode={mode:?} duration_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
