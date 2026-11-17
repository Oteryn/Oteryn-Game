//! Exact source selectors through the production composed chest USE owner.
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
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;
use tokio::sync::Mutex;
type TestResult<T = ()> = fixture::TestResult<T>;

#[derive(Clone, Copy)]
struct Case {
    tag: &'static str,
    claim: &'static str,
    position: (u16, u16, u8),
    unique: u16,
    appearance: u16,
    item: u16,
}
#[test]
fn qaitems_boots_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    execute(
        admin,
        Case {
            tag: "boots",
            claim: "oteryn:reward-claim.quest.u8_4.the_hidden_city_of_beregar.firewalker_boots",
            position: (32582, 31405, 15),
            unique: 6284,
            appearance: 2472,
            item: 9019,
        },
    )
}
#[test]
fn qaitems_hammer_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    execute(
        admin,
        Case {
            tag: "hammer",
            claim: "oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.hammer",
            position: (32776, 32253, 8),
            unique: 6078,
            appearance: 2472,
            item: 3208,
        },
    )
}
#[test]
fn qaitems_lottery_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    execute(
        admin,
        Case {
            tag: "lottery",
            claim: "oteryn:reward-claim.quest.u8_1.secret_service.lottery_ticket",
            position: (32872, 31958, 11),
            unique: 6250,
            appearance: 2469,
            item: 5956,
        },
    )
}
#[test]
fn qaitems_poem_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    execute(
        admin,
        Case {
            tag: "poem",
            claim: "oteryn:reward-claim.quest.u8_1.secret_service.poem_scroll",
            position: (32643, 32733, 7),
            unique: 6251,
            appearance: 2472,
            item: 347,
        },
    )
}
fn execute(admin: String, case: Case) -> TestResult {
    std::thread::Builder::new().stack_size(128 << 20).spawn(move || -> Result<(),String> {
        fixture::runtime().map_err(debug)?.block_on(async {
            let started = std::time::Instant::now();
            let (controller,pin) = oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(fixture::fence(1)?.runtime_scope.world_id(), None);
            let h = fixture::Harness::create_with_admission_seed(admin, &format!("qaitems_use_{}",case.tag),true,false).await?;
            let result=exercise(&h,case,&controller,&pin).await;
            let cleanup=h.cleanup().await;
            result?;cleanup?;
            assert!(started.elapsed().as_secs_f64()>0.01);
            eprintln!("QAITEMS_USE_EXECUTED {} duration={:?}",case.tag,started.elapsed());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(debug)
    })?.join().map_err(|_| "qaitems USE PG thread failed")?.map_err(Into::into)
}
async fn exercise(
    h: &fixture::Harness,
    case: Case,
    controller: &oteryn_gameplay_content::content::ContentActivationController,
    pin: &oteryn_gameplay_content::content::NativeEntryContentPin,
) -> TestResult {
    super::engnpc2_live_transport::fresh(h).await?;
    let scope = fixture::fence(1)?.runtime_scope;
    let world = scope.world_id();
    let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
    let session = GameSessionId::decode(&id(50)).map_err(debug)?;
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?;
    let (x, y, z) = case.position;
    // Match the existing catalogue harness: preserve source identity in a
    // minimal reachable scene; the merged World does not retain donor UIDs.
    let scene = source_scene(case, world, channel)?;
    let start = (x - 1, y);
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
                floor: i16::from(z),
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
    assert_eq!((at.x, at.y, at.floor), (start.x, start.y, i16::from(z)));
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
    owner.quest_catalogue = Some(Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let fence = composed_test_owner::load_quests(&owner, session)
        .await?
        .item_fence();
    let fresh = composed_test_owner::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence,
            generation.identity().server_artifact_digest(),
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let template = crate::durability::starter_grant::StarterTemplate {
        definition: super::engitem_reward_text::definition(2854),
        ..crate::durability::starter_grant::StarterTemplate::main()
    };
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence,
            &super::engitem_reward_text::Weights(native),
            &template,
        )
        .await
        .map_err(debug)?;
    let report =
        oteryn_gameplay_content::content::world_reward_claims::bind_embedded_reward_claims(
            scene.base().as_ref(),
            &|_| true,
        )?;
    let selected = report
        .served
        .iter()
        .filter(|row| row.key.as_str() == case.claim)
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 1, "exact claim must bind");
    assert_eq!(selected[0].placements.len(), 1);
    let bound = &selected[0].placements[0];
    assert_eq!(
        (bound.cell.x, bound.cell.y, bound.cell.floor),
        (x, y, -i8::try_from(z)?)
    );
    assert!(bound.contents.is_empty());
    assert_eq!(bound.placement.items.len(), 1);
    assert_eq!(
        bound.placement.items[0].item.key().as_str(),
        format!("oteryn:item.tibia.i{}", case.item)
    );
    assert_eq!(bound.placement.items[0].count, 1);
    if case.item == 5956 {
        assert_eq!(
            bound.root_text.as_deref(),
            Some(
                "Damn you Thais! Damn you TBI! I am no longer an agent or a noble but a black knight. A dark avenger that will have his revenge!"
            )
        );
    } else {
        assert!(bound.root_text.is_none());
    }
    let key = bound.bundle_placement_key;
    use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;
    let oteryn_gameplay_content::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
        index,
    ) = room.movement_cells().index()
    else {
        return Err("bundle scene".into());
    };
    let map = index.map();
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let fence = composed_test_owner::load_quests(&owner, session)
        .await?
        .item_fence();
    let command = UseCommand {
        game_session_id: session,
        command_id: 19600,
        item_fence: Some(fence),
    };
    assert_eq!(
        owner
            .use_object(actor, command, target.clone())
            .await
            .disposition,
        UseDisposition::Committed
    );
    let inventory:Vec<String>=sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_instances i JOIN game_reward_claim_mint_receipts m USING(item_instance_id) JOIN game_item_container_entries e USING(item_instance_id) WHERE m.claim_production_key=$1 AND e.character_id=m.character_id").bind(case.claim).fetch_all(&h.pool).await?;
    assert_eq!(inventory, vec![format!("oteryn:item.tibia.i{}", case.item)]);
    if case.item == 5956 {
        let stored: (String,bool,bool) = sqlx::query_as("SELECT t.text,t.writer_character_id IS NULL,t.text=r.root_text AND r.root_text=v.root_text FROM game_item_texts t JOIN game_reward_claim_mint_receipts r USING(item_instance_id) JOIN game_reward_claim_mint_reservations v ON v.transaction_id=r.transaction_id WHERE r.claim_production_key=$1").bind(case.claim).fetch_one(&h.pool).await?;
        assert_eq!(
            stored,
            (bound.root_text.clone().ok_or("lottery text")?, true, true)
        );
        assert_eq!(h.count("game_item_text_write_receipts").await?, 0);
    }
    // The same durable command must replay without a second item or receipt.
    assert_eq!(
        owner
            .use_object(actor, command, target.clone())
            .await
            .disposition,
        UseDisposition::Committed
    );
    let next = UseCommand {
        command_id: 19601,
        ..command
    };
    assert_eq!(
        owner.use_object(actor, next, target).await.disposition,
        UseDisposition::Rejected
    );
    assert_eq!(h.count("game_reward_claim_mint_receipts").await?, 1);
    Ok(())
}

