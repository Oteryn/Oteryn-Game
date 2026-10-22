//! Confirmed NPC choice through the real dispatch, followed by replay and root reopen.
use super::fixture::{Harness, debug, id, run};
use super::tutorial_vitals_fixture::admit;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{BuildChangeRequest, BuildFormula};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::vocation_choice::DAWNPORT_CHOICE_NPC;
use crate::spell::Vocation;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::{ExactI64, SemanticTimeMicros};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn first_vocation_vitals_dispatch_preserves_pools_once_and_restores_after_restart_postgres()
-> TestResult {
    super::kill_reward_e2e::scenario_thread::run("tutorial-first-vocation", || {
        scenario("tutorial_first_vocation", 70, 30)
    })
}

#[test]
fn first_vocation_vitals_full_pools_do_not_refill_after_restart_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("tutorial-first-vocation-maxima", || {
        scenario("tutorial_first_vocation_maxima", 245, 150)
    })
}

// Each scenario owns its tag: the harness derives its retained recovery directory from the
// tag, so two tests sharing one tag remove each other's directory when they run in parallel.
fn scenario(tag: &str, health: u32, mana: u32) -> TestResult {
    run(async |admin| {
        let mut h = Harness::create_with_admission_seed(admin, tag, false, false).await?;
        let request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let (actor, session) = (dependencies.actor, dependencies.admitted.game_session_id);
        let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
            dependencies.admitted.world_id,
            "content-1",
        )?;
        let active = controller.active().ok_or("active generation")?;
        let mut owner = dependencies.owner(&h, &authority);
        owner.active_generation = Some(active);
        let mut admitted = dependencies.admitted;
        Box::pin(owner.admit_progression_session(&mut admitted)).await;
        let progression = owner
            .progression_binding(session)
            .ok_or("progression binding")?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence read")?
            .ok_or("fence")?;
        h.root
            .commit_character_experience(
                &authority,
                &h.node,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                    amount: ExactI64::new(98_800),
                    context: progression.context.clone(),
                    policy_revision: progression.policy_revision.clone(),
                    reward_revision: progression.reward_revision.clone(),
                    policy: progression.policy.clone(),
                },
            )
            .await
            .map_err(debug)?;
        // The fixture has no durably activated equipment catalogue; exercise the production
        // Character/vitals admission without unrelated equipment/familiar admission writes.
        owner.active_generation = None;
        admit(&owner, &dependencies, &request).await?;
        owner.active_generation = Some(active);
        let npc = {
            let mut runtime = dependencies.runtime.lock().await;
            let at = runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: DAWNPORT_CHOICE_NPC.into(),
                            placements: vec![NpcPlacement {
                                x: u16::try_from(at.x + 1)?,
                                y: u16::try_from(at.y)?,
                                floor: -i8::try_from(at.floor)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.visible_entities().npcs[0];
            let mut states = owner.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("vitals")?;
            assert_eq!(state.character_facts().vocation, Vocation::None);
            assert_eq!(
                (state.vitals().max_health, state.vitals().max_mana),
                (245, 150)
            );
            state.restore_saved_vitals(health, mana);
            let formula = active
                .native_gameplay()
                .and_then(|native| native.training_formula())
                .ok_or("training formula")?;
            state
                .enable_owned_training(
                    &crate::durability::character_build::DurableBuildState::default(),
                    formula,
                )
                .map_err(debug)?;
            state
                .train_skill(
                    oteryn_gameplay_progression::hit_training::SkillFamily::Sword,
                    1,
                    formula,
                    |_| 3,
                )
                .map_err(debug)?;
            assert!(state.has_training_to_flush());
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        owner.chat.admit(
            session,
            "Tutorial candidate",
            SemanticTimeMicros::from_micros(0),
        );
        owner.chat.set_npc_service(session);
        owner
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::default()));
        for (command_id, text) in [(1, "hi"), (2, "knight")] {
            owner.clock_origin = Instant::now()
                .checked_sub(Duration::from_secs(command_id))
                .ok_or("clock")?;
            let result = Box::pin(owner.npc_talk(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: None,
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
            ))
            .await;
            assert_eq!(result.disposition, NpcIntentDisposition::Ok, "{text}");
        }
        // Pause at the actual dialogue/commit boundary to retain the topic that a lost
        // acknowledgement would leave. The subsequent commit still uses `npc_talk`.
        owner.clock_origin = Instant::now()
            .checked_sub(Duration::from_secs(3))
            .ok_or("clock")?;
        {
            let runtime = dependencies.runtime.lock().await;
            let ordinary = owner.npc_conversations.talk_in(
                &owner.chat,
                &runtime,
                actor,
                session,
                &NpcTalkIntent {
                    npc_actor: npc,
                    text: "yes".into(),
                },
                owner.owner_now(),
                None,
            );
            assert_eq!(ordinary.disposition, NpcIntentDisposition::Ok);
        }
        assert_eq!(
            owner.save_spell_training(actor, session, true).await,
            crate::gameplay_transport::actor_spell::TrainingSave::Saved
        );
        let topic = owner
            .npc_conversations
            .vocation_topic_for_test(session)
            .ok_or("confirmed topic")?;
        let live_before = {
            let runtime = dependencies.runtime.lock().await;
            owner
                .spell_states
                .lock()
                .await
                .get(&runtime, actor, session)
                .ok_or("prechoice state")?
                .clone()
        };
        owner.clock_origin = Instant::now()
            .checked_sub(Duration::from_secs(4))
            .ok_or("clock")?;
        let chosen = Box::pin(owner.npc_talk(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: 3,
                item_fence: None,
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: "yes".into(),
            },
        ))
        .await;
        assert_eq!(chosen.disposition, NpcIntentDisposition::Ok);
        let (installed_revision, occurrence) = {
            let runtime = dependencies.runtime.lock().await;
            let states = owner.spell_states.lock().await;
            let state = states
                .get(&runtime, actor, session)
                .ok_or("converted vitals")?;
            assert_eq!(state.character_facts().vocation, Vocation::Knight);
            assert_eq!(
                (
                    state.vitals().health,
                    state.vitals().mana,
                    state.vitals().max_health
                ),
                (health, mana, 365)
            );
            let occurrence: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(build_occurrence_id) FROM game_character_build_receipts WHERE cause='vocation_choice'").fetch_one(&h.pool).await?;
            (state.revision(), occurrence)
        };
        let occurrence = crate::durability::character_build::BuildOccurrence::from_bytes(
            occurrence.try_into().map_err(|_| "occurrence bytes")?,
        )
        .map_err(debug)?;
        let receipt = h
            .root
            .reconcile_character_build(&authority, occurrence)
            .await
            .map_err(debug)?
            .ok_or("committed choice")?;
        assert_eq!(
            receipt.before.skills()[2].1,
            3,
            "prechoice training was flushed"
        );
        let mut fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("current session")?;
        fence.expected_character_revision = receipt.original_character_revision;
        let retained = crate::gameplay_transport::vocation_choice::RetainedNpcVocation {
            fence,
            request: BuildChangeRequest {
                occurrence,
                cause: receipt.cause,
                before: receipt.before,
                after: receipt.after,
                pruned_stance: receipt.pruned_stance,
            },
            formula_digest: active
                .native_gameplay()
                .and_then(|native| native.training_formula())
                .ok_or("formula")?
                .digest(),
        };
        // Replay both after installation and after a commit whose live installation was lost.
        for (command_id, restore_before) in [(4, false), (5, true)] {
            if restore_before {
                let runtime = dependencies.runtime.lock().await;
                *owner
                    .spell_states
                    .lock()
                    .await
                    .get_mut(&runtime, actor, session)
                    .ok_or("prechoice actor")? = live_before.clone();
            }
            let mut replay = topic.clone();
            assert!(replay.retain(occurrence, retained.clone()));
            assert!(
                owner
                    .npc_conversations
                    .restore_vocation_topic_for_test(session, replay)
            );
            owner.clock_origin = Instant::now()
                .checked_sub(Duration::from_secs(command_id + 1))
                .ok_or("clock")?;
            let result = Box::pin(owner.npc_talk(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: None,
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "yes".into(),
                },
            ))
            .await;
            assert_eq!(result.disposition, NpcIntentDisposition::Ok);
            let runtime = dependencies.runtime.lock().await;
            let states = owner.spell_states.lock().await;
            let state = states
                .get(&runtime, actor, session)
                .ok_or("converted vitals")?;
            assert_eq!(state.revision(), installed_revision);
            assert_eq!((state.vitals().health, state.vitals().mana), (health, mana));
        }
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_character_build_receipts WHERE cause='vocation_choice'"
            )
            .fetch_one(&h.pool)
            .await?,
            1
        );
        drop(owner);
        drop(authority);
        h.root = super::fixture::tutorial_vitals_fixture::reopen_root(&h)?;
        assert!(h.root.maintain_ready_once().await?);
        let reopened = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(
            h.root
                .reconcile_character_build(&reopened, occurrence)
                .await
                .map_err(debug)?
                .is_some()
        );
        let restarted = Dependencies::new()?;
        let owner = restarted.owner(&h, &reopened);
        admit(&owner, &restarted, &request).await?;
        {
            let runtime = restarted.runtime.lock().await;
            let states = owner.spell_states.lock().await;
            let state = states
                .get(&runtime, restarted.actor, session)
                .ok_or("restored classed vitals")?;
            assert_eq!(state.character_facts().vocation, Vocation::Knight);
            assert_eq!((state.vitals().health, state.vitals().mana), (health, mana));
        }
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_character_build_receipts WHERE cause='vocation_choice'"
            )
            .fetch_one(&h.pool)
            .await?,
            1
        );
        drop(owner);
        drop(reopened);
        drop(seal);
        h.cleanup().await
    })
}
