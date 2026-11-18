//! Crystal Mind potion through the ordinary carried USE owner and PostgreSQL.
use super::TestResult;
use super::fixture::{Harness, debug, id, run};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::{
    SeamRooms,
    connection::{FreshAdmissionAuthority, UseCommand},
    item_view::ItemKey,
};
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn kilmaresh_mind_use_requires_equipped_ring_and_burns_one_potion_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| exercise().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Kilmaresh Mind case panicked")??;
    Ok(())
}

fn exercise() -> TestResult {
    run(async |admin| {
        let start = MovementLocalPosition {
            x: 33865,
            y: 31533,
            floor: 7,
        };
        let scene = super::srx3_soul_war_world_fixture::source_world(start)?;
        let deps = scene.dependencies()?;
        let h = Harness::create_with_admission_seed(admin, "kilmaresh_mind", false, false).await?;
        super::house_transfer::prepare_owned_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = scene.controller.active();
        owner.rooms = SeamRooms::new(None, Some(&scene.room));
        super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
        let mut admitted = deps.admitted;
        super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 250).await?;
        super::item_look::seed_main_backpack(
            &h,
            &[
                ("oteryn:item.tibia.i31350", 1),
                ("oteryn:item.tibia.i31306", 1),
            ],
        )
        .await?;
        let actor = admitted.runtime_actor.ok_or("player")?;
        let use_command = |command_id| UseCommand {
            game_session_id: admitted.game_session_id,
            command_id,
            item_fence: admitted.item_fence,
        };
        let source = ItemKey::Instance(id(71));
        let refused = owner
            .use_item_completion(actor, use_command(1000), source)
            .await
            .ok_or("potion refusal")?;
        assert_eq!(refused.disposition, UseDisposition::NotPossible);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            start
        );
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(id(72).as_slice())
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,8,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(72).as_slice()).bind(id(42).as_slice()).bind(id(76).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let used = owner
            .use_item_completion(actor, use_command(1001), source)
            .await
            .ok_or("potion result")?;
        assert_eq!(used.disposition, UseDisposition::Committed);
        let landed = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        assert!(matches!(
            (landed.x, landed.y, landed.floor),
            (33913, 31653, 9) | (33921, 31650, 9)
        ));
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i31350' AND lifecycle=1").fetch_one(&h.pool).await?;
        assert_eq!(count, 0);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records WHERE recipe='oteryn:kilmaresh.mind-potion'").fetch_one(&h.pool).await?;
        assert_eq!(receipts, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn kilmaresh_mind_step_removes_one_possessed_shard_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| field_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Kilmaresh Mind field case panicked")??;
    Ok(())
}

fn field_scenario() -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
    run(async |admin| {
        let start = MovementLocalPosition {
            x: 33865,
            y: 31533,
            floor: 7,
        };
        let mut scene = super::srx3_soul_war_world_fixture::source_world(start)?;
        let world = scene.pin.world_id();
        let map = field_world(world)?;
        let room = scene
            .room
            .bundle_view(
                map.movement_cells(scene.room.movement_cells())
                    .map_err(debug)?,
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: start.x,
                    y: start.y,
                    floor: start.floor,
                },
            )
            .ok_or("energy field fixture room")?;
        scene.room = room;
        scene.pin = crate::foundation::ChannelContentPin::from_activation(
            world,
            scene.pin.activation_sequence(),
            scene.pin.server_artifact_digest(),
            scene.pin.client_artifact_digest(),
            scene.room.frame_binding().digest(),
            scene.room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let deps = scene.dependencies()?;
        let h = Harness::create_with_admission_seed(admin, "kilmaresh_field", false, false).await?;
        super::house_transfer::prepare_owned_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = scene.controller.active();
        owner.rooms = SeamRooms::new(None, Some(&scene.room));
        super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
        let mut admitted = deps.admitted;
        super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 250).await?;
        super::item_look::seed_main_backpack(
            &h,
            &[
                ("oteryn:item.tibia.i31354", 1),
                ("oteryn:item.tibia.i31355", 1),
            ],
        )
        .await?;
        let actor = admitted.runtime_actor.ok_or("field player")?;
        let (step, _) = owner
            .paced_step(actor, admitted.game_session_id, StepDirection::East)
            .await;
        assert_eq!(step.disposition, StepDisposition::Moved);
        let remaining: i64 = sqlx::query_scalar(
            "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances
             WHERE definition_production_key IN ('oteryn:item.tibia.i31354','oteryn:item.tibia.i31355') AND lifecycle=1"
        ).fetch_one(&h.pool).await?;
        assert_eq!(remaining, 1);
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_consume_transform_records WHERE recipe='oteryn:kilmaresh.mind-field' AND source_count=1"
        ).fetch_one(&h.pool).await?;
        assert_eq!(receipts, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn field_world(world: crate::foundation::WorldId) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::{boot, overlay::TilePos};
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.world = bundle::Extent {
        min_x: 33865,
        min_y: 31533,
        max_x: 33867,
        max_y: 31534,
        floors: vec![-7],
    };
    let start = TilePos {
        x: 33865,
        y: 31533,
        floor: -7,
    };
    let tiles = [start.x, start.x + 1]
        .into_iter()
        .map(|x| sector::Tile {
            x,
            y: start.y,
            flags: 0,
            house: 0,
            zones: Vec::new(),
            items: vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: sector::Attrs {
                    action: (x != start.x).then_some(40004),
                    ..Default::default()
                },
            }],
        })
        .collect();
    let sectors = [bundle::Sector {
        floor: start.floor,
        sx: start.x / 32,
        sy: start.y / 32,
        tiles,
    }];
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start,
        },
        world,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(Into::into)
}
