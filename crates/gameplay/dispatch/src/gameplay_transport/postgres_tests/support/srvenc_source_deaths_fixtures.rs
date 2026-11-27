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

pub(super) fn scenario(name: &'static str, creature: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        run(async |admin| {
            let rule = source_deaths::rules()?
                .iter()
                .find(|r| r.creature == format!("oteryn:creature.{creature}"))
                .ok_or("source rule")?;
            let h = Harness::create_with_admission_seed(admin, "srvenc_m24", false, false).await?;
            let result = exercise(&h, rule, name).await;
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

async fn exercise(h: &Harness, rule: &source_deaths::Rule, mode: &str) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activated) = if matches!(
        mode,
        "overlordcr_ended_secondary_xp" | "overlordcr_live_conflict"
    ) {
        use sha2::{Digest, Sha256};

        let mut input = super::death_ml_loss_composed_fixtures::production_input_with_source("content-1", None)?;
        let loot = input.loot_tables.as_mut().ok_or("pinned loot section")?;
        let mut section: serde_json::Value = serde_json::from_slice(&loot.bytes)?;
        let table = section["tables"]
            .as_array_mut()
            .ok_or("pinned loot tables")?
            .iter_mut()
            .find(|table| table["identity"]["key"] == "oteryn:loot.creature.earth_overlord")
            .ok_or("Earth Overlord loot table")?;
        let entries = table["entries"].as_array_mut().ok_or("Earth Overlord loot entries")?;
        // This fixture needs a private reward row. Its production pin contains a
        // non-materializable drop, so exercise the chest path with the admitted subset.
        entries.retain(|entry| entry["item"]["key"] != "oteryn:item.tibia.i947");
        loot.bytes = serde_json::to_vec(&section)?;
        loot.sha256 = Sha256::digest(&loot.bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        crate::content::native_gameplay::test_support::try_activate_input(deps.admitted.world_id, &input)
            .map_err(debug)?
    } else {
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?
    };
    let generation = controller.active().ok_or("generation")?;
    let native = generation.native_gameplay().ok_or("native policies")?;
    let without_rewards = crate::content::creature_reward::CreatureRewardTable::default();
    let ended_secondary = matches!(
        mode,
        "overlordcr_ended_secondary_xp" | "overlordcr_ended_secondary_no_xp"
    );
    let rewards = if matches!(mode, "overlordcr_rowless" | "overlordcr_ended_secondary_no_xp") {
        &without_rewards
    } else {
        native.reward_table()
    };
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
    // This fixture has mission coordinates, not a qualified native callback
    // room. Match generic activity runtimes: quest credit still uses real damage.
    runtime.use_classic_creature_deaths().map_err(debug)?;
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
    let imported_predecessor = rule.top_damage_only || rule.top_damage_party;
    let catalogue = if seeds.is_empty() || imported_predecessor {
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
    if imported_predecessor && !seeds.is_empty() {
        // Herald and Nomad start after their source mission admission. Retain
        // the production catalogue while importing the mission predecessor.
        super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(h, &owner, &admitted, &seeds, 0xa6)
            .await
            .map_err(|error| format!("principal mission predecessor: {error}"))?;
    } else if !seeds.is_empty() {
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
                top_damage_character: imported_predecessor
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
            .map_err(|error| format!("detached source death: {error:?}"))?;
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
    // Both peers are independently admitted durable Characters in this Channel.
    // Member one damages the boss; member two only stands nearby.
    let nomad = rule.creature == "oteryn:creature.nomad";
    let left_party = mode == "nomad_excludes_former_member_with_stale_top_damage_roster";
    let kicked_by_leader = mode == "nomad_excludes_member_kicked_by_leader";
    let shared_xp_disabled = mode == "nomad_respects_leader_disabling_shared_xp";
    let remote_shared_xp_disabled = mode == "nomad_ignores_shared_xp_disabled_in_the_world_owner";
    let enabled_after_death = mode == "nomad_shared_xp_enabled_after_death";
    let disabled_after_death = mode == "nomad_shared_xp_disabled_after_death";
    let leader_changed_party = kicked_by_leader || shared_xp_disabled;
    let mut death_peers = Vec::new();
    let mut lesser_contributor = None;
    let peer_source = if nomad || rule.creature.ends_with("_overlord") || rule.top_damage_only {
        Some(super::house_transfer_fixtures::RecoverySource::start(h).await?)
    } else {
        None
    };
    if let Some(source) = &peer_source {
        use crate::gameplay_transport::connection::{
            AdmittedSession, FirstEntryOutcome, SessionContinuity,
        };
        let peer_count = if rule.top_damage_only {
            1
        } else if mode == "overlordcr_seventeen" {
            17u8
        } else if leader_changed_party {
            3
        } else {
            2
        };
        for member in 1..=peer_count {
            let peer_session =
                super::boss_rewards_fixtures::privacy::admit_gameplay_peer(h, &source.evidence, member)
                    .await?;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(peer_session)
                    .await?
                    .0;
            let fence = crate::gameplay_transport::item_fence_of(current).ok_or("peer fence")?;
            let peer_actor = {
                let mut runtime = owner.runtime.lock().await;
                let reservation = runtime.reserve_fresh_session(peer_session).map_err(debug)?;
                let peer_actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .bind_attacker_lease(
                        peer_actor,
                        peer_session,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                                .map_err(debug)?,
                            fence.character_lease_generation,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
                runtime
                    .initialize_pinned_test_position(
                        peer_actor,
                        MovementLocalPosition {
                            x: position[0] - i32::from(member),
                            y: position[1],
                            floor: i16::try_from(-position[2])?,
                        },
                    )
                    .map_err(debug)?;
                peer_actor
            };
            deps.initialize_player(&owner, peer_actor, peer_session)
                .await;
            let mut peer = AdmittedSession {
                game_session_id: peer_session,
                world_id: admitted.world_id,
                channel_id: admitted.channel_id,
                runtime_actor: Some(peer_actor),
                first_entry: FirstEntryOutcome::Positioned,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: Some(fence),
            };
            owner.admit_progression_session(&mut peer).await;
            assert!(
                owner.progression_binding(peer_session).is_some(),
                "peer completed progression admission"
            );
            if nomad {
                super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
                    h, &owner, &peer, &seeds, member,
                )
                .await
                .map_err(|error| format!("peer {member} mission predecessor: {error}"))?;
            }
            if mode == "overlordcr_missing_cache" && member == 1 {
                assert!(
                    owner
                        .states
                        .quest_sessions
                        .lock()
                        .map_err(|_| "quest cache")?
                        .get(&peer_session)
                        .is_none_or(|entry| entry.copy.is_none()),
                    "the eligible damager has no loaded quest copy"
                );
            } else {
                owner.admit_quest_session(&peer).await;
                assert!(
                    owner
                        .states
                        .quest_sessions
                        .lock()
                        .map_err(|_| "quest session lock")?
                        .get(&peer_session)
                        .is_some_and(|state| state.copy.is_some()),
                    "peer has an admitted quest snapshot"
                );
                owner.refresh_quest_session(peer_session, false).await;
            }
            let peer_character = *fence.character_id.as_bytes();
            if rule.top_damage_only {
                // This actor contributes to durable XP settlement but cannot earn Herald credit.
                lesser_contributor = Some((
                    peer_actor,
                    peer_session,
                    peer_character,
                    fence.character_lease_generation,
                ));
            } else {
                death_peers.push((peer_actor, peer_session, peer_character));
            }
        }
    }
    if nomad {
        // The World owner has committed the toggle, while both killing-Channel
        // projections below still reflect the earlier shared-XP roster.
        let mut tx = h.pool.begin().await?;
        sqlx::query(
            "INSERT INTO game_parties \
             (party_id,world_id,leader_character_id,revision,next_seq,shared_xp_enabled) \
             VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,2,4,$4)",
        )
        .bind(id(81).as_slice())
        .bind(owner.world_id.as_bytes().as_slice())
        .bind(
            if leader_changed_party {
                death_peers[2].2
            } else {
                id(41)
            }
            .as_slice(),
        )
        .bind(!remote_shared_xp_disabled && !shared_xp_disabled && !enabled_after_death)
        .execute(&mut *tx)
        .await?;
        let characters = std::iter::once(id(41))
            .chain((!left_party && !kicked_by_leader).then_some(death_peers[1].2))
            .chain(leader_changed_party.then(|| death_peers[2].2));
        for (sequence, character) in (1_i64..).zip(characters) {
            sqlx::query(
                "INSERT INTO game_party_members (character_id,party_id,seq,presence_state,presence_channel_id) \
                 VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,1,encode($4,'hex')::uuid)",
            )
            .bind(character.as_slice())
            .bind(id(81).as_slice())
            .bind(sequence)
            .bind(owner.channel_id.as_bytes().as_slice())
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
    }
    if mode == "overlordcr_old_pin" {
        let previous = source_deaths::overlord_predecessor(&catalogue)?;
        let hash = previous
            .definition_hash(&rule.quest)
            .ok_or("existing Elemental Spheres pin")?;
        assert_ne!(Some(hash), catalogue.definition_hash(&rule.quest));
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "old pin fence")?
            .ok_or("old pin fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key:
                        "oteryn:quest-transition/quest/u8_2/elemental_spheres/boss_room/action_1"
                            .into(),
                    cause: QuestCause::Command(CommandRef::new(
                        session,
                        CommandId::new(1).map_err(debug)?,
                    )),
                },
                Arc::new(previous),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(outcome, QuestTransitionOutcome::Committed(_)),
            "existing pin seed: {outcome:?}"
        );
        drop(slot);
        let stored: Vec<u8> = sqlx::query_scalar("SELECT definition_hash FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(&rule.quest).fetch_one(&h.pool).await?;
        assert_eq!(stored, hash);
    }
    let drain_session = if mode == "overlordcr_fallback" {
        death_peers[0].1
    } else {
        session
    };
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
        let mut crash_replay = None;
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
            if nomad {
                use crate::foundation::{PartyMemberSnapshot, PartyPresence, PartySnapshot};

                let character = *lease.character_id().as_bytes();
                let (peer, peer_session, idle_member) = death_peers[1];
                // Freeze the party observation at the lethal turn. The first
                // peer deals damage, but only the second belongs to this party.
                let leader = if leader_changed_party {
                    death_peers[2].2
                } else {
                    character
                };
                let members = [character, idle_member]
                    .into_iter()
                    .chain(leader_changed_party.then_some(leader))
                    .zip(1..)
                    .map(|(character, invitation_order)| PartyMemberSnapshot {
                        character,
                        invitation_order,
                        presence_revision: 1,
                        presence: PartyPresence::Channel,
                        channel: Some(*owner.channel_id.as_bytes()),
                    })
                    .collect();
                let party = PartySnapshot {
                    world: *owner.world_id.as_bytes(),
                    character,
                    party_id: Some(id(81)),
                    revision: Some(1),
                    leader: Some(leader),
                    shared_xp_enabled: !enabled_after_death,
                    members,
                    observed_database_micros: 1,
                };
                let mut peer_party = party.clone();
                peer_party.character = idle_member;
                if left_party {
                    // The Leave command projects the departing actor without refreshing the leader.
                    peer_party.party_id = None;
                    peer_party.revision = None;
                    peer_party.leader = None;
                    peer_party.shared_xp_enabled = false;
                    peer_party.members.clear();
                    peer_party.observed_database_micros = 2;
                }
                if leader_changed_party {
                    // The leader's committed command updates only the leader's actor projection.
                    let (leader_actor, leader_session, _) = death_peers[2];
                    let mut updated = party.clone();
                    updated.character = leader;
                    updated.revision = Some(2);
                    updated.observed_database_micros = 2;
                    if kicked_by_leader {
                        updated.members.retain(|member| member.character != idle_member);
                    } else {
                        updated.shared_xp_enabled = false;
                    }
                    runtime.project_world_party(leader_actor, leader_session, updated).map_err(debug)?;
                }
                runtime.project_world_party(actor, session, party).map_err(debug)?;
                runtime.project_world_party(peer, peer_session, peer_party).map_err(debug)?;
            }
            let mut health = runtime.companion_snapshot(target).map_err(debug)?.health;
            if matches!(mode, "overlordcr_seventeen" | "overlordcr_fallback") {
                let prime = if mode == "overlordcr_fallback" {
                    health - 2
                } else {
                    1
                };
                runtime
                    .borrow_exact_actor_commit_at(now)
                    .commit_damage_for_bound_attacker(
                        target,
                        actor,
                        CommandRef::new(session, CommandId::new(1).map_err(debug)?),
                        0,
                        crate::foundation::OwnerDamageCommand {
                            target: rule.creature.as_bytes(),
                            occurrence: session.as_bytes(),
                            binding: b"overlordcr-primary-first-damage",
                            damage: prime,
                        },
                    )
                    .map_err(debug)?;
                health -= prime;
            }
            let damaging_peers = if mode == "overlordcr_seventeen" {
                16
            } else {
                1
            };
            for (peer_actor, peer_session, _) in death_peers.iter().take(damaging_peers) {
                runtime
                    .borrow_exact_actor_commit_at(now)
                    .commit_damage_for_bound_attacker(
                        target,
                        *peer_actor,
                        CommandRef::new(*peer_session, CommandId::new(1).map_err(debug)?),
                        0,
                        crate::foundation::OwnerDamageCommand {
                            target: rule.creature.as_bytes(),
                            occurrence: peer_session.as_bytes(),
                            binding: b"overlordcr-peer-damage",
                            damage: 1,
                        },
                    )
                    .map_err(debug)?;
                health -= 1;
            }
            let (actor, session, lease) = if mode == "overlordcr_fallback" {
                runtime
                    .remove_terminal_session(session, actor)
                    .map_err(debug)?;
                let (peer_actor, peer_session, _) = death_peers[0];
                let peer_lease = runtime
                    .bound_attacker_lease(peer_actor, peer_session)
                    .map_err(debug)?;
                (peer_actor, peer_session, peer_lease)
            } else {
                (actor, session, lease)
            };
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
                        rewards,
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
            if matches!(mode, "overlordcr_crash_replay" | "overlordcr_rowless") {
                crash_replay = Some((target, observed));
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
        if ended_secondary {
            use crate::gameplay_transport::kill_reward::TakeOutcome;
            let (peer_actor, peer_session, _) = death_peers[0];
            let mut runtime = owner.runtime.lock().await;
            let lease = runtime.bound_attacker_lease(actor, session).map_err(debug)?;
            let TakeOutcome::Taken(taken) = queue.take(session, lease.generation()) else {
                return Err("captured Overlord queue entry".into());
            };
            let entry = taken.entry().ok_or("captured Overlord death")?;
            let recipients = crate::gameplay_transport::quest_kill_objectives::boss_objective_recipients(entry);
            assert_eq!(recipients.len(), 2);
            assert!(recipients.iter().any(|(_, captured, _)| *captured == peer_session));
            drop(taken);
            runtime.remove_terminal_session(peer_session, peer_actor).map_err(debug)?;
            drop(runtime);
            let ended = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3,current_transport_ref=NULL,prepared_attempt_ref=NULL WHERE game_session_id=encode($1,'hex')::uuid AND session_state IN (1,2)")
                .bind(peer_session.as_bytes().as_slice()).execute(&h.pool).await?;
            assert_eq!(ended.rows_affected(), 1);
            assert!(owner.current_quest_fence(peer_session).await.map_err(|_| "secondary fence read")?.is_none());
        }
        if mode == "overlordcr_live_conflict" {
            use crate::durability::character_progression::CharacterProgressionError;
            use crate::durability::spell_owner_commit::SpellLane;
            use crate::gameplay_transport::kill_reward::TakeOutcome;

            let (_, peer_session, peer_character) = death_peers[0];
            let peer_fence = owner.current_quest_fence(peer_session).await
                .map_err(|_| "secondary fence read")?.ok_or("live secondary fence")?;
            let lease = owner.runtime.lock().await.bound_attacker_lease(actor, session).map_err(debug)?;
            let TakeOutcome::Taken(taken) = queue.take(session, lease.generation()) else {
                return Err("captured Overlord queue entry".into());
            };
            let entry = taken.entry().ok_or("captured Overlord death")?;
            assert!(crate::gameplay_transport::quest_kill_objectives::boss_objective_recipients(entry)
                .contains(&(peer_fence.character_id, peer_session, peer_fence.character_lease_generation)));

            assert!(entry.boss_reward.is_some(), "capture includes private rewards");
            seed_conflicting_overlord_receipt(h, rule, entry, peer_character).await?;
            let lane = SpellLane::new(admitted.world_id, admitted.channel_id);
            let permit = lane.acquire().await.map_err(|_| "spell lane")?;
            let outcome = owner.settle_party_experience(&permit, entry).await;
            let counts: (i64, i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM game_boss_death_settlements),
                        (SELECT count(*) FROM game_character_boss_reward_chests),
                        (SELECT count(*) FROM game_party_kill_xp_receipts)",
            ).fetch_one(&h.pool).await?;
            assert_eq!(counts, (0, 0, 0), "live conflict rolls back death, chests and XP");
            assert!(matches!(outcome.map(drop), Err(CharacterProgressionError::ConflictingOccurrence)));
            return Ok(());
        }
        if mode == "overlordcr_atomic" {
            for statement in [
                "CREATE SEQUENCE overlordcr_second_receipt",
                "CREATE FUNCTION overlordcr_refuse_second() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.transition_key LIKE 'oteryn:quest-transition/elemental-spheres/%' AND nextval('overlordcr_second_receipt')=2 THEN RAISE EXCEPTION 'overlordcr second recipient unavailable' USING ERRCODE='23514'; END IF; RETURN NEW; END $$",
                "CREATE TRIGGER overlordcr_refuse_second BEFORE INSERT ON game_character_quest_receipts FOR EACH ROW EXECUTE FUNCTION overlordcr_refuse_second()",
            ] {
                sqlx::query(statement).execute(&h.pool).await?;
            }
            assert_eq!(
                owner.drain_kill_rewards(drain_session).await,
                DrainOutcome::Deferred
            );
            let attempted: i64 =
                sqlx::query_scalar("SELECT last_value FROM overlordcr_second_receipt")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                attempted, 2,
                "the injected failure reached the second recipient"
            );
            let counts: (i64,i64,i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u8_2/elemental_spheres/boss_storage'), (SELECT count(*) FROM game_party_kill_xp_receipts), (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-transition/elemental-spheres/%')"
            ).fetch_one(&h.pool).await?;
            assert_eq!(
                counts,
                (0, 0, 0),
                "a second-recipient failure rolls back every flag and the durable death"
            );
            for statement in [
                "DROP TRIGGER overlordcr_refuse_second ON game_character_quest_receipts",
                "DROP FUNCTION overlordcr_refuse_second()",
                "DROP SEQUENCE overlordcr_second_receipt",
            ] {
                sqlx::query(statement).execute(&h.pool).await?;
            }
        }
        if enabled_after_death || disabled_after_death {
            sqlx::query("UPDATE game_parties SET shared_xp_enabled=$1, revision=revision+1")
                .bind(enabled_after_death)
                .execute(&h.pool)
                .await?;
        }
        let drained = owner.drain_kill_rewards(drain_session).await;
        if drained != DrainOutcome::Drained {
            let roots: Vec<(String, String)> = sqlx::query_as(
                "SELECT character_id::text,character_revision::text FROM game_character_roots ORDER BY character_id",
            ).fetch_all(&h.pool).await?;
            let tracks: Vec<(String, String, i64)> = sqlx::query_as(
                "SELECT character_id::text,track_key,value FROM game_character_quest_tracks ORDER BY character_id,track_key",
            ).fetch_all(&h.pool).await?;
            let xp: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
                .fetch_one(&h.pool)
                .await?;
            eprintln!(
                "SOURCE_DEATH_DEFERRED creature={} roots={roots:?} tracks={tracks:?} xp_receipts={xp}",
                rule.creature
            );
        }
        assert_eq!(drained, DrainOutcome::Drained);
        if ended_secondary {
            assert!(!queue.has_session(drain_session), "the settled death leaves the queue");
            let (xp, chests, deaths): (i64, i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM game_character_xp_receipts WHERE character_id=encode($1,'hex')::uuid AND experience_awarded>0), (SELECT count(*) FROM game_character_boss_reward_chests WHERE character_id=encode($1,'hex')::uuid), (SELECT count(*) FROM game_boss_death_settlements)",
            ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!((xp, chests, deaths), if mode == "overlordcr_ended_secondary_xp" { (1, 1, 1) } else { (0, 0, 1) });
        }

        if let Some((target, observed)) = crash_replay {
            let canonical: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_boss_death_settlements")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(canonical, 1, "the canonical death committed once");
            let before: (i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_boss_reward_deaths), (SELECT count(*) FROM game_character_boss_reward_chests), (SELECT count(*) FROM game_character_xp_receipts), (SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?;
            let private_admitted = rewards.row(&rule.creature).is_ok();
            assert_eq!(before.0, i64::from(private_admitted));
            assert_eq!(
                before.1,
                if private_admitted { 2 } else { 0 },
                "retained private-loot refusal never invents a chest"
            );
            if mode == "overlordcr_rowless" {
                assert_eq!(before, (0, 0, 0, 2), "only the two quest credits commit");
            } else {
                assert!(
                    before.2 > 0 && before.3 >= 2,
                    "XP and both storage credits committed before recovery"
                );
            }
            {
                let replay_observed = observed.checked_add(1_000).map_err(debug)?;
                let mut runtime = owner.runtime.lock().await;
                let lease = runtime
                    .bound_attacker_lease(actor, session)
                    .map_err(debug)?;
                crate::gameplay_transport::kill_reward::record_creature_kill(
                    &mut runtime,
                    KillSink {
                        prey: None,
                        activity: &[],
                        queue: &queue,
                        rewards,
                        charms: None,
                    },
                    target,
                    crate::combat::CapturedRewardPrincipal {
                        character: lease.character_id(),
                        lease_generation: lease.generation(),
                        session,
                        actor,
                    },
                    replay_observed.get() / 1000,
                    Some(replay_observed),
                );
                let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(replay) =
                    queue.take(session, lease.generation())
                else {
                    return Err("late native replay must reach the real durable drain".into());
                };
                let entry = replay.entry().ok_or("retained replay entry")?;
                assert_eq!(entry.activity_observed_at, Some(observed));
                drop(replay);
            }
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            let after: (i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_boss_reward_deaths), (SELECT count(*) FROM game_character_boss_reward_chests), (SELECT count(*) FROM game_character_xp_receipts), (SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?;
            assert_eq!(
                before, after,
                "durable death replay cannot mint or credit twice"
            );
            let canonical_after: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_boss_death_settlements")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                canonical_after, canonical,
                "replay never inserts a second canonical death"
            );
        }
    }
    assert_eq!(
        owner.drain_kill_rewards(drain_session).await,
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
            if mode == "overlordcr_fallback" {
                None
            } else {
                Some(expected)
            },
            "native {} must commit {}",
            rule.creature,
            effect.track
        );
    }
    for (index, (_, _, character)) in death_peers.iter().enumerate() {
        let value: Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
            .bind(character.as_slice()).bind(&rule.effects[0].track).fetch_optional(&h.pool).await?;
        if nomad {
            let before = seeds.first().ok_or("Nomad mission predecessor")?.1;
            assert_eq!(
                value,
                if index == 0
                    || left_party
                    || shared_xp_disabled
                    || remote_shared_xp_disabled
                    || enabled_after_death
                    || (kicked_by_leader && index == 1)
                {
                    Some(before)
                } else {
                    rule.effects[0].set
                },
                "Nomad credits the idle party member and excludes the damaging outsider (peer {index})",
            );
            continue;
        }
        assert_eq!(
            value,
            if !ended_secondary && index
                < if mode == "overlordcr_seventeen" {
                    16
                } else {
                    1
                }
            {
                Some(1)
            } else {
                None
            },
            "only damage participants receive BossStorage"
        );
    }
    if !rule.creature.ends_with("_overlord") {
        let flags: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u8_2/elemental_spheres/boss_storage'")
            .fetch_one(&h.pool).await?;
        assert_eq!(flags, 0, "other creature deaths do not set BossStorage");
    }
    if rule.creature == "oteryn:creature.arbaziloth" {
        let questlog: Option<i64> = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks \
             WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
        )
        .bind(id(41).as_slice())
        .bind("oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/questlog")
        .fetch_optional(&h.pool)
        .await?;
        assert_eq!(questlog, Some(1), "death must start the quest log");
        let hard_kills: Option<i64> = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks \
             WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
        )
        .bind(id(41).as_slice())
        .bind("oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/hard_kills")
        .fetch_optional(&h.pool)
        .await?;
        assert_eq!(hard_kills, None, "an easy victory earns no hard-win credit");
        assert!(
            !owner.runtime.lock().await.arbaziloth_doomforge_open(),
            "an easy victory leaves the Doomforge portal closed"
        );
        let outfits: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_appearance_unlock_receipts \
             WHERE granting_character_id=encode($1,'hex')::uuid AND unlock_key=$2",
        )
        .bind(id(41).as_slice())
        .bind("oteryn:outfit.fiend_slayer/quest-grant/0")
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(outfits, 1, "duplicate death must grant Fiend Slayer once");
    }
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&rule.transition)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        receipts,
        if mode == "overlordcr_fallback" || ended_secondary {
            1
        } else {
            i64::try_from(kills)?
                + if death_peers.is_empty()
                    || left_party
                    || shared_xp_disabled
                    || remote_shared_xp_disabled
                    || enabled_after_death
                {
                    0
                } else if mode == "overlordcr_seventeen" {
                    16
                } else {
                    1
                }
        },
        "duplicate corpse projection must retain the first captured receipt"
    );
    if mode == "overlordcr_old_pin" {
        let previous = source_deaths::overlord_predecessor(&catalogue)?;
        let stored: Vec<u8> = sqlx::query_scalar("SELECT definition_hash FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(&rule.quest).fetch_one(&h.pool).await?;
        assert_eq!(
            Some(stored.as_slice()),
            previous
                .definition_hash(&rule.quest)
                .as_ref()
                .map(|hash| hash.as_slice()),
            "compatible quest pins remain immutable"
        );
        let skipped: i64 = sqlx::query_scalar("SELECT count(*) FROM game_boss_death_quest_skips")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(skipped, 0, "a compatible predecessor is never skipped");
    }
    if mode == "overlordcr_fallback" {
        let xp: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            xp, 0,
            "the existing top-damager departure XP refusal is preserved"
        );
    }
    if mode == "overlordcr_rowless" {
        let counts: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_boss_death_settlements), (SELECT count(*) FROM game_character_xp_receipts), (SELECT count(*) FROM game_character_boss_reward_chests)",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(counts, (1, 0, 0), "the death settles quest credit alone");
    }
    Ok(())
}


