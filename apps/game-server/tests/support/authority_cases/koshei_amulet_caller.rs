//! KOSHEI: real framed USE on current Ground handles through the composed owner.
use crate::bestiary_postgres_harness as fixture;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId, NodeId,
};
use fixture::{debug, id};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{
    self, AdmittedSession, ComposedTestOwner, ConnectionEnd, ControllerBinding, FirstEntryOutcome,
    FreshAdmissionAuthority, IDLE_LIVENESS, SelectedCapabilities, SessionContinuity,
    serve_admitted,
};
use oteryn_gameplay_dispatch::gameplay_transport::{
    ComposedFreshAdmission, FreshEvidenceSource, SeamRooms,
};
use oteryn_gameplay_quests::quest::{koshei_amulet as source, world_callbacks};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, decode_use_intent_target,
    encode_item_use_intent,
};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
type TestResult<T = ()> = fixture::TestResult<T>;
#[path = "engnpc2_paint_transport/world.rs"]
mod world;

#[test]
fn koshei_definition_validator_keeps_appearance_only_closed() -> TestResult {
    let valid = source::callback()?;
    world_callbacks::validate_callbacks(std::slice::from_ref(&valid))?;
    let mut bad = valid;
    bad.selector.target_definition = None;
    bad.selector.appearance = Some(7535);
    assert!(world_callbacks::validate_callbacks(&[bad]).is_err());
    Ok(())
}

#[test]
fn koshei_current_ground_handles_real_use_with_refusals_door_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create_with_admission_seed(
                        admin,
                        "koshei_wire",
                        true,
                        false,
                    )
                    .await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "KOSHEI_AUTHORITY_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Koshei wire thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> TestResult {
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
    let source = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/world/placements/region-z07-x124-y124.b3"
    ))
    .as_slice();
    let (region, bounds, start) = ((7, 124, 124), (31926, 31895, 31928, 31897), (31927, 31895));
    let scene = world::native_scene(
        native,
        vec![world::NativeCrop {
            bytes: source,
            region,
            bounds,
            start,
            expected: vec![world::ExpectedTile {
                position: (31926, 31896),
                ids: vec![4515, 5485, 53439],
                counted_entry: None,
            }],
        }],
        &[],
        &[7535, 7537],
        world,
        channel,
    )?;
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
                x: i32::from(start.0),
                y: i32::from(start.1),
                floor: 7,
            },
        )
        .ok_or("qualified native seal scene")?;
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
        12,
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
    let at = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!((at.x, at.y, at.floor), (start.x, start.y, 7));
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
    let mut owner: ComposedFreshAdmission<'_, '_, '_> = ComposedTestOwner {
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
    owner.rooms = SeamRooms::new(None, Some(&room));
    owner.quest_catalogue = Some(Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let fence = composed_test_owner::load_quests(&owner, session).await?;
    super::engnpc2_live_transport::seed_tool(h, native, "oteryn:item.tibia.i7532").await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3031','definition-r1',1,1,encode($3,'hex')::uuid)").bind(id(93).as_slice()).bind(world.as_bytes().as_slice()).bind(id(103).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,encode($5,'hex')::uuid)").bind(id(41).as_slice()).bind(id(91).as_slice()).bind(id(93).as_slice()).bind(world.as_bytes().as_slice()).bind(id(113).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;
    let oteryn_gameplay_content::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
        index,
    ) = room.movement_cells().index()
    else {
        return Err("bundle scene".into());
    };
    let map = index.map();
    let target = crate::foundation::MovementLocalPosition {
        x: 31926,
        y: 31896,
        floor: 7,
    };
    for (n, key) in [
        (903, "oteryn:item.tibia.i7535"),
        (904, "oteryn:item.tibia.i7537"),
    ] {
        ground_item(&owner, map, fence.item_fence(), target, key, n).await?;
    }
    let index = native.item_index().ok_or("index")?;
    let reference = |id| {
        index
            .definition_ref("Item", &format!("oteryn:item.tibia.i{id}"), "definition-r1")
            .ok_or("definition")
    };
    let (amulet, gold, corpse, other) = (
        reference(7532)?,
        reference(3031)?,
        reference(7535)?,
        reference(7537)?,
    );
    let before:serde_json::Value=sqlx::query_scalar("SELECT jsonb_agg(jsonb_build_object('i',to_jsonb(i),'g',to_jsonb(g)) ORDER BY i.item_instance_id) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key IN ('oteryn:item.tibia.i7535','oteryn:item.tibia.i7537')").fetch_one(&h.pool).await?;
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
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
            .ok_or("capabilities")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Item(std::num::NonZeroU64::MIN)),
    })
    .map_err(debug)?;
    for (tool, target) in [(amulet, other), (gold, corpse)] {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let (next, result, _) = wire(
            &owner,
            admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(tool),
            Some(target),
        )
        .await?;
        admitted = next;
        assert_ne!(result, UseDisposition::Committed);
        assert_eq!(snapshot(h).await?, (0, 1, 0, 1));
        assert!(
            !h.root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .meets_door_storage(41351)
        );
    }
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let command = admitted.continuity.next_command_id;
    let (next, result, actual) = wire(
        &owner,
        admitted,
        command,
        &payload,
        Some(amulet),
        Some(corpse),
    )
    .await?;
    admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(snapshot(h).await?, (1, 0, 1, 1));
    let copy = h
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(copy.tracks().get(source::TRACK), Some(&1));
    assert!(copy.meets_door_storage(41351));
    let (_, replay, _) = wire(&owner, admitted, command, &actual, None, None).await?;
    assert_eq!(replay, UseDisposition::Committed);
    assert_eq!(snapshot(h).await?, (1, 0, 1, 1));
    let after:serde_json::Value=sqlx::query_scalar("SELECT jsonb_agg(jsonb_build_object('i',to_jsonb(i),'g',to_jsonb(g)) ORDER BY i.item_instance_id) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key IN ('oteryn:item.tibia.i7535','oteryn:item.tibia.i7537')").fetch_one(&h.pool).await?;
    assert_eq!(before, after);
    assert_eq!(
        map.facts
            .movement_effects_since(0)
            .iter()
            .filter(|(_, at, effect)| at.x == 31926
                && at.y == 31896
                && at.floor == -7
                && *effect == 14)
            .count(),
        1
    );
    Ok(())
}

