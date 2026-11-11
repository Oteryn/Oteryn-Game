//! Literal source rite with four genuine Fresh Characters, owned builds and altar custody.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
    convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::foundation::{
    ChannelRuntimeV1, CharacterLease, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FirstEntryOutcome},
};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_simulation_determinism::ExactI64;

const KEY: &str = "oteryn:quest-callback/desert-dungeon-party-rite";
const START: TilePos = TilePos {
    x: 32673,
    y: 32087,
    floor: -8,
};

#[test]
fn desert_dungeon_four_station_rite_uses_real_fenced_players() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Desert party scenario panicked")??;
    println!(
        "DESERT_PARTY_RITE POSTGRES_CASE_COMPLETED duration_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

pub(super) fn scenario(earned: bool) -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "desert_party_rite", false, false).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&h.pool)
            .await?;
        println!("DESERT_PARTY_RITE POSTGRES_CASE_OBSERVED server_version_num={version}");
        let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .find(|row| row.key == KEY)
            .ok_or("source four-vocation party/offering callback missing")?;
        let party = callback
            .effects
            .party_transfer
            .as_ref()
            .ok_or("party transfer")?;
        assert_eq!(party.stations.len(), 4);
        let fresh = prepare_current_session(&h).await?;
        let evidence = super::house_transfer::RecoverySource::start(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        if earned {
            for key in ["oteryn:item.tibia.i5102", "oteryn:item.tibia.i5103"] {
                assert!(
                    native.item_policy(key, "definition-r1").is_some(),
                    "source level20 gate requires native profile {key}"
                );
            }
        }
        let start = if earned {
            TilePos {
                x: 32673,
                y: 32101,
                floor: -8,
            }
        } else {
            START
        };
        let load_source = if earned {
            super::source_world_crop::compiled_region
        } else {
            super::source_world_crop::world
        };
        let world = load_source(
            TilePos {
                x: 32665,
                y: if earned { 32059 } else { 32069 },
                floor: -8,
            },
            TilePos {
                x: 32681,
                y: if earned { 32102 } else { 32096 },
                floor: if earned { -7 } else { -8 },
            },
            start,
            native,
        )?;
        let key = crate::map::view::placement_key(
            TilePos {
                x: 32673,
                y: 32086,
                floor: -8,
            },
            1,
        )
        .ok_or("lever key")?;
        assert!(
            world
                .facts()
                .quest_callback_matches(key, Some(30030), None, Some(2772)),
            "source lever startup UID30030 missing"
        );
        if earned {
            super::desert_earned_route::assert_source_bindings(&world)?;
        }
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (i32::from(start.x), i32::from(start.y), 8),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        super::tutorial_vitals_fixture::publish_native_activation(&h, &activated).await?;
        super::maintain_ready(&h.root).await?;
        let mut sessions = Vec::new();
        for n in 0..3 {
            let character = create_peer(&h, &authority, &evidence.evidence, n).await?;
            sessions.push(
                super::activity_deadline_paging::admit_created_member(
                    &h,
                    &evidence.evidence,
                    n,
                    character,
                )
                .await?,
            );
        }
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.test_step_wait_receipts = Some(true);
        owner.evidence = &evidence.evidence;
        owner.movement_cells = (&cells).into();
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        select_movement(&owner, &mut admitted, earned)?;
        let equipment_fence = current(&owner, admitted.game_session_id)
            .await?
            .item_fence();
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &equipment_fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &equipment_fence,
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &crate::durability::starter_grant::StarterTemplate {
                    definition: crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i2854".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    ..crate::durability::starter_grant::StarterTemplate::main()
                },
            )
            .await
            .map_err(debug)?;

        owner.admit_progression_session(&mut admitted).await;
        let mut members = Vec::new();
        for (station, session) in [
            sessions[0],
            sessions[1],
            admitted.game_session_id,
            sessions[2],
        ]
        .into_iter()
        .enumerate()
        {
            let actor = if session == admitted.game_session_id {
                deps.actor
            } else {
                let mut runtime = owner.runtime.lock().await;
                let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
                let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .initialize_pinned_test_position(
                        actor,
                        super::quest_world_callbacks::local(party.stations[station].from),
                    )
                    .map_err(debug)?;
                actor
            };
            let mut peer = admitted;
            peer.game_session_id = session;
            peer.runtime_actor = Some(actor);
            owner.admit_progression_session(&mut peer).await;
            let progression = owner
                .progression_binding(session)
                .ok_or_else(|| format!("progression binding station {station}"))?;
            let fence = current(&owner, session).await?;
            assert!(matches!(
                h.root
                    .commit_character_experience(
                        &authority,
                        &h.node,
                        fence,
                        ExperienceAwardRequest {
                            occurrence: ExperienceRewardOccurrence::from_bytes(id(
                                180 + u8::try_from(station)?
                            ))
                            .map_err(debug)?,
                            amount: ExactI64::new(98800),
                            context: progression.context.clone(),
                            policy_revision: progression.policy_revision.clone(),
                            reward_revision: progression.reward_revision.clone(),
                            policy: progression.policy.clone(),
                        }
                    )
                    .await
                    .map_err(debug)?,
                ExperienceCommitOutcome::Committed(_)
            ));
            let formula = native.training_formula().ok_or("formula")?;
            let build = convert_vocation(
                formula,
                &DurableBuildState::default(),
                &party.stations[station].vocation,
            )
            .map_err(debug)?;
            assert!(matches!(
                h.root
                    .commit_character_build(
                        &authority,
                        &h.node,
                        current(&owner, session).await?,
                        BuildChangeRequest {
                            occurrence: BuildOccurrence::from_bytes(id(
                                190 + u8::try_from(station)?
                            ))
                            .map_err(debug)?,
                            cause: BuildCause::VocationChoice,
                            before: DurableBuildState::default(),
                            after: build.clone(),
                            pruned_stance: None,
                        },
                        formula
                    )
                    .await
                    .map_err(debug)?,
                BuildCommitOutcome::Committed(_)
            ));
            let fence = current(&owner, session).await?;
            let facts =
                match crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
                    &h.root,
                    &authority,
                    fence.character_id,
                )
                .await
                {
                    crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
                        facts,
                        ..
                    } => facts,
                    _ => return Err("owned Character facts unavailable".into()),
                };
            assert_eq!(facts.level, 20);
            {
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .bind_attacker_lease(
                        actor,
                        session,
                        CharacterLease::new(
                            crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                                .map_err(debug)?,
                            fence.character_lease_generation,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
                let mut states = owner.states.spell_states.lock().await;
                assert!(
                    states
                        .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
                        .is_some()
                );
                assert!(states.load_owned_build(&runtime, actor, session, &build));
            }
            owner.refresh_quest_session(session, false).await;
            members.push((actor, session));
        }
        let real_sessions:i64 = sqlx::query_scalar("SELECT count(DISTINCT character_id) FROM game_durability_reconnect_sessions WHERE session_state=2").fetch_one(&h.pool).await?;
        assert_eq!(
            real_sessions, 4,
            "every station uses an independently admitted Character"
        );
        if earned {
            super::desert_earned_route::enter(&mut owner, &mut admitted).await?;
        }
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = |revision| WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        };
        // First pull only transforms. A missing member on the return pull consumes nothing.
        assert_eq!(
            pull_next(&owner, &mut admitted, &target(0)).await?,
            UseDisposition::Committed
        );
        assert_eq!(
            pull_next(&owner, &mut admitted, &target(1)).await?,
            UseDisposition::Rejected
        );
        assert_eq!(burns(&h).await?, 0);
        assert_eq!(
            owner.bundle_map().ok_or("map")?.facts.object_revision(key),
            2
        );
        if earned {
            super::desert_earned_route::walk_to(&mut owner, &mut admitted, party.stations[2].from)
                .await?;
        } else {
            relocate(
                &owner,
                deps.actor,
                admitted.game_session_id,
                party.stations[2].from,
            )
            .await?;
        }

        // Every item is born through native MINT/TRANSFER/DROP. Omit the fourth altar.
        for (n, offering) in callback.offerings.iter().take(3).enumerate() {
            let mut input = offering.clone();
            if n == 1 {
                input.count = 7;
            }
            super::quest_world_callbacks::input_item(
                &owner,
                current(&owner, admitted.game_session_id)
                    .await?
                    .item_fence(),
                &input,
                300 + u64::try_from(n)?,
                false,
            )
            .await?;
        }
        let before = positions(&owner, &members).await?;
        assert_eq!(
            pull_next(&owner, &mut admitted, &target(2)).await?,
            UseDisposition::Committed
        );
        assert_eq!(
            pull_next(&owner, &mut admitted, &target(3)).await?,
            UseDisposition::Rejected
        );
        assert_eq!(positions(&owner, &members).await?, before);
        assert_eq!(
            burns(&h).await?,
            0,
            "partial offering set never burns any altar"
        );
        super::quest_world_callbacks::input_item(
            &owner,
            current(&owner, admitted.game_session_id)
                .await?
                .item_fence(),
            &callback.offerings[3],
            303,
            false,
        )
        .await?;
        let swapped = party.stations[0].from;
        if earned {
            super::desert_earned_route::walk_to(
                &mut owner,
                &mut admitted,
                Position {
                    x: START.x,
                    y: START.y,
                    floor: START.floor,
                },
            )
            .await?;
        } else {
            relocate(
                &owner,
                members[0].0,
                members[0].1,
                Position {
                    x: swapped.x + 1,
                    ..swapped
                },
            )
            .await?;
            relocate(&owner, members[1].0, members[1].1, swapped).await?;
        }
        assert_eq!(
            pull_next(&owner, &mut admitted, &target(4)).await?,
            UseDisposition::Committed
        );
        assert_eq!(
            pull_next(&owner, &mut admitted, &target(5)).await?,
            UseDisposition::Rejected
        );
        assert_eq!(burns(&h).await?, 0);
        if earned {
            super::desert_earned_route::walk_to(&mut owner, &mut admitted, party.stations[2].from)
                .await?;
        } else {
            relocate(&owner, members[1].0, members[1].1, party.stations[1].from).await?;
            relocate(&owner, members[0].0, members[0].1, swapped).await?;
        }
        durable_refusals(&owner, &h, &members, &callback).await?;
        // Lost acknowledgement after the one immutable offering commit: no member
        // moves until reconciliation, and no peer may bypass the pending rite.
        assert_eq!(
            pull_next(&owner, &mut admitted, &target(6)).await?,
            UseDisposition::Committed
        );
        let before = positions(&owner, &members).await?;
        owner
            .states
            .quest_callbacks
            .interrupt_after_commit
            .store(true, std::sync::atomic::Ordering::Release);
        let successful_command = admitted.continuity.next_command_id;
        let interrupted = pull_next(&owner, &mut admitted, &target(7)).await;
        assert!(
            interrupted.is_err(),
            "an unknown durable outcome closes without an acknowledgement"
        );
        assert_eq!(burns(&h).await?, 4);
        assert_eq!(positions(&owner, &members).await?, before);
        for (_, session) in &members {
            assert!(owner.has_pending_quest_callback(*session));
        }
        assert!(owner.reconcile_quest_callbacks(members[0].1).await);
        for ((actor, session), station) in members.iter().zip(&party.stations) {
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(*actor)
                    .map_err(debug)?
                    .position(),
                super::quest_world_callbacks::local(station.to)
            );
            assert!(!owner.has_pending_quest_callback(*session));
        }
        assert_eq!(burns(&h).await?, 4);
        assert_eq!(
            owner.bundle_map().ok_or("map")?.facts.object_revision(key),
            8
        );
        assert_eq!(
            pull(&owner, &mut admitted, successful_command, &target(7)).await?,
            UseDisposition::Committed,
            "exact original command replays the reconciled outcome"
        );
        assert_eq!(burns(&h).await?, 4);
        assert_eq!(
            owner.bundle_map().ok_or("map")?.facts.object_revision(key),
            8
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind(KEY)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, 1);
        let quantity:i64=sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$1").bind(KEY).fetch_one(&h.pool).await?;
        assert_eq!(
            quantity, 10,
            "source remove() consumes the entire seven-apple stack and each other exact sacrifice"
        );
        let xp: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(xp, 4);
        let builds: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_build_receipts WHERE cause='vocation_choice'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(builds, 4);
        if earned {
            super::desert_earned_route::earn(&mut owner, &mut admitted, &h).await?;
        }
        drop(owner);
        drop(evidence);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn current(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> TestResult<crate::durability::character_progression::CurrentCharacterGameplayFence> {
    owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "fence")?
        .ok_or_else(|| "current session".into())
}
fn select_movement(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    diagonals: bool,
) -> TestResult {
    let offered = owner.offered_capabilities();
    let mut selected = std::collections::BTreeSet::new();
    let mut pending = vec![11];
    if diagonals {
        pending.push(13);
        pending.push(oteryn_protocol_oteryn::world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1);
    }
    while let Some(key) = pending.pop() {
        if selected.insert(key) {
            pending.extend(
                offered
                    .iter()
                    .find(|row| row.id == key)
                    .ok_or("capability")?
                    .requires
                    .iter()
                    .copied(),
            );
        }
    }
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            offered,
            &selected.into_iter().collect::<Vec<_>>(),
        )
        .ok_or("selected")?;
    Ok(())
}
async fn pull_next(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    let command = admitted.continuity.next_command_id;
    pull(owner, admitted, command, target).await
}
async fn pull(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    admitted.item_fence = Some(current(owner, admitted.game_session_id).await?.item_fence());
    let response = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(target).map_err(debug)?,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&response).map_err(debug)
}
async fn burns(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1",
    )
    .bind(KEY)
    .fetch_one(&h.pool)
    .await?)
}
async fn positions(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    members: &[(ExactActorRef, GameSessionId)],
) -> TestResult<Vec<MovementLocalPosition>> {
    let runtime = owner.runtime.lock().await;
    members
        .iter()
        .map(|(actor, _)| {
            runtime
                .read_actor_position(*actor)
                .map(|p| p.position())
                .map_err(debug)
        })
        .collect()
}
async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    to: Position,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let closed = crate::map::floor::BundleDoors(map.facts.doors());
    let mut runtime = owner.runtime.lock().await;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        session,
        expected,
        super::quest_world_callbacks::local(to),
        false,
        &closed,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