fn source_scene(
    case: Case,
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<oteryn_gameplay_world::map::boot::BundleWorld> {
    use oteryn_gameplay_world::map;
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.palette.push(bundle::PaletteEntry {
        key: format!("oteryn:terrain.tibia.i{}", case.appearance),
        family: bundle::Family::Terrain,
        id: 1,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
            blocks_projectile: None,
        }),
    });
    let (x, y, z) = case.position;
    let floor = -i8::try_from(z)?;
    data.manifest.world = bundle::Extent {
        min_x: x - 2,
        min_y: y,
        max_x: x + 1,
        max_y: y + 1,
        floors: vec![floor],
    };
    let mut grouped = std::collections::BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for tx in x - 2..=x {
        let mut items = vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: Default::default(),
        }];
        if tx == x {
            items.push(sector::Item {
                palette: 1,
                depth: 0,
                attrs: sector::Attrs {
                    unique: Some(case.unique),
                    ..Default::default()
                },
            });
        }
        grouped
            .entry((y / 32, tx / 32))
            .or_default()
            .push(sector::Tile {
                x: tx,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
    }
    let sectors = grouped
        .into_iter()
        .map(|((sy, sx), tiles)| bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = map::load(&bytes, &pins).map_err(debug)?;
    map::boot::boot(
        &bytes,
        &map::boot::BootPins {
            map_revision: map::overlay::map_revision(&base),
            bundle: pins,
            start: map::overlay::TilePos { x: x - 1, y, floor },
        },
        world,
        channel,
        |_| None,
    )
    .map_err(|e| debug(e).into())
}
