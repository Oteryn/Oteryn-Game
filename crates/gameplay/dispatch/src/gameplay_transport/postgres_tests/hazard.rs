//! HAZARD: NPC bytes through serve_admitted and committed native boss death through the kill
//! router, with durable qualification, selection, replay, restart and session fence checks.
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, GameSessionId, MessageType, MovementLocalPosition, NodeId,
    WorldId, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, FirstEntryOutcome, IDLE_LIVENESS, SessionContinuity,
    serve_admitted,
};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
use crate::gameplay_transport::{ComposedFreshAdmission, character_gameplay_fence};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::hazard;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::npc_service::{self as wire, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

// Level 55 admits the native Lightning chain without changing its requirements.
const SPELL_CASTER_XP: i64 = 2_485_800;

const OPEN_PLAYER: MovementLocalPosition = MovementLocalPosition {
    x: 33502,
    y: 32874,
    floor: 14,
};
const OPEN_BOSS: MovementLocalPosition = MovementLocalPosition {
    x: 33503,
    y: 32874,
    floor: 14,
};

#[test]
fn hazard_postgres_real_dispatch_qualification_selection_replay_reload_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("hazard-e2e", scenario)
}

async fn exchange_raw(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    text: &str,
    replay: Option<u64>,
) -> TestResult<(Option<CommandStatus>, Vec<String>)> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(3))
        .ok_or("clock")?;
    // EOF ends the existing NPC conversation owner. Ordinary exchanges therefore greet and
    // speak on one connection; retained unknown choices and duplicate replay keep their ID.
    let greet = replay.is_none()
        && text != "hi"
        && owner
            .states
            .npc_conversations
            .pending_hazard(admitted.game_session_id)
            .is_none();
    let first = replay.unwrap_or(admitted.continuity.next_command_id);
    let command_id = first + u64::from(greet);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let messages: Vec<_> = if greet {
        vec![(first, "hi"), (command_id, text)]
    } else {
        vec![(command_id, text)]
    };
    for (id, message) in messages {
        let payload = wire::encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc,
            text: message.into(),
        })
        .map_err(debug)?;
        let bytes = oteryn_protocol_oteryn::encode_client_command(
            admitted.continuity.connection_generation,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: id,
                command_type: wire::COMMAND_TYPE_NPC_TALK_INTENT,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        client
            .write_all(&u32::try_from(bytes.len())?.to_be_bytes())
            .await?;
        client.write_all(&bytes).await?;
    }
    client.shutdown().await?;
    let end = tokio::time::timeout(
        Duration::from_secs(30),
        serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS),
    )
    .await
    .map_err(|_| format!("HAZARD exchange timed out: {text:?}, command {command_id}"))?;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    if let crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current)
    | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    let mut cursor = 0;
    let mut status = None;
    let mut lines = Vec::new();
    while cursor < output.len() {
        let size = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + size]).map_err(debug)?;
        cursor += size + 4;
        if envelope.message_type() == MessageType::CommandResult {
            let result =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            if result.command_id != command_id {
                continue;
            }
            if result.status == CommandStatus::Rejected {
                eprintln!("HAZARD rejected text={text:?} payload={:?}", result.payload);
            }
            status = Some(result.status);
        } else if envelope.message_type() == MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == wire::STATE_DOMAIN_NPC_CONVERSATION {
                lines.extend(
                    wire::decode_npc_conversation(delta.payload)
                        .map_err(debug)?
                        .lines,
                );
            }
        }
    }
    if status.is_none() {
        eprintln!("HAZARD refused before result: text={text:?} command={command_id}");
    }
    Ok((status, lines))
}

async fn exchange(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    text: &str,
    replay: Option<u64>,
) -> TestResult<(CommandStatus, Vec<String>)> {
    let (status, lines) = exchange_raw(owner, admitted, npc, text, replay).await?;
    Ok((status.ok_or("missing command result")?, lines))
}

async fn hold_choice(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    text: &str,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("session")?;
    let command = crate::gameplay_transport::connection::UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: admitted.continuity.next_command_id,
        item_fence: admitted.item_fence,
    };
    let greeting = owner
        .hazard_talk(
            admitted.runtime_actor.ok_or("actor")?,
            command,
            NpcTalkIntent {
                npc_actor: npc,
                text: "hi".into(),
            },
        )
        .await;
    assert_eq!(greeting.disposition, wire::NpcIntentDisposition::Ok);
    let hold = owner.revision_sequencer.acquire(fence.character_id).await;
    let mut talk = Box::pin(owner.hazard_talk(
        admitted.runtime_actor.ok_or("actor")?,
        crate::gameplay_transport::connection::UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: admitted.continuity.next_command_id,
            item_fence: admitted.item_fence,
        },
        NpcTalkIntent {
            npc_actor: npc,
            text: text.into(),
        },
    ));
    tokio::time::timeout(
        Duration::from_secs(10),
        std::future::poll_fn(|context| {
            if std::future::Future::poll(talk.as_mut(), context).is_ready() {
                return std::task::Poll::Ready(Err("choice completed despite held revision slot"));
            }
            if owner
                .states
                .npc_conversations
                .pending_hazard(admitted.game_session_id)
                .is_some()
            {
                std::task::Poll::Ready(Ok(()))
            } else {
                std::task::Poll::Pending
            }
        }),
    )
    .await?
    .map_err(debug)?;
    drop(talk);
    drop(hold);
    Ok(())
}

async fn stored(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<hazard::HazardLevel> {
    let copy = owner
        .root
        .read_character_quest_state(
            owner.character,
            crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
        )
        .await
        .map_err(debug)?;
    hazard::level(copy.tracks()).ok_or_else(|| "hazard state".into())
}

async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(hazard::QUEST)
            .fetch_one(&h.pool)
            .await?,
    )
}

