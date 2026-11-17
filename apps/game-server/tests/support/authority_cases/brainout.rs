//! Smaller dedicated-PG mount of the same brain mission assertions and real USE transport.
use crate::bestiary_postgres_harness as fixture;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId, NodeId,
};
use fixture::{debug, id};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{
    self, AdmittedSession, ComposedTestOwner, ControllerBinding, FirstEntryOutcome,
    SessionContinuity, fresh_fixture,
};
use oteryn_gameplay_dispatch::gameplay_transport::{
    ComposedFreshAdmission, FreshEvidenceSource, SeamRooms,
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use std::sync::Arc;
use tokio::sync::Mutex;
type TestResult<T = ()> = fixture::TestResult<T>;

// Reuse every assertion; this mount changes only the fixture's crate boundary.
#[path = "../../../../../crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/brainout.rs"]
mod cases;
mod kill_reward_e2e {
    pub(super) mod scenario_thread {
        pub fn run(
            name: &str,
            body: impl FnOnce() -> super::super::TestResult + Send + 'static,
        ) -> super::super::TestResult {
            std::thread::Builder::new()
                .name(name.into())
                .stack_size(256 << 20)
                .spawn(move || body().map_err(|e| e.to_string()))?
                .join()
                .map_err(|_| "brain authority transport thread failed")??;
            Ok(())
        }
    }
}
mod l6_world_pickups {
    pub(super) use super::world::{ExpectedTile, NativeCrop};
    pub(super) fn native_scene(
        native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
        specs: Vec<NativeCrop>,
        teleports: &[super::world::ExpectedTeleport],
    ) -> super::TestResult<oteryn_gameplay_world::map::boot::BundleWorld> {
        super::world::native_scene(
            native,
            specs,
            teleports,
            &[],
            crate::foundation::WorldId::decode(&super::id(42)).map_err(super::debug)?,
            crate::foundation::ChannelId::decode(&super::id(43)).map_err(super::debug)?,
        )
    }
}
mod quest_world_callbacks {
    use super::{AdmittedSession, TestResult, composed_test_owner, debug, id};
    use crate::foundation::{MessageType, decode_wire_envelope};
    pub(super) fn scenario(
        name: &str,
        trigger: oteryn_gameplay_quests::quest::world_callbacks::Position,
        lever: bool,
    ) -> TestResult {
        super::scenario(name, trigger, lever)
    }
    pub(super) async fn input_item(
        owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        offering: &oteryn_gameplay_quests::quest::world_callbacks::Offering,
        n: u64,
        carried: bool,
    ) -> TestResult {
        use crate::durability::{
            item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest},
            item_mint::{
                GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest,
                TypedDefinitionRef,
            },
            item_transfer::{ItemTransferDestination, ItemTransferRequest},
        };
        use composed_test_owner::native_item_weights;
        use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: offering.item.clone(),
            revision_ref: "definition-r1".into(),
        };
        let facts = generation_item_facts(native, &definition)
            .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?;
        let map = super::cases::bundle_map(owner)?;
        let (digest, frame) = {
            let r = owner.runtime.lock().await;
            (
                r.content_pin().server_artifact_digest(),
                r.content_pin().frame_binding_digest(),
            )
        };
        let revision = format!(
            "sha256:{}",
            digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let mut candidate = owner
            .root
            .freeze_item_mint(
                owner.character,
                owner.holder,
                ItemMintRequest {
                    cause: ItemMintCause::for_test(
                        owner.world_id,
                        owner.channel_id,
                        fence.scope_ownership_generation,
                        1,
                        n,
                        TypedDefinitionRef {
                            family: "LootTable".into(),
                            production_key: "fixture:quest-callbacks".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        "fixture:quest-callbacks".into(),
                        0,
                    ),
                    item: definition,
                    quantity: u32::from(offering.count),
                    ground: GroundPlacement {
                        spatial_position:
                            crate::durability::map_item_mint_audit::map_item_spatial_position(
                                offering.position.x,
                                offering.position.y,
                                offering.position.floor,
                            ),
                        corpse_ref: id(3).to_vec(),
                        map_revision: oteryn_gameplay_world::map::overlay::map_revision(
                            map.overlay.base(),
                        ),
                        content_revision: revision.clone(),
                        native_room_placement_context: frame.to_vec(),
                    },
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await
            .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?;
        let permit = owner
            .spell_lane
            .acquire()
            .await
            .map_err(|_| "unresolved spell lane")?;
        let ItemMintOutcome::Committed(item) = owner
            .root
            .commit_item_mint(&permit, owner.character, owner.holder, &mut candidate)
            .await
            .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?
        else {
            return Err("mint".into());
        };
        let command = |offset| -> TestResult<_> {
            Ok(crate::foundation::CommandRef::new(
                fence.game_session_id,
                crate::foundation::CommandId::new(1000 + n * 10 + offset)
                    .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?,
            ))
        };
        let mut transfer = owner
            .root
            .freeze_item_transfer(
                owner.character,
                owner.holder,
                fence,
                ItemTransferRequest {
                    weight_generation_digest: native.source_digest(),
                    command: command(0)?,
                    source_item_instance_id: item.item_instance_id,
                    destination: ItemTransferDestination::MainBackpack,
                    item: facts,
                    backpack: Some(
                        generation_item_facts(
                            native,
                            &TypedDefinitionRef {
                                family: "Item".into(),
                                production_key: "oteryn:item.tibia.i2854".into(),
                                revision_ref: "definition-r1".into(),
                            },
                        )
                        .map_err(|error| {
                            format!("input item n={n} at line {}: {error:?}", line!())
                        })?,
                    ),
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
                &native_item_weights(native),
            )
            .await
            .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?;
        owner
            .root
            .commit_item_transfer(
                &permit,
                owner.character,
                owner.holder,
                fence,
                &mut transfer,
                &native_item_weights(native),
            )
            .await
            .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?;
        if carried {
            return Ok(());
        }
        let moved = owner
            .root
            .commit_item_loop_move(
                &permit,
                owner.character,
                owner.holder,
                &fence,
                command(1)?,
                &native_item_weights(native),
                LoopMoveRequest {
                    item: item.item_instance_id,
                    count: None,
                    pickup_cell: None,
                    destination: LoopDestination::Ground(LoopGround {
                        spatial_position:
                            crate::durability::map_item_mint_audit::map_item_spatial_position(
                                offering.position.x,
                                offering.position.y,
                                offering.position.floor,
                            ),
                        map_revision: oteryn_gameplay_world::map::overlay::map_revision(
                            map.overlay.base(),
                        ),
                        content_revision: revision,
                        placement_context: frame.to_vec(),
                    }),
                },
            )
            .await
            .map_err(|error| format!("input item n={n} at line {}: {error:?}", line!()))?;
        assert!(moved.is_ok(), "drop: {moved:?}");
        Ok(())
    }

    pub(super) async fn wire(
        owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
        admitted: &mut AdmittedSession,
        id: u64,
        kind: u32,
        payload: Vec<u8>,
    ) -> TestResult<Vec<u8>> {
        use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{
            ConnectionEnd, IDLE_LIVENESS, serve_admitted,
        };
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: id,
                command_type: kind,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        client
            .write_all(&u32::try_from(frame.len())?.to_be_bytes())
            .await?;
        client.write_all(&frame).await?;
        // Keep the transport alive while a paced command is buffered. Closing its write half
        // before the result correctly disconnects a real client before the queued step runs.
        let receiving = async move {
            let mut result = None;
            loop {
                let mut prefix = [0; 4];
                match client.read_exact(&mut prefix).await {
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(error) => return Err(error.into()),
                }
                let mut frame = vec![0; u32::from_be_bytes(prefix) as usize];
                client.read_exact(&mut frame).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let command = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        .map_err(debug)?;
                    assert_eq!(command.command_id, id);
                    result = Some(command.payload.to_vec());
                    client.shutdown().await?;
                }
            }
            result.ok_or_else(|| "no command result".into())
        };
        let reader = tokio::spawn(async move {
            let result: TestResult<_> = receiving.await;
            result.map_err(|error| error.to_string())
        });
        let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
        if let ConnectionEnd::AdmittedThenClosed(_, error) = &end {
            eprintln!("quest wire command={id} type={kind} closed={error:?}");
        }
        drop(server);
        let result = reader.await?.map_err(Into::into);
        if let ConnectionEnd::AdmittedThenDisconnected(a)
        | ConnectionEnd::AdmittedThenClosed(a, _) = end
        {
            *admitted = a;
        }
        result
    }
}

#[path = "engnpc2_paint_transport/world.rs"]
mod world;

fn scenario(name: &str, _trigger: Position, lever: bool) -> TestResult {
    assert!(!lever);
    let admin = fixture::configured_admin().ok_or("configured PostgreSQL required")?;
    fixture::runtime()?.block_on(async {
        let h = fixture::Harness::create_with_admission_seed(
            admin,
            &name.replace('-', "_"),
            true,
            false,
        )
        .await?;
        let result = exercise(&h, name).await;
        h.cleanup().await?;
        result
    })
}
async fn exercise(h: &fixture::Harness, name: &str) -> TestResult {
    super::engnpc2_live_transport::fresh(h).await?;
    let scope = fixture::fence(1)?.runtime_scope;
    let world = scope.world_id();
    let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
    let session = GameSessionId::decode(&id(50)).map_err(debug)?;
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            world, None,
        );
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?;
    let callback = cases::callback(name)?;
    let scene = cases::world(native)?;
    let cells = scene
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let room = pin
        .qualified_room()
        .bundle_view(
            cells,
            scene.frame_binding_digest(world),
            scene.map_revision_digest(),
            oteryn_gameplay_content::content::NativeEntryStart {
                x: i32::from(cases::START.x),
                y: i32::from(cases::START.y),
                floor: -i16::from(cases::START.floor),
            },
        )
        .ok_or("qualified brain scene")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    let start = room.entry_start();
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        channel_pin,
    )
    .map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(debug)?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(
                CharacterId::decode(&id(41)).map_err(debug)?,
                current.current_character_lease().generation(),
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let runtime = Mutex::new(runtime);
    let content =
        oteryn_gameplay_world::world_runtime::ReferenceContentGeneration::from_content(room.door())
            .map_err(debug)?;
    let fence = oteryn_gameplay_world::world_runtime::ScopeContentGenerationFence::for_test(
        scope,
        fixture::fence(1)?.scope_ownership_generation,
        content,
    );
    let door = Mutex::new(
        oteryn_gameplay_world::world_runtime::bind_native_entry_door(
            room.door(),
            &fence,
            scope,
            fixture::fence(1)?.scope_ownership_generation,
        )
        .map_err(debug)?,
    );
    let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
        "platform-test".into(),
        ("127.0.0.1".into(), 1),
        "source.test".into(),
        "source.test".into(),
        vec![tokio_rustls::rustls::pki_types::CertificateDer::from(
            include_bytes!("../spell-save-tls-fixture/ca.der").to_vec(),
        )],
        vec![tokio_rustls::rustls::pki_types::CertificateDer::from(
            include_bytes!("../spell-save-tls-fixture/client.der").to_vec(),
        )],
        tokio_rustls::rustls::pki_types::PrivateKeyDer::try_from(
            include_bytes!("../spell-save-tls-fixture/client-key.der").to_vec(),
        )?,
    )
    .map_err(debug)?;
    let evidence = FreshEvidenceSource::new(descriptor);
    let spells = oteryn_gameplay_spell::spell::cast::v1_spell_book().map_err(debug)?;
    let achievements =
        oteryn_gameplay_social::achievement_catalogue::AchievementCatalogue::embedded()
            .map_err(debug)?;
    let charms =
        oteryn_gameplay_content::content::charm_source::CanonicalCharmCatalogue::embedded()
            .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = ComposedTestOwner {
        root: &h.root,
        character: &authority,
        holder: &h.node,
        evidence: &evidence,
        world_id: world,
        channel_id: channel,
        required_capabilities: composed_test_owner::empty_capabilities(world, channel)?,
        runtime: &runtime,
        room: &room,
        door: &door,
        spells: &spells,
        achievements: &achievements,
        imported_charms: &charms,
    }
    .build();
    owner.active_generation = Some(generation);
    owner.rooms = SeamRooms::new(Some(&room), Some(&room));
    let original = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    owner.quest_catalogue = Some(Arc::new(cases::catalogue(
        original.catalogue(),
        &callback,
        name.ends_with("-wrong"),
    )?));
    let fence = composed_test_owner::load_quests(&owner, session).await?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            generation.identity().server_artifact_digest(),
            fresh_fixture::fresh::Source::new(0)?.request()?.operation(),
        )
        .await
        .map_err(debug)?;
    let template = crate::durability::starter_grant::StarterTemplate {
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        ..crate::durability::starter_grant::StarterTemplate::main()
    };
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &composed_test_owner::native_item_weights(native),
            &template,
        )
        .await
        .map_err(debug)?;
    {
        let runtime = runtime.lock().await;
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
                    oteryn_gameplay_spell::spell::cast::CharacterCastFacts {
                        vocation: oteryn_gameplay_spell::spell::Vocation::Monk,
                        level: 8,
                        magic_level: 0,
                        max_health: 185,
                        max_mana: 90,
                        max_soul: 100,
                    },
                    (3, 7_000_000),
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(u64::try_from(
                        owner.clock_origin.elapsed().as_micros()
                    )?)
                )
                .is_some()
        );
    }
    let mut admitted = AdmittedSession {
        game_session_id: session,
        world_id: world,
        channel_id: channel,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16])
                .map_err(debug)?,
            account_id: id(40),
        }),
        continuity: SessionContinuity::FRESH,
        item_fence: Some(fence.item_fence()),
    };
    cases::exercise(&mut owner, &mut admitted, h, fence, &callback, name).await
}