async fn durable_refusals(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    members: &[(ExactActorRef, GameSessionId)],
    callback: &Callback,
) -> TestResult {
    use crate::durability::{
        quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
        quest_world_callback::{QuestWorldContext, QuestWorldPartyMember},
    };
    let mut party = Vec::new();
    for (actor, session) in members {
        let fence = current(owner, *session).await?;
        let position = owner
            .runtime
            .lock()
            .await
            .read_actor_position(*actor)
            .map_err(debug)?;
        party.push(QuestWorldPartyMember {
            character: *fence.character_id.as_bytes(),
            session: *session.as_bytes(),
            connection_generation: fence.connection_generation.get(),
            lease_generation: fence.character_lease_generation,
            scope_generation: fence.scope_ownership_generation.get(),
            actor_placement: actor.placement_identity(),
            movement_revision: position.revision(),
        });
    }
    let map = owner.bundle_map().ok_or("map")?;
    let mut context = {
        let runtime = owner.runtime.lock().await;
        QuestWorldContext {
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_revision: format!(
                "sha256:{}",
                runtime
                    .content_pin()
                    .server_artifact_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
            reset_epoch: owner.reset_epoch,
            actor_placement: members[2].0.placement_identity(),
            movement_revision: runtime
                .read_actor_position(members[2].0)
                .map_err(debug)?
                .revision(),
            party: party.clone(),
        }
    };
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    for change in 0..4 {
        context.party = party.clone();
        match change {
            0 => context.party.swap(0, 1),
            1 => context.party[3].connection_generation += 1,
            2 => context.party[3].scope_generation += 1,
            3 => context.party[3].character = context.party[0].character,
            _ => unreachable!(),
        }
        let fence = current(owner, members[2].1).await?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_world_callback(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: callback.key.clone(),
                    cause: QuestCause::Use(crate::foundation::CommandRef::new(
                        members[2].1,
                        crate::foundation::CommandId::new(1000 + change).map_err(debug)?,
                    )),
                },
                std::sync::Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
                &permit,
                context.clone(),
                owner.active_generation.and_then(|g| g.native_gameplay()),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(outcome, QuestTransitionOutcome::Refused(_)),
            "invalid party context {change}: {outcome:?}"
        );
        assert_eq!(
            burns(h).await?,
            0,
            "one stale/foreign/wrong-vocation member refuses every offering"
        );
    }
    Ok(())
}