fn combat_room(
    world: WorldId,
    channel: ChannelId,
    pin: &crate::content::NativeEntryContentPin,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::content::QualifiedNativeEntryRoom> {
    use crate::map::boot::{BootPins, boot, tests as fixture};
    let index = native.item_index().ok_or("native Item index")?;
    let wire_id = |key| {
        index
            .definition_ref("Item", key, "definition-r1")
            .map(|id| id.get() - 1)
            .ok_or("item")
    };
    let (bytes, mut pins) = fixture::bundle_with_item_ids(
        (
            "oteryn:item.tibia.i2854",
            wire_id("oteryn:item.tibia.i2854")?,
        ),
        (
            "oteryn:item.tibia.i3031",
            wire_id("oteryn:item.tibia.i3031")?,
        ),
    );
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    bundle.manifest.world = oteryn_world_bundle::bundle::Extent {
        min_x: 33498,
        min_y: 32872,
        max_x: 33508,
        max_y: 32878,
        floors: vec![-14],
    };
    bundle.manifest.palette[0].key = "oteryn:item.tibia.i100".into();
    let mut sectors = std::collections::BTreeMap::new();
    for y in 32872..32878 {
        for x in 33498..33508 {
            sectors
                .entry((y / 32, x / 32))
                .or_insert_with(|| oteryn_world_bundle::bundle::Sector {
                    floor: -14,
                    sx: x / 32,
                    sy: y / 32,
                    tiles: Vec::new(),
                })
                .tiles
                .push(oteryn_world_bundle::sector::Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items: vec![oteryn_world_bundle::sector::Item {
                        palette: 0,
                        depth: 0,
                        attrs: Default::default(),
                    }],
                });
        }
    }
    bundle.sectors = sectors.into_values().collect();
    let bytes =
        oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
            .map_err(debug)?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let booted = boot(
        &bytes,
        &BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 33502,
                y: 32874,
                floor: -14,
            },
        },
        world,
        channel,
        |key| {
            Some(crate::map::facts::ItemDefinition {
                reference: index.definition_ref("Item", key, "definition-r1")?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        },
    )
    .map_err(debug)?;
    let cells = booted
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    pin.qualified_room()
        .bundle_view(
            cells,
            booted.frame_binding_digest(world),
            booted.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: 33502,
                y: 32874,
                floor: 14,
            },
        )
        .ok_or_else(|| "hazard bundle room".into())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "hazard_e2e", false, false).await?;
        prepare_current_session(&harness).await?;
        // Admit the second genuine session while the independently published signing facts
        // are fresh, before the expensive content/NPC catalogue qualification.
        let database_now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&harness.pool)
                .await?;
        let second = crate::world_party_runtime_postgres_cases::committed_fresh_player_from_owners(
            &harness.root,
            &harness.recovery,
            &harness.node,
            60,
            62,
            70,
            database_now,
        )
        .await?;
        let remote = crate::world_party_runtime_postgres_cases::committed_fresh_player_on_channel_from_owners(
            &harness.root, &harness.recovery, &harness.node, 61, 63, 71, 44, database_now,
        ).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let session = GameSessionId::decode(&id(50)).map_err(debug)?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_source(
            world,
            "content-1",
            Some(include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        )?;
        let active = controller.active().expect("activated generation");
        let native = active.native_gameplay().expect("native gameplay state");
        let room = combat_room(world, channel, &pin, native)?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (OPEN_PLAYER.x, OPEN_PLAYER.y, OPEN_PLAYER.floor),
        );
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3::text::numeric(20,0), NULL, $4, $5, $6)")
            .bind(id(42).as_slice())
            .bind(id(43).as_slice())
            .bind(pin.activation_sequence().to_string())
            .bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice())
            .bind(room.frame_binding().digest().as_slice())
            .execute(&harness.pool)
            .await?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            channel_pin.clone(),
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        let door_content =
            crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
                .map_err(debug)?;
        let fence = super::fixture::fence(1)?;
        let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
            fence.runtime_scope,
            fence.scope_ownership_generation,
            door_content,
        );
        let door = crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            fence.runtime_scope,
            fence.scope_ownership_generation,
        )
        .map_err(debug)?;
        let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec(),
            )],
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec(),
            )],
            rustls::pki_types::PrivateKeyDer::try_from(
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der")
                    .to_vec(),
            )?,
        )
        .map_err(debug)?;
        let evidence =
            crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
        let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
        let achievements =
            crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
        let charms =
            crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
        let runtime = Mutex::new(runtime);
        let door = Mutex::new(door);
        let npc = {
            use crate::foundation::{
                MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
            };
            runtime
                .lock()
                .await
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: hazard::GNOMADNESS.into(),
                            placements: vec![NpcPlacement {
                                x: 33500,
                                y: 32874,
                                floor: -14,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.lock().await.visible_entities().npcs[0];
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        let mut admitted = AdmittedSession {
            game_session_id: session,
            world_id: world,
            channel_id: channel,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::NotApplicable,
            controller: Some(ControllerBinding {
                transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16])
                    .map_err(debug)?,
                account_id: id(40),
            }),
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
            root: &harness.root,
            character: &authority,
            holder: &harness.node,
            evidence: &evidence,
            world_id: world,
            channel_id: channel,
            required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[],
                crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                1,
                "policy-1",
                crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
            )
            .expect("an empty declared set binds"),
            runtime: &runtime,
            door: &door,
            spells: &spells,
            achievements: &achievements,
            imported_charms: &charms,
            room: &room,
        }.build();
        owner.active_generation = Some(active);
        owner.movement_cells = room.movement_cells();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        owner.clock_origin = std::time::Instant::now()
            .checked_sub(Duration::from_secs(60))
            .expect("a monotonic clock older than a minute");
        {
            let runtime = owner.runtime.lock().await;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        crate::spell::cast::CharacterCastFacts {
                            vocation: crate::spell::Vocation::Sorcerer,
                            level: 55,
                            max_health: 100_000,
                            max_mana: 1_000,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (0, 0),
                        owner.owner_now(),
                    )
                    .is_some()
            );
        }
        owner.admit_progression_session(&mut admitted).await;
        assert!(owner.progression_binding(session).is_some());

        // The same code-built hazard catalogue that production boot installs.
        let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        owner.quest_catalogue = Some(std::sync::Arc::new(
            hazard::extend(lowered.catalogue()).map_err(debug)?,
        ));
        owner.refresh_quest_session(session, true).await;
        let current = character_gameplay_fence::current(&owner, session, false)
            .await
            .ok_or("current fence")?;
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;

        eprintln!(
            "POSTGRES_STARTED HAZARD hazard_postgres_real_dispatch_qualification_selection_replay_reload_and_fences"
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[3])
                .ok_or("capabilities")?;
        owner
            .states
            .chat
            .admit(session, "Hazard explorer", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let catalogue = crate::content::npc_catalogue::NpcDataCatalogue::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content"),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue::npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        eprintln!("HAZARD NPC catalogue loaded; starting wire exchanges");
        owner.states.chat.install_npc_talk(std::sync::Arc::new(
            crate::gameplay_transport::NpcTalkBook::from_model(&model),
        ));
        assert!(
            owner
                .states
                .chat
                .npc_talk()
                .and_then(|book| book.get(hazard::GNOMADNESS))
                .is_some()
        );
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_NPC_SERVICE_V1)
        );
        let selected = admitted.continuity.selected_capabilities;
        admitted.continuity.selected_capabilities = SelectedCapabilities::NONE;
        let (status, lines) = exchange(&mut owner, &mut admitted, npc, "2", None).await?;
        assert_eq!(status, CommandStatus::Rejected);
        assert!(lines.is_empty());
        assert_eq!(receipts(&harness).await?, 0);
        admitted.continuity.selected_capabilities = selected;
        let (status, _) = exchange(&mut owner, &mut admitted, npc, "hi", None).await?;
        assert_eq!(status, CommandStatus::Accepted, "greeting: {status:?}");
        let (_, lines) = exchange(&mut owner, &mut admitted, npc, "hazard", None).await?;
        assert!(
            lines.iter().any(|line| line.contains("hazard level is 1")
                && line.contains("active area level is 1")),
            "{lines:?}"
        );
        // Membership is recomputed from the current movement owner, including crossing
        // the area boundary while the same nearby NPC remains reachable.
        {
            let mut runtime = owner.runtime.lock().await;
            let mut movement = runtime.borrow_movement_position();
            let before = movement.read(actor).map_err(debug)?;
            movement
                .commit_cardinal(
                    before,
                    MovementLocalPosition {
                        x: OPEN_PLAYER.x - 1,
                        ..OPEN_PLAYER
                    },
                )
                .map_err(debug)?;
        }
        let (_, lines) = exchange(&mut owner, &mut admitted, npc, "hazard", None).await?;
        assert!(
            lines
                .iter()
                .any(|line| line.contains("active area level is 0")),
            "{lines:?}"
        );
        {
            let mut runtime = owner.runtime.lock().await;
            let mut movement = runtime.borrow_movement_position();
            let before = movement.read(actor).map_err(debug)?;
            movement
                .commit_cardinal(before, OPEN_PLAYER)
                .map_err(debug)?;
        }
        // The existing Primal Ordeal prerequisite owns access; a hazard choice cannot grant it.
        let (_, lines) = exchange(&mut owner, &mut admitted, npc, "2", None).await?;
        assert!(
            lines.iter().any(|line| line.contains("Magma Bubble")),
            "{lines:?}"
        );
        let mut slot = owner.revision_sequencer.acquire(current.character_id).await;
        slot.commit_quest_transition(owner.root, owner.character, owner.holder, current,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key:"oteryn:quest-transition/quest/u12_90/primal_ordeal/bosses/magma_bubble_killed/creature_event_1".into(),
                cause:crate::durability::quest_state::QuestCause::CreatureDeath(crate::durability::character_progression::ExperienceRewardOccurrence::from_bytes(occurrence(44)).map_err(debug)?),
            }, std::sync::Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?)).await.map_err(debug)?;
        drop(slot);
        owner.refresh_quest_session(session, false).await;
        let before = receipts(&harness).await?;
        let (_, lines) = exchange(&mut owner, &mut admitted, npc, "2", None).await?;
        assert!(
            lines.iter().any(|line| line.contains("qualification")),
            "{lines:?}"
        );
        assert_eq!(receipts(&harness).await?, before);

        // A real native Primal Menace slot, source profile and lethal receipt enter the same
        // production death queue as swings and spells. Damage setup uses the physical owner;
        // the new behaviour under test begins at the existing kill router, not a fake store.
        assert!(
            owner.reward_table().row(hazard::PRIMAL_MENACE).is_err(),
            "the existing corpse gate stays closed"
        );
        let qualification = {
            let mut runtime = owner.runtime.lock().await;
            let sources = [
                (
                    "canary:creature/rat",
                    vec![
                        OPEN_BOSS,
                        MovementLocalPosition {
                            y: OPEN_PLAYER.y - 1,
                            ..OPEN_BOSS
                        },
                    ],
                ),
                (
                    hazard::PRIMAL_MENACE,
                    vec![MovementLocalPosition {
                        x: 33503,
                        y: 32875,
                        ..OPEN_BOSS
                    }],
                ),
            ]
            .into_iter()
            .map(|(key, p)| {
                let policy = runtime.companion_policy(key).map_err(debug)?;
                Ok((
                    p.into_iter().map(|p| (p.x, p.y, p.floor)).collect(),
                    60_000,
                    5_000,
                    key.to_owned(),
                    u64::try_from(policy.maximum_health)?,
                ))
            })
            .collect::<TestResult<Vec<_>>>()?;
            runtime.realize_activation_spawns(sources).map_err(debug)?;
            let target = runtime
                .visible_entities()
                .creatures
                .iter()
                .find_map(|c| {
                    runtime
                        .companion_snapshot(c.actor)
                        .ok()
                        .filter(|s| s.state.policy.definition_key == hazard::PRIMAL_MENACE)
                        .map(|s| s.actor)
                })
                .ok_or("native primal spawn")?;
            assert!(crate::gameplay_transport::hazard::combat::eligible(
                &runtime, target
            ));
            drop(runtime);
            outgoing_swing_checks(&mut owner, actor, session).await?;
            let next_command = outgoing_cleave_checks(&mut owner, actor, session, &harness).await?;
            let next_command =
                outgoing_distance_checks(&mut owner, actor, session, &harness, next_command)
                    .await?;
            // Cast before the AI turn moves these newly admitted source actors.
            let kill_command_id =
                outgoing_spell_checks(&mut owner, actor, session, target, &harness, next_command)
                    .await?;
            outgoing_condition_checks(&mut owner, actor, session, target).await?;
            combat_checks(&mut owner, actor, session, target).await?;
            let mut runtime = owner.runtime.lock().await;
            let health = runtime
                .creature_combat_facts(target, 0)
                .map_err(debug)?
                .health;
            let command = crate::foundation::CommandRef::new(
                session,
                crate::foundation::CommandId::new(kill_command_id).map_err(debug)?,
            );
            runtime
                .borrow_exact_actor_commit_at(owner.owner_now().get() / 1000)
                .commit_damage_for_bound_attacker(
                    target,
                    actor,
                    command,
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: hazard::PRIMAL_MENACE.as_bytes(),
                        occurrence: b"hazard-kill",
                        binding: b"hazard-qualification",
                        damage: health,
                    },
                )
                .map_err(debug)?;
            let attack = owner.states.attack.lock().await;
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink {
                    queue: attack.kills(),
                    rewards: owner.reward_table(),
                    activity: &[],
                    hazard: Some(&owner.states.quest_sessions),
                },
                target,
                crate::combat::CapturedRewardPrincipal {
                    character: crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    lease_generation: 2,
                    session,
                    actor,
                },
                owner.owner_now().get() / 1000,
                None,
            );
            assert!(attack.kills().has_session(session));
            // Preserve the captured packet to prove that selection after settlement cannot
            // turn replay of this same death into a second advance.
            let shares = runtime
                .borrow_combat_death()
                .reward_damage_shares(target)
                .map_err(debug)?;
            crate::gameplay_transport::hazard::capture(
                &runtime,
                Some(&owner.states.quest_sessions),
                hazard::PRIMAL_MENACE,
                &shares,
            )
            .ok_or("qualification")?
        };
        let mut drains = 0;
        while owner.drain_kill_rewards(session).await != DrainOutcome::Drained {
            drains += 1;
            assert!(drains < 8, "kill settlement is bounded");
        }
        assert_eq!(
            stored(&owner).await?,
            hazard::HazardLevel {
                current: 1,
                unlocked: 2
            }
        );
        let xp:i64=sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(
            xp, SPELL_CASTER_XP,
            "a qualification without an admitted reward row grants no XP"
        );
        // Query reflects the upgrade but never auto-selects the new level.
        let (_, lines) = exchange(&mut owner, &mut admitted, npc, "hazard", None).await?;
        assert!(
            lines
                .iter()
                .any(|line| line.contains("hazard level is 1")
                    && line.contains("unlocked level is 2")),
            "{lines:?}"
        );
        // Cancel at the real revision-slot boundary, after the NPC owner retained the exact
        // choice. A durable completion without acknowledgement is then reconciled by resume.
        hold_choice(&owner, &admitted, npc, "2").await?;
        let select_id = admitted.continuity.next_command_id;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("session")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        slot.commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key: hazard::select_transition(2),
                cause: crate::durability::quest_state::QuestCause::Command(
                    crate::foundation::CommandRef::new(
                        session,
                        crate::foundation::CommandId::new(select_id).map_err(debug)?,
                    ),
                ),
            },
            std::sync::Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
        )
        .await
        .map_err(debug)?;
        drop(slot);

        let (_, lines) = exchange(&mut owner, &mut admitted, npc, "2", None).await?;
        assert!(
            lines.iter().any(|line| line.contains("hazard level is 2")),
            "{lines:?}"
        );
        assert_eq!(
            stored(&owner).await?,
            hazard::HazardLevel {
                current: 2,
                unlocked: 2
            }
        );
        let selected_receipts = receipts(&harness).await?;
        let (status, _) = exchange(&mut owner, &mut admitted, npc, "2", Some(select_id)).await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(receipts(&harness).await?, selected_receipts);
        party_checks(
            &harness,
            &mut owner,
            actor,
            &second,
            &remote,
            &mut admitted,
            npc,
        )
        .await?;
        // A cancelled choice whose transaction never committed is classified as a refusal,
        // and is never executed again under replacement input.
        hold_choice(&owner, &admitted, npc, "1").await?;
        let before_cancel = receipts(&harness).await?;
        exchange(&mut owner, &mut admitted, npc, "1", None).await?;
        assert_eq!(receipts(&harness).await?, before_cancel);
        assert_eq!(stored(&owner).await?.current, 2);
        assert!(
            owner
                .states
                .npc_conversations
                .pending_hazard(session)
                .is_none()
        );
        // A qualification at lower-than-maximum difficulty is not eligible.
        exchange(&mut owner, &mut admitted, npc, "1", None).await?;
        {
            let runtime = owner.runtime.lock().await;
            let shares = [crate::foundation::RewardDamageShare {
                character: crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                total: 1,
                lease_generation: Some(2),
                session: Some(session),
            }];
            let lower = crate::gameplay_transport::hazard::capture(
                &runtime,
                Some(&owner.states.quest_sessions),
                hazard::PRIMAL_MENACE,
                &shares,
            )
            .ok_or("captured lower")?;
            drop(runtime);
            assert!(
                owner
                    .qualify_hazard_kill(Some(&lower), occurrence(99))
                    .await
            );
        }
        assert_eq!(
            stored(&owner).await?,
            hazard::HazardLevel {
                current: 1,
                unlocked: 2
            }
        );
        // Reopen the durable owner and discard session copies: selected and qualified levels
        // come from PostgreSQL, not this Channel's acknowledgement/cache.
        owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest sessions")?
            .clear();
        owner.refresh_quest_session(session, true).await;
        assert_eq!(
            stored(&owner).await?,
            hazard::HazardLevel {
                current: 1,
                unlocked: 2
            }
        );
        let reopened_seal = harness.recovery.seal_current().map_err(debug)?;
        let reopened = harness
            .root
            .open_character_authority(&reopened_seal)
            .await
            .map_err(debug)?;
        let copy = harness
            .root
            .read_character_quest_state(
                &reopened,
                crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            hazard::level(copy.tracks()),
            Some(hazard::HazardLevel {
                current: 1,
                unlocked: 2
            })
        );
        drop(reopened);
        drop(reopened_seal);
        // The same immutable qualification's key cannot advance a now-higher maximum.
        assert!(
            owner
                .qualify_hazard_kill(Some(&qualification), occurrence(100))
                .await
        );
        assert_eq!(stored(&owner).await?.unlocked, 2);
        let count = receipts(&harness).await?;
        // A stale connection generation cannot write through the real NPC router. This
        // intentionally invalid connection is last: an unresolved ingress reservation must
        // not be reused by repairing the test copy of its admitted fence.
        let original = admitted.item_fence;
        let mut stale = original.ok_or("fence")?;
        stale.connection_generation =
            crate::foundation::ConnectionGeneration::new(stale.connection_generation.get() + 1)
                .map_err(debug)?;
        admitted.item_fence = Some(stale);
        // Setup may refuse the stale admitted connection before its command is dispatched.
        // If it reaches the command router, the result must be a rejection.
        let (status, _) = exchange_raw(&mut owner, &mut admitted, npc, "2", None).await?;
        assert!(matches!(status, None | Some(CommandStatus::Rejected)));
        assert_eq!(receipts(&harness).await?, count);
        assert_eq!(stored(&owner).await?.current, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await?;
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
async fn party_checks(
    harness: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    second: &crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >,
    remote: &crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
) -> TestResult {
    use crate::durability::spell_item_transaction as item_tx;
    use crate::foundation::{CommandId, CommandRef, PartyAction};
    let session = admitted.game_session_id;
    let other_session = second.current_game_session_id();
    let mut runtime = owner.runtime.lock().await;
    let reservation = runtime
        .reserve_fresh_session(other_session)
        .map_err(debug)?;
    let other = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .bind_attacker_lease(other, other_session, second.current_character_lease())
        .map_err(debug)?;
    runtime
        .initialize_first_entry_position(other)
        .map_err(debug)?;
    let at = runtime.read_actor_position(other).map_err(debug)?;
    runtime
        .borrow_movement_position()
        .commit_cardinal(
            at,
            MovementLocalPosition {
                y: OPEN_PLAYER.y + 1,
                ..OPEN_PLAYER
            },
        )
        .map_err(debug)?;
    drop(runtime);
    let primary = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "primary fence")?
        .ok_or("primary fence")?
        .item_fence();
    let secondary = crate::durability::item_transfer::CurrentCharacterItemFence {
        character_id: crate::domain::CharacterId::from_bytes(
            *second.current_character_lease().character_id().as_bytes(),
        )
        .map_err(debug)?,
        game_session_id: other_session,
        connection_generation: second.current_connection_generation(),
        character_lease_generation: second.current_character_lease().generation(),
        runtime_scope: second.current_runtime_scope(),
        scope_ownership_generation: second.current_scope_generation(),
    };
    let permit = owner.spell_lane_permit().await.ok_or("party lane")?;
    let runtime = owner.runtime.lock().await;
    let mut party = None;
    for (fence, number, proposed) in [
        (
            primary,
            70000,
            PartyAction::Invite {
                target: *secondary.character_id.as_bytes(),
            },
        ),
        (secondary, 70001, PartyAction::Leave),
    ] {
        let mut tx = harness.pool.begin().await?;
        let authority = item_tx::assert_spell_item_authority_in_transaction(
            &mut tx,
            &permit,
            owner.root,
            owner.character,
            owner.holder,
            &fence,
            CommandRef::new(
                fence.game_session_id,
                CommandId::new(number).map_err(debug)?,
            ),
            runtime.content_pin().server_artifact_digest(),
        )
        .await
        .map_err(debug)?;
        let action = if number == 70001 {
            PartyAction::Accept {
                party: party.ok_or("party")?,
            }
        } else {
            proposed
        };
        let visible = if number == 70000 {
            crate::durability::admission_journal::party_target_binding::prove_visible_party_target_in_transaction(&mut tx,owner.root,&authority,&runtime,actor,other,other_session).await.map_err(debug)?
        } else {
            None
        };
        let receipt = crate::durability::world_party::apply_world_party_command_in_transaction(
            &mut tx,
            &authority,
            action,
            visible.as_ref(),
        )
        .await
        .map_err(debug)?;
        assert_eq!(receipt.disposition, "APPLIED");
        party = receipt.party_id;
        tx.commit().await?;
    }
    drop(runtime);
    drop(permit);
    // Membership persists across Channels. Seed the remote membership predecessor, but
    // use a genuinely committed independent admission as its current Channel authority.
    // Deliberately stale social presentation claims the LOCAL Channel: it cannot govern combat.
    sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,3,1,encode($3,'hex')::uuid)")
        .bind(remote.current_character_lease().character_id().as_bytes().as_slice())
        .bind(party.ok_or("party")?.as_slice()).bind(owner.channel_id.as_bytes().as_slice())
        .execute(&harness.pool).await?;
    // This member has neither damaged a creature nor installed a combat state. Membership,
    // rather than damage contribution or a Slot cache, still lowers the selected level 2.
    let (_, lines) = exchange(owner, admitted, npc, "hazard", None).await?;
    assert!(
        lines
            .iter()
            .any(|l| l.contains("hazard level is 2") && l.contains("active area level is 1")),
        "{lines:?}"
    );
    let mut runtime = owner.runtime.lock().await;
    let at = runtime.read_actor_position(other).map_err(debug)?;
    runtime
        .borrow_movement_position()
        .commit_cardinal(
            at,
            MovementLocalPosition {
                x: 33501,
                y: OPEN_PLAYER.y + 1,
                ..OPEN_PLAYER
            },
        )
        .map_err(debug)?;
    drop(runtime);
    let (_, lines) = exchange(owner, admitted, npc, "hazard", None).await?;
    assert!(
        lines.iter().any(|l| l.contains("active area level is 0")),
        "{lines:?}"
    );
    // Privacy never changes gameplay membership. A genuinely missing actor is unavailable.
    {
        let permit = owner
            .spell_lane_permit()
            .await
            .ok_or("party visibility lane")?;
        let runtime = owner.runtime.lock().await;
        let mut tx = harness.pool.begin().await?;
        let authority = item_tx::assert_spell_item_authority_in_transaction(
            &mut tx,
            &permit,
            owner.root,
            owner.character,
            owner.holder,
            &secondary,
            CommandRef::new(other_session, CommandId::new(70002).map_err(debug)?),
            runtime.content_pin().server_artifact_digest(),
        )
        .await
        .map_err(debug)?;
        let receipt = crate::durability::world_party::apply_world_party_command_in_transaction(
            &mut tx,
            &authority,
            PartyAction::SetChannelVisibility { hidden: true },
            None,
        )
        .await
        .map_err(debug)?;
        assert_eq!(receipt.disposition, "APPLIED");
        tx.commit().await?;
    }
    let (_, lines) = exchange(owner, admitted, npc, "hazard", None).await?;
    assert!(
        lines.iter().any(|l| l.contains("active area level is 0")),
        "{lines:?}"
    );
    {
        let mut runtime = owner.runtime.lock().await;
        let at = runtime.read_actor_position(other).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_cardinal(
                at,
                MovementLocalPosition {
                    y: OPEN_PLAYER.y + 1,
                    ..OPEN_PLAYER
                },
            )
            .map_err(debug)?;
    }
    let (_, lines) = exchange(owner, admitted, npc, "hazard", None).await?;
    assert!(
        lines.iter().any(|l| l.contains("active area level is 1")),
        "{lines:?}"
    );
    owner
        .runtime
        .lock()
        .await
        .remove_test_actor(other)
        .map_err(debug)?;
    let (_, lines) = exchange(owner, admitted, npc, "hazard", None).await?;
    assert!(
        lines
            .iter()
            .any(|l| l.contains("active area level is unavailable")),
        "{lines:?}"
    );
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("party cleanup lane")?;
    let runtime = owner.runtime.lock().await;
    let mut tx = harness.pool.begin().await?;
    let authority = item_tx::assert_spell_item_authority_in_transaction(
        &mut tx,
        &permit,
        owner.root,
        owner.character,
        owner.holder,
        &primary,
        CommandRef::new(session, CommandId::new(70003).map_err(debug)?),
        runtime.content_pin().server_artifact_digest(),
    )
    .await
    .map_err(debug)?;
    let receipt = crate::durability::world_party::apply_world_party_command_in_transaction(
        &mut tx,
        &authority,
        PartyAction::Kick {
            target: *secondary.character_id.as_bytes(),
        },
        None,
    )
    .await
    .map_err(debug)?;
    assert_eq!(receipt.disposition, "APPLIED");
    tx.commit().await?;
    drop(runtime);
    drop(permit);
    let (_, lines) = exchange(owner, admitted, npc, "hazard", None).await?;
    assert!(
        lines.iter().any(|l| l.contains("active area level is 2")),
        "remote member must not lower local benefit: {lines:?}"
    );
    // Privacy must not turn a remote participant into an unavailable local participant.
    sqlx::query("UPDATE game_party_members SET presence_state=2,presence_channel_id=NULL WHERE character_id=encode($1,'hex')::uuid")
        .bind(remote.current_character_lease().character_id().as_bytes().as_slice())
        .execute(&harness.pool).await?;
    let (_, lines) = exchange(owner, admitted, npc, "hazard", None).await?;
    assert!(
        lines.iter().any(|l| l.contains("active area level is 2")),
        "hidden remote: {lines:?}"
    );
    let before = owner
        .observe_actor_vitals(actor, session)
        .await
        .ok_or("remote party vitals before")?
        .1
        .health;
    let mut after = before;
    for _ in 0..32 {
        owner.clock_origin -= Duration::from_secs(3);
        owner.drain_monster_melee().await;
        after = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("remote party vitals after")?
            .1
            .health;
        if after < before {
            break;
        }
    }
    assert!(
        after < before,
        "cross-Channel membership must not block real monster combat"
    );
    Ok(())
}

