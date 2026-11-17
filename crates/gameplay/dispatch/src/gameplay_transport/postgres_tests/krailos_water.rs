//! Exercise the authored offer and both lake tools through normal NPC and USE dispatch.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, id};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::map::{
    overlay::TilePos,
    view::{self, MapEntryKey, MapFacts},
};
use oteryn_gameplay_quests::quest::krailos_lake::START;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[test]
fn krailos_water_npc_and_use_dispatch_observe_the_world_save_and_water_phases() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Krailos water scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED krailos_water duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario(admin: String) -> TestResult {
    let runtime = crate::bestiary_postgres_harness::runtime()?;
    let h = runtime.block_on(Harness::create_with_admission_seed(
        admin,
        "krailos_water",
        false,
        false,
    ))?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(exercise(&h))
    }));
    let cleanup = runtime.block_on(h.cleanup());
    match result {
        Ok(result) => {
            result?;
            cleanup
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

async fn exercise(h: &Harness) -> TestResult {
    super::reset();
    let fresh = prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = super::fixture::fence(1)?.runtime_scope.world_id();
    let (controller, activated) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("production content")?;
    let native = active.native_gameplay().ok_or("native content")?;
    let map = native_lake(native)?;
    let cells = map
        .movement_cells(activated.qualified_room().movement_cells())
        .map_err(debug)?;
    let server = active.identity().server_artifact_digest();
    let client = active.identity().client_artifact_digest();
    let frame = map.frame_binding_digest(world);
    let start = crate::content::NativeEntryStart {
        x: 33642,
        y: 31645,
        floor: 7,
    };
    let room = activated
        .qualified_room()
        .bundle_view(cells, frame, map.map_revision_digest(), start)
        .ok_or("qualified lake crop")?;
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        server,
        client,
        frame,
        map.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let deps = Dependencies::with_content_and_capacity(pin, room, native.spell_book().clone(), 2)?;
    deps.runtime
        .lock()
        .await
        .initialize_pinned_test_position(
            deps.actor,
            crate::foundation::MovementLocalPosition {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&deps.room), Some(&deps.room));
    owner.active_generation = Some(active);
    owner.quest_catalogue = Some(std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    admitted.item_fence = Some(fence.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(server.as_slice())
        .bind(client.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            server,
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let backpack = crate::durability::starter_grant::StarterTemplate {
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
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            &backpack,
        )
        .await
        .map_err(debug)?;
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_quest_session(&admitted).await;
    deps.initialize(&owner).await;
    let [ghorza] = install(&owner, &admitted, ["oteryn:npc.ghorza"]).await?;
    sqlx::query("INSERT INTO game_krailos_lake VALUES (encode($1,'hex')::uuid,0,6)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;

    talk(&mut owner, &admitted, ghorza, 100, "hi").await?;
    talk(&mut owner, &admitted, ghorza, 101, "water").await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("NPC clock")?;
    let denied = owner
        .npc_talk(
            deps.actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 102,
                item_fence: admitted.item_fence,
            },
            NpcTalkIntent {
                npc_actor: ghorza,
                text: "yes".into(),
            },
        )
        .await;
    assert_ne!(denied.disposition, NpcIntentDisposition::Ok);
    assert!(!denied.inventory_changed);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(START)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 0);

    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,3,$2,'krailos-fixture',$2,$3,$4,'TIBIA_SERVER_SAVE_BERLIN_06',1,TIMESTAMPTZ '2030-02-04 06:00 Europe/Berlin','2030-02')")
        .bind(world.as_bytes().as_slice()).bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES (encode($1,'hex')::uuid,3)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    talk(&mut owner, &admitted, ghorza, 103, "bye").await?;
    talk(&mut owner, &admitted, ghorza, 104, "hi").await?;
    talk(&mut owner, &admitted, ghorza, 105, "water").await?;
    let accepted = talk(&mut owner, &admitted, ghorza, 106, "yes").await?;
    assert!(accepted.iter().any(|line| line == "Here you are."));
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(START)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 1);
    let progress: (String, i16) =
        sqlx::query_as("SELECT reset_epoch::text,progress FROM game_krailos_lake")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(progress, ("3".into(), 0));

    owner.reset_epoch = 3;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("USE capabilities")?;
    // Prepare the boundary with five captured souls and five prior World deliveries.
    // Keep the catalogue and the following item, World and map writers production-owned.
    seed_filled_carafe(h).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    sqlx::query("UPDATE game_krailos_lake SET reset_epoch=2,progress=5")
        .execute(&h.pool)
        .await?;
    let pour = TilePos {
        x: 33642,
        y: 31645,
        floor: -7,
    };
    let purify = TilePos {
        x: 33643,
        y: 31645,
        floor: -7,
    };
    let carafe = "oteryn:item.tibia.i22692";
    let powder = "oteryn:item.tibia.i22708";
    let item_fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?
        .item_fence();
    super::quest_world_callbacks::input_item(
        &owner,
        item_fence,
        &Offering {
            position: Position {
                x: purify.x,
                y: purify.y,
                floor: purify.floor,
            },
            item: powder.into(),
            count: 1,
        },
        951,
        true,
    )
    .await?;
    assert_eq!(
        use_tool(&mut owner, &mut admitted, purify, powder).await?,
        UseDisposition::Rejected,
    );
    assert_eq!(
        use_tool(&mut owner, &mut admitted, pour, carafe).await?,
        UseDisposition::Committed
    );
    let progress: (String, i16) =
        sqlx::query_as("SELECT reset_epoch::text,progress FROM game_krailos_lake")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(progress, ("3".into(), 1));

    seed_filled_carafe(h).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    sqlx::query("UPDATE game_krailos_lake SET progress=5")
        .execute(&h.pool)
        .await?;
    assert_eq!(
        use_tool(&mut owner, &mut admitted, pour, carafe).await?,
        UseDisposition::Committed
    );
    assert_eq!(appearance_at(&owner, purify)?, 12561);

    sqlx::query("UPDATE game_krailos_lake SET progress=35")
        .execute(&h.pool)
        .await?;
    assert_eq!(
        use_tool(&mut owner, &mut admitted, purify, powder).await?,
        UseDisposition::Committed
    );
    assert_eq!(appearance_at(&owner, purify)?, 21312);
    let progress: i16 = sqlx::query_scalar("SELECT progress FROM game_krailos_lake")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(progress, 36);
    sqlx::query(
        "INSERT INTO game_world_reset_epochs
         SELECT world_id,6,target_digest,map_revision,server_digest,client_digest,frame_digest,
                calendar,calendar_revision,activated_at+interval '3 days',reset_month
         FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=3",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=6 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    owner.reset_epoch = 6;
    owner.refresh_terrain_tools().await;
    assert_eq!(appearance_at(&owner, pour)?, 22420);
    assert_eq!(appearance_at(&owner, purify)?, 22414);
    Ok(())
}

async fn seed_filled_carafe(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let seeded = sqlx::query(
        "INSERT INTO game_character_quest_tracks
         SELECT character_id,'oteryn:quest-progress/krailos-water/souls',
                'oteryn:quest.krailos_quest',5,character_revision
         FROM game_character_roots
         WHERE character_id=encode($1,'hex')::uuid
         ON CONFLICT(character_id,track_key) DO UPDATE
         SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision",
    )
    .bind(id(41).as_slice()).execute(&mut *tx).await?;
    assert_eq!(seeded.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}

fn native_lake(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x131-y123.b3"
            )),
            region: (7, 131, 123),
            bounds: (33640, 31644, 33644, 31646),
            start: (33642, 31645),
            expected: vec![
                ExpectedTile {
                    position: (33642, 31645),
                    ids: vec![22420],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33643, 31645),
                    ids: vec![22414, 22444, 1779],
                    counted_entry: None,
                },
            ],
        }],
        &[],
        &[12561, 21312],
    )
}

fn appearance_at(owner: &ComposedFreshAdmission<'_, '_, '_>, pos: TilePos) -> TestResult<u16> {
    let map = owner.bundle_map().ok_or("map")?;
    let key = view::placement_key(pos, 0).ok_or("ground placement")?;
    Ok(view::compose_tile(&map.overlay, &map.facts.pinned(), pos)
        .map_err(debug)?
        .ok_or("tile")?
        .entries
        .into_iter()
        .find(|entry| entry.source.key() == MapEntryKey::Base(key))
        .ok_or("ground entry")?
        .facts
        .appearance_id)
}

async fn use_tool(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: TilePos,
    tool: &str,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("USE clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = view::placement_key(target, 0).ok_or("ground placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", tool, "definition-r1"))
        .ok_or("tool definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, result, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(reference),
    )
    .await?;
    *admitted = next;
    Ok(result)
}
