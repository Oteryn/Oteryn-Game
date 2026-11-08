//! Source-qualified native USE crosses the real activity barrier and starts real actors.
#[path = "srx3_soulpit_native/capture.rs"]
mod capture;
#[path = "srx3_soulpit_native/early_exit.rs"]
mod early_exit;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
    decode_use_intent_target, encode_item_use_intent,
};

#[test]
fn srx3_soulpit_native_use_scope_barrier_and_spawn_postgres() -> TestResult {
    native_soulpit_scenario(false, false, false)
}

#[test]
fn srx3_soulpit_committed_entry_pending_physical_transfer_never_becomes_leave_postgres()
-> TestResult {
    native_soulpit_scenario(true, false, false)
}

#[test]
fn srx3_soulpit_living_native_grace_avatar_remains_in_encounter_postgres() -> TestResult {
    native_soulpit_scenario(false, true, false)
}

#[test]
fn srx3_soulpit_native_early_exit_retires_private_encounter_postgres() -> TestResult {
    native_soulpit_scenario(false, false, true)
}

#[test]
fn srx3_soulpit_native_entry_delivers_source_teleport_effect_postgres() -> TestResult {
    native_soulpit_scenario_with_entry_cue(false, false, false, true)
}

fn native_soulpit_scenario(
    fault_after_commit: bool,
    retained_grace: bool,
    early_exit: bool,
) -> TestResult {
    native_soulpit_scenario_with_entry_cue(fault_after_commit, retained_grace, early_exit, false)
}