fn critical_time(
    runtime: &ChannelRuntimeV1,
    source: crate::foundation::ExactActorRef,
    target: crate::foundation::ExactActorRef,
    earliest: u64,
) -> TestResult<u64> {
    use crate::gameplay_transport::hazard::combat;
    (earliest..earliest + 10_000)
        .find(|now| {
            combat::draw(runtime, source, target, *now, "hazard.critical")
                .and_then(|roll| combat::incoming(100, 1, true, roll))
                .is_some_and(|(_, critical)| critical)
        })
        .ok_or_else(|| "qualified critical occurrence in bounded source stream".into())
}

// The real native cast dispatcher joins durable build/payment, World hazard
// locks and the physical batch. A dodge still spends mana and advances cooldown.
async fn outgoing_spell_checks(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    target: crate::foundation::ExactActorRef,
    harness: &Harness,
    mut next_id: u64,
) -> TestResult<u64> {
    use super::death_ml_loss_composed::current_fence;
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
    };
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceRewardOccurrence,
    };
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::actor_spell::{
        SpellCastDisposition, SpellCastIntent, SpellTarget,
    };
    use crate::gameplay_transport::hazard::combat;

    let native = owner
        .active_generation
        .ok_or("spell generation")?
        .native_gameplay()
        .ok_or("spell content")?;
    let formula = native.training_formula().ok_or("spell training")?;
    let build =
        convert_vocation(formula, &DurableBuildState::default(), "sorcerer").map_err(debug)?;
    owner
        .root
        .commit_character_build(
            owner.character,
            owner.holder,
            current_fence(owner, session).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(occurrence(145)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: build.clone(),
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    let progression = owner
        .progression_binding(session)
        .ok_or("spell progression")?;
    owner
        .root
        .commit_character_experience(
            owner.character,
            owner.holder,
            current_fence(owner, session).await?,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(occurrence(146))
                    .map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(SPELL_CASTER_XP),
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    owner
        .root
        .initialize_familiar_group(
            owner.character,
            owner.holder,
            current_fence(owner, session).await?,
        )
        .await
        .map_err(debug)?;
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        assert!(states.load_owned_build(&runtime, actor, session, &build));
        let player = states
            .get_mut(&runtime, actor, session)
            .ok_or("spell player")?;
        player
            .enable_owned_training(&build, formula)
            .map_err(debug)?;
    }
    let original_book = owner.spells;
    owner.spells = native.spell_book();
    for (key, mana_cost, native_cast, should_dodge) in [
        ("candidate:spell/energy_strike", 20, false, true),
        ("candidate:spell/energy_strike", 20, false, false),
        ("candidate:spell/energy_beam", 40, true, true),
        ("candidate:spell/energy_beam", 40, true, false),
    ] {
        let index = (1..=owner.spells.source_len())
            .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
            .find(|i| owner.spells.indexed(*i).is_some_and(|s| s.key == key))
            .ok_or("outgoing combat spell")?;
        let original_position = if native_cast {
            let mut runtime = owner.runtime.lock().await;
            let target_position = runtime
                .read_actor_position(target)
                .map_err(debug)?
                .position();
            let mut movement = runtime.borrow_movement_position();
            let before = movement.read(actor).map_err(debug)?;
            let original = before.position();
            let moved = movement
                .commit_cardinal(
                    before,
                    MovementLocalPosition {
                        x: target_position.x - 1,
                        ..target_position
                    },
                )
                .map_err(debug)?;
            movement
                .commit_turn(moved, crate::foundation::MovementFacing::East)
                .map_err(debug)?;
            Some(original)
        } else {
            None
        };
        owner.clock_origin -= Duration::from_secs(5);
        let (command_id, position, before) = {
            let runtime = owner.runtime.lock().await;
            assert!(combat::eligible(&runtime, target));
            let command_id = (next_id..next_id + 100_000)
                .find(|id| {
                    let command = CommandRef::new(session, CommandId::new(*id).expect("nonzero"));
                    combat::spell_dodge_roll(&runtime, actor, target, command, 0)
                        .is_some_and(|roll| combat::dodges(1, roll) == should_dodge)
                })
                .ok_or("bounded spell dodge occurrence")?;
            (
                command_id,
                runtime
                    .read_actor_position(target)
                    .map_err(debug)?
                    .position(),
                runtime
                    .creature_combat_facts(target, owner.owner_now().get() / 1000)
                    .map_err(debug)?
                    .health,
            )
        };
        next_id = command_id + 1;
        let intent = SpellCastIntent {
            spell: index,
            target: if native_cast {
                SpellTarget::None
            } else {
                SpellTarget::Position(oteryn_protocol_oteryn::actor_spell::SpellTargetPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                })
            },
            aim_at_target: false,
        };
        let mana_before = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("spell mana")?
            .1
            .mana;
        if native_cast {
            let unavailable = owner
                .cast_spell_dispatch(actor, session, command_id, intent)
                .await
                .ok_or("native missing stance refusal")?;
            assert_eq!(unavailable.disposition, SpellCastDisposition::Rejected);
            assert_eq!(
                owner
                    .observe_actor_vitals(actor, session)
                    .await
                    .ok_or("native refusal mana")?
                    .1
                    .mana,
                mana_before
            );
        }
        let outcome = outgoing_cast(owner, actor, session, command_id, intent, native_cast)
            .await
            .ok_or("spell completed")?;
        assert_eq!(
            outcome.disposition,
            SpellCastDisposition::Cast,
            "forced spell dodge={should_dodge}"
        );
        let after = owner
            .runtime
            .lock()
            .await
            .creature_combat_facts(target, owner.owner_now().get() / 1000)
            .map_err(debug)?
            .health;
        if should_dodge {
            assert_eq!(before, after, "forced spell dodge");
        } else {
            assert!(
                after < before,
                "forced spell non-dodge: {before} -> {after}"
            );
        }
        let mana_after = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("paid spell mana")?
            .1
            .mana;
        assert_eq!(
            mana_before - mana_after,
            mana_cost,
            "dodge still pays the source cost"
        );
        let replay = outgoing_cast(owner, actor, session, command_id, intent, native_cast)
            .await
            .ok_or("spell replay completed")?;
        assert_eq!(replay.disposition, SpellCastDisposition::Cast);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .creature_combat_facts(target, owner.owner_now().get() / 1000)
                .map_err(debug)?
                .health,
            after
        );
        assert_eq!(
            owner
                .observe_actor_vitals(actor, session)
                .await
                .ok_or("replay mana")?
                .1
                .mana,
            mana_after
        );
        let cooling = outgoing_cast(owner, actor, session, next_id, intent, native_cast)
            .await
            .ok_or("cooldown completed")?;
        assert_eq!(cooling.disposition, SpellCastDisposition::CoolingDown);
        next_id += 1;
        if let Some(original) = original_position {
            let mut runtime = owner.runtime.lock().await;
            let mut movement = runtime.borrow_movement_position();
            let before = movement.read(actor).map_err(debug)?;
            movement.commit_cardinal(before, original).map_err(debug)?;
        }
    }
    next_id = outgoing_chain_checks(owner, actor, session, target, next_id).await?;
    // A source wand uses the same swing owner as fists, with the real
    // equipment reader and mana payment before the hazard health parser.
    {
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(
                190,
                "oteryn:item.tibia.i25760",
                "definition-r1",
                1,
            ),
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,6,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(190).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    for should_dodge in [true, false] {
        owner.clock_origin -= Duration::from_secs(3);
        let (command_id, before) = {
            let runtime = owner.runtime.lock().await;
            let command_id = (next_id..next_id + 100_000)
                .find(|id| {
                    let command = CommandRef::new(session, CommandId::new(*id).expect("nonzero"));
                    combat::swing_dodge_roll(&runtime, actor, target, command, 0)
                        .is_some_and(|roll| combat::dodges(1, roll) == should_dodge)
                })
                .ok_or("bounded wand dodge")?;
            let generation = runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|c| c.actor == target)
                .ok_or("wand target")?
                .generation;
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    owner.rooms.gate_auto_attack(),
                    actor,
                    session,
                    owner.owner_now(),
                    Some(EntityRef {
                        identity: target.placement_identity(),
                        generation
                    }),
                    command_id,
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
            (
                command_id,
                runtime
                    .creature_combat_facts(target, owner.owner_now().get() / 1000)
                    .map_err(debug)?
                    .health,
            )
        };
        next_id = command_id + 1;
        owner.refresh_attack_weapon(actor, session).await;
        let mana_before = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("wand mana")?
            .1
            .mana;
        owner.drain_auto_attacks().await;
        let after = owner
            .runtime
            .lock()
            .await
            .creature_combat_facts(target, owner.owner_now().get() / 1000)
            .map_err(debug)?
            .health;
        if should_dodge {
            assert_eq!(before, after, "forced wand dodge");
        } else {
            assert!(after < before, "forced wand non-dodge: {before} -> {after}");
        }
        let mana_after = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("paid wand mana")?
            .1
            .mana;
        assert!(
            mana_after < mana_before,
            "the ranged owner must pay even on dodge"
        );
        owner.drain_auto_attacks().await;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .creature_combat_facts(target, owner.owner_now().get() / 1000)
                .map_err(debug)?
                .health,
            after,
            "timer prevents duplicate wand hit"
        );
        assert_eq!(
            owner
                .observe_actor_vitals(actor, session)
                .await
                .ok_or("wand replay mana")?
                .1
                .mana,
            mana_after
        );
    }
    {
        let runtime = owner.runtime.lock().await;
        owner.states.attack.lock().await.set_target(
            &runtime,
            owner.rooms.gate_auto_attack(),
            actor,
            session,
            owner.owner_now(),
            None,
            next_id,
        );
    }
    owner.spells = original_book;
    Ok(next_id)
}

