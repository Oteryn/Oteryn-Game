//! Actual source Primal entry, callbacks, durable Pod Item and native terminal.
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn srx3_primal_real_source_entry_wave_pods_and_terminal_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-primal-production", || {
        run(async |admin| {
            reset();
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::Primal,
                MovementLocalPosition {
                    x: 33548,
                    y: 32752,
                    floor: 14,
                },
            )?;
            let deps: Dependencies = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_primal_production", false, false)
                    .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::Primal)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            let (offered, scope, generation) = {
                let runtime = owner.runtime.lock().await;
                let offers=crate::gameplay_transport::world_required_capabilities::with_native_map_presentation_offer(
                    BUNDLE_WORLD_OFFERED_CAPABILITIES,owner.active_generation,
                    owner.effective_rooms().gate_wire_position(),runtime.content_pin());
                (
                    offers,
                    runtime.binding().runtime_scope(),
                    runtime.binding().scope_generation().get(),
                )
            };
            owner.required_capabilities=crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[11,18,30],scope,generation,"policy-1",offered).map_err(debug)?;
            let supported = [
                4,
                6,
                7,
                10,
                11,
                15,
                18,
                23,
                30,
                92,
                wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                385,
                oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_NATIVE_OBJECT_APPEARANCE_V1,
            ];
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::connection::fresh_capability_selection(
                    &owner, &supported,
                )
                .await
                .ok_or("actual source-qualified native boss/cue negotiation")?;
            assert!(
                admitted.continuity.selected_capabilities.contains(385),
                "actual qualified explicit-source presentation must be negotiated"
            );
            assert!(
                admitted.continuity.selected_capabilities.contains(
                    oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_NATIVE_OBJECT_APPEARANCE_V1,
                ),
                "actual qualified native corpse appearance must be negotiated"
            );
            owner
                .required_capabilities
                .authenticate_resume_support(&supported, admitted.continuity.selected_capabilities)
                .map_err(debug)?;
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let source_equipment =
                crate::gameplay_transport::equipment_move::read_current_equipment(
                    &owner,
                    admitted.game_session_id,
                )
                .await
                .ok_or("actual source equipment snapshot before handoff")?;
            let source_admitted = admitted;
            let before_quest = h
                .root
                .read_character_quest_state(
                    &authority,
                    admitted
                        .item_fence
                        .ok_or("real Character fence")?
                        .character_id,
                )
                .await
                .map_err(debug)?;
            assert!(
                !before_quest
                    .tracks()
                    .contains_key("oteryn:quest-progress/storage/47855")
            );
            let instance = super::activity_runtime::allocate_ready_instance(&h, 209).await?;
            super::srx3_primal_prewarm::assert_empty_ready_instance(&h, &mut owner, instance)
                .await?;
            let mut frames = Vec::new();
            let (_, reply, scope) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                "oteryn:creature.the_primal_menace",
                true,
                &mut frames,
            )
            .await?;
            assert!(matches!(reply, wire::BossActivityReply::Accepted { .. }));
            assert!(
                scope.is_some(),
                "actual command crosses the native scope barrier"
            );
            let fence = admitted.item_fence.ok_or("actual transferred fence")?;
            assert_eq!(fence.runtime_scope, instance);
            assert!(matches!(instance, RuntimeScopeRefV1::Instance { .. }));
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("production source runtime")?;
            let view = owner.runtime_view(&registered);
            let instance_equipment =
                crate::gameplay_transport::equipment_move::read_current_equipment(
                    &view,
                    admitted.game_session_id,
                )
                .await
                .ok_or("actual Instance equipment snapshot after committed handoff")?;
            assert_eq!(
                instance_equipment.items, source_equipment.items,
                "real private admission retains the same portable equipment Item custody"
            );
            assert_eq!(
                instance_equipment.content_digest,
                source_equipment.content_digest
            );
            let actual_fence = admitted
                .item_fence
                .ok_or("actual Instance equipment fence")?;
            let origin_fence = source_admitted
                .item_fence
                .ok_or("actual origin equipment fence")?;
            let mut counterfeit = actual_fence;
            counterfeit.runtime_scope = origin_fence.runtime_scope;
            counterfeit.scope_ownership_generation = origin_fence.scope_ownership_generation;
            assert!(
                h.root
                    .read_movement_equipment(
                        owner.character,
                        owner.holder,
                        &counterfeit,
                        instance_equipment.content_digest
                    )
                    .await
                    .is_err(),
                "an Instance session cannot read by pretending its authenticated origin is its scope"
            );
            let mut wrong_content = instance_equipment.content_digest;
            wrong_content[0] ^= 1;
            assert!(
                h.root
                    .read_movement_equipment(
                        owner.character,
                        owner.holder,
                        &actual_fence,
                        wrong_content
                    )
                    .await
                    .is_err(),
                "Instance gear needs independently pinned Content"
            );
            assert_eq!(
                view.effective_rooms()
                    .gate_monster_ai()
                    .ok_or("source scene")?
                    .pinned_activity_source_room(),
                Some(PinnedActivitySourceRoom::Primal)
            );
            {
                let runtime = view.runtime.lock().await;
                let actor = admitted.runtime_actor.ok_or("actual private player")?;
                assert_eq!(
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    MovementLocalPosition {
                        x: 33565,
                        y: 32758,
                        floor: 15
                    }
                );
                assert_eq!(
                    runtime.hazard_level(actor, admitted.game_session_id),
                    Some(1)
                );
                let boss = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|creature| {
                        runtime.matches_live_creature_identity(
                            creature.actor,
                            b"oteryn:creature.the_primal_menace",
                        )
                    })
                    .ok_or("unchanged source Primal")?;
                assert_eq!(
                    runtime
                        .companion_snapshot(boss.actor)
                        .map_err(debug)?
                        .maximum_health,
                    400_000
                );
            }
            // Following stages use the same production native owner and Item writer;
            // no test boss policy, manually minted Pod, or synthetic death is seeded.
            let update_admission = admitted;
            let mut update_frames = Vec::new();
            exercise_callbacks(&h, &mut owner, &mut admitted, instance, &mut update_frames).await?;
            super::srx3_primal_terminal::final_hit(&h, &mut owner, admitted).await?;
            let after_quest = h
                .root
                .read_character_quest_state(
                    &authority,
                    admitted
                        .item_fence
                        .ok_or("real Character fence after source kill")?
                        .character_id,
                )
                .await
                .map_err(debug)?;
            assert_eq!(
                after_quest
                    .tracks()
                    .get("oteryn:quest-progress/storage/47855"),
                Some(&1),
                "the actual native Primal kill exposes the donor first-kill storage flag"
            );
            let hazard:(i16,i16,i16)=sqlx::query_as("SELECT unlocked_before,unlocked_after,selected FROM game_character_primal_hazard_receipts").fetch_one(&h.pool).await?;
            assert_eq!(
                hazard,
                (1, 2, 1),
                "real native kill settles the source hazard unlock in the composed progression transaction"
            );
            let (terminal,exit,now):(i64,i64,i64)=sqlx::query_as("SELECT terminal_at,exit_deadline,floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_boss_room_activities WHERE boss_key='oteryn:boss.the_primal_menace'").fetch_one(&h.pool).await?;
            assert_eq!(
                exit - terminal,
                60_000,
                "unchanged source after-kill duration"
            );
            let remaining = u64::try_from((exit - now).max(0))?;
            assert!(
                remaining < 60_000,
                "corpse stage consumer already consumed part of after-kill time"
            );
            tokio::time::sleep(std::time::Duration::from_millis(remaining + 5)).await;
            let before_return = admitted;
            let mut return_frames = Vec::new();
            let (_, returned, scope) = super::boss_activity::exchange_command_capture(
                &owner,
                &mut admitted,
                "oteryn:boss.the_primal_menace",
                true,
                wire::COMMAND_TYPE_BOSS_DEFEATED,
                &mut return_frames,
            )
            .await?;
            assert!(matches!(returned, wire::BossActivityReply::Accepted { .. }));
            assert!(
                scope.is_some(),
                "native return crosses the actual scope barrier"
            );
            assert!(matches!(
                admitted
                    .item_fence
                    .ok_or("returned actual fence")?
                    .runtime_scope,
                RuntimeScopeRefV1::Channel { .. }
            ));

            assert!(
                !frames.is_empty(),
                "production server frames retained for native-client playback"
            );
            write_native_capture(
                owner
                    .active_generation
                    .and_then(|active| active.native_gameplay())
                    .ok_or("actual native capture index")?,
                &frames,
                &return_frames,
                &update_frames,
                source_admitted,
                update_admission,
                before_return,
                admitted,
            )?;
            drop(view);
            drop(registered);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_PRIMAL_PRODUCTION_POSTGRES_CASE_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn exercise_callbacks(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    instance: RuntimeScopeRefV1,
    updates: &mut Vec<Vec<u8>>,
) -> TestResult {
    let generation = admitted
        .item_fence
        .ok_or("actual private fence")?
        .scope_ownership_generation
        .get();
    let registered = owner
        .activity_runtimes
        .get(instance, generation)
        .ok_or("actual private owner")?;
    let view = owner.runtime_view(&registered);
    drain_actual_native_through(&view, view.owner_now().get(), true).await?;
    drop(view);
    // CreatureCheck wakes from idle on its actual reserved tick. Source wave
    // deadlines begin in the FIRST real Lua onThink, not at allocation or at
    // handoff completion. Observe that boundary instead of inventing it.
    let mut started = false;
    for attempt in 0..=4 {
        let view = owner.runtime_view(&registered);
        if view
            .states
            .spell_states
            .lock()
            .await
            .primal_callbacks
            .source_started_for_test()
        {
            started = true;
            break;
        }
        if attempt == 4 {
            break;
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("first native think clock")?;
        let view = owner.runtime_view(&registered);
        drain_actual_native_through(&view, view.owner_now().get(), true).await?;
    }
    assert!(
        started,
        "actual reserved native Primal onThink starts the source clock"
    );
    for seconds in [11, 3, 7, 3] {
        // Existing owner-clock injection, as in the other native dispatch play
        // fixtures; no client or production command can advance this clock.
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(seconds))
            .ok_or("owner clock")?;
        let view = owner.runtime_view(&registered);
        drain_actual_native_through(&view, view.owner_now().get(), true).await?;
    }
    let view = owner.runtime_view(&registered);
    let beasts_before = {
        let runtime = view.runtime.lock().await;
        let beasts=runtime.visible_entities().creatures.into_iter().filter(|creature|
            oteryn_gameplay_encounters::bosses::hazard_primal::primal_menace_think::PRIMAL_BEAST_POOL.iter()
                .any(|key|runtime.matches_live_creature_identity(creature.actor,key.as_bytes()))).count();
        assert!(
            (3..=4).contains(&beasts),
            "source hazard1 Beast wave must produce real native children: {beasts}"
        );
        beasts
    };
    view.drain_primal_pod_callbacks().await;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = instance else {
        return Err("Instance".into());
    };
    let rows:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE r.cause_kind=2 AND r.spell_family='Encounter' AND r.spell_production_key='oteryn:encounter.the_primal_menace' AND r.runtime_scope_kind=2 AND r.channel_id IS NULL AND r.instance_id=encode($1,'hex')::uuid AND l.operation_kind=1 AND l.definition_production_key='oteryn:item.tibia.i39176' AND g.scope_kind=2 AND g.channel_id IS NULL AND g.instance_id=r.instance_id")
        .bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    let (pending_pods, next_due) = {
        let states = view.states.spell_states.lock().await;
        (
            states.primal_callbacks.pods.len(),
            states.primal_callbacks.next_due_us(),
        )
    };
    assert!(
        (1..=2).contains(&rows),
        "source hazard1 Pod wave reaches the single durable Item owner: rows={rows},pending={pending_pods},next_due={next_due:?},now={}",
        view.owner_now().get()
    );
    let stepped = super::srx3_primal_step::step(h, owner, admitted, updates).await?;
    let view = owner.runtime_view(&registered);
    tokio::time::sleep(std::time::Duration::from_millis(4100)).await;
    view.drain_primal_pod_expiries().await;
    drain_actual_native_through(&view, view.owner_now().get(), true).await?;
    let expired:i64=sqlx::query_scalar("SELECT count(*) FROM game_primal_pod_callbacks WHERE instance_id=encode($1,'hex')::uuid AND completion_kind=2 AND effect_acknowledged")
        .bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        expired,
        rows - stepped,
        "every real Pod consumes current scoped custody and executes one native expiry"
    );
    let runtime = view.runtime.lock().await;
    let fungus = runtime
        .visible_entities()
        .creatures
        .into_iter()
        .filter(|creature| {
            runtime.matches_live_creature_identity(creature.actor, b"oteryn:creature.fungosaurus")
        })
        .count();
    let beasts_after=runtime.visible_entities().creatures.into_iter().filter(|creature|
        oteryn_gameplay_encounters::bosses::hazard_primal::primal_menace_think::PRIMAL_BEAST_POOL.iter()
        .any(|key|runtime.matches_live_creature_identity(creature.actor,key.as_bytes()))).count();
    assert_eq!(
        fungus,
        usize::try_from(rows - stepped)? + beasts_before - beasts_after,
        "actual forced Pod expiry plus exact aged native Beast replacements"
    );
    drop(runtime);
    let emptied:i64=sqlx::query_scalar("SELECT count(*) FROM game_primal_pod_callbacks WHERE instance_id=encode($1,'hex')::uuid AND completion_kind=3 AND effect_acknowledged").bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        emptied, stepped,
        "removed Pod retains original timer, whose real current-position expiry finds no Item"
    );
    Ok(())
}