fn native_soulpit_scenario_with_entry_cue(
    fault_after_commit: bool,
    retained_grace: bool,
    early_exit: bool,
    entry_cue_only: bool,
) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-soulpit-native", move || {
        run(async |admin| {
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
            let (controller, pin) =
                super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
            let active = controller.active().ok_or("active")?;
            let native = active.native_gameplay().ok_or("native")?;
            let source = active
                .qualify_activity_source_room(
                    pin.qualified_room(),
                    PinnedActivitySourceRoom::Soulpit,
                )
                .map_err(debug)?;
            let room = crate::gameplay_transport::activity_runtime::source_bundle::qualify(
                &source,
                PinnedActivitySourceRoom::Soulpit,
                active,
                world,
                channel,
            )
            .map_err(|_| "Soulpit exact source Bundle")?;
            assert_eq!(
                room.pinned_activity_source_room(),
                Some(PinnedActivitySourceRoom::Soulpit)
            );
            // Use the same sealed template and canonical handoff owner as serving.rs.
            // The source room remains actual pinned geometry; no private runtime is fabricated.
            let soulpit_template = controller
                .qualify_soulpit_activity_template(
                    &room,
                    native
                        .training_formula()
                        .ok_or("active Character interpretation")?
                        .content_revision(),
                )
                .map_err(debug)?;
            // Issue all ready/session proofs after native catalogue parsing.
            let h = Harness::create_with_admission_seed(admin, "srx3_soulpit_native", false, false)
                .await?;
            let result: TestResult = async {
            super::reset();
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let account:String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            evidence
                .evidence
                .refresh_account(&h.root, &h.node, &account)
                .await
                .map_err(debug)?;
            super::activity_runtime::allocate_ready_instance(&h, 88).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
            super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i47398", 2)]).await?;
            let content = ChannelContentPin::from_activation(
                world,
                active.activation_sequence(),
                active.identity().server_artifact_digest(),
                active.identity().client_artifact_digest(),
                room.frame_binding().digest(),
                room.map_revision_digest(),
                (32375, 31158, 8),
            );
            let deps = Dependencies::with_content_and_capacity(
                content,
                room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                32,
            )?;
            {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .install_companion_policies(native.policies().clone())
                    .map_err(debug)?;
            }
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            owner.active_generation = Some(active);
            owner.activity_runtimes.register_soulpit_template(&soulpit_template)
                .map_err(|_| "register canonical Soulpit activity template")?;
            owner.proficiency_catalogue = crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue::embedded();
            assert!(
                owner
                    .proficiency_catalogue
                    .as_ref()
                    .and_then(|catalogue| catalogue.empty_track("oteryn:item.tibia.i3264"))
                    .is_some(),
                "actual promoted sword catalogue"
            );
            owner.movement_cells = room.movement_cells().into();
            owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
            {
                let runtime = owner.runtime.lock().await;
                let offers = crate::gameplay_transport::world_required_capabilities::with_native_map_presentation_offer(
                    owner.offered_capabilities(),Some(active),Some(&room),runtime.content_pin());
                owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                    &[11,18,30],runtime.binding().runtime_scope(),runtime.binding().scope_generation().get(),"policy-1",offers)
                    .map_err(debug)?;
            }
            owner
                .states
                .boss_activity_handoff
                .install(std::sync::Arc::new(
                    crate::gameplay_transport::soulpit::SharedSoulpitOwner,
                ))
                .map_err(|_| "install shared Soulpit owner")?;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(deps.admitted.game_session_id)
                    .await?
                    .0;
            let mut admitted = deps.admitted;
            admitted.controller.as_mut().ok_or("controller")?.transport =
                current.current_transport().ok_or("current transport")?;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            // This native scene exercises inventory, combat, map, presentation,
            // Source surfaces/icons and their dependencies. Unrelated feature
            // providers are not installed by this focused fixture.
            let supported = [4,6,7,10,11,12,15,17,18,23,30,92,117,370,371,385];
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::connection::fresh_capability_selection(&owner, &supported)
                    .await.ok_or("native selection")?;
            assert_eq!(
                owner.required_capabilities.authenticate_resume_support(
                    &supported,admitted.continuity.selected_capabilities,
                ).map_err(debug)?.selected(),
                admitted.continuity.selected_capabilities,
                "the actual BOOT descriptor preserves the source native selection",
            );
            super::arena_match::fixture::equip_weapon(&h, "oteryn:item.tibia.i3264").await?;
            super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 1999)
                .await?;
            let effects = owner
                .read_admission_effects(admitted.game_session_id)
                .await
                .map_err(|_| "native equipped weapon facts")?;
            {
                let runtime = owner.runtime.lock().await;
                owner
                    .install_admission_suppressions(
                        &runtime,
                        admitted.runtime_actor.ok_or("source player")?,
                        admitted.game_session_id,
                        effects,
                    )
                    .await
                    .ok_or("publish actual native equipped weapon")?;
            }
            owner
                .activity_runtimes
                .register_source_room(PinnedActivitySourceRoom::Soulpit, &room)
                .map_err(|_| "retain Soulpit room")?;
            let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
                room.movement_cells().index()
            else {
                return Err("native Bundle missing".into());
            };
            let mut placement = index.map().overlay.base().digest().to_vec();
            placement.extend_from_slice(
                &oteryn_world_bundle::bundle::placement_key(-8, 32375, 31157, 3)
                    .ok_or("source obelisk placement")?
                    .to_be_bytes(),
            );
            let reference = native
                .item_index()
                .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i47398", "definition-r1"))
                .ok_or("native Soul Core reference")?;
            let intent = UseIntent {
                target: UseTarget::ItemByDefinition(reference),
                use_with: None,
                use_with_position: None,
                use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                    placement,
                    expected_revision: 0,
                })),
            };
            let encoded = encode_item_use_intent(&intent).map_err(debug)?;
            let decoded = decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                &encoded,
            )
            .map_err(debug)?;
            let core_inventory = h.root.read_item_use_inventory(&authority, &h.node,
                admitted.item_fence.ok_or("current Core inventory fence")?).await.map_err(debug)?;
            let interpretation = native.training_formula().ok_or("actual active Character interpretation")?.content_revision();
            assert_eq!(core_inventory.content_revision,interpretation,
                "native entry must use the active interpretation already held by the current Character");
            assert_ne!(active.identity().content_revision(),interpretation,
                "this regression fixture keeps artifact and Character revision namespaces distinct");
            eprintln!("SRX3_SOULPIT PREFLIGHT stage=interpretation active={interpretation} artifact={}",active.identity().content_revision());
            let actual_core = core_inventory.backpack.as_ref().into_iter().flat_map(|bag| &bag.entries)
                .find(|entry| entry.item.definition.production_key == "oteryn:item.tibia.i47398")
                .ok_or("actual fenced Core stack")?;
            eprintln!("SRX3_SOULPIT PREFLIGHT stage=core_inventory definition={} quantity={} instance={:02x?}",
                actual_core.item.definition.production_key, actual_core.item.quantity, actual_core.item.item_instance_id);
            assert!(admitted.continuity.selected_capabilities.contains(
                oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1),
                "the existing activity registry requires selected capability117");
            assert!(owner.activity_runtimes.connection(admitted.game_session_id).is_some(),
                "the actual source activity connection was remembered");
            // A direct composed fixture has no node ready-maintenance loop.
            // Refresh current evidence after all class/equipment preparation.
            h.root.request_ready();
            assert!(h.root.maintain_ready_once().await.map_err(debug)?);
            evidence.evidence.refresh_account(&h.root,&h.node,&account).await.map_err(debug)?;
            let preflight = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id).await?.0;
            let runtime = owner.runtime.lock().await;
            let current_at = runtime.read_actor_position(deps.actor).map_err(debug)?.position();
            let control = runtime.player_control_facts(deps.actor,admitted.game_session_id).map_err(debug)?;
            eprintln!("SRX3_SOULPIT PREFLIGHT stage=before_native_use duration={:?} session_state={:?} connection_generation={} character_lease={} position={current_at:?} control_loss={} fence_matches={}",
                started.elapsed(),preflight.session_state(),preflight.current_connection_generation().get(),
                preflight.current_character_lease().generation(),control.control_loss.is_some(),
                crate::gameplay_transport::item_fence_of(preflight)==admitted.item_fence);
            let states = owner.states.spell_states.lock().await;
            let facts = states.get(&runtime, deps.actor, admitted.game_session_id)
                .ok_or("actual Source PlayerSpellState")?.character_facts();
            eprintln!("SRX3_SOULPIT PREFLIGHT stage=actual_player_state level={} vocation={:?} health={}",
                facts.level, facts.vocation, states.get(&runtime,deps.actor,admitted.game_session_id).ok_or("Source vitals")?.vitals().health);
            drop(states);
            drop(runtime);
            let source_admission = admitted;
            let mut entry_frames = Vec::new();
            let mut actual_entry_command = Vec::new();
            let mut entry_scope = None;
            if fault_after_commit {
                crate::gameplay_transport::native_activity::srx3_faults::stop_after_soulpit_commit_once();
            }
            let result = if fault_after_commit { owner
                .dispatch_soulpit_items(
                    deps.actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 1,
                        item_fence: admitted.item_fence,
                    },
                    &decoded,
                    None,
                    None,
                )
                .await } else {
                let build_payload = |frames: &[Vec<u8>]| capture::core_use_payload(frames,&intent,reference.get(),
                    source_admission.continuity.selected_capabilities.as_slice().contains(&oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_EQUIP_DROP_V1));
                let (status, payload, scope, actual_command) = if entry_cue_only {
                    // Canary47df soulpit fight:75 -> libs/functions/lever.lua165-175:
                    // the actual destination player receives CONST_ME_TELEPORT (11).
                    super::guild_wire::exchange_capture_switch_effect(&owner,&mut admitted,1,
                        oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,
                        build_payload,11,&mut entry_frames).await?
                } else {
                    super::guild_wire::exchange_capture(&owner,&mut admitted,1,
                        oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,
                        build_payload,true,&mut entry_frames).await?
                };
                actual_entry_command = actual_command;
                assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
                entry_scope = scope;
                let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(&payload).map_err(debug)?;
                Ok(Some(crate::gameplay_transport::connection::UseOutcome {
                    disposition, ..crate::gameplay_transport::connection::UseOutcome::rejected()
                }))
            };
            if fault_after_commit {
                assert!(
                    result.is_err(),
                    "the simulated native transfer stopped after durable commit: {result:?}"
                );
                let registered = owner
                    .activity_runtimes
                    .activity_owners()
                    .map_err(|_| "actual pending owner")?
                    .into_iter()
                    .find(|owner| {
                        owner
                            .template
                            .as_ref()
                            .is_some_and(|template| template.key == "oteryn:room.soulpit-source")
                    })
                    .ok_or("retained physical Soulpit owner")?;
                let generation =
                    crate::foundation::ScopeOwnershipGeneration::new(registered.generation)
                        .map_err(debug)?;
                let bound = h
                    .root
                    .read_soulpit_scope_binding(&authority, &h.node, registered.scope, generation)
                    .await?
                    .ok_or("actually committed Soulpit handoff")?;
                let step = owner
                    .drive_one_soulpit(&registered, owner.owner_now().get(), false)
                    .await;
                assert!(matches!(step,Err(crate::gameplay_transport::soulpit::runtime::SoulpitRuntimeError::AdapterUnavailable)),
                    "unmaterialized ACTIVE successor is pending transfer, never a departure: {step:?}");
                let stored = h
                    .root
                    .read_soulpit_encounter(
                        &authority,
                        &h.node,
                        bound.handoff_id,
                        registered.scope,
                        generation,
                    )
                    .await?
                    .ok_or("retained unmodified fight")?;
                assert_eq!(
                    stored.revision, 0,
                    "physical uncertainty cannot commit Leave"
                );
                assert_eq!(stored.encounter.snapshot().surviving_slots, vec![0]);
                let now = owner.owner_now().get();
                let due = tokio::time::timeout(std::time::Duration::from_secs(5),
                    owner.drive_shared_soulpits(now,false)).await
                    .map_err(|_| "pending native transfer must yield so lifecycle repair and other Instances can run")?
                    .map_err(|_| "pending native transfer schedules a retry instead of stopping the owner")?
                    .ok_or("pending native transfer keeps an owner retry deadline")?;
                assert!(due > now,"uncertain entry schedules a later bounded owner retry");
                let retry = owner
                    .dispatch_soulpit_items(
                        deps.actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: 1,
                            item_fence: admitted.item_fence,
                        },
                        &decoded,
                        None,
                        None,
                    )
                    .await
                    .map_err(|_| "repair actual pending entry")?
                    .ok_or("repaired pending native USE")?;
                assert_eq!(retry.disposition, UseDisposition::Committed);
                let quantity:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(quantity, 1, "repair never debits the core again");
                return Ok(());
            }
            let result = result
                .map_err(|_| "native Soulpit USE")?
                .ok_or("Soulpit USE owner")?;
            assert_eq!(result.disposition, UseDisposition::Committed);
            let barrier = entry_scope.ok_or("actual continuous native scope barrier")?;
            assert!(matches!(
                barrier.scope,
                oteryn_protocol_oteryn::activity_scope::ActivityScope::Instance { .. }
            ));
            assert_eq!(barrier.source_session, *source_admission.game_session_id.as_bytes());
            assert_ne!(
                barrier.session, barrier.source_session,
                "the destination command stream binds its fresh GameSession identity"
            );
            assert_eq!(
                barrier.connection_generation,
                admitted.continuity.connection_generation
            );
            let destination = admitted;
            let fence = destination.item_fence.ok_or("destination fence")?;
            let registered = owner
                .activity_runtimes
                .get(fence.runtime_scope, fence.scope_ownership_generation.get())
                .ok_or("native Instance")?;
            let destination_view = owner.runtime_view(&registered);
            if retained_grace {
                let actor = destination.runtime_actor.ok_or("grace actor")?;
                let controller = destination.controller.ok_or("grace controller")?;
                assert_eq!(
                    destination_view
                        .commit_control_loss(destination.game_session_id, actor, controller,)
                        .await,
                    crate::gameplay_transport::connection::ControlLossResult::Recorded
                );
                let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                    h.root.clone(),
                )
                .current_session_at(destination.game_session_id)
                .await
                .map_err(debug)?
                .0;
                assert_eq!(
                    current.session_state(),
                    crate::foundation::GameSessionState::Reconnectable
                );
                {
                    let runtime = registered.runtime.lock().await;
                    let states = registered.states.spell_states.lock().await;
                    assert!(
                        runtime
                            .player_control_facts(actor, destination.game_session_id)
                            .map_err(debug)?
                            .control_loss
                            .is_some()
                    );
                    assert!(
                        states
                            .get(&runtime, actor, destination.game_session_id)
                            .ok_or("retained grace avatar spell state")?
                            .vitals()
                            .health
                            > 0
                    );
                    assert_eq!(
                        runtime
                            .read_actor_position(actor)
                            .map_err(debug)?
                            .position(),
                        MovementLocalPosition {
                            x: 32373,
                            y: 31151,
                            floor: 8
                        }
                    );
                    assert!(crate::durability::admission_journal::party_target_binding::observe_current_party_actor_for_source_read(
                        &runtime, actor, destination.game_session_id,
                    ).is_none(), "a visible grace avatar acquires no active input observation");
                    let visible = crate::durability::admission_journal::party_target_binding::observe_current_visible_actor_for_source_read(
                        &runtime, actor, destination.game_session_id,
                    ).ok_or("actual living grace avatar remains visible for Source reads")?;
                    assert!(visible.matches(&runtime), "readonly observation binds the actual current physical owner");
                }
                owner
                    .drive_one_soulpit(&registered, owner.owner_now().get(), false)
                    .await
                    .map_err(|error| {
                        format!(
                            "living retained grace avatar is not a Source zone leave: {error:?}"
                        )
                    })?;
                let bound = h
                    .root
                    .read_soulpit_scope_binding(
                        &authority,
                        &h.node,
                        fence.runtime_scope,
                        fence.scope_ownership_generation,
                    )
                    .await
                    .map_err(debug)?
                    .ok_or("retained Source grace encounter")?;
                let stored = h
                    .root
                    .read_soulpit_encounter(
                        &authority,
                        &h.node,
                        bound.handoff_id,
                        fence.runtime_scope,
                        fence.scope_ownership_generation,
                    )
                    .await
                    .map_err(debug)?
                    .ok_or("current Source grace checkpoint")?;
                assert!(
                    !stored.encounter.snapshot().phase.is_terminal(),
                    "transport loss cannot abandon a living retained zone avatar"
                );
                assert_eq!(stored.encounter.snapshot().surviving_slots, vec![0]);
                return Ok(());
            }
            for required in [6, 23, 371, 385] {
                assert!(source_admission.continuity.selected_capabilities.as_slice().contains(&required),
                    "actual native source proof selects capability{required}");
            }
            let mut native_capture = capture::NativeCapture {
                source: source_admission, entry_command: actual_entry_command, entry_frames,
                stages: Vec::new(), before_return: None, return_frames: Vec::new(), returned: None,
            };
            if entry_cue_only {
                let mut destination_cue = false;
                for raw in &native_capture.entry_frames {
                    let frame = oteryn_protocol_oteryn::decode_wire_envelope(raw).map_err(debug)?;
                    if frame.message_type() != oteryn_protocol_oteryn::MessageType::StateDelta { continue; }
                    let delta = oteryn_protocol_oteryn::decode_state_delta(frame.payload()).map_err(debug)?;
                    if delta.domain_id != oteryn_protocol_oteryn::presentation::STATE_DOMAIN_WORLD_PRESENTATION { continue; }
                    assert_eq!(delta.delta_type,oteryn_protocol_oteryn::native_map_cues::DELTA_TYPE_NATIVE_MAP_PRESENTATION_V1);
                    destination_cue |= oteryn_protocol_oteryn::native_map_cues::decode_native_map_presentation_batch(delta.payload)
                        .map_err(debug)?.iter().any(|event| matches!(event.kind,
                            oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect { position, effect_id:11, .. }
                            if position == oteryn_protocol_oteryn::world_spatial::ActorPosition{x:32373,y:31151,floor:-8}));
                }
                assert!(destination_cue, "the real entry teleport cue uses the destination Instance's native frame");
                eprintln!("SRX3_SOULPIT NATIVE_ENTRY_CUE_COMPLETED duration={:?}", started.elapsed());
                return Ok(());
            }
            if early_exit {
                return early_exit::scenario(&h,&mut owner,&native_capture,destination,registered).await;
            }
            let own = destination_view
                .observe(destination.runtime_actor.ok_or("destination actor")?)
                .await
                .ok_or("domain1 own actor")?;
            assert_eq!(
                own.actor_position,
                oteryn_protocol_oteryn::world_spatial::ActorPosition {
                    x: 32373,
                    y: 31151,
                    floor: -8
                }
            );
            let mut map_view = crate::gameplay_transport::world_map::SessionMapView::default();
            let mut items =
                crate::gameplay_transport::item_view::SessionItemView::resume(Default::default())
                    .with_map_view();
            let update = destination_view
                .observe_world_map(
                    destination.runtime_actor.ok_or("destination actor")?,
                    &mut map_view,
                    &mut items,
                )
                .await
                .ok_or("domain17 observation")?
                .map_err(debug)?
                .ok_or("domain17 first snapshot")?;
            let crate::gameplay_transport::world_map::MapUpdate::Snapshot(snapshot) = update else {
                return Err("expected destination map snapshot".into());
            };
            let window =
                oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
                    .map_err(debug)?;
            assert_eq!(
                window.header.origin, own.actor_position,
                "native map and own actor must use the exact same floor/frame"
            );
            let active_obelisk = native
                .item_index()
                .ok_or("native Item index")?
                .definition_ref("Item", "oteryn:item.tibia.i47379", "definition-r1")
                .ok_or("pinned active Soulpit obelisk")?;
            let obelisk = window
                .tiles
                .iter()
                .find(|tile| {
                    (tile.position.x, tile.position.y, tile.position.floor) == (32375, 31157, -8)
                })
                .ok_or("native Instance obelisk tile")?;
            assert!(
                obelisk.items.iter().any(|item| item.definition
                    == oteryn_protocol_oteryn::world_map::MapDefinition::Item(active_obelisk)),
                "the active fight transforms only its independently owned Instance obelisk"
            );
            let channel_map = owner.bundle_map().ok_or("actual source Channel map")?;
            let channel_glyph = crate::map::view::compose_tile(
                &channel_map.overlay,&channel_map.facts,
                crate::map::overlay::TilePos {x:32375,y:31157,floor:-8},
            ).map_err(debug)?.ok_or("actual source Channel obelisk")?;
            assert!(channel_glyph.entries.iter().any(|entry|entry.facts.appearance_id==47367)
                && channel_glyph.entries.iter().all(|entry|entry.facts.appearance_id!=47379),
                "an active private fight leaves the real Channel obelisk available to another party");
            let runtime = registered.runtime.lock().await;
            assert_eq!(
                runtime
                    .read_actor_position(destination.runtime_actor.ok_or("destination actor")?)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    x: 32373,
                    y: 31151,
                    floor: 8
                }
            );
            drop(runtime);
            owner
                .drive_shared_soulpits(owner.owner_now().get(), false)
                .await
                .map_err(|_| "prepare actual Source wave before its deadline")?;
            let mut preparation = owner.runtime_view(&registered);
            leave_blocked_creation_cell(&mut preparation, destination, 1).await?;
            owner.clock_origin = preparation.clock_origin;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_micros(4_000_001))
                .ok_or("advance actual native owner clock")?;
            owner
                .drive_shared_soulpits(owner.owner_now().get(), false)
                .await
                .map_err(|_| "source four-second wave")?;
            let runtime = registered.runtime.lock().await;
            let creatures = runtime
                .positioned_actor_census()
                .map_err(debug)?
                .into_iter()
                .filter(|(_, _, session)| session.is_none())
                .count();
            assert!(
                creatures >= 1,
                "a retained real Source spawn cell is deterministically unoccupied"
            );
            assert!(
                creatures <= 7,
                "seven source attempts never redraw a blocked cell"
            );
            drop(runtime);
            let entities = destination_view
                .observe_channel_entities(destination.runtime_actor.ok_or("destination actor")?)
                .await
                .ok_or("native visible entities")?;
            assert!(
                !entities.entities.is_empty(),
                "the native player is visible"
            );
            assert!(
                entities
                    .entities
                    .iter()
                    .all(|entity| entity.position.floor == -8),
                "players, creatures and corpses share the native map wire frame"
            );
            let bound = h
                .root
                .read_soulpit_scope_binding(
                    &authority,
                    &h.node,
                    fence.runtime_scope,
                    fence.scope_ownership_generation,
                )
                .await
                .map_err(debug)?
                .ok_or("current source fight")?;
            let stored = h
                .root
                .read_soulpit_encounter(
                    &authority,
                    &h.node,
                    bound.handoff_id,
                    fence.runtime_scope,
                    fence.scope_ownership_generation,
                )
                .await
                .map_err(debug)?
                .ok_or("acknowledged source wave")?;
            assert_eq!(
                stored.encounter.snapshot().monsters_alive,
                creatures,
                "the acknowledged seven-attempt receipt describes the exact native population"
            );
            assert_eq!(
                stored.encounter.snapshot().phase,
                oteryn_gameplay_domain::domain::soulpit::EncounterPhase::Fighting
            );
            eprintln!("SRX3_SOULPIT native source wave actual_actors={creatures} attempts=7");
            let mut player = destination;
            let actor = player.runtime_actor.ok_or("actual Instance player")?;
            let mut instance = owner.runtime_view(&registered);
            let mut native_bites = 0;
            let mut completed_return = None;
            for wave in 1..=4 {
                let checkpoint = h.root.read_soulpit_encounter(
                    &authority,&h.node,bound.handoff_id,fence.runtime_scope,fence.scope_ownership_generation,
                ).await?.ok_or("actual current wave receipt")?;
                let saved = serde_json::to_value(checkpoint.encounter.checkpoint())?;
                let receipt: [Option<u64>;oteryn_gameplay_domain::domain::soulpit::MONSTERS_PER_WAVE] =
                    serde_json::from_value(saved["spawned_waves"][wave.to_string()].clone())?;
                let stacks = oteryn_gameplay_domain::domain::soulpit::wave_stacks(wave).map_err(debug)?;
                let mut expected_counts = std::collections::BTreeMap::from([(1_u8,0_usize),(5,0),(15,0),(40,0)]);
                for (id,stack) in receipt.into_iter().zip(stacks) {
                    if let Some(id) = id {
                        *expected_counts.get_mut(&(stack as u8)).ok_or("source receipt stack")? += 1;
                        let monster = checkpoint.encounter.monster(id).ok_or("live actual receipt monster")?;
                        assert_eq!(monster.stack,stack);
                        assert!(!monster.is_summon);
                    }
                }
                let (targets,observer_target) = {
                    let runtime = registered.runtime.lock().await;
                    let targets = runtime
                        .positioned_actor_census()
                        .map_err(debug)?
                        .into_iter()
                        .filter_map(|(target, _, session)| {
                            (session.is_none()
                                && runtime
                                    .soulpit_variant(target)
                                    .is_some_and(|tag| tag.wave == wave && !tag.is_summon)
                                && runtime.contains_live_creature(target))
                            .then_some(target)
                        })
                        .collect::<Vec<_>>();
                    let mut actual_counts = std::collections::BTreeMap::from([(1_u8,0_usize),(5,0),(15,0),(40,0)]);
                    let mut observer = None;
                    for target in &targets {
                        let tag = runtime.soulpit_variant(*target).ok_or("actual visible-wave tag")?;
                        assert!(receipt.contains(&Some(tag.monster_id)));
                        *actual_counts.get_mut(&(tag.stack as u8)).ok_or("source actor stack")? += 1;
                        if observer.is_none_or(|(_,stack)| (tag.stack as u8)>stack) {
                            observer = Some((*target,tag.stack as u8));
                        }
                    }
                    assert_eq!(actual_counts,expected_counts,
                        "wave{wave} actual non-summon stack census matches all seven retained creation outcomes");
                    eprintln!("SRX3_SOULPIT WAVE_STACK_COUNTS wave={wave} actual={actual_counts:?}");
                    (targets,observer.ok_or("actual wave observation target")?.0)
                };
                assert!(
                    !targets.is_empty(),
                    "source wave{wave} has actual physical monsters"
                );
                walk_source_adjacent(&mut instance,player,observer_target).await?;
                native_capture.wave(&instance,&mut player,wave).await?;
                for target in targets {
                    native_bites += usize::from(
                        fight_source_monster(&mut instance, &mut player, actor, target).await?,
                    );
                    let _ = instance.drain_kill_rewards(player.game_session_id).await;
                }
                let xp: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(sum(base_xp),0)::bigint FROM game_party_kill_xp_receipts",
                )
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(
                    xp,
                    if wave == 4 { 140 } else { 0 },
                    "only the physically killed non-summon stack40 rat grants28x5 baseXP"
                );
                assert_eq!(
                    instance
                        .states
                        .proficiencies
                        .lock()
                        .map_err(debug)?
                        .get(&player.game_session_id)
                        .and_then(|state| state.track("oteryn:item.tibia.i3264"))
                        .map(|track| track.progress),
                    Some(if wave == 4 { 1500 } else { 0 }),
                    "only non-summon stack40 awards1500 points through the original equipped weapon owner"
                );
                owner.clock_origin = instance.clock_origin;
                if wave == 4 {
                    native_capture.before_return = Some(player);
                    let returning_player = player;
                    completed_return = Some(super::guild_wire::capture_activity_return(
                        &owner, &mut player, &mut native_capture.return_frames,
                        async {
                            // The real Node owner (soulpit/cadence.rs) honors returned
                            // retry deadlines and keeps global clearance on its one-second
                            // clock. This standalone fixture has no Node maintenance task.
                            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                                let mut global_due = owner.owner_now().get();
                                loop {
                                    let now = owner.owner_now().get();
                                    let global = now >= global_due;
                                    if global { global_due = now.saturating_add(1_000_000); }
                                    let deadline = owner.drive_shared_soulpits(now,global).await
                                        .map_err(|_| "actual Source completion return")?;
                                    if owner.activity_runtimes.current_connection(returning_player).game_session_id
                                        != returning_player.game_session_id { break; }
                                    // try_acquire_ready coalesces demand on exhaustion.
                                    // Use the normal bounded maintenance owner, never an
                                    // extended pass/lease or a bypass of current fences.
                                    h.root.maintain_ready_once().await.map_err(|error| format!("{error:?}"))?;
                                    let due = deadline.map_or(global_due,|at|at.min(global_due));
                                    tokio::time::sleep(std::time::Duration::from_micros(
                                        due.saturating_sub(owner.owner_now().get()))).await;
                                }
                                Ok::<(),String>(())
                            }).await.map_err(|_| "actual Source completion retry deadline exceeded")??;
                            Ok(())
                        },
                    ).await?);
                    native_capture.returned = Some(player);
                } else {owner
                    .drive_shared_soulpits(owner.owner_now().get(), true)
                    .await
                    .map_err(|_| "source wave clearance/terminal composition")?;}
                if wave < 4 {
                    let mut preparation = owner.runtime_view(&registered);
                    leave_blocked_creation_cell(&mut preparation, player, wave + 1).await?;
                    owner.clock_origin = preparation.clock_origin;
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_micros(4_000_001))
                        .ok_or("source next wave deadline")?;
                    owner
                        .drive_shared_soulpits(owner.owner_now().get(), false)
                        .await
                        .map_err(|_| "next actual native wave")?;
                    instance = owner.runtime_view(&registered);
                }
            }
            assert!(
                native_bites > 0,
                "the actual native monster AI commits incoming Source bites"
            );
            eprintln!(
                "SRX3_SOULPIT INCOMING_PROOF actual native attackers_with_bite_receipts={native_bites}"
            );
            let mastery: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_animus_masteries WHERE character_id=encode($1,'hex')::uuid AND species_key='oteryn:creature.rat'")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                mastery, 1,
                "four real native waves award Animus exactly once"
            );
            let return_scope = completed_return.ok_or("actual autonomous terminal barrier")?;
            assert!(matches!(
                return_scope.scope,
                oteryn_protocol_oteryn::activity_scope::ActivityScope::Channel { .. }
            ));
            assert_eq!(
                return_scope.source_session,
                *native_capture.before_return.ok_or("return source epoch")?.game_session_id.as_bytes()
            );
            assert_ne!(return_scope.session, return_scope.source_session);
            assert_eq!(return_scope.session,*player.game_session_id.as_bytes());
            native_capture.write()?;
            let ordinary_corpses: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_reward_claim_mint_receipts)+(SELECT count(*) FROM game_item_corpse_container_entries)")
                .fetch_one(&h.pool).await?;
            assert_eq!(
                ordinary_corpses, 0,
                "Source variants never materialize ordinary corpses"
            );
            let quantity:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(quantity, 1);
            Ok(())
            }.await;
            result?;
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_SOULPIT POSTGRES_CASE_COMPLETED native_use_scope_barrier_spawn fault_after_commit={fault_after_commit} retained_grace={retained_grace} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn fight_source_monster(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    actor: crate::foundation::ExactActorRef,
    target: crate::foundation::ExactActorRef,
) -> TestResult<bool> {
    use oteryn_protocol_oteryn::attack::{self, ChaseMode, FightMode, FightModes};
    walk_source_adjacent(owner, *admitted, target).await?;
    let reference = {
        let runtime = owner.runtime.lock().await;
        let generation = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|creature| creature.actor == target)
            .ok_or("visible source monster")?
            .generation;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: target.placement_identity(),
            generation,
        }
    };
    let command = admitted.continuity.next_command_id;
    let (status, payload) = super::guild_wire::exchange(
        owner,
        admitted,
        command,
        attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
        &attack::encode_attack_target_intent(Some(&reference)).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    assert_eq!(
        attack::decode_attack_intent_result(&payload).map_err(debug)?,
        attack::AttackIntentDisposition::Ok
    );
    assert_eq!(
        owner
            .fight_modes(
                actor,
                admitted.game_session_id,
                FightModes {
                    fight_mode: FightMode::Offensive,
                    chase: ChaseMode::Chase,
                    secure: true,
                }
            )
            .await,
        attack::AttackIntentDisposition::Ok
    );
    let stack = owner
        .runtime
        .lock()
        .await
        .soulpit_variant(target)
        .ok_or("actual Source tag")?
        .stack as u8;
    let mut saw_bite = false;
    for turn in 1..=512 {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(3))
            .ok_or("advance native combat owner clock")?;
        owner.drain_monster_melee().await;
        owner
            .drive_shared_soulpits(owner.owner_now().get(), false)
            .await
            .map_err(|_| "native Soulpit boss ability deadline")?;
        let _ = owner.chase_step(actor, admitted.game_session_id).await;
        // Let the actual adjacent native attacker meet its own scheduling window
        // before the high-level player's counterattack. No damage is injected.
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_micros(1_600_001))
            .ok_or("actual native opponent attack window")?;
        owner.drain_monster_melee().await;
        owner
            .drive_shared_soulpits(owner.owner_now().get(), false)
            .await
            .map_err(|_| "actual Source ability scheduling")?;
        if !saw_bite
            && let Some(bite) = owner
                .states
                .spell_states
                .lock()
                .await
                .monster_melee
                .last_native_bite_for_test(target, 0)
        {
            eprintln!(
                "SRX3_SOULPIT INCOMING_COMPONENT stack={stack} actual_preblock_requested={} actual_applied={}",
                bite.requested, bite.damage.applied
            );
            saw_bite = true;
        }
        owner
            .refresh_attack_weapon(actor, admitted.game_session_id)
            .await;
        owner.drain_auto_attacks().await;
        if !owner.runtime.lock().await.contains_live_creature(target) {
            eprintln!("SRX3_SOULPIT COMBAT_PROOF actual native target defeated turns={turn}");
            return Ok(saw_bite);
        }
    }
    Err("native Soulpit monster survived512 real combat turns".into())
}