// Explicit test Premium admission; the due callback uses the normal refreshed access
// reader and does not re-admit an already paid spell. No production grant is installed.
struct HazardChainPremium;
impl crate::gameplay_transport::spell_access_facts::owner_registration::Registered
    for HazardChainPremium
{
}
impl crate::gameplay_transport::spell_access_facts::CurrentSpellAccessOwner for HazardChainPremium {
    fn read_current(
        &self,
        binding: &crate::spell::owned_cast_facts::CastFactsBinding,
        now: u64,
    ) -> Result<
        crate::spell::owned_cast_facts::AccessProjections,
        crate::gameplay_transport::spell_access_facts::AccessFactsError,
    > {
        Ok(crate::spell::owned_cast_facts::AccessProjections {
            premium: Some(crate::spell::owned_cast_facts::CurrentProjection {
                binding: binding.clone(),
                authority_revision: 1,
                valid_until_micros: now + 1_000_000,
                value: true,
            }),
            ..Default::default()
        })
    }
}

async fn outgoing_chain_checks(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    target: crate::foundation::ExactActorRef,
    mut next_id: u64,
) -> TestResult<u64> {
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::actor_spell::{
        NativeCastDispatch, SpellCastDisposition, SpellCastIntent, SpellTarget,
    };
    use crate::gameplay_transport::hazard::combat;
    let index = (1..=owner.spells.source_len())
        .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
        .find(|i| {
            owner
                .spells
                .indexed(*i)
                .is_some_and(|s| s.key == "candidate:spell/lightning")
        })
        .ok_or("native Lightning")?;
    let (position, recipients) = {
        let runtime = owner.runtime.lock().await;
        let recipients = runtime
            .visible_entities()
            .creatures
            .iter()
            .map(|c| {
                Ok((
                    c.actor,
                    runtime
                        .creature_combat_facts(c.actor, 0)
                        .map_err(debug)?
                        .health,
                ))
            })
            .collect::<TestResult<Vec<_>>>()?;
        (
            runtime
                .read_actor_position(target)
                .map_err(debug)?
                .position(),
            recipients,
        )
    };
    let first_bounce = {
        let runtime = owner.runtime.lock().await;
        runtime
            .visible_entities()
            .creatures
            .iter()
            .find(|c| c.position == OPEN_BOSS)
            .ok_or("Lightning first bounce")?
            .actor
    };
    let intent = SpellCastIntent {
        spell: index,
        target: SpellTarget::Position(oteryn_protocol_oteryn::actor_spell::SpellTargetPosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        }),
        aim_at_target: false,
    };
    for should_dodge in [true, false] {
        owner.clock_origin -= Duration::from_secs(10);
        let command_id = {
            let mut runtime = owner.runtime.lock().await;
            for (recipient, _) in &recipients {
                runtime
                    .set_test_creature_health(*recipient, 10_000)
                    .map_err(debug)?;
            }
            (next_id..next_id + 100_000)
                .find(|id| {
                    let command = CommandRef::new(session, CommandId::new(*id).expect("nonzero"));
                    combat::spell_dodge_roll(&runtime, actor, first_bounce, command, 1)
                        .is_some_and(|roll| combat::dodges(1, roll) == should_dodge)
                })
                .ok_or("bounded delayed dodge occurrence")?
        };
        next_id = command_id + 1;
        let mana_before = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("chain mana")?
            .1
            .mana;
        let NativeCastDispatch::Outcome(outcome) = owner
            .cast_native_combat(actor, session, command_id, &intent, &HazardChainPremium)
            .await
        else {
            return Err("Lightning dispatch did not complete".into());
        };
        assert_eq!(outcome.disposition, SpellCastDisposition::Cast);
        let mana_after = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("paid chain mana")?
            .1
            .mana;
        assert_eq!(mana_before - mana_after, 60);
        let saved = owner
            .states
            .spell_states
            .lock()
            .await
            .spell_timers
            .as_ref()
            .ok_or("chain timer lane")?
            .due_ordinary_payloads(
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                    owner.owner_now().get() + 1_000_000,
                ),
            );
        assert_eq!(saved.len(), 3, "primary and two source-selected bounces");
        assert!(saved.iter().all(|s| s.binding.command.command_id().get() == command_id && s.resolved.is_none()));
        let bounce = saved
            .iter()
            .find(|s| s.step == 1)
            .ok_or("first delayed phase")?;
        assert_eq!(bounce.target, first_bounce);
        assert_eq!(bounce.sub_ordinal, 1);
        assert_eq!(bounce.delay_ms, 50);
        {
            let runtime = owner.runtime.lock().await;
            for (recipient, _) in &recipients {
                assert_eq!(
                    runtime
                        .creature_combat_facts(*recipient, 0)
                        .map_err(debug)?
                        .health,
                    10_000,
                    "cast schedules damage instead of applying a bounce early"
                );
            }
        }
        owner.clock_origin -= Duration::from_secs(1);
        let mut seen = Vec::new();
        for _ in 0..saved.len() {
            let report = owner.drain_native_ai_timers().await.map_err(debug)?;
            assert!(
                report.blocked.is_none(),
                "due blocked: {:?}",
                report.blocked
            );
            assert!(report.dropped.is_empty());
            for receipt in report.receipts {
                assert!(receipt.batch.applied);
                assert!(!seen.contains(&receipt.occurrence));
                let source = saved
                    .iter()
                    .find(|s| s.phase == receipt.occurrence.phase)
                    .ok_or("due source phase")?;
                assert_eq!(receipt.occurrence.command, source.binding.command);
                let runtime = owner.runtime.lock().await;
                let dodge = combat::dodges(
                    1,
                    combat::spell_dodge_roll(
                        &runtime,
                        actor,
                        source.target,
                        source.binding.command,
                        source.sub_ordinal,
                    )
                    .ok_or("due dodge")?,
                );
                let after = runtime
                    .creature_combat_facts(source.target, 0)
                    .map_err(debug)?
                    .health;
                let [effect] = receipt.batch.effects.as_slice() else {
                    return Err("one retained effect per chain phase".into());
                };
                assert_eq!(effect.target, source.target);
                assert_eq!(effect.sub_ordinal, source.sub_ordinal);
                if source.step == 1 {
                    assert_eq!(dodge, should_dodge);
                }
                if dodge {
                    assert_eq!(effect.health, None, "zero damage has no health mutation");
                    assert_eq!(after, 10_000, "delayed dodge retains a zero-damage receipt");
                } else {
                    let health = effect.health.ok_or("chain health receipt")?;
                    assert!(health.applied);
                    assert_eq!(health.health_before, 10_000);
                    assert_eq!(health.health_after, after);
                    assert!(
                        after < 10_000,
                        "non-dodged due phase applies positive damage"
                    );
                }
                seen.push(receipt.occurrence);
            }
        }
        assert_eq!(seen.len(), saved.len(), "each phase fires exactly once");
        let after = {
            let runtime = owner.runtime.lock().await;
            recipients
                .iter()
                .map(|(a, _)| {
                    runtime
                        .creature_combat_facts(*a, 0)
                        .map(|f| f.health)
                        .map_err(debug)
                })
                .collect::<TestResult<Vec<_>>>()?
        };
        let NativeCastDispatch::Outcome(replay) = owner
            .cast_native_combat(actor, session, command_id, &intent, &HazardChainPremium)
            .await
        else {
            return Err("Lightning replay did not complete".into());
        };
        assert_eq!(replay.disposition, SpellCastDisposition::Cast);
        let retry = owner.drain_native_ai_timers().await.map_err(debug)?;
        assert!(retry.receipts.is_empty());
        assert!(retry.blocked.is_none());
        let runtime = owner.runtime.lock().await;
        for ((recipient, _), expected) in recipients.iter().zip(after) {
            assert_eq!(
                runtime
                    .creature_combat_facts(*recipient, 0)
                    .map_err(debug)?
                    .health,
                expected,
                "cast replay and timer retry apply no late damage"
            );
        }
        drop(runtime);
        assert_eq!(
            owner
                .observe_actor_vitals(actor, session)
                .await
                .ok_or("chain replay mana")?
                .1
                .mana,
            mana_after
        );
        assert_eq!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .spell_timers
                .as_ref()
                .ok_or("drained chain lane")?
                .pending_len(),
            0
        );
    }
    let mut runtime = owner.runtime.lock().await;
    for (recipient, health) in recipients {
        runtime
            .set_test_creature_health(recipient, health)
            .map_err(debug)?;
    }
    Ok(next_id)
}

