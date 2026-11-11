//! Fugue native body → most-damage source party → existing fenced quest writer.
//! Coordinates and the pre-existing source mission stage are explicit fixture data.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition, NodeId};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestComparison, loader::load_embedded_quest_state, source_deaths,
};
use std::sync::Arc;

#[test]
fn engenc2_fugue_native_death_settles_kesar_and_resets_attempt_postgres() -> TestResult {
    scenario("engenc2-fugue", "fugue")
}

fn scenario(name: &'static str, _creature: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        run(async |admin| {
            let counter = oteryn_gameplay_quests::quest::native_kills::counter(
                "oteryn:quest-transition/native-kills/fugue",
            )
            .ok_or("Fugue counter")?;
            let mut rule: source_deaths::Rule = serde_json::from_value(serde_json::json!({
                "creature": counter.creature, "transition": counter.transition, "room": null,
                "define_transition": false, "effects": [
                    {"track": counter.track, "default": -1, "from": {"kind": "eq", "value": 2}, "set": 3},
                    {"track": "oteryn:quest-progress/crystalserver/quest/u12_40/the_order_of_the_lion/fugue_spawned", "default": -1, "from": {"kind": "any"}, "set": 0}
                ]
            }))?;
            rule.quest = counter.quest.into();
            let h =
                Harness::create_with_admission_seed(admin, "engenc2_fugue", false, false).await?;
            let result = exercise(&h, &rule).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup?;
            Ok(())
        })
    });
    assert!(started.elapsed().as_secs_f64() > 0.05);
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn exercise(h: &Harness, rule: &source_deaths::Rule) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activated) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("generation")?;
    let native = generation.native_gameplay().ok_or("native policies")?;
    let position = rule
        .room
        .map_or([100, 100, -7], |[x, y, _, _, z]| [x, y, z]);
    let frame = activated.frame_binding().digest();
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        frame,
        [4; 32],
        (position[0], position[1], i16::try_from(-position[2])?),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        64,
        pin,
    )
    .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(
            deps.actor,
            MovementLocalPosition {
                x: position[0],
                y: position[1],
                floor: i16::try_from(-position[2])?,
            },
        )
        .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    let catalogue = load_embedded_quest_state("content-1")
        .map_err(debug)?
        .catalogue()
        .clone();
    // The fight starts at its source mission precondition. Its protected tracks
    // are historical fixture state, not authority for a generic quest command.
    let native_transition = catalogue
        .transition(&rule.transition)
        .cloned()
        .ok_or("source transition")?;
    let mut seeds = Vec::new();
    for effect in &native_transition.effects {
        if let QuestComparison::Eq(value) = effect.from
            && catalogue
                .track(&effect.track)
                .is_some_and(|t| t.initial != value)
        {
            seeds.push((effect.track.clone(), value));
        }
    }
    seeds.push((
        "oteryn:quest-progress/crystalserver/quest/u12_40/the_order_of_the_lion/fugue_spawned"
            .into(),
        1,
    ));
    let catalogue = Arc::new(catalogue);
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    let mut admitted = deps.admitted;
    admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
    let fresh = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fresh actor fence")?
        .ok_or("fresh actor fence")?;
    // Match first_entry's predecessor: the current SQL lease, not a record-derived default.
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            deps.actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fresh.character_id.as_bytes())
                    .map_err(debug)?,
                fresh.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.item_fence = Some(fresh.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    let fresh_request = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fresh.item_fence(),
            generation.identity().server_artifact_digest(),
            fresh_request.operation(),
        )
        .await
        .map_err(debug)?;
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    let session = admitted.game_session_id;
    if !seeds.is_empty() {
        super::qengine_outfit_stories::seed_quest_prerequisites(h, &owner, &admitted, &seeds, 91)
            .await?;
    }
    // A client-style command cannot substitute for the captured native death.
    {
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "negative fence")?
            .ok_or("negative fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let generic = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: rule.transition.clone(),
                    cause: QuestCause::Command(CommandRef::new(
                        session,
                        CommandId::new(2).map_err(debug)?,
                    )),
                },
                Arc::clone(&catalogue),
            )
            .await;
        assert!(matches!(generic, Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)), "generic Fugue close: {generic:?}");
    }
    owner.admit_quest_session(&admitted).await;
    owner.refresh_quest_session(session, false).await;
    let actor = deps.actor;
    let queue = owner.states.attack.lock().await.kills().clone();
    let kills = if rule.creature == "oteryn:creature.cake_golem" {
        20usize
    } else {
        1
    };
    let actual_kills = if kills == 20 { 21 } else { 1 };
    // Two deaths share each frozen quest snapshot. A preceding counter increment
    // must not invalidate the next queued death from the same admitted mission run.
    for batch in (0..actual_kills).step_by(2) {
        owner.refresh_quest_session(session, false).await;
        assert!(owner.refresh_source_party_presence(actor, session).await);
        for ordinal in batch..(batch + 2).min(actual_kills) {
            let observed = owner.owner_now();
            let now = observed.get() / 1000;
            let mut runtime = owner.runtime.lock().await;
            let policy = runtime.companion_policy(&rule.creature).map_err(debug)?;
            assert_eq!(policy.maximum_health, 17000);
            let target = runtime
                .admit_encounter_actor(
                    &rule.creature,
                    policy.maximum_health,
                    MovementLocalPosition {
                        x: position[0] + 1,
                        y: position[1],
                        floor: i16::try_from(-position[2])?,
                    },
                )
                .map_err(debug)?;
            runtime
                .install_creature_policy(target, &rule.creature)
                .map_err(debug)?;
            let lease = runtime
                .bound_attacker_lease(actor, session)
                .map_err(debug)?;
            // The actual admitted native body heals through the shared physical owner.
            runtime
                .borrow_exact_actor_commit_at(now)
                .commit_damage_for_bound_attacker(
                    target,
                    actor,
                    CommandRef::new(session, CommandId::new(9).map_err(debug)?),
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: rule.creature.as_bytes(),
                        occurrence: session.as_bytes(),
                        binding: b"fugue-before-heal",
                        damage: 2000,
                    },
                )
                .map_err(debug)?;
            let source = native
                .creature_profiles()
                .records
                .iter()
                .find(|row| row.profile.target.key == rule.creature)
                .ok_or("Fugue native body")?;
            let crate::content::ProjectV2AuthoringProfileData::Behavior(behavior) =
                &source.behavior.as_ref().ok_or("Fugue behavior")?.data
            else {
                return Err("Fugue behavior kind".into());
            };
            let placement = target
                .placement_identity()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            let revisions = crate::ability::RevisionSet::new(
                "ruleset-1",
                "content-1",
                "world-1",
                "definition-r1",
                "sim-1",
            )
            .map_err(debug)?;
            let proposal = crate::creature_auto_attack::ProfileAbilityProposal {
                issuer: target,
                target,
                ability: behavior.defenses[0].ability.clone(),
                list: crate::ai_think::profile_schedule::ScheduleList::Defence,
                entry_index: 0,
                magnitude: None,
                range_tiles: 0,
                occurrence: crate::ability::AbilityOccurrence::new(
                    &format!("ai-profile:{placement}:5:defence:0"),
                    revisions,
                )
                .map_err(debug)?,
                intent: crate::ability::AiAbilityAdapter::normalize("fugue", &["fugue"])
                    .map_err(debug)?,
            };
            let mut heal_ledger = crate::foundation::CreatureSelfHealLedger::default();
            let healed = crate::gameplay_transport::monster_profile_cycle::source_heals::apply(
                &mut runtime,
                &mut heal_ledger,
                native,
                &proposal,
            )
            .map_err(debug)?;
            assert!(healed.applied);
            assert!((15200..=15650).contains(&healed.health_after));
            let replay = crate::gameplay_transport::monster_profile_cycle::source_heals::apply(
                &mut runtime,
                &mut heal_ledger,
                native,
                &proposal,
            )
            .map_err(debug)?;
            assert!(!replay.applied);
            let mut stale_heal = proposal.clone();
            stale_heal.occurrence = crate::ability::AbilityOccurrence::new(
                &format!("ai-profile:{placement}:4:defence:0"),
                proposal.occurrence.revisions().clone(),
            )
            .map_err(debug)?;
            assert!(matches!(
                crate::gameplay_transport::monster_profile_cycle::source_heals::apply(
                    &mut runtime,
                    &mut heal_ledger,
                    native,
                    &stale_heal
                ),
                Err(crate::foundation::CarrierError::StaleAttackerSequence)
            ));

            assert_eq!(
                runtime.companion_snapshot(target).map_err(debug)?.health,
                healed.health_after
            );
            let health = runtime.companion_snapshot(target).map_err(debug)?.health;
            runtime
                .borrow_exact_actor_commit_at(now)
                .commit_damage_for_bound_attacker(
                    target,
                    actor,
                    CommandRef::new(
                        session,
                        CommandId::new(10 + u64::try_from(ordinal)?).map_err(debug)?,
                    ),
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: rule.creature.as_bytes(),
                        occurrence: session.as_bytes(),
                        binding: b"srvenc-source-native-death",
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
        }
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
    }
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    for effect in &rule.effects {
        let value:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
            .bind(id(41).as_slice()).bind(&effect.track).fetch_optional(&h.pool).await?;
        let before = seeds
            .iter()
            .find(|(track, _)| track == &effect.track)
            .map_or(effect.default, |(_, value)| *value);
        let expected = effect
            .set
            .unwrap_or(before + effect.add.unwrap_or(0) * i64::try_from(kills)?);
        assert_eq!(
            value,
            Some(expected),
            "native {} must commit {}",
            rule.creature,
            effect.track
        );
    }
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&rule.transition)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        receipts,
        i64::try_from(kills)?,
        "duplicate corpse projection must retain the first captured receipt"
    );
    Ok(())
}