/// Setup drains actual retained work at one execution clock. Equal deadlines
/// alone cannot show progress: preserve source IDs and the real Profile cursor.
pub(super) async fn drain_actual_native_through(
    view: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    now: u64,
    finish_thinks: bool,
) -> TestResult {
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
    for turn in 0..256 {
        let (due, before) = actual_native_progress(view, now.get(), finish_thinks).await?;
        if !due {
            return Ok(());
        }
        Box::pin(view.drain_monster_melee_at(now)).await;
        let (due, after) = actual_native_progress(view, now.get(), finish_thinks).await?;
        if !due {
            return Ok(());
        }
        if before == after {
            return Err(format!(
                "actual native predecessor made no progress at fixed clock {}; turn={turn}; {after}",
                now.get()
            ).into());
        }
    }
    Err("actual native predecessor drain exceeded its finite 256-turn fixture bound".into())
}

async fn actual_native_progress(
    view: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    now: u64,
    finish_thinks: bool,
) -> TestResult<(bool, String)> {
    let native = view
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("actual native generation for predecessor observation")?;
    let runtime = view.runtime.lock().await;
    let states = view.states.spell_states.lock().await;
    let original =
        crate::gameplay_transport::source_death::next_source_work(&runtime, &states, native)
            .map_err(|_| "actual original owner deadline unavailable")?;
    let profile = states.native_profile_pass.pending().map(|pending| {
        (
            format!("{:p}", std::sync::Arc::as_ptr(&pending)),
            format!("{pending:?}"),
        )
    });
    let normal = states.primal_callbacks.normal_pending().map(|pending| {
        (
            format!("{:p}", std::sync::Arc::as_ptr(&pending)),
            pending.batch().observed_at_us(),
            pending.source_known(),
            pending.source_ready(),
            pending.ai_known(),
        )
    });
    let deferred = states
        .primal_callbacks
        .deferred_original_deadlines_for_test()
        .map_err(|_| "actual retained Primal owner unavailable")?;
    let pods = states
        .primal_callbacks
        .pods
        .iter()
        .map(|pod| (pod.source_transaction_id(), pod.due_us()))
        .collect::<Vec<_>>();
    let chains = states
        .monster_chains
        .as_ref()
        .map(|owner| owner.pending_sources())
        .transpose()
        .map_err(debug)?
        .unwrap_or_default()
        .into_iter()
        .map(|pending| {
            (
                pending.source,
                pending.proposal,
                pending.session,
                pending.due,
                pending.from,
            )
        })
        .collect::<Vec<_>>();
    let due = original.is_some_and(|(_, due)| due <= now)
        || profile.is_some()
        || normal.is_some()
        || runtime.has_pending_source_creature_deaths()
        || (finish_thinks
            && (states.next_monster_ai_pass_us <= now
                || states
                    .monster_melee
                    .creature_ai
                    .earliest_due()
                    .is_some_and(|due| due <= now)));
    Ok((
        due,
        format!(
            "original={original:?}; profile={profile:?}; normal={normal:?}; deferred={deferred:?}; pods={pods:?}; executions={:?}; expiries={:?}; native={:?}; steps={:?}; windups={:?}; chains={chains:?}; ai={:?}; next_pass={}; ai_sequence={}",
            states.primal_callbacks.pod_execution_deadlines_for_test(),
            states.primal_callbacks.queued_pod_expiry_sources_for_test(),
            states.native_callbacks,
            states.spite_steps.creatures,
            states.monster_spells.pending_windups(),
            states.monster_melee.creature_ai,
            states.next_monster_ai_pass_us,
            states.monster_ai_sequence
        ),
    ))
}