// Only the elemental-stance input is supplied by this explicit test producer.
// Production's missing stance remains unavailable; the profile, all durable cast
// facts, current World level, native dispatcher and physical owner remain real.
struct HazardNativeStance;
impl crate::gameplay_transport::spell_access_facts::owner_registration::Registered
    for HazardNativeStance
{
}
impl crate::gameplay_transport::spell_access_facts::CurrentSpellAccessOwner for HazardNativeStance {
    fn read_current(
        &self,
        binding: &crate::spell::owned_cast_facts::CastFactsBinding,
        now: u64,
    ) -> Result<
        crate::spell::owned_cast_facts::AccessProjections,
        crate::gameplay_transport::spell_access_facts::AccessFactsError,
    > {
        Ok(crate::spell::owned_cast_facts::AccessProjections {
            elemental_stance: Some(crate::spell::owned_cast_facts::CurrentProjection {
                binding: binding.clone(),
                authority_revision: 1,
                valid_until_micros: now + 1_000_000,
                value: crate::spell::native_combat::ElementalStance::None,
            }),
            ..Default::default()
        })
    }
}

async fn outgoing_cast(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    command: u64,
    intent: crate::gameplay_transport::actor_spell::SpellCastIntent,
    native: bool,
) -> Option<crate::gameplay_transport::actor_spell::SpellCastOutcome> {
    if !native {
        return owner
            .cast_spell_dispatch(actor, session, command, intent)
            .await;
    }
    match owner
        .cast_native_combat(actor, session, command, &intent, &HazardNativeStance)
        .await
    {
        crate::gameplay_transport::actor_spell::NativeCastDispatch::Outcome(outcome) => {
            Some(outcome)
        }
        _ => None,
    }
}

// Fixture-only custody provisioning. Every shot cost uses the production item owner.
async fn swing_equipment(
    harness: &Harness,
    weapon: Option<(u8, &str, u32)>,
    ammo: bool,
) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid AND slot IN (6,10)")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    for (seed, key, count, slot) in weapon
        .into_iter()
        .map(|(seed, key, count)| (seed, key, count, 6_i16))
        .chain(ammo.then_some((194, "oteryn:item.tibia.i3447", 100, 10)))
    {
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(seed, key, "definition-r1", count),
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(slot).bind(id(seed).as_slice()).execute(&mut *tx).await?;
    }
    // Custody setup is fixture-only. Leave the revision/last-transaction pair owned by
    // the real shot receipts intact; every tested payment advances it through that owner.
    tx.commit().await?;
    Ok(())
}

async fn swing_target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    target: crate::foundation::ExactActorRef,
    command: u64,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let generation = runtime
        .visible_entities()
        .creatures
        .iter()
        .find(|c| c.actor == target)
        .ok_or("hazard swing target")?
        .generation;
    let mut attack = owner.states.attack.lock().await;
    assert!(attack.set_modes(
        &runtime,
        actor,
        session,
        oteryn_protocol_oteryn::attack::FightModes {
            fight_mode: oteryn_protocol_oteryn::attack::FightMode::Offensive,
            ..oteryn_protocol_oteryn::attack::FightModes::DEFAULT
        }
    ));
    assert_eq!(
        attack.set_target(
            &runtime,
            owner.rooms.gate_auto_attack(),
            actor,
            session,
            owner.owner_now(),
            Some(EntityRef {
                identity: target.placement_identity(),
                generation
            }),
            command
        ),
        oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
    );
    Ok(())
}

async fn outgoing_cleave_checks(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    harness: &Harness,
) -> TestResult<u64> {
    use crate::combat_attack::{AttackConstants, FightMode, player_melee_formula};
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::hazard::combat;
    use oteryn_gameplay_combat_rules::resolution;
    use oteryn_simulation_determinism::GameplayDecisionRoot;
    use sha2::{Digest, Sha256};

    swing_equipment(harness, Some((191, "oteryn:item.tibia.i3264", 1)), false).await?;
    let constants = AttackConstants::checked_in().ok_or("cleave constants")?;
    let (primary, flank, before_primary, before_flank, bounds, policy, digest, original_offence) = {
        let runtime = owner.runtime.lock().await;
        let primary = runtime
            .visible_entities()
            .creatures
            .iter()
            .find(|c| {
                runtime
                    .companion_snapshot(c.actor)
                    .is_ok_and(|s| s.state.policy.definition_key == "canary:creature/rat")
                    && runtime
                        .read_actor_position(c.actor)
                        .is_ok_and(|p| p.position() == OPEN_BOSS)
            })
            .ok_or("cleave primary")?
            .actor;
        let p = MovementLocalPosition {
            y: OPEN_PLAYER.y - 1,
            ..OPEN_BOSS
        };
        let policy = runtime
            .companion_policy("canary:creature/rat")
            .map_err(debug)?;
        let flank = runtime
            .visible_entities()
            .creatures
            .iter()
            .find(|c| {
                c.actor != primary
                    && runtime
                        .read_actor_position(c.actor)
                        .is_ok_and(|at| at.position() == p)
            })
            .ok_or("cleave flank")?
            .actor;
        assert!(combat::eligible(&runtime, primary) && combat::eligible(&runtime, flank));
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("cleave state")?;
        let original = state.equipped_offence();
        // Explicit 100% cleave fixture isolates the hazard parser from low-skill rounding
        // and rat defence. It does not assert an item's production cleave coefficient.
        state.install_equipped_offence(resolution::equipped_offence::EquippedOffence {
            cleave: 10_000,
            ..Default::default()
        });
        let facts = state.character_facts();
        let skills = state
            .swing_skills(2, owner.owner_now().get())
            .ok_or("sword skills")?
            .unwrap_or(crate::spell::cast::SwingSkills {
                attack: constants.starting_skill,
                shielding: constants.starting_skill,
            });
        let bounds = player_melee_formula(constants)
            .bounds(&crate::spell::formula::FormulaInputs {
                level: facts.level,
                magic_level: facts.magic_level,
                base_power: None,
                attack_skill: skills.attack,
                attack_value: 14,
                attack_factor: constants.fight_mode(FightMode::Offensive).attack_factor,
                shielding_skill: skills.shielding,
                shield_defense: None,
            })
            .map_err(debug)?;
        (
            primary,
            flank,
            runtime
                .creature_combat_facts(primary, 0)
                .map_err(debug)?
                .health,
            runtime
                .creature_combat_facts(flank, 0)
                .map_err(debug)?
                .health,
            bounds,
            policy,
            runtime.content_pin().server_artifact_digest(),
            original,
        )
    };
    let mut next_id = 1_000_000;
    for (primary_dodge, flank_dodge) in [(true, false), (false, true), (true, true), (false, false)]
    {
        owner.clock_origin -= Duration::from_secs(3);
        let (command, expected) = {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .set_test_creature_health(primary, 10_000)
                .map_err(debug)?;
            runtime
                .set_test_creature_health(flank, 10_000)
                .map_err(debug)?;
            (next_id..next_id + 10_000_000)
                .find_map(|command| {
                    let lineage = CommandRef::new(session, CommandId::new(command).ok()?);
                    let mut expected = [0; 2];
                    for (i, (target, dodge)) in [(primary, primary_dodge), (flank, flank_dodge)]
                        .into_iter()
                        .enumerate()
                    {
                        if combat::dodges(
                            1,
                            combat::swing_dodge_roll(&runtime, actor, target, lineage, 0)?,
                        ) != dodge
                        {
                            return None;
                        }
                        let root = GameplayDecisionRoot::from_bytes(if i == 0 {
                            digest
                        } else {
                            Sha256::new()
                                .chain_update(digest)
                                .chain_update(target.placement_identity())
                                .finalize()
                                .into()
                        });
                        let raw = resolution::swing_draw(
                            &root,
                            actor.placement_identity(),
                            (*session.as_bytes(), command),
                            0,
                            b"damage-draw",
                            "attack.damage",
                            bounds.0,
                            bounds.1,
                        )?;
                        let damage = resolution::creature_mitigated(
                            &root,
                            constants,
                            actor.placement_identity(),
                            (*session.as_bytes(), command),
                            0,
                            raw,
                            true,
                            policy.defense?,
                            policy.armor?,
                        )?;
                        let damage = resolution::creature_physical_response(&policy, damage)?;
                        if damage <= 0 {
                            return None;
                        }
                        expected[i] = if dodge { 0 } else { damage };
                    }
                    Some((command, expected))
                })
                .ok_or("bounded independent cleave draws")?
        };
        next_id = command + 1;
        swing_target(owner, actor, session, primary, command).await?;
        owner.refresh_attack_weapon(actor, session).await;
        let equipment =
            crate::gameplay_transport::equipment_move::read_current_equipment(owner, session)
                .await
                .ok_or("cleave equipment read")?;
        assert!(matches!(
            crate::gameplay_transport::attack::swing_weapon(
                owner
                    .active_generation
                    .ok_or("cleave generation")?
                    .native_gameplay()
                    .ok_or("cleave native")?,
                &equipment.items
            ),
            Some(crate::gameplay_transport::attack::SwingWeapon::Melee { attack: 14, .. })
        ));
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert_eq!(
                states
                    .get(&runtime, actor, session)
                    .ok_or("cleave offence")?
                    .equipped_offence()
                    .cleave,
                10_000
            );
        }
        owner.drain_auto_attacks().await;
        for (target, damage) in [(primary, expected[0]), (flank, expected[1])] {
            assert_eq!(
                10_000
                    - owner
                        .runtime
                        .lock()
                        .await
                        .creature_combat_facts(target, 0)
                        .map_err(debug)?
                        .health,
                damage,
                "independent cleave primary={primary_dodge} flank={flank_dodge}"
            );
        }
        owner.drain_auto_attacks().await;
        for (target, damage) in [(primary, expected[0]), (flank, expected[1])] {
            assert_eq!(
                10_000
                    - owner
                        .runtime
                        .lock()
                        .await
                        .creature_combat_facts(target, 0)
                        .map_err(debug)?
                        .health,
                damage,
                "cleave retry consumes no second hit"
            );
        }
    }
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .set_test_creature_health(primary, before_primary)
            .map_err(debug)?;
        runtime
            .set_test_creature_health(flank, before_flank)
            .map_err(debug)?;
        owner
            .states
            .spell_states
            .lock()
            .await
            .get_mut(&runtime, actor, session)
            .ok_or("restore cleave offence")?
            .install_equipped_offence(original_offence);
    }
    owner
        .states
        .attack
        .lock()
        .await
        .clear_target(actor, session);
    swing_equipment(harness, None, false).await?;
    Ok(next_id)
}

