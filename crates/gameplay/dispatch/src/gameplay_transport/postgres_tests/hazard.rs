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
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world,
                Some(include_bytes!(
                    "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                )),
            );
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
            8,
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
                            vocation: crate::spell::Vocation::Monk,
                            max_health: 100_000,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
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
                ("canary:creature/rat", OPEN_BOSS),
                (
                    hazard::PRIMAL_MENACE,
                    MovementLocalPosition {
                        x: 33506,
                        ..OPEN_BOSS
                    },
                ),
            ]
            .into_iter()
            .map(|(key, p)| {
                let policy = runtime.companion_policy(key).map_err(debug)?;
                Ok((
                    vec![(p.x, p.y, p.floor)],
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
            combat_checks(&mut owner, actor, session, target).await?;
            let mut runtime = owner.runtime.lock().await;
            let health = runtime
                .creature_combat_facts(target, 0)
                .map_err(debug)?
                .health;
            let command = crate::foundation::CommandRef::new(
                session,
                crate::foundation::CommandId::new(1000).map_err(debug)?,
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
            xp, 0,
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
