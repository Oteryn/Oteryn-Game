//! Stage-two transition, bridge map projection, replay and event-end restoration.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition, NodeId};
use crate::map::overlay::TilePos;
use crate::map::view::{self, MapEntryKey, MapFacts};
use oteryn_protocol_oteryn::world_map::MapDefinition;

#[test]
fn w05d_cake_bridge_installs_48227_and_restores() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            run(async |admin| {
                let h =
                    Harness::create_with_admission_seed(admin, "cake_bridge", true, false).await?;
                eprintln!("w05dcake-marker: w05d_cake_bridge_installs_48227_and_restores db ready");
                let result = exercise(&h).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Cake bridge thread panicked")??;
    Ok(())
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            deps.admitted.world_id,
            None,
        );
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let index = native.item_index().ok_or("Item index")?;
    // Crystal a_piece_of_cake_config.lua:249-255. The two outer columns are rails.
    let position = |x, y| TilePos { x, y, floor: -7 };
    let start = position(33303, 32060);
    let world = super::source_world_crop::world(
        position(33300, 32052),
        position(33306, 32060),
        start,
        native,
    )?;
    let local_start = MovementLocalPosition {
        x: i32::from(start.x),
        y: i32::from(start.y),
        floor: 7,
    };
    let cells = world
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let room = super::rust_lottery_fixtures::bundle_action_room(
        pin.qualified_room(),
        &world,
        deps.admitted.world_id,
        &cells,
    )?;
    let content_pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (local_start.x, local_start.y, local_start.floor),
    );
    deps.runtime = tokio::sync::Mutex::new(
        ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            content_pin,
        )
        .map_err(debug)?,
    );
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells =
        crate::gameplay_transport::activity_runtime::SharedOwner::Borrowed(&cells);
    owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
    owner.reset_epoch = 1;
    let map = owner.bundle_map().ok_or("bundle map")?;
    let bundle = map.overlay.base().digest();
    let world_id = owner.world_id.as_bytes().as_slice();
    let channel_id = owner.channel_id.as_bytes().as_slice();
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world_id).bind(channel_id).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world_id).bind(channel_id).bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice())
        .execute(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,$3,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2026-02-21 05:00:00+00','2026-02')")
        .bind(world_id).bind(bundle.as_slice()).bind(world.map_revision())
        .bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice())
        .bind(room.frame_binding().digest().as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let snapshot = || -> TestResult<Vec<_>> {
        (33300..=33306)
            .flat_map(|x| (32052..=32059).map(move |y| position(x, y)))
            .map(|pos| {
                let tile = view::compose_tile(&map.overlay, &map.facts, pos)
                    .map_err(debug)?
                    .ok_or_else(|| format!("missing donor bridge tile {pos:?}"))?;
                Ok((tile, world.enterable(pos)))
            })
            .collect()
    };
    let original = snapshot()?;
    assert!(
        original.iter().any(|(_, walkable)| !walkable),
        "the original crossing includes blocked terrain"
    );
    assert!(owner.refresh_cake_world_map().await);
    assert_eq!(snapshot()?, original);

    // Seed the completed phase-one contribution totals. The normal stage-two
    // trigger and Channel projection still run, with all their own checks.
    let mut tx = h.pool.begin().await?;
    sqlx::query(
        "ALTER TABLE game_cake_world_progress DISABLE TRIGGER game_cake_progress_admission",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_cake_world_progress VALUES(encode($1,'hex')::uuid,2026,200,200)")
        .bind(world_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("ALTER TABLE game_cake_world_progress ENABLE TRIGGER game_cake_progress_admission")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let stage: i16 = sqlx::query_scalar("SELECT stage FROM game_cake_phases WHERE world_id=encode($1,'hex')::uuid AND event_year=2026")
        .bind(world_id).fetch_one(&h.pool).await?;
    assert_eq!(stage, 2);
    assert!(
        owner.refresh_cake_world_map().await,
        "stage two projects the bridge"
    );
    let installed = snapshot()?;
    let mut revisions = Vec::new();
    for (tile, walkable) in &installed {
        let [entry] = tile.entries.as_slice() else {
            return Err(format!(
                "bridge replaces the entire original stack at {:?}",
                tile.pos
            )
            .into());
        };
        let item = if matches!(tile.pos.x, 33300 | 33306) {
            12113
        } else {
            48227
        };
        let reference = index
            .definition_ref(
                "Item",
                &format!("oteryn:item.tibia.i{item}"),
                "definition-r1",
            )
            .ok_or("canonical bridge definition")?;
        assert_eq!(
            entry.facts.definition,
            MapDefinition::Item(reference),
            "{:?}",
            tile.pos
        );
        assert_eq!(entry.facts.appearance_id, item);
        assert!(*walkable, "{:?} must be walkable", tile.pos);
        assert!(!entry.facts.pickupable);
        let MapEntryKey::Base(key) = entry.source.key() else {
            return Err("bridge placement must be revision-keyed".into());
        };
        let revision = map.facts.object_revision(key);
        assert!(revision > 0);
        revisions.push((key, revision));
    }
    assert!(owner.refresh_cake_world_map().await);
    assert_eq!(snapshot()?, installed);
    for &(key, revision) in &revisions {
        assert_eq!(map.facts.object_revision(key), revision);
    }

    // Simulate the event-end date within this fixture's reset scope, so the
    // original live map must restore in place and advance its object revisions.
    let mut tx = h.pool.begin().await?;
    sqlx::query(
        "ALTER TABLE game_world_reset_epochs DISABLE TRIGGER game_world_reset_epochs_immutable",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE game_world_reset_epochs SET activated_at='2026-02-27 05:00:00+00' WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world_id).execute(&mut *tx).await?;
    sqlx::query(
        "ALTER TABLE game_world_reset_epochs ENABLE TRIGGER game_world_reset_epochs_immutable",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    assert!(
        owner.refresh_cake_world_map().await,
        "the reset continuation restores the bridge"
    );
    assert_eq!(snapshot()?, original);
    assert!(owner.refresh_cake_world_map().await);
    assert_eq!(snapshot()?, original);
    for (key, revision) in revisions {
        assert_eq!(map.facts.object_revision(key), revision + 1);
    }
    Ok(())
}