async fn outgoing_distance_checks(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    harness: &Harness,
    mut next_id: u64,
) -> TestResult<u64> {
    use crate::combat_attack::{AttackConstants, FightMode};
    use crate::content::{ProjectV2RangedElement, ProjectV2RangedWeapon};
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::hazard::combat;
    use oteryn_gameplay_combat_rules::{attack::ranged, resolution};
    use oteryn_simulation_determinism::GameplayDecisionRoot;
    let native = owner
        .active_generation
        .ok_or("ranged generation")?
        .native_gameplay()
        .ok_or("ranged content")?;
    let constants = AttackConstants::checked_in().ok_or("distance constants")?;
    let (target, before_health, policy, root, skills, facts) = {
        let runtime = owner.runtime.lock().await;
        let target = runtime
            .visible_entities()
            .creatures
            .iter()
            .find(|c| {
                runtime
                    .companion_snapshot(c.actor)
                    .is_ok_and(|s| s.state.policy.definition_key == "canary:creature/rat")
                    && runtime
                        .read_actor_position(c.actor)
                        .is_ok_and(|p| p.position() == OPEN_BOSS)
            })
            .ok_or("distance rat")?
            .actor;
        assert!(combat::eligible(&runtime, target));
        let states = owner.states.spell_states.lock().await;
        let state = states
            .get(&runtime, actor, session)
            .ok_or("distance state")?;
        (
            target,
            runtime
                .creature_combat_facts(target, 0)
                .map_err(debug)?
                .health,
            runtime
                .companion_snapshot(target)
                .map_err(debug)?
                .state
                .policy
                .clone(),
            GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest()),
            state
                .swing_skills(4, owner.owner_now().get())
                .ok_or("distance skills")?
                .unwrap_or(crate::spell::cast::SwingSkills {
                    attack: constants.starting_skill,
                    shielding: constants.starting_skill,
                }),
            state.character_facts(),
        )
    };
    for (seed, key, bow) in [
        (192, "oteryn:item.tibia.i3350", true),
        (193, "oteryn:item.tibia.i3287", false),
    ] {
        swing_equipment(harness, Some((seed, key, if bow { 1 } else { 100 })), bow).await?;
        let profile = native
            .item_policy(key, "definition-r1")
            .ok_or("distance weapon")?
            .record()
            .authoring
            .ranged
            .as_ref()
            .ok_or("ranged authoring")?;
        let (attack, shot, cap, bonus) = match profile {
            ProjectV2RangedWeapon::Distance {
                attack, hit_chance, ..
            } => {
                let ammo = native
                    .item_policy("oteryn:item.tibia.i3447", "definition-r1")
                    .ok_or("arrow")?
                    .record()
                    .authoring
                    .ranged
                    .as_ref()
                    .ok_or("arrow profile")?;
                let ProjectV2RangedWeapon::Ammunition {
                    attack: ammo_attack,
                    hit_chance: shot,
                    max_hit_chance,
                    element,
                    elemental_attack,
                    ..
                } = ammo
                else {
                    return Err("arrow kind".into());
                };
                assert_eq!(
                    (*element, *elemental_attack),
                    (ProjectV2RangedElement::Physical, 0)
                );
                (
                    u32::from(*attack) + u32::from(*ammo_attack),
                    *shot,
                    *max_hit_chance,
                    i32::from(*hit_chance),
                )
            }
            ProjectV2RangedWeapon::Throwing {
                attack,
                hit_chance,
                max_hit_chance,
                element,
                elemental_attack,
                ..
            } => {
                assert_eq!(
                    (*element, *elemental_attack),
                    (ProjectV2RangedElement::Physical, 0)
                );
                (u32::from(*attack), *hit_chance, *max_hit_chance, 0)
            }
            _ => return Err("distance fixture kind".into()),
        };
        let bounds = ranged::player_distance_formula()
            .bounds(&crate::spell::formula::FormulaInputs {
                level: facts.level,
                magic_level: facts.magic_level,
                base_power: None,
                attack_skill: skills.attack,
                attack_value: attack,
                attack_factor: constants.fight_mode(FightMode::Offensive).attack_factor,
                shielding_skill: skills.shielding,
                shield_defense: None,
            })
            .map_err(debug)?;
        let chance = ranged::shot_hit_chance(
            u32::from(cap),
            u32::try_from(shot).ok(),
            Some(bonus),
            1,
            skills.attack,
        )
        .ok_or("shot chance")?;
        let quantity_seed = if bow { 194 } else { seed };
        for (dodge, breaks, reconcile) in if bow {
            vec![
                (true, true, false),
                (false, true, false),
                (false, true, true),
            ]
        } else {
            vec![
                (true, true, false),
                (true, false, false),
                (false, true, false),
                (false, false, false),
            ]
        } {
            owner.clock_origin -= Duration::from_secs(3);
            let (command, damage) = {
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .set_test_creature_health(target, 10_000)
                    .map_err(debug)?;
                (next_id..next_id + 1_000_000)
                    .find_map(|command| {
                        let lineage = CommandRef::new(session, CommandId::new(command).ok()?);
                        if combat::dodges(
                            1,
                            combat::swing_dodge_roll(&runtime, actor, target, lineage, 0)?,
                        ) != dodge
                        {
                            return None;
                        }
                        let draw = |tag, domain, min, max| {
                            resolution::swing_draw(
                                &root,
                                actor.placement_identity(),
                                (*session.as_bytes(), command),
                                0,
                                tag,
                                domain,
                                min,
                                max,
                            )
                        };
                        if !ranged::shot_hits(
                            chance,
                            u64::try_from(draw(b"hit-chance", "attack.hit-chance", 0, 99)?).ok()?,
                        ) {
                            return None;
                        }
                        if !bow && (draw(b"break", "attack.break", 0, 99)? < 10) != breaks {
                            return None;
                        }
                        let raw = draw(b"damage-draw", "attack.damage", bounds.0, bounds.1)?;
                        let damage = resolution::swing_mitigated(
                            &root,
                            constants,
                            actor.placement_identity(),
                            (*session.as_bytes(), command),
                            0,
                            raw,
                            None,
                            policy.armor?,
                        )?;
                        let damage =
                            resolution::creature_damage_response(&policy, damage, "physical")?;
                        (damage > 0).then_some((command, damage))
                    })
                    .ok_or("bounded hazard distance/break draw")?
            };
            next_id = command + 1;
            swing_target(owner, actor, session, target, command).await?;
            owner.refresh_attack_weapon(actor, session).await;
            let quantity = || async {
                sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(quantity_seed).as_slice()).fetch_one(&harness.pool).await
            };
            let before = quantity().await?;
            let mana = owner
                .observe_actor_vitals(actor, session)
                .await
                .ok_or("distance mana")?
                .1
                .mana;
            if reconcile {
                owner
                    .park_committed_ranged_shot_for_test(actor, session)
                    .await?;
            }
            owner.drain_auto_attacks().await;
            let expected = if dodge || reconcile { 0 } else { damage };
            assert_eq!(
                10_000
                    - owner
                        .runtime
                        .lock()
                        .await
                        .creature_combat_facts(target, 0)
                        .map_err(debug)?
                        .health,
                expected,
                "bow={bow} dodge={dodge} breaks={breaks} reconcile={reconcile}"
            );
            assert_eq!(
                quantity().await?,
                before - i64::from(breaks),
                "payment/break is independent of hazard dodge"
            );
            assert_eq!(
                owner
                    .observe_actor_vitals(actor, session)
                    .await
                    .ok_or("paid distance mana")?
                    .1
                    .mana,
                mana
            );
            owner.drain_auto_attacks().await;
            assert_eq!(
                quantity().await?,
                before - i64::from(breaks),
                "retry spends no second projectile"
            );
            assert_eq!(
                10_000
                    - owner
                        .runtime
                        .lock()
                        .await
                        .creature_combat_facts(target, 0)
                        .map_err(debug)?
                        .health,
                expected,
                "retry deals no late damage"
            );
        }
        owner
            .states
            .attack
            .lock()
            .await
            .clear_target(actor, session);
    }
    owner
        .runtime
        .lock()
        .await
        .set_test_creature_health(target, before_health)
        .map_err(debug)?;
    swing_equipment(harness, None, false).await?;
    let mut tx = harness.pool.begin().await?;
    crate::durability::spell_item_transaction::verify_spell_item_chain(&mut tx)
        .await
        .map_err(debug)?;
    tx.rollback().await?;
    Ok(next_id)
}

