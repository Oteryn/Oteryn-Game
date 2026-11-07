//! W05d actual source B3 -> native map owner -> paced movement -> existing map wire.
use super::fixture::{debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
#[path = "seasonal_cake_machine/body.rs"]
mod body;

#[test]
fn srv7_cake_golem_real_machine_runtime_extracts_and_removes_once() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-cake-machine", scenario)
}
fn scenario() -> TestResult {
    run(async |admin| {
        super::reset();
        let h = super::fixture::Harness::create_appearance_with_admission_seed(
            admin,
            "srv7_machine",
            true,
            false,
        )
        .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
        let (controller, pin) = super::raid_live::hare_production_activation::activate(
            world,
            "OTERYN_SRV7_GOLEM_CLIENT_ARTIFACT",
        )?;
        let active = controller.active().ok_or("current native generation")?;
        let native = active.native_gameplay().ok_or("current native")?;
        let index = native.item_index().ok_or("Item index")?;
        let bytes = std::fs::read(
            std::env::var_os("OTERYN_SRV7_MACHINE_BUNDLE").ok_or("actual compiler Bundle path")?,
        )?;
        let bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        let pins = crate::map::BundlePins {
            digest: bundle.digest,
            project_format_version: bundle.manifest.identity.project_format_version.clone(),
            world_schema_version: bundle.manifest.identity.world_schema_version.clone(),
            content_revision: bundle.manifest.identity.content_revision.clone(),
            production: false,
        };
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let start = crate::map::overlay::TilePos {
            x: 32345,
            y: 31822,
            floor: -7,
        };
        let map=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start},world,channel,|key|{
            let revision=index.revision(key)?;
            let p=native.item_policy(key,revision).map(|p|p.record());
            Some(crate::map::facts::ItemDefinition{
                reference:index.definition_ref("Item",key,revision)?,
                solid:p.and_then(|p|p.attributes.blocks_movement),
                blocks_projectile:p.and_then(|p|p.attributes.blocks_projectile).or_else(||crate::map::spell_tiles::qualified_seasonal_ground_projectile(key)).unwrap_or(true),
                pickupable:p.is_some_and(|p|matches!(&p.semantics.physical,crate::content::ReferenceItemField::Known(p) if p.pickupable==crate::content::ReferenceItemField::Known(true))),door:false,
            })
        }).map_err(debug)?;
        let room = pin
            .qualified_room()
            .bundle_view(
                map.movement_cells(pin.qualified_room().movement_cells())
                    .map_err(debug)?,
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 32345,
                    y: 31822,
                    floor: 7,
                },
            )
            .ok_or("real Cake bundle room view")?;
        let content_pin = ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (32345, 31822, 7),
        );
        let deps =
            Dependencies::with_content(content_pin, room.clone(), native.spell_book().clone())?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = room.movement_cells();
        deps.initialize(&owner).await;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 32345,
                    y: 31822,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let live = owner.bundle_map().ok_or("actual native map")?;
        assert!(live.facts.project_cake_machines(live.overlay.base(), true));
        for y in 32801..=32802 {
            let pos = crate::map::overlay::TilePos {
                x: 32377,
                y,
                floor: -7,
            };
            let tile = crate::map::view::compose_tile(&live.overlay, &live.facts.pinned(), pos)
                .map_err(debug)?
                .ok_or("Liberty source cell")?;
            assert!(tile.entries.iter().any(|e| e.facts.appearance_id == 8886));
            assert!(map.enterable(pos));
        }
        assert!(live.facts.project_cake_machines(live.overlay.base(), false));
        for (y, appearance) in [(32801, 4517), (32802, 4522)] {
            let pos = crate::map::overlay::TilePos {
                x: 32377,
                y,
                floor: -7,
            };
            let tile = crate::map::view::compose_tile(&live.overlay, &live.facts.pinned(), pos)
                .map_err(debug)?
                .ok_or("Liberty restored cell")?;
            assert!(
                tile.entries
                    .iter()
                    .any(|e| e.facts.appearance_id == appearance)
            );
            assert!(!tile.entries.iter().any(|e| e.facts.appearance_id == 8886));
        }
        assert!(live.facts.project_cake_machines(live.overlay.base(), true));
        const NOW: i64 = 1_771_650_300;
        let _clock = crate::gameplay_transport::seasonal_cake::test_clock::set(NOW);
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        owner
            .initialize_cake_world()
            .await
            .map_err(|_| "World initialize")?;
        {
            let mut runtime = deps.runtime.lock().await;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            assert_eq!(
                runtime
                    .companion_policy("oteryn:creature.cake_golem")
                    .map_err(debug)?
                    .definition_key,
                "oteryn:creature.cake_golem",
                "actual qualified source policy must be installed before the source machine spawn"
            );
        }
        let golem = deps
            .runtime
            .lock()
            .await
            .spawn_arena_opponent(
                "oteryn:creature.cake_golem",
                MovementLocalPosition {
                    x: 32346,
                    y: 31822,
                    floor: 7,
                },
                |runtime, at| Some(runtime.native_summon_cell_free(at)),
            )
            .map_err(debug)?;
        body::capture(&owner, &deps, golem).await?;
        let permit = owner.spell_lane_permit().await.ok_or("source spell lane")?;
        assert!(owner.extract_arrived_cake_golems(&permit).await);
        assert!(!deps.runtime.lock().await.contains_live_creature(golem));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_seasonal_cake_extractions")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        let state: serde_json::Value =
            sqlx::query_scalar("SELECT state FROM game_seasonal_cake_worlds")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(state["golems"], 1);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_party_kill_xp_receipts")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        assert!(owner.extract_arrived_cake_golems(&permit).await);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_seasonal_cake_extractions")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        drop(permit);
        // Separate controlled historical predecessor: the real completed tower is
        // edible-food removal, never replacement grass or collision bypass.
        let cells: Vec<(u16, u16, u16)> =
            serde_json::from_str(include_str!("seasonal_cake_goal_cells.json"))?;
        assert_eq!(cells.len(), 1066);
        let mut history = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *history)
            .await?;
        sqlx::query("UPDATE game_seasonal_cake_worlds SET state=state || jsonb_build_object('stage',3,'substances',200,'golems',200,'layer',4,'layer_progress',1066,'window_start',NULL,'window_end',NULL,'bonus_from',1771736400::bigint,'bonus_until',1772341200::bigint,'revision',2)").execute(&mut *history).await?;
        for &(x, y, appearance) in &cells {
            sqlx::query("INSERT INTO game_seasonal_cake_tiles(world_id,event_start,layer,x,y,floor,appearance,window_start,removed) VALUES(encode($1,'hex')::uuid,1771650000,4,$2,$3,7,$4,1771664400,true)")
                .bind(id(42).as_slice()).bind(i32::from(x)).bind(i32::from(y)).bind(i32::from(appearance)).execute(&mut *history).await?;
        }
        history.commit().await?;
        let _completed = crate::gameplay_transport::seasonal_cake::test_clock::set(1_771_668_000);
        let removed = cells.iter().map(|&(x, y, _)| (x, y)).collect();
        assert!(
            live.facts
                .project_cake_tower(live.overlay.base(), 3, 4, &removed)
        );
        for (x, y) in [(33302, 32080), (32345, 31822)] {
            let at = MovementLocalPosition { x, y, floor: 7 };
            {
                let mut runtime = deps.runtime.lock().await;
                let before = runtime.read_actor_position(deps.actor).map_err(debug)?;
                runtime
                    .relocate_arena_player(deps.actor, deps.admitted.game_session_id, before, at)
                    .map_err(debug)?;
            }
            owner.clock_origin = std::time::Instant::now()
                .checked_sub(std::time::Duration::from_secs(120))
                .ok_or("clock")?;
            let (result, _) = owner
                .ordinary_paced_step(
                    deps.actor,
                    deps.admitted.game_session_id,
                    crate::gameplay_transport::world_spatial::StepDirection::East,
                    None,
                    None,
                )
                .await;
            assert_eq!(
                result.disposition,
                crate::gameplay_transport::world_spatial::StepDisposition::Moved
            );
        }
        let progress:(bool,i16)=sqlx::query_as("SELECT isle_armed,visited_cities FROM game_seasonal_cake_characters WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(progress, (false, 1));
        eprintln!(
            "SRV7_CAKE_GOLEM_POSTGRES_ASSERTION_EXECUTED actual_machine_native_golem_removed_once=true"
        );
        Ok(())
    })
}