// Test navigation uses real paced movement over current qualified collision.
// It cannot move actors directly, invent tiles, redraw spawns or select unseen
// native attack references. Cardinal paths also avoid diagonal corner shortcuts.
async fn walk_source_adjacent(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    player: crate::gameplay_transport::connection::AdmittedSession,
    target: crate::foundation::ExactActorRef,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
    use std::collections::{BTreeSet, VecDeque};
    let actor = player
        .runtime_actor
        .ok_or("actual navigating Source player")?;
    for _ in 0..256 {
        let direction = {
            let runtime = owner.runtime.lock().await;
            let current = runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            let census = runtime.positioned_actor_census().map_err(debug)?;
            let destination = census
                .iter()
                .find(|(a, _, _)| *a == target)
                .ok_or("actual live navigation target")?
                .1
                .position();
            assert_eq!(current.floor, destination.floor);
            if (current.x - destination.x).abs() + (current.y - destination.y).abs() == 1 {
                return Ok(());
            }
            let occupied = census
                .iter()
                .filter(|(a, _, _)| *a != actor)
                .map(|(_, at, _)| {
                    let at = at.position();
                    (at.x, at.y)
                })
                .collect::<BTreeSet<_>>();
            let mut pending = VecDeque::from([((current.x, current.y), None)]);
            let mut seen = BTreeSet::from([(current.x, current.y)]);
            let mut selected = None;
            while let Some(((x, y), first)) = pending.pop_front() {
                for (dx, dy, step) in [
                    (0, -1, StepDirection::North),
                    (1, 0, StepDirection::East),
                    (0, 1, StepDirection::South),
                    (-1, 0, StepDirection::West),
                ] {
                    let next = (x + dx, y + dy);
                    if !(32362..=32390).contains(&next.0)
                        || !(31132..=31153).contains(&next.1)
                        || occupied.contains(&next)
                        || seen.contains(&next)
                        || owner.effective_movement_cells().index().lookup(
                            owner.effective_movement_cells().scope(),
                            crate::content::LogicalCell {
                                x: next.0,
                                y: next.1,
                                z: i32::from(current.floor),
                            },
                        ) != Ok(crate::content::CollisionClass::Walkable)
                    {
                        continue;
                    }
                    seen.insert(next);
                    let first = first.unwrap_or(step);
                    if (next.0 - destination.x).abs() + (next.1 - destination.y).abs() == 1 {
                        selected = Some(first);
                        break;
                    }
                    pending.push_back((next, Some(first)));
                }
                if selected.is_some() {
                    break;
                }
            }
            selected.ok_or("no actual unoccupied Source path reaches the native target")?
        };
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_millis(500))
            .ok_or("advance native paced navigation clock")?;
        let (step, _) = owner
            .paced_step(actor, player.game_session_id, direction)
            .await;
        assert_eq!(
            step.disposition,
            StepDisposition::Moved,
            "Source navigation uses the actual movement owner"
        );
    }
    Err("Source navigation exceeded its bounded real arena path".into())
}