async fn create_peer(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    evidence: &crate::gameplay_transport::fresh_evidence::FreshEvidenceSource,
    member: u8,
) -> TestResult<[u8; 16]> {
    let account = crate::gameplay_transport::canonical_uuid(&id(120 + member));
    evidence
        .refresh_account(&h.root, &h.node, &account)
        .await
        .map_err(debug)?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let operation = id(80 + member);
    let source = serde_json::json!({
        "contract_version":3,
        "variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY",
        "issuer_decision_id":crate::gameplay_transport::canonical_uuid(&id(90+member)),
        "source_revision":(2+u64::from(member)).to_string(),
        "operation_id":crate::gameplay_transport::canonical_uuid(&operation),
        "operation":"INITIAL_CHARACTER_BOOTSTRAP",
        "account_id":account,
        "target_world_id":crate::gameplay_transport::canonical_uuid(&id(42)),
        "requested_name":format!("Desert A{}",char::from(b'a'+member)),
        "requested_sex":"Male",
        "interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},
        "issued_at_source":now.to_string(),
        "expires_at_source":(now+120).to_string(),
        "audience":"OTERYN_GAME_CHARACTER_AUTHORITY",
    });
    // The strict producer fixture decoder is also used by Character authority
    // tests. Birth, creation receipt and audit are committed by the real owner.
    let intent = crate::character_bootstrap_intent::decode_producer_response(
        &serde_json::to_vec(&source)?,
        operation,
    )
    .map_err(debug)?;
    let created = h
        .root
        .bootstrap_character(authority, &h.node, &intent)
        .await
        .map_err(debug)?;
    assert_eq!(created.revision.get(), 1);
    Ok(*created.character_id.as_bytes())
}