async fn snapshot(h: &fixture::Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE track_key=$1 AND value=1), (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i7532' AND lifecycle=1), (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$2), (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3031' AND lifecycle=1)").bind(source::TRACK).bind(source::KEY).fetch_one(&h.pool).await?)
}

async fn ground_item(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    map: &oteryn_gameplay_world::map::boot::BundleMap,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    at: crate::foundation::MovementLocalPosition,
    item: &str,
    n: u64,
) -> TestResult {
    use crate::durability::item_mint::{
        GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
    };
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let mut mint = owner
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
                item: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: item.into(),
                    revision_ref: "definition-r1".into(),
                },
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            u16::try_from(at.x)?,
                            u16::try_from(at.y)?,
                            -i8::try_from(at.floor)?,
                        ),
                    corpse_ref: id(3).to_vec(),
                    map_revision: oteryn_gameplay_world::map::overlay::map_revision(
                        map.overlay.base(),
                    ),
                    content_revision: revision,
                    native_room_placement_context: frame.to_vec(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = owner
        .spell_lane
        .acquire()
        .await
        .map_err(|_| "spell lane unresolved".to_owned())?;
    assert!(matches!(
        owner
            .root
            .commit_item_mint(&permit, owner.character, owner.holder, &mut mint)
            .await
            .map_err(debug)?,
        ItemMintOutcome::Committed(_)
    ));
    Ok(())
}

async fn wire(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    payload: &[u8],
    source: Option<std::num::NonZeroU32>,
    target: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_side = async {
        let (mut source_handle, mut target_handle) = (None, None);
        loop {
            let bytes = read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&bytes)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                    {
                        let inventory =
                            oteryn_protocol_oteryn::item_view::decode_character_inventory(
                                domain.payload,
                            )
                            .map_err(debug)?;
                        source_handle = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == source)
                            .map(|entry| entry.handle)
                            .or(source_handle);
                    }
                    if domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW
                    {
                        use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
                        let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        target_handle =
                            map.tiles
                                .iter()
                                .flat_map(|tile| &tile.items)
                                .find_map(|item| match (item.definition, item.origin) {
                                    (
                                        MapDefinition::Item(definition),
                                        MapOrigin::Handle(handle),
                                    ) if Some(definition) == target => Some(handle),
                                    _ => None,
                                })
                                .or(target_handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let actual = if source.is_some() || target.is_some() {
            let mut intent = decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                payload,
            )
            .map_err(debug)?;
            if source.is_some() {
                intent.target = UseTarget::Item(source_handle.ok_or("current source handle")?);
            }
            if target.is_some() {
                intent.use_with_map_item = Some(MapUseTarget::Item(
                    target_handle.ok_or("current Ground handle")?,
                ));
            }
            encode_item_use_intent(&intent).map_err(debug)?
        } else {
            payload.to_vec()
        };
        let bytes = oteryn_protocol_oteryn::encode_client_command(
            admitted.continuity.connection_generation,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: command,
                command_type: 2,
                payload: &actual,
            },
        )?;
        client.write_u32(u32::try_from(bytes.len())?).await?;
        client.write_all(&bytes).await?;
        client.shutdown().await?;
        loop {
            let bytes = read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&bytes)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command {
                    return Ok::<_, Box<dyn std::error::Error>>((
                        oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                            .map_err(debug)?,
                        actual,
                    ));
                }
            }
        }
    };
    let server_side = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let exchange = async {
        let (result, end) = tokio::join!(client_side, server_side);
        let (result, actual) = result?;
        let ConnectionEnd::AdmittedThenDisconnected(next) = end else {
            return Err(format!("unexpected server end {end:?}").into());
        };
        Ok::<_, Box<dyn std::error::Error>>((next, result, actual))
    };
    tokio::time::timeout(std::time::Duration::from_secs(30),async {tokio::select! {result=exchange=>result,result=maintain_ready(owner)=>{result?;Err("maintenance stopped".into())}}}).await?
}

async fn read_frame(stream: &mut tokio::io::DuplexStream) -> TestResult<Vec<u8>> {
    let length = usize::try_from(stream.read_u32().await?)?;
    if length > 1 << 20 {
        return Err("frame bound".into());
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).await?;
    Ok(bytes)
}

async fn maintain_ready(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    loop {
        if owner.root.has_ready_demand() {
            owner.root.maintain_ready_once().await.map_err(debug)?;
        }
        tokio::task::yield_now().await;
    }
}