async fn leave_blocked_creation_cell(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    player: crate::gameplay_transport::connection::AdmittedSession,
    wave: u8,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
    let fence = player.item_fence.ok_or("actual Instance fence")?;
    let actor = player.runtime_actor.ok_or("actual Instance actor")?;
    let bound = owner
        .root
        .read_soulpit_scope_binding(
            owner.character,
            owner.holder,
            fence.runtime_scope,
            fence.scope_ownership_generation,
        )
        .await?
        .ok_or("actual preparing Source group")?;
    let (plan, current) = {
        let mut runtime = owner.runtime.lock().await;
        let current = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let (carrier, continuity, _) = runtime.soulpit_kernel_parts().map_err(debug)?;
        let plan = carrier
            .soulpit_wave_plan(continuity, bound.handoff_id, wave)
            .map_err(debug)?
            .ok_or("production retained Source wave plan")?;
        (plan, current)
    };
    let points = plan.positions.iter().flatten().copied().collect::<Vec<_>>();
    assert!(
        !points.is_empty(),
        "actual Source Zone has a creation candidate"
    );
    let blocks_all = points.iter().all(|at| *at == current);
    let blocks_boss = wave == 4 && current.x == 32376 && current.y == 31144;
    if !blocks_all && !blocks_boss {
        return Ok(());
    }
    for (dx, dy, direction) in [
        (0, -1, StepDirection::North),
        (1, 0, StepDirection::East),
        (0, 1, StepDirection::South),
        (-1, 0, StepDirection::West),
    ] {
        let at = MovementLocalPosition {
            x: current.x + dx,
            y: current.y + dy,
            ..current
        };
        if !(32362..=32390).contains(&at.x)
            || !(31132..=31153).contains(&at.y)
            || owner.effective_movement_cells().index().lookup(
                owner.effective_movement_cells().scope(),
                crate::content::LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            ) != Ok(crate::content::CollisionClass::Walkable)
        {
            continue;
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("advance actual native step clock")?;
        let (step, _) = owner
            .paced_step(actor, player.game_session_id, direction)
            .await;
        assert_eq!(
            step.disposition,
            StepDisposition::Moved,
            "real movement frees the Source creation cell"
        );
        let moved = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        assert_ne!(moved, current);
        eprintln!("SRX3_SOULPIT SPAWN_PROOF actual paced step frees retained cell wave={wave}");
        return Ok(());
    }
    Err("no actual adjacent cell can free the source spawn".into())
}
