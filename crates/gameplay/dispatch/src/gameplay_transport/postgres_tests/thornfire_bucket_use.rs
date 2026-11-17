use super::{Dependencies, Harness, TestResult, debug, id, prepare_current_session, reset, run};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition, NodeId};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, placement_key};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
use std::num::NonZeroU64;

const EMPTY: &str = "oteryn:item.tibia.i12819";
const FILLED: &str = "oteryn:item.tibia.i12722";
const FIREFIGHTER: &str = "oteryn:achievement/firefighter";
// World id(42), fire at (33093,32148,7), stack ordinal 1, and revision 0
// yield a winning draw at the first authored wolf position in this epoch.
const EPOCH: i64 = 1991;

#[test]
fn thornfire_postgres_bucket_use_refills_extinguishes_and_projects_one_wolf() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Thornfire bucket USE thread panicked")??;
    Ok(())
}

fn scenario(admin: String) -> TestResult {
    run(async |_| {
        let started = std::time::Instant::now();
        reset();
        let h = Harness::create_with_admission_seed(admin, "thornfire_bucket", true, false).await?;
        {
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut deps = Dependencies::new()?;
            let (world, channel) = (deps.admitted.world_id, deps.admitted.channel_id);
            let (controller, pin) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    world, None,
                );
            let active = controller.active().ok_or("active content")?;
            let native = active.native_gameplay().ok_or("native gameplay")?;
            let start = MovementLocalPosition {
                x: 33092,
                y: 32148,
                floor: 7,
            };
            let fire = MovementLocalPosition { x: 33093, ..start };
            let water = MovementLocalPosition { y: 32149, ..start };
            let wolf = MovementLocalPosition {
                x: 33091,
                y: 32149,
                floor: 7,
            };
            let map = super::cake_phases::cake_bundle(
                native,
                world,
                channel,
                start,
                &[(water, 12721), (fire, 5067)],
            )?;
            let cells = map
                .movement_cells(pin.qualified_room().movement_cells())
                .map_err(debug)?;
            let room =
                super::rust_lottery::bundle_action_room(pin.qualified_room(), &map, world, &cells)?;
            let server = pin.identity().server_artifact_digest();
            let client = pin.identity().client_artifact_digest();
            let frame = room.frame_binding().digest();
            let content_pin = ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                server,
                client,
                frame,
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
                content_pin,
            )
            .map_err(debug)?;
            runtime
                .install_companion_policies(native.policies().clone())
                .map_err(debug)?;
            let session = deps.admitted.game_session_id;
            let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(deps.actor, start)
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            owner.movement_cells = (&cells).into();
            owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
            deps.initialize(&owner).await;
            let current = FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
            let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
                .bind(id(41).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                .bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
            super::item_look::seed_main_backpack(&h, &[(EMPTY, 1)]).await?;
            super::achievement_granters::seed_progress(&h, FIREFIGHTER, 499).await?;
            let map_state = owner.bundle_map().ok_or("bundle overlay")?;
            let bundle = map_state.overlay.base().digest();
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2026-02-21 05:00:00+00','2026-02')")
                .bind(world.as_bytes().as_slice()).bind(EPOCH).bind(bundle.as_slice()).bind(map.map_revision())
                .bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2)")
                .bind(world.as_bytes().as_slice())
                .bind(EPOCH)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            owner.reset_epoch = u64::try_from(EPOCH)?;
            owner.refresh_terrain_tools().await;
            let map_state = owner.bundle_map().ok_or("bundle overlay")?;
            let key = |at: MovementLocalPosition| -> TestResult<u64> {
                Ok(placement_key(
                    TilePos {
                        x: u16::try_from(at.x)?,
                        y: u16::try_from(at.y)?,
                        floor: i8::try_from(-at.floor)?,
                    },
                    1,
                )
                .ok_or("map placement key")?)
            };
            let fire_key = key(fire)?;
            let command = |command_id| UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(fence),
            };
            let intent = |key| -> TestResult<UseIntent> {
                Ok(UseIntent {
                    target: UseTarget::Item(NonZeroU64::new(1).ok_or("bucket handle")?),
                    use_with: None,
                    use_with_position: None,
                    use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                        placement: bundle.into_iter().chain(u64::to_be_bytes(key)).collect(),
                        expected_revision: map_state.facts.object_revision(key),
                    })),
                })
            };
            let refill = owner
                .use_craft(
                    deps.actor,
                    command(1),
                    intent(key(water)?)?,
                    Some(ItemKey::Instance(id(71))),
                    None,
                )
                .await;
            assert_eq!(
                refill.map(|outcome| outcome.disposition),
                Some(UseDisposition::Committed),
                "Prepared Bucket must refill through USE on bog water"
            );
            let read_bucket = |expected: &'static str| async {
                let backpack = h
                    .root
                    .read_character_backpack(&authority, fence.character_id)
                    .await?
                    .ok_or("backpack")?;
                assert_eq!(backpack.entries.len(), 1);
                let bucket = &backpack.entries[0].item;
                assert_eq!(bucket.definition.production_key, expected);
                assert_eq!(bucket.quantity, 1);
                Ok::<_, Box<dyn std::error::Error>>(bucket.item_instance_id)
            };
            let filled = read_bucket(FILLED).await?;
            let fire_intent = intent(fire_key)?;
            for _ in 0..2 {
                let result = owner
                    .use_craft(
                        deps.actor,
                        command(2),
                        fire_intent.clone(),
                        Some(ItemKey::Instance(filled)),
                        None,
                    )
                    .await;
                assert_eq!(
                    result.map(|outcome| outcome.disposition),
                    Some(UseDisposition::Committed),
                    "extinguish and exact replay"
                );
                read_bucket(EMPTY).await?;
                assert_eq!(
                    map_state.facts.object_revision(fire_key),
                    1,
                    "one committed fire mutation"
                );
                super::achievement_granters::assert_progress(
                    &h,
                    FIREFIGHTER,
                    500,
                    true,
                    "oteryn:thornfire-extinguish",
                )
                .await?;
                let receipts: Vec<(Vec<u8>, bool)> = sqlx::query_as(
                    "SELECT uuid_send(spawn_id),realized FROM game_thornfire_wolf_spawns",
                )
                .fetch_all(&h.pool)
                .await?;
                assert_eq!(receipts.len(), 1);
                assert!(receipts[0].1, "winning USE must acknowledge its wolf");
                let runtime = deps.runtime.lock().await;
                let creatures = runtime.current_live_creature_census().map_err(debug)?;
                assert_eq!(creatures.len(), 1);
                let body = runtime.companion_snapshot(creatures[0]).map_err(debug)?;
                assert_eq!(
                    body.state.policy.definition_key,
                    "oteryn:creature.thornfire_wolf"
                );
                assert_eq!(body.position, wolf);
                let occurrence = runtime
                    .raid_identity_for_actor(creatures[0])
                    .map_err(debug)?
                    .ok_or("wolf occurrence")?;
                assert_eq!(occurrence.firing_id.as_slice(), receipts[0].0);
            }
        }
        h.cleanup().await?;
        eprintln!(
            "THORNFIRE_BUCKET_USE_PG_EXECUTED elapsed_ms={}",
            started.elapsed().as_millis()
        );
        Ok(())
    })
}