// The canonical Creature store is seeded with a typed player DOT, then the real
// admitted-session scheduler acquires the live source/World/tile owners and consumes it.
async fn outgoing_condition_checks(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    target: crate::foundation::ExactActorRef,
) -> TestResult {
    use crate::foundation::condition::{
        ApplicationFacts, ConditionDefinition, ConditionLineage, ConditionSourceKind, TickFacts,
    };
    use crate::foundation::{ConditionValues, DotElement};
    use crate::gameplay_transport::hazard::combat;
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    for (agony, should_dodge, moved_outside, historical) in [
        (false, true, false, false),
        (false, false, false, false),
        (true, true, false, false),
        (true, false, false, false),
        (false, true, true, false),
        (false, true, false, true),
    ] {
        let (before_health, due) = {
            let mut runtime = owner.runtime.lock().await;
            let before = runtime.companion_snapshot(target).map_err(debug)?;
            let root =
                GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
            let lease = runtime
                .bound_attacker_lease(actor, session)
                .map_err(debug)?;
            let lineage = ConditionLineage {
                source_character: *lease.character_id().as_bytes(),
                source_session: if historical {
                    [0x72; 16]
                } else {
                    *session.as_bytes()
                },
                source_lease_generation: lease.generation(),
                source_actor_placement: actor.placement_identity(),
                source_scope_generation: actor.scope_generation().get(),
                source_content_digest: runtime.content_pin().server_artifact_digest(),
                source_catalog_digest: runtime.content_pin().server_artifact_digest(),
                source_creation_command: 800_000,
            };
            let definition = ConditionDefinition::new(
                "test.hazard.outgoing.condition",
                1,
                if agony {
                    ConditionValues::Agony {
                        total_min: 100,
                        total_max: 100,
                        per_tick: 100,
                        interval_ms: 1000,
                        delayed: true,
                    }
                } else {
                    ConditionValues::DamageOverTime {
                        element: DotElement::Energy,
                        total_min: 100,
                        total_max: 100,
                        per_tick: 100,
                        interval_ms: 1000,
                        delayed: true,
                    }
                },
            )
            .ok_or("hazard condition definition")?;
            let earliest = owner.owner_now().get() + 1_000_000;
            let (next, due) = (earliest..earliest + 100_000)
                .find_map(|due| {
                    let facts = ApplicationFacts {
                        now: due - 1_000_000,
                        base_speed: 100,
                        mana_shield_capacity: 0,
                        target_reentry_protected: false,
                        source_reentry_protected: false,
                        target_is_player: false,
                        decision_root: &root,
                        occurrence: DecisionOccurrenceId::from_bytes([0x71; 16]),
                    };
                    let mut next = before.state.clone();
                    next.conditions
                        .apply_with_lineage(
                            &definition,
                            Some(actor),
                            ConditionSourceKind::Player,
                            &[],
                            &facts,
                            &lineage,
                        )
                        .ok()?;
                    let tick = next
                        .conditions
                        .clone()
                        .take_due(due, TickFacts::default())
                        .pop()?;
                    let roll = combat::condition_dodge_roll(&runtime, actor, target, &tick)?;
                    (combat::dodges(1, roll) == should_dodge).then_some((next, due))
                })
                .ok_or("bounded condition dodge occurrence")?;
            runtime
                .compare_companion_state(&before, next)
                .map_err(debug)?;
            (before.health, due)
        };
        let advance = due.saturating_sub(owner.owner_now().get()) + 10_000;
        owner.clock_origin -= Duration::from_micros(advance);
        // Without the current World level this lower owner must leave HP and cursor intact.
        if !historical {
            let mut runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let before = runtime.companion_snapshot(target).map_err(debug)?;
            assert!(
                states
                    .tick_source_creature_conditions(
                        &mut runtime,
                        target,
                        TickFacts::default(),
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                            owner.owner_now().get()
                        ),
                    )
                    .is_err()
            );
            assert_eq!(runtime.companion_snapshot(target).map_err(debug)?, before);
        }
        let original_position = if moved_outside {
            let mut runtime = owner.runtime.lock().await;
            let mut movement = runtime.borrow_movement_position();
            let before = movement.read(actor).map_err(debug)?;
            let original = before.position();
            movement
                .commit_cardinal(
                    before,
                    MovementLocalPosition {
                        x: original.x - 1,
                        ..original
                    },
                )
                .map_err(debug)?;
            Some(original)
        } else {
            None
        };
        owner.drain_monster_melee().await;
        let after = {
            let runtime = owner.runtime.lock().await;
            let after = runtime.companion_snapshot(target).map_err(debug)?;
            assert!(
                after.state.conditions.instances().is_empty(),
                "real scheduler consumes DOT"
            );
            if should_dodge && !moved_outside && !historical {
                assert_eq!(after.health, before_health, "forced condition dodge");
            } else {
                assert!(after.health < before_health, "forced condition non-dodge");
            }
            after.health
        };
        owner.drain_monster_melee().await;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .companion_snapshot(target)
                .map_err(debug)?
                .health,
            after,
            "condition retry deals no duplicate damage"
        );
        if let Some(original) = original_position {
            let mut runtime = owner.runtime.lock().await;
            let mut movement = runtime.borrow_movement_position();
            let before = movement.read(actor).map_err(debug)?;
            movement.commit_cardinal(before, original).map_err(debug)?;
        }
    }
    Ok(())
}

// Force the real swing owner through both sides of the hazard draw. The bounded
// occurrence search selects positive post-defence damage, so an unchanged HP value
// cannot pass merely because the rat blocked an ordinary zero hit.
async fn outgoing_swing_checks(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
) -> TestResult {
    use crate::combat_attack::{AttackConstants, FightMode, player_fist_formula};
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::hazard::combat;
    use oteryn_gameplay_combat_rules::resolution;
    use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, FightModes};
    use oteryn_simulation_determinism::GameplayDecisionRoot;

    let constants = AttackConstants::checked_in().ok_or("attack constants")?;
    {
        let permit = owner.spell_lane_permit().await.ok_or("swing level lane")?;
        let mut runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        assert_eq!(
            owner
                .with_hazard_level(
                    &permit,
                    &mut runtime,
                    &mut states,
                    actor,
                    session,
                    |_, _, level| level
                )
                .await,
            Some(1)
        );
    }
    let (target, reference, bounds, policy, root) = {
        let runtime = owner.runtime.lock().await;
        let visible = runtime.visible_entities();
        let rat = visible
            .creatures
            .iter()
            .find(|c| {
                runtime
                    .companion_snapshot(c.actor)
                    .is_ok_and(|s| s.state.policy.definition_key == "canary:creature/rat")
                    && runtime
                        .read_actor_position(c.actor)
                        .is_ok_and(|p| p.position() == OPEN_BOSS)
            })
            .ok_or("hazard swing rat")?;
        let snapshot = runtime.companion_snapshot(rat.actor).map_err(debug)?;
        assert!(combat::eligible(&runtime, rat.actor));
        let states = owner.states.spell_states.lock().await;
        let player = states.get(&runtime, actor, session).ok_or("swing player")?;
        let facts = player.character_facts();
        let skills = player
            .swing_skills(0, owner.owner_now().get())
            .ok_or("swing skills")?
            .unwrap_or(crate::spell::cast::SwingSkills {
                attack: constants.starting_skill,
                shielding: constants.starting_skill,
            });
        let bounds = player_fist_formula(constants)
            .bounds(&crate::spell::formula::FormulaInputs {
                level: facts.level,
                magic_level: facts.magic_level,
                base_power: None,
                attack_skill: skills.attack,
                attack_value: constants.fist.attack,
                attack_factor: constants.fight_mode(FightMode::Offensive).attack_factor,
                shielding_skill: skills.shielding,
                shield_defense: None,
            })
            .map_err(debug)?;
        (
            rat.actor,
            EntityRef {
                identity: rat.actor.placement_identity(),
                generation: rat.generation,
            },
            bounds,
            snapshot.state.policy.clone(),
            GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest()),
        )
    };
    let defense = policy.defense.ok_or("rat defense")?;
    let armor = policy.armor.ok_or("rat armor")?;
    for should_dodge in [true, false] {
        owner.clock_origin -= Duration::from_secs(3);
        let (command_id, expected) = {
            let runtime = owner.runtime.lock().await;
            (2000..100_000)
                .find_map(|id| {
                    let lineage = CommandRef::new(session, CommandId::new(id).ok()?);
                    let roll = combat::swing_dodge_roll(&runtime, actor, target, lineage, 0)?;
                    if combat::dodges(1, roll) != should_dodge {
                        return None;
                    }
                    let raw = resolution::swing_draw(
                        &root,
                        actor.placement_identity(),
                        (*session.as_bytes(), id),
                        0,
                        b"damage-draw",
                        "attack.damage",
                        bounds.0,
                        bounds.1,
                    )?;
                    let damage = resolution::creature_mitigated(
                        &root,
                        constants,
                        actor.placement_identity(),
                        (*session.as_bytes(), id),
                        0,
                        raw,
                        true,
                        defense,
                        armor,
                    )?;
                    let damage = resolution::creature_physical_response(&policy, damage)?;
                    (damage > 0).then_some((id, damage))
                })
                .ok_or("bounded positive hazard swing occurrence")?
        };
        let before = {
            let runtime = owner.runtime.lock().await;
            let mut attack = owner.states.attack.lock().await;
            assert!(attack.set_modes(
                &runtime,
                actor,
                session,
                FightModes {
                    fight_mode: oteryn_protocol_oteryn::attack::FightMode::Offensive,
                    ..FightModes::DEFAULT
                }
            ));
            assert_eq!(
                attack.set_target(
                    &runtime,
                    owner.rooms.gate_auto_attack(),
                    actor,
                    session,
                    owner.owner_now(),
                    Some(reference),
                    command_id
                ),
                AttackIntentDisposition::Ok
            );
            runtime
                .creature_combat_facts(target, owner.owner_now().get() / 1000)
                .map_err(debug)?
                .health
        };
        owner.refresh_attack_weapon(actor, session).await;
        let swing_started = owner.owner_now();
        owner.drain_auto_attacks().await;
        let runtime = owner.runtime.lock().await;
        assert!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .hazard_level
                .is_none()
        );
        let after = runtime
            .creature_combat_facts(target, owner.owner_now().get() / 1000)
            .map_err(debug)?
            .health;
        assert_eq!(
            before - after,
            if should_dodge { 0 } else { expected },
            "forced hazard dodge={should_dodge}"
        );
        let attack = owner.states.attack.lock().await;
        assert!(
            attack.combat_state(actor, session, swing_started).in_fight,
            "the real swing must run even when dodged"
        );
        let due = attack
            .party_attack_state(actor, session)
            .and_then(|s| s.next_due(constants.attack_interval_micros()))
            .ok_or("swing timer advanced")?;
        assert!(due > swing_started);
    }
    owner
        .states
        .attack
        .lock()
        .await
        .clear_target(actor, session);
    Ok(())
}

