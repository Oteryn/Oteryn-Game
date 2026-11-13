//! Three source-native seals through the actual carried World-tool caller.
use crate::bestiary_postgres_harness as fixture;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId, NodeId,
};
use fixture::{debug, id};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{
    self, ComposedTestOwner, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_dispatch::gameplay_transport::{ComposedFreshAdmission, FreshEvidenceSource};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, decode_use_intent_target, encode_item_use_intent,
};
use std::sync::Arc;
use tokio::sync::Mutex;
type TestResult<T = ()> = fixture::TestResult<T>;
#[path = "engnpc2_paint_transport/world.rs"]
mod world;

#[test]
fn engnpc2_paint_postgres_native_three_seals_through_caller() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(128 << 20).spawn(move || -> Result<(),String> {
        fixture::runtime().map_err(debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = fixture::Harness::create_with_admission_seed(admin, "engnpc2_paint_caller",true,false).await?;
            let result=exercise(&h).await;
            h.cleanup().await?;result?;
            assert!(started.elapsed().as_secs_f64()>0.01);
            eprintln!("PG_EXECUTED engnpc2_paint_postgres_native_three_seals_through_caller duration={:?}",started.elapsed());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(debug)
    })?.join().map_err(|_| "paint caller PG thread failed")?.map_err(Into::into)
}
#[test]
fn engnpc2_ram_postgres_native_turnip_through_caller() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(128 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = fixture::Harness::create_with_admission_seed(admin, "engnpc2_ram_caller", true, false).await?;
            let result = exercise_case(&h, true).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("PG_EXECUTED engnpc2_ram_postgres_native_turnip_through_caller duration={:?}", started.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(debug)
    })?.join().map_err(|_| "ram caller PG thread failed")?.map_err(Into::into)
}
async fn exercise(h: &fixture::Harness) -> TestResult {
    exercise_case(h, false).await
}
async fn exercise_case(h: &fixture::Harness, ram: bool) -> TestResult {
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
    let (source, region, bounds, start) = if ram {
        (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../content/world/placements/region-z07-x124-y124.b3"
            ))
            .as_slice(),
            (7, 124, 124),
            (31926, 31895, 31928, 31897),
            (31927, 31895),
        )
    } else {
        (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../content/world/placements/region-z07-x125-y121.b3"
            ))
            .as_slice(),
            (7, 125, 121),
            (32000, 31164, 32003, 31168),
            (32000, 31165),
        )
    };
    let scene = world::native_scene(
        native,
        vec![world::NativeCrop {
            bytes: source,
            region,
            bounds,
            start,
            expected: if ram {
                vec![world::ExpectedTile {
                    position: (31926, 31896),
                    ids: vec![4515, 5485, 53439],
                    counted_entry: None,
                }]
            } else {
                vec![]
            },
        }],
        &[],
        &[7252],
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
    if ram {
        use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;
        let oteryn_gameplay_content::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) = room.movement_cells().index() else { return Err("native bundle index".into()); };
        return exercise_ram(h, native, &mut owner, index.map(), actor, session).await;
    }
    use oteryn_gameplay_quests::quest::{
        QuestStateCatalogue, QuestTrack, ice_islands_seals as seals, world_callbacks,
    };
    let callbacks = [9, 10, 11]
        .into_iter()
        .map(seals::callbacks::definition)
        .collect::<Result<Vec<_>, _>>()
        .map_err(debug)?;
    let tracks = seals::TRACKS
        .into_iter()
        .zip([(8, 0, 9), (0, 0, (1_i64 << 48) - 1), (1, 0, 2)])
        .map(|(key, (initial, min, max))| QuestTrack {
            key: key.into(),
            quest: seals::QUEST.into(),
            initial,
            min,
            max,
        })
        .collect();
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            tracks,
            callbacks
                .iter()
                .map(world_callbacks::primary_transition)
                .collect::<Result<Vec<_>, _>>()
                .map_err(debug)?,
        )
        .map_err(debug)?,
    ));
    super::engnpc2_live_transport::seed_tool(h, native, seals::PAINT).await?;
    use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;
    let oteryn_gameplay_content::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
        index,
    ) = room.movement_cells().index()
    else {
        return Err("native bundle index".into());
    };
    let map = index.map();
    let definition = native
        .item_index()
        .ok_or("native index")?
        .definition_ref("Item", seals::PAINT, "definition-r1")
        .ok_or("paint reference")?;
    for (i, seal) in [9, 10, 11].into_iter().enumerate() {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock advance")?;
        let target = seals::targets::target(seal).ok_or("seal target")?;
        let (x, y, z) = target.position;
        let key = oteryn_world_bundle::bundle::placement_key(z, x, y, target.ordinal)
            .ok_or("seal key")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let intent = UseIntent {
            target: UseTarget::ItemByDefinition(definition),
            use_with: None,
            use_with_map_item: Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(
                oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                    placement,
                    expected_revision: map.facts.object_revision(key),
                },
            )),
            use_with_position: None,
        };
        let Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(target)) =
            intent.use_with_map_item.as_ref()
        else {
            return Err("native map object target".into());
        };
        let selected = composed_test_owner::world_tool_callbacks(&owner, target)?;
        assert_eq!(
            selected.len(),
            1,
            "seal {seal} must select exactly its own callback"
        );
        assert_eq!(
            selected.first().map(String::as_str),
            callbacks.get(i).map(|c| c.key.as_str())
        );
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let intent = decode_use_intent_target(&[4, 6, 11, 15, 17], &payload).map_err(debug)?;
        let fence = composed_test_owner::load_quests(&owner, session).await?;
        let result = owner
            .tame_mount(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 14000 + i as u64,
                    item_fence: Some(fence.item_fence()),
                },
                intent,
                None,
                None,
                false,
            )
            .await
            .map_err(|_| "paint caller failed")?
            .ok_or_else(|| format!("native paint caller fell through for seal {seal}"))?;
        assert_eq!(result.disposition, UseDisposition::Committed, "seal {seal}");
        let quantity:i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(92).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(quantity, if i < 2 { 1 } else { 0 });
    }
    let states: Vec<(String, i64)> =
        sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks")
            .fetch_all(&h.pool)
            .await?;
    assert!(states.contains(&(seals::QUESTLINE.into(), 9)));
    assert!(states.contains(&(seals::MISSION04.into(), 2)));
    assert_eq!(h.count("game_character_quest_receipts").await?, 3);
    Ok(())
}