async fn seed_conflicting_overlord_receipt(
    h: &Harness,
    rule: &source_deaths::Rule,
    entry: &crate::gameplay_transport::kill_reward::PendingKillSettlement,
    character: [u8; 16],
) -> TestResult {
    use sha2::{Digest, Sha256};

    // Match the captured death's receipt key while injecting a different binding.
    let death = crate::durability::party_kill_xp::encode_death_key(entry.facts.death);
    let hash = Sha256::new()
        .chain_update(b"oteryn:quest-native-death:v1")
        .chain_update(death)
        .chain_update(character)
        .finalize();
    let mut occurrence: [u8; 16] = hash[..16].try_into()?;
    occurrence[6] = (occurrence[6] & 15) | 0x70;
    occurrence[8] = (occurrence[8] & 63) | 0x80;

    // Stage a historical conflicting receipt without advancing the live Character.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    let inserted = sqlx::query(
        "INSERT INTO game_character_quest_receipts (
            character_id, cause_kind, cause_id, cause_ordinal, transition_key, quest_key,
            request_binding, pinned_content_revision, definition_hash, completes,
            track_keys, values_before, values_after, original_character_revision,
            committed_character_revision, level_before, level_after, experience_before,
            experience_after, profile_revision, ruleset_revision, content_revision,
            simulation_revision, evidence_revision, declaration_revision, policy_revision,
            reward_revision, committed_at
        ) SELECT character_id, 'creature_death', encode($2,'hex')::uuid, 0, $3, $4,
            decode('00','hex'), content_revision, decode(repeat('00',32),'hex'), false,
            ARRAY[$5::text], ARRAY[0::bigint], ARRAY[1::bigint], character_revision,
            character_revision+1, level, level, total_experience, total_experience,
            profile_revision, ruleset_revision, content_revision, simulation_revision,
            evidence_revision, declaration_revision, policy_revision, reward_revision, 0
        FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice()).bind(occurrence.as_slice()).bind(&rule.transition)
    .bind(&rule.quest).bind(&rule.effects[0].track).execute(&mut *tx).await?;
    assert_eq!(inserted.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}
