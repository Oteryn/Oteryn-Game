//! Actual native lethal capture → existing queue → fenced source quest progress.
//! Coordinates and the pre-existing source mission stage are explicit fixture data.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition, NodeId};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    loader::load_embedded_quest_state, source_deaths,
};
use std::sync::Arc;

macro_rules! native_case {
    ($name:ident,$creature:literal) => {
        #[test]
        fn $name() -> TestResult {
            scenario(stringify!($name), $creature)
        }
    };
}
native_case!(srvenc_m24_postgres_anomaly_room, "anomaly");
native_case!(srvenc_m24_postgres_world_devourer_room, "world_devourer");
native_case!(srvenc_m24_postgres_court_warlock_room, "court_warlock");
native_case!(srvenc_m24_postgres_goblin_leader_damage, "goblin_leader");
native_case!(srvenc_m24_postgres_magma_bubble_damage, "magma_bubble");
native_case!(srvenc_m24_postgres_cake_golem_twenty_and_cap, "cake_golem");
native_case!(srvenc_m24_postgres_omruc_stage, "omruc");
native_case!(srvenc_m24_postgres_thalas_stage, "thalas");
native_case!(srvenc_m24_postgres_dipthrah_stage, "dipthrah");
native_case!(srvenc_m24_postgres_mahrdis_stage, "mahrdis");
native_case!(srvenc_m24_postgres_vashresamun_stage, "vashresamun");
native_case!(srvenc_m24_postgres_morguthis_stage, "morguthis");
native_case!(srvenc_m24_postgres_rahemos_stage, "rahemos");
native_case!(srvenc_m24_postgres_herald_top_damage, "herald_of_fire");