async fn exercise_ram(
    h: &fixture::Harness,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    map: &oteryn_gameplay_world::map::boot::BundleMap,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
) -> TestResult {
    use oteryn_gameplay_quests::quest::{
        QuestStateCatalogue, QuestTrack, targuna_ram as ram, world_callbacks,
    };
    let callback = ram::callback().map_err(debug)?;
    let tracks = [(ram::TURNIPS, 1, 1), (ram::MISSION, 4, 5), (ram::RAM, 0, 1)]
        .into_iter()
        .map(|(key, initial, max)| QuestTrack {
            key: key.into(),
            quest: ram::QUEST.into(),
            initial,
            min: 0,
            max,
        })
        .collect();
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            tracks,
            vec![world_callbacks::primary_transition(&callback).map_err(debug)?],
        )
        .map_err(debug)?,
    ));
    super::engnpc2_live_transport::seed_tool(h, native, "oteryn:item.tibia.i53073").await?;
    let key = oteryn_world_bundle::bundle::placement_key(-7, 31926, 31896, 2).ok_or("ram key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = oteryn_protocol_oteryn::world_object::WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    assert_eq!(
        composed_test_owner::world_tool_callbacks(owner, &target)?,
        vec![ram::KEY]
    );
    let definition = native
        .item_index()
        .ok_or("native index")?
        .definition_ref("Item", "oteryn:item.tibia.i53073", "definition-r1")
        .ok_or("turnip definition")?;
    let intent = UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(
            target,
        )),
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent).map_err(debug)?;
    let intent = decode_use_intent_target(&[4, 6, 11, 15, 17], &payload).map_err(debug)?;
    let fence = composed_test_owner::load_quests(owner, session).await?;
    let result = owner
        .tame_mount(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: 15000,
                item_fence: Some(fence.item_fence()),
            },
            intent,
            None,
            None,
            false,
        )
        .await
        .map_err(|_| "ram caller failed")?
        .ok_or("ram caller fell through")?;
    assert_eq!(result.disposition, UseDisposition::Committed);
    let state: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid), (SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:content.mount.reliable_ram/quest-grant'), (SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/a_reliable_friend')").bind(id(92).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(state, (0, 1, 1));
    let states: Vec<(String, i64)> =
        sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks")
            .fetch_all(&h.pool)
            .await?;
    assert!(states.contains(&(ram::MISSION.into(), 5)));
    assert!(states.contains(&(ram::RAM.into(), 1)));
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);
    assert!(
        callback.selector.native.is_some(),
        "Ram must have its exact native map counterpart"
    );
    Ok(())
}