#[allow(
    clippy::too_many_arguments,
    reason = "capture retains three independent transport epochs and their actual admission metadata"
)]
fn write_native_capture(
    native: &crate::content::native_gameplay::NativeGameplayState,
    entry: &[Vec<u8>],
    returned: &[Vec<u8>],
    updates: &[Vec<u8>],
    source: crate::gameplay_transport::connection::AdmittedSession,
    update_admission: crate::gameplay_transport::connection::AdmittedSession,
    before_return: crate::gameplay_transport::connection::AdmittedSession,
    returned_session: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::{MessageType, decode_wire_envelope};
    let boundary = entry
        .iter()
        .position(|frame| {
            decode_wire_envelope(frame)
                .is_ok_and(|e| e.message_type() == MessageType::SnapshotCommit)
        })
        .ok_or("actual initial snapshot boundary")?
        + 1;
    let encode = |raw: &[u8]| {
        raw.iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let payload = wire::encode_boss_key("oteryn:boss.the_primal_menace").map_err(debug)?;
    let session_metadata = |s: crate::gameplay_transport::connection::AdmittedSession| {
        serde_json::json!({
            "game_session_id":encode(s.game_session_id.as_bytes()),"world_id":encode(s.world_id.as_bytes()),"channel_id":encode(s.channel_id.as_bytes()),
            "connection_generation":s.continuity.connection_generation,"current_server_sequence":s.continuity.server_sequence,"next_command_id":s.continuity.next_command_id,
            "schema_revision":crate::gameplay_transport::connection::SERVER_SCHEMA_REVISION,"selected_capabilities":s.continuity.selected_capabilities.as_slice(),
        })
    };
    let item_index = native.item_index().ok_or("actual capture Item index")?;
    let pod = item_index
        .definition_ref("Item", "oteryn:item.tibia.i39176", "definition-r1")
        .ok_or("actual capture Pod39176 reference")?;
    let corpse = item_index
        .definition_ref("Item", "oteryn:item.tibia.i39529", "definition-r1")
        .ok_or("actual capture corpse39529 reference")?;
    let mut native_corpse = false;
    for frame in returned {
        let envelope = decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotCommit {
            break;
        }
        if envelope.message_type() != MessageType::SnapshotChunk {
            continue;
        }
        let (_, domains) =
            oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
        for domain in domains {
            if domain.domain_id
                != oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
            {
                continue;
            }
            assert_eq!(
                domain.snapshot_type, 2,
                "actual cold native entity snapshot"
            );
            use oteryn_protocol_oteryn::world_spatial_entities::{
                EntityDetail, EntityFields, EntityKind,
            };
            let (snapshot, _) = oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_snapshot_with_fields(
                domain.payload,
                EntityFields::selected(before_return.continuity.selected_capabilities.as_slice()),
            ).map_err(debug)?;
            for entity in snapshot.entities {
                if entity.kind == EntityKind::Corpse
                    && let EntityDetail::ObjectWithAppearance {
                        item_definition_ref,
                        appearance_id,
                        ..
                    } = entity.detail
                    && item_definition_ref == corpse.get()
                {
                    assert_eq!(
                        appearance_id.get(),
                        39529,
                        "actual pinned corpse resource, not its generation-local definition ordinal"
                    );
                    native_corpse = true;
                }
            }
        }
    }
    assert!(
        native_corpse,
        "actual first cold return snapshot carries corpse39529 with native appearance"
    );
    let Ok(path) = std::env::var("SRX3_PRIMAL_CAPTURE_PATH") else {
        return Ok(());
    };
    let capture = serde_json::json!({
        "schema":"OTERYN_NATIVE_SCOPE_REPLAY/v1",
        "native_server_artifact_digest":encode(&native.source_digest()),
        "native_item_refs":{"pod39176":pod.get(),"corpse39529":corpse.get()},
        "character_id":encode(source.item_fence.ok_or("actual capture Character identity")?.character_id.as_bytes()),
        "transport_epochs":{"entry":"one continuous source178-to-Instance stream","updates":"separate cold private observation stream","return":"separate continuous private179-to-origin stream"},
        "source_admission":session_metadata(source),"update_admission":session_metadata(update_admission),"before_return_admission":session_metadata(before_return),"returned_admission":session_metadata(returned_session),
        "capabilities":source.continuity.selected_capabilities.as_slice(),
        "initial_frames":entry[..boundary].iter().map(|b|encode(b)).collect::<Vec<_>>(),
        "entry_command":{"kind":wire::COMMAND_TYPE_BOSS_LEVER,"payload":encode(&wire::encode_boss_key("oteryn:creature.the_primal_menace").map_err(debug)?)},
        "entry_frames":entry[boundary..].iter().map(|b|encode(b)).collect::<Vec<_>>(),
        "update_frames":updates.iter().map(|b|encode(b)).collect::<Vec<_>>(),
        "return_command":{"kind":wire::COMMAND_TYPE_BOSS_DEFEATED,"payload":encode(&payload)},
        "return_frames":returned.iter().map(|b|encode(b)).collect::<Vec<_>>(),
        "source_position":[33548,32752,-14],"destination_position":[33565,32758,-15],"return_position":[33520,32871,-15],
        "physical_runtime_positions":{"source":[33548,32752,14],"destination":[33565,32758,15],"return":[33520,32871,15]},
        "expected_status":["Accepted","Accepted"],
    });
    std::fs::write(path, serde_json::to_vec_pretty(&capture)?)?;
    Ok(())
}