fn scenario(name: &'static str, creature: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        run(async |admin| {
            let rule = source_deaths::rules()?
                .iter()
                .find(|r| r.creature == format!("oteryn:creature.{creature}"))
                .ok_or("source rule")?;
            let h = Harness::create_with_admission_seed(admin, "srvenc_m24", false, false).await?;
            let result = exercise(&h, rule).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup?;
            Ok(())
        })
    });
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
    // The fight starts at its source mission precondition. Seed that explicit
    // fixture stage through the same fenced writer; never update tracks by SQL.
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
    let seed_key = format!(
        "oteryn:quest-transition/srvenc-fixture/seed-{}",
        rule.creature.rsplit('.').next().ok_or("creature")?
    );
    let catalogue = if seeds.is_empty() {
        Arc::new(catalogue)
    } else {
        let mut transitions = vec![native_transition];
        transitions.push(QuestTransition {
            key: seed_key.clone(),
            quest: rule.quest.clone(),
            effects: seeds
                .iter()
                .map(|(track, value)| QuestEffect {
                    track: track.clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(*value),
                })
                .collect(),
            completes: false,
            experience: None,
            world_change: None,
        });
        let tracks = rule
            .effects
            .iter()
            .map(|e| catalogue.track(&e.track).cloned().ok_or("source track"))
            .collect::<Result<Vec<_>, _>>()?;
        Arc::new(QuestStateCatalogue::new("content-1", tracks, transitions).map_err(debug)?)
    };
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
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: seed_key,
                    cause: QuestCause::Command(CommandRef::new(
                        session,
                        CommandId::new(1).map_err(debug)?,
                    )),
                },
                Arc::clone(&catalogue),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(outcome, QuestTransitionOutcome::Committed(_)),
            "seed: {outcome:?}"
        );
    }
    // Neither a generic command nor a detached source-generation snapshot is
    // authority for progress, even at the correct source mission stage.
    {
        use crate::durability::character_progression::{
            CharacterProgressionError, ExperienceRewardOccurrence,
        };
        use crate::durability::quest_state::{
            native_kills::{NativeDeathSource, NativeQuestDeath},
            source_deaths::SourceDeathContext,
        };
        use oteryn_gameplay_quests::quest::QuestRefusal;
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
        assert!(
            matches!(generic, Err(CharacterProgressionError::InvalidInput)),
            "generic source-death request: {generic:?}"
        );
        let detached = NativeQuestDeath {
            transition: rule.transition.clone(),
            creature: rule.creature.clone(),
            observed_revision: fence.expected_character_revision.get(),
            definition_hash: None,
            source: NativeDeathSource::Source(SourceDeathContext {
                scope: fence.runtime_scope,
                scope_generation: fence.scope_ownership_generation.get() + 1,
                character: *fence.character_id.as_bytes(),
                session,
                lease_generation: fence.character_lease_generation,
                position,
                dealt_damage: true,
                top_damage_character: rule
                    .top_damage_only
                    .then_some(*fence.character_id.as_bytes()),
            }),
        };
        let outcome = slot
            .commit_native_quest_kill(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestCause::CreatureDeath(
                    ExperienceRewardOccurrence::from_bytes(id(0xa5)).map_err(debug)?,
                ),
                &detached,
                Arc::clone(&catalogue),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(
                outcome,
                QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
            ),
            "detached native source generation: {outcome:?}"
        );
    }
    owner.admit_quest_session(&admitted).await;
    owner.refresh_quest_session(session, false).await;
    let actor = deps.actor;
    let queue = owner.states.attack.lock().await.kills().clone();
    // This second current native actor really deals damage. It remains an invalid
    // Herald recipient even with the same admitted mission snapshot as the winner.
    let lesser_contributor = if rule.top_damage_only {
        let mut runtime = owner.runtime.lock().await;
        let session = crate::foundation::GameSessionId::decode(&id(77)).map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                actor,
                MovementLocalPosition {
                    x: position[0] - 1,
                    y: position[1],
                    floor: i16::try_from(-position[2])?,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(78)).map_err(debug)?,
                    1,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        Some((actor, session, id(78), 1))
    } else {
        None
    };
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
            let health = runtime.companion_snapshot(target).map_err(debug)?.health;
            let damage = if let Some((other_actor, other_session, _, _)) = lesser_contributor {
                assert!(
                    health > 2,
                    "Herald has distinct most-damage and lesser contributors"
                );
                runtime
                    .borrow_exact_actor_commit_at(now)
                    .commit_damage_for_bound_attacker(
                        target,
                        other_actor,
                        CommandRef::new(other_session, CommandId::new(1).map_err(debug)?),
                        0,
                        crate::foundation::OwnerDamageCommand {
                            target: rule.creature.as_bytes(),
                            occurrence: other_session.as_bytes(),
                            binding: b"srvenc-source-lesser-damage",
                            damage: 1,
                        },
                    )
                    .map_err(debug)?;
                health - 1
            } else {
                health
            };
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
                        damage,
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
            if let Some((other_actor, other_session, other_character, other_lease)) =
                lesser_contributor
            {
                use crate::gameplay_transport::{kill_reward::TakeOutcome, quest_batch3_c};
                let TakeOutcome::Taken(taken) = queue.take(session, lease.generation()) else {
                    return Err("captured Herald queue entry".into());
                };
                let entry = taken.entry().ok_or("captured Herald death")?;
                let winner = entry
                    .grizzly
                    .as_ref()
                    .ok_or("captured source recipients")?
                    .native
                    .iter()
                    .find(|recipient| recipient.death.transition == rule.transition)
                    .ok_or("captured Herald winner")?;
                assert_eq!(entry.facts.principal.actor, actor);
                assert!(entry.damage_shares.iter().any(|share| {
                    share.character.as_bytes() == &other_character
                        && share.total > 0
                        && share.session == Some(other_session)
                        && share.lease_generation == Some(other_lease)
                }));
                let objective = source_deaths::rule(&rule.transition)
                    .ok_or("Herald rule")?
                    .transition
                    .as_str();
                let snapshots = [
                    quest_batch3_c::Snapshot {
                        actor,
                        session,
                        character: *lease.character_id().as_bytes(),
                        lease_generation: lease.generation(),
                        observed_revision: winner.death.observed_revision,
                        objectives: vec![objective],
                    },
                    quest_batch3_c::Snapshot {
                        actor: other_actor,
                        session: other_session,
                        character: other_character,
                        lease_generation: other_lease,
                        observed_revision: winner.death.observed_revision,
                        objectives: vec![objective],
                    },
                ];
                let recipients =
                    quest_batch3_c::capture(&runtime, entry, true, snapshots.iter().cloned());
                assert_eq!(
                    recipients.len(),
                    1,
                    "damage alone cannot earn Herald credit"
                );
                assert_eq!(recipients[0].actor, actor);
                assert_eq!(recipients[0].character, *lease.character_id().as_bytes());
                assert!(
                    quest_batch3_c::capture(&runtime, entry, false, snapshots.into_iter())
                        .is_empty(),
                    "a lethal fallback is not the present most-damage player"
                );
                drop(taken); // Leave the real original queue entry for durable settlement below.
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