async fn combat_checks(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    source: crate::foundation::ExactActorRef,
) -> TestResult {
    use crate::ability::creature_bite::CreatureBiteVitals;
    use crate::gameplay_transport::attack::player_block::PlayerBiteVitals;
    use crate::gameplay_transport::hazard::combat;
    let permit = owner.spell_lane_permit().await.ok_or("combat lane")?;
    let mut runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let mut attacks = owner.states.attack.lock().await;
    let mut trained = Vec::new();
    let now =
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(owner.owner_now().get());
    // Suppress the independent critical draw for a precise ordinary damage assertion.
    states.hazard_criticals.record(actor, session, now.get());
    let hit = owner
        .with_hazard_level(
            &permit,
            &mut runtime,
            &mut states,
            actor,
            session,
            |runtime, states, level| {
                assert_eq!(level, 1);
                PlayerBiteVitals {
                    hazard: Some((actor, session, level)),
                    states,
                    attack: &mut attacks,
                    formula: None,
                    trained: &mut trained,
                }
                .apply_creature_damage_from(runtime, source, actor, session, 100, now)
            },
        )
        .await
        .flatten()
        .ok_or("hazard physical hit")?;
    assert_eq!(hit.damage.applied, 102);
    // An ordinary environmental hit must never acquire the monster modifier.
    let hit = states
        .apply_creature_damage(&mut runtime, actor, session, 100, now)
        .ok_or("environment hit")?;
    assert_eq!(hit.damage.applied, 100);
    let other_source = runtime
        .visible_entities()
        .creatures
        .iter()
        .find_map(|c| {
            runtime
                .companion_snapshot(c.actor)
                .ok()
                .filter(|s| s.state.policy.definition_key == "canary:creature/rat")
                .map(|s| s.actor)
        })
        .ok_or("second hazard attacker")?;
    let first = critical_time(&runtime, source, actor, now.get() + 2_000_000)?;
    let second = critical_time(&runtime, other_source, actor, first + 1)?;
    assert!(second < first + 2_000_000);
    let third = critical_time(&runtime, source, actor, first + 2_000_000)?;
    for (attacker, at, expected) in [
        (source, first, 153),
        (other_source, second, 102),
        (source, third, 153),
    ] {
        let hit = owner
            .with_hazard_level(
                &permit,
                &mut runtime,
                &mut states,
                actor,
                session,
                |runtime, states, level| {
                    PlayerBiteVitals {
                        hazard: Some((actor, session, level)),
                        states,
                        attack: &mut attacks,
                        formula: None,
                        trained: &mut trained,
                    }
                    .apply_creature_damage_from(
                        runtime,
                        attacker,
                        actor,
                        session,
                        100,
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(at),
                    )
                },
            )
            .await
            .flatten()
            .ok_or("critical physical hit")?;
        assert_eq!(
            hit.damage.applied, expected,
            "per-victim hazard critical cooldown"
        );
    }

    use crate::player_lethal::PlayerLethalVitals;
    // Direct MP loss takes the pinned combatChangeMana path, which never calls
    // the hazard HP parser. Even an occurrence that would critical must stay unscaled.
    let mana_at = critical_time(&runtime, source, actor, third + 2_000_000)?;
    let mana_now = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(mana_at);
    assert!(
        states
            .hazard_criticals
            .ready(&runtime, actor, session, mana_at)
    );
    let before_mana = states
        .get(&runtime, actor, session)
        .ok_or("mana predecessor")?
        .vitals();
    let drain = owner
        .with_hazard_level(
            &permit,
            &mut runtime,
            &mut states,
            actor,
            session,
            |runtime, states, level| {
                assert_eq!(level, 1);
                states.apply_attack_mana_drain_from(
                    runtime,
                    source,
                    actor,
                    session,
                    40,
                    "hazard:mana",
                    mana_now,
                )
            },
        )
        .await
        .flatten()
        .ok_or("hazard mana drain")?;
    assert_eq!(drain.applied, before_mana.mana.min(40));
    assert_eq!(drain.mana_after, before_mana.mana.saturating_sub(40));
    assert_eq!(
        states
            .get(&runtime, actor, session)
            .ok_or("mana successor")?
            .vitals()
            .health,
        before_mana.health
    );
    assert!(
        states
            .hazard_criticals
            .ready(&runtime, actor, session, mana_at)
    );
    // Replay needs neither a new hazard read nor a fresh random draw, but it must
    // retain the exact source and original magnitude.
    assert_eq!(
        states.apply_attack_mana_drain_from(
            &mut runtime,
            source,
            actor,
            session,
            40,
            "hazard:mana",
            mana_now,
        ),
        Some(drain)
    );
    for (attacker, amount) in [(other_source, 40), (source, 41)] {
        assert!(
            states
                .apply_attack_mana_drain_from(
                    &mut runtime,
                    attacker,
                    actor,
                    session,
                    amount,
                    "hazard:mana",
                    mana_now,
                )
                .is_none()
        );
    }
    assert!(
        states
            .apply_attack_mana_drain(&mut runtime, actor, session, 40, "hazard:mana", mana_now,)
            .is_none(),
        "an unsourced retry cannot replace a sourced receipt"
    );
    let clamped = states
        .apply_attack_mana_drain_from(
            &mut runtime,
            source,
            actor,
            session,
            u32::MAX,
            "hazard:mana:clamp",
            mana_now,
        )
        .ok_or("clamped mana")?;
    assert_eq!(clamped.applied, drain.mana_after);
    assert_eq!(clamped.mana_after, 0);
    let empty = states
        .apply_attack_mana_drain_from(
            &mut runtime,
            source,
            actor,
            session,
            40,
            "hazard:mana:empty",
            mana_now,
        )
        .ok_or("empty mana")?;
    assert_eq!(empty.applied, 0);
    assert_eq!(empty.vitals_revision, clamped.vitals_revision);
    assert!(
        states
            .hazard_criticals
            .ready(&runtime, actor, session, mana_at)
    );
    states
        .apply_mana_gain(&runtime, actor, session, u64::from(before_mana.max_mana))
        .ok_or("restore fixture mana")?;

    // The source-aware sink used by standalone spells and source melee shares this same
    // current World observation and the victim's critical clock with native bites.
    let spell = owner
        .with_hazard_level(
            &permit,
            &mut runtime,
            &mut states,
            actor,
            session,
            |runtime, states, _| {
                states.apply_attack_damage_from(
                    runtime,
                    source,
                    actor,
                    session,
                    100,
                    "hazard:standalone",
                    now,
                )
            },
        )
        .await
        .flatten()
        .ok_or("source-aware spell hit")?;
    assert_eq!(spell.applied, 102);
    let health = states
        .get(&runtime, actor, session)
        .ok_or("source vitals")?
        .vitals()
        .health;
    let replay = owner
        .with_hazard_level(
            &permit,
            &mut runtime,
            &mut states,
            actor,
            session,
            |runtime, states, _| {
                states.apply_attack_damage_from(
                    runtime,
                    source,
                    actor,
                    session,
                    100,
                    "hazard:standalone",
                    now,
                )
            },
        )
        .await
        .flatten()
        .ok_or("source-aware replay")?;
    assert_eq!(replay, spell);
    assert_eq!(
        states
            .get(&runtime, actor, session)
            .ok_or("replay vitals")?
            .vitals()
            .health,
        health
    );
    assert!(
        states
            .apply_attack_damage_from(
                &mut runtime,
                other_source,
                actor,
                session,
                100,
                "hazard:standalone",
                now
            )
            .is_none(),
        "replay cannot change its monster source"
    );
    assert!(
        states.hazard_level.is_none(),
        "locked level must not escape its synchronous consumer"
    );

    let definition = crate::foundation::ConditionDefinition::new_damage_schedule(
        "hazard.source.fire",
        1,
        oteryn_gameplay_domain::domain::condition::DotElement::Fire,
        crate::foundation::DamageSchedule::Fixed {
            delayed: true,
            segments: vec![crate::foundation::DamageSegment {
                count: 1,
                interval_ms: 1000,
                amount: 100,
            }],
        },
    )
    .ok_or("hazard damage condition")?;
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
        runtime.content_pin().server_artifact_digest(),
    );
    let facts = crate::foundation::ApplicationFacts {
        now: now.get(),
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(occurrence(
            108,
        )),
    };
    let binding = runtime.binding();
    let (fence, stamp) = crate::foundation::crystal_timer_fixture(
        crate::foundation::RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        binding.scope_generation(),
    )
    .map_err(debug)?;
    let composite = owner
        .with_hazard_level(
            &permit,
            &mut runtime,
            &mut states,
            actor,
            session,
            |runtime, states, _| {
                states.apply_composite_attack_damage(
                    runtime,
                    actor,
                    session,
                    100,
                    "hazard:composite",
                    source,
                    std::slice::from_ref(&definition),
                    &facts,
                    &[],
                    &fence,
                    stamp,
                )
            },
        )
        .await
        .flatten()
        .ok_or("composite spell")?;
    assert_eq!(composite.applied, 102);
    let replay = owner
        .with_hazard_level(
            &permit,
            &mut runtime,
            &mut states,
            actor,
            session,
            |runtime, states, _| {
                states.apply_composite_attack_damage(
                    runtime,
                    actor,
                    session,
                    100,
                    "hazard:composite",
                    source,
                    std::slice::from_ref(&definition),
                    &facts,
                    &[],
                    &fence,
                    stamp,
                )
            },
        )
        .await
        .flatten()
        .ok_or("composite replay")?;
    assert_eq!(replay, composite);

    let outside = MovementLocalPosition {
        x: 33501,
        ..OPEN_PLAYER
    };
    let position = runtime.read_actor_position(actor).map_err(debug)?;
    runtime
        .borrow_movement_position()
        .commit_cardinal(position, outside)
        .map_err(debug)?;
    let level = owner
        .with_hazard_level(
            &permit,
            &mut runtime,
            &mut states,
            actor,
            session,
            |_, _, level| level,
        )
        .await;
    assert_eq!(level, Some(0));
    let position = runtime.read_actor_position(actor).map_err(debug)?;
    runtime
        .borrow_movement_position()
        .commit_cardinal(position, OPEN_PLAYER)
        .map_err(debug)?;
    assert!(!combat::dodges(0, 0));
    drop(attacks);
    drop(states);
    drop(runtime);
    drop(permit);
    // Real periodic dispatch consumes the condition installed atomically with the spell.
    // The selected level and area are read again, rather than frozen at application.
    assert!(
        owner.ensure_source_map_initialized().await,
        "real periodic map predecessor"
    );
    owner.clock_origin -= Duration::from_millis(1100);
    let before = owner
        .observe_actor_vitals(actor, session)
        .await
        .ok_or("condition vitals before")?
        .1
        .health;
    {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let state = states
            .get(&runtime, actor, session)
            .ok_or("periodic state")?;
        assert!(
            crate::spell::actor_conditions::has_periodic_damage(state),
            "installed periodic damage"
        );
        assert_eq!(
            crate::gameplay_transport::actor_spell::qualified_protection_zone(
                &mut runtime,
                owner.movement_cells,
                actor
            ),
            Some(false),
            "qualified standing PZ"
        );
        let preview = crate::spell::actor_conditions::stage_source_owner_cycle(
            state,
            owner.owner_now(),
            Some(crate::foundation::TickFacts {
                in_protection_zone: false,
                standing_on_field: None,
            }),
            None,
        )
        .map_err(debug)?
        .ok_or("due preview")?;
        assert!(
            !preview.combat_ticks.is_empty(),
            "installed condition is due: {:?}",
            states.owned_source_condition_fixture_snapshot(&runtime, actor, session)
        );
    }
    let periodic = owner.tick_spell_periodic(actor, session).await;
    assert!(
        periodic.is_some(),
        "real periodic owner must publish the condition: {periodic:?}"
    );
    let after = owner
        .observe_actor_vitals(actor, session)
        .await
        .ok_or("condition vitals after")?
        .1
        .health;
    assert_eq!(
        before - after,
        102,
        "hazard condition through real periodic dispatch"
    );
    owner.tick_spell_periodic(actor, session).await;
    assert_eq!(
        owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("condition replay")?
            .1
            .health,
        after,
        "periodic cursor prevents duplicate damage"
    );
    // The real Channel AI router consumes the same attacker-aware vitals method.
    owner.clock_origin -= Duration::from_secs(30);
    let before = owner
        .observe_actor_vitals(actor, session)
        .await
        .ok_or("vitals before")?
        .1
        .health;
    let mut after = before;
    for _ in 0..32 {
        owner.clock_origin -= Duration::from_secs(3);
        owner.drain_monster_melee().await;
        after = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("vitals after")?
            .1
            .health;
        if after < before {
            break;
        }
    }
    assert!(
        after < before,
        "native melee must traverse hazard dispatch: {before} -> {after}"
    );
    Ok(())
}

fn occurrence(tag: u8) -> [u8; 16] {
    let mut value = [tag; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}
