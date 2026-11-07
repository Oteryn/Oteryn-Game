//! W05d actual source B3 -> native map owner -> paced movement -> existing map wire.
use super::fixture::{debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
#[path = "seasonal_cake_bridge/food_wire.rs"]
mod food_wire;
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use crate::gameplay_transport::{
    connection::FreshAdmissionAuthority,
    item_view::{ItemViewContinuity, SessionItemView},
    world_map::{MapUpdate, SessionMapView},
};

#[test]
fn srv7_cake_bridge_real_bundle_paced_step_and_native_map_capture() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-cake-bridge", || scenario(false))
}
fn scenario(bite: bool) -> TestResult {
    scenario_mode(bite, false)
}
fn scenario_mode(bite: bool, goal: bool) -> TestResult {
    scenario_flags(bite, goal, false, false)
}
fn scenario_flags(bite: bool, goal: bool, food: bool, legacy_speech: bool) -> TestResult {
    run(async |admin| {
        super::reset();
        let h =
            super::fixture::Harness::create_with_admission_seed(admin, "srv7_bridge", true, false)
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
        let (controller, pin) = if food {
            super::raid_live::hare_production_activation::activate(
                world,
                if legacy_speech {
                    "OTERYN_SRV7_CAKE_FOOD_LEGACY_CLIENT_ARTIFACT"
                } else {
                    "OTERYN_SRV7_CAKE_FOOD_CLIENT_ARTIFACT"
                },
            )?
        } else {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None)
        };
        let active = controller.active().ok_or("current native generation")?;
        let native = active.native_gameplay().ok_or("current native")?;
        let index = native.item_index().ok_or("Item index")?;
        let bytes = std::fs::read(
            std::env::var_os("OTERYN_SRV7_BRIDGE_BUNDLE").ok_or("actual compiler Bundle path")?,
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
            x: 33303,
            y: if goal || bite || food { 32066 } else { 32060 },
            floor: -7,
        };
        let map=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start},world,channel,|key|{
            let revision=index.revision(key)?;
            let p=native.item_policy(key,revision).map(|p|p.record());
            Some(crate::map::facts::ItemDefinition{
                reference:index.definition_ref("Item",key,revision)?,
                solid:p.and_then(|p|p.attributes.blocks_movement),
                blocks_projectile:p.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),
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
                    x: 33303,
                    y: if goal || bite || food { 32066 } else { 32060 },
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
            (33303, if goal || bite || food { 32066 } else { 32060 }, 7),
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
                    x: 33303,
                    y: if goal || bite || food { 32066 } else { 32060 },
                    floor: 7,
                },
            )
            .map_err(debug)?;
        if food {
            return native_food(
                &mut owner,
                legacy_speech,
                deps.actor,
                deps.admitted,
                &h,
                &map,
                (
                    pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                ),
            )
            .await;
        }
        if goal {
            return native_final_goal(
                &owner,
                deps.actor,
                deps.admitted,
                &h,
                &map,
                (
                    pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                ),
            )
            .await;
        }
        if bite {
            return native_bite(
                &owner,
                deps.actor,
                deps.admitted,
                &h,
                &map,
                (
                    pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                ),
            )
            .await;
        }
        let mut view = SessionMapView::default();
        let mut items = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
        let observer = owner
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        let closed = owner
            .observe_world_map(deps.actor, &mut view, &mut items)
            .await
            .ok_or("closed observation")?
            .map_err(debug)?
            .ok_or("closed snapshot")?;
        let MapUpdate::Snapshot(closed) = closed else {
            return Err("closed native snapshot".into());
        };
        let (result, _) = owner
            .ordinary_paced_step(
                deps.actor,
                deps.admitted.game_session_id,
                crate::gameplay_transport::world_spatial::StepDirection::North,
                None,
                None,
            )
            .await;
        assert_ne!(
            result.disposition,
            crate::gameplay_transport::world_spatial::StepDisposition::Moved,
            "source water border must block closed bridge"
        );
        let live = owner.bundle_map().ok_or("actual native map")?;
        assert!(live.facts.project_cake_bridge(live.overlay.base(), true));
        for x in 33301..=33305 {
            for y in 32052..=32059 {
                let pos = crate::map::overlay::TilePos { x, y, floor: -7 };
                assert!(
                    map.enterable(pos),
                    "all40 source bridge floor cells must be walkable: {pos:?}"
                );
                let tile = crate::map::view::compose_tile(&live.overlay, &live.facts.pinned(), pos)
                    .map_err(debug)?
                    .ok_or("source bridge tile")?;
                assert_eq!(
                    tile.ground_speed, 150,
                    "native ground projection cannot retain source water speed0"
                );
                assert!(tile.entries.iter().any(|e| e.facts.appearance_id == 48227));
            }
        }

        let open = owner
            .observe_world_map(deps.actor, &mut view, &mut items)
            .await
            .ok_or("open observation")?
            .map_err(debug)?
            .ok_or("open delta")?;
        let MapUpdate::Delta(open) = open else {
            return Err("open native delta".into());
        };
        let (result, _) = owner
            .ordinary_paced_step(
                deps.actor,
                deps.admitted.game_session_id,
                crate::gameplay_transport::world_spatial::StepDirection::North,
                None,
                None,
            )
            .await;
        assert_eq!(
            result.disposition,
            crate::gameplay_transport::world_spatial::StepDisposition::Moved
        );
        let (result, _) = owner
            .ordinary_paced_step(
                deps.actor,
                deps.admitted.game_session_id,
                crate::gameplay_transport::world_spatial::StepDirection::South,
                None,
                None,
            )
            .await;
        assert_eq!(
            result.disposition,
            crate::gameplay_transport::world_spatial::StepDisposition::Moved
        );
        assert!(live.facts.project_cake_bridge(live.overlay.base(), false));
        let cleanup = owner
            .observe_world_map(deps.actor, &mut view, &mut items)
            .await
            .ok_or("cleanup observation")?
            .map_err(debug)?
            .ok_or("cleanup delta")?;
        let MapUpdate::Delta(cleanup) = cleanup else {
            return Err("cleanup native delta".into());
        };
        if let Some(path) = std::env::var_os("OTERYN_SRV7_BRIDGE_CAPTURE") {
            use sha2::Digest;
            let hex = |b: &[u8]| b.iter().map(|b| format!("{b:02x}")).collect::<String>();
            std::fs::write(
                path,
                serde_json::to_vec_pretty(
                    &serde_json::json!({"source":{"b3_sha256":"2798be76221d6dc00695258d46d1eb375a64e804a7a84265014e2af0c679b05d","producer_test":"srv7_cake_bridge_real_bundle_paced_step_and_native_map_capture","donor_commit":"201189f871d23597b82caf88fd611454a2ddefb8","observer_runtime":{"x":observer.x,"y":observer.y,"floor":observer.floor},"observer_native":{"x":observer.x,"y":observer.y,"floor":-i32::from(observer.floor)},"observer_actor_hex":hex(&deps.actor.placement_identity()),"world_id_hex":hex(world.as_bytes()),"bundle_digest_hex":hex(&bundle.digest),"bundle_sha256":hex(&sha2::Sha256::digest(&bytes))},"closed_snapshot_hex":hex(&closed.payload),"open_delta_hex":hex(&open.payload),"cleanup_delta_hex":hex(&cleanup.payload)}),
                )?,
            )?;
        }
        eprintln!(
            "SRV7_CAKE_BRIDGE_POSTGRES_ASSERTION_EXECUTED native_map_closed_open_restored=true"
        );
        Ok(())
    })
}

#[test]
fn srv7_cake_native_bite_waits_fixed_window_and_commits_food_cas_once() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-cake-native-bite", || scenario(true))
}
async fn native_bite(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    h: &super::fixture::Harness,
    map: &crate::map::boot::BundleWorld,
    digests: ([u8; 32], [u8; 32], [u8; 32]),
) -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use oteryn_protocol_oteryn::world_object::{
        UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
    };
    const START: i64 = 1_771_650_000;
    const NOW: i64 = START + 3601;
    let _clock = crate::gameplay_transport::seasonal_cake::test_clock::set(NOW);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(digests.0.as_slice()).bind(digests.1.as_slice()).bind(digests.2.as_slice()).execute(&h.pool).await?;
    owner
        .initialize_cake_world()
        .await
        .map_err(|_| "actual World owner initialize")?;
    let mut tx = h.pool.begin().await?;
    // Controlled predecessor only; every tested actor request uses actual runtime fences.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_seasonal_cake_worlds SET state=jsonb_set(jsonb_set(jsonb_set(jsonb_set(state,'{stage}','2'),'{substances}','200'),'{golems}','200'),'{revision}','2') WHERE world_id=encode($1,'hex')::uuid").bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_seasonal_cake_stage_polls(world_id,event_start,poll_slot,observed_at,opened_bridge) VALUES(encode($1,'hex')::uuid,$2,3,$2+1800,true)").bind(id(42).as_slice()).bind(START).execute(&mut *tx).await?;
    tx.commit().await?;
    admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
    admitted.runtime_actor = Some(actor);
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "current item fence")?
            .ok_or("current item fence")?
            .item_fence(),
    );
    let actual = admitted.item_fence.ok_or("actual source actor fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(actual.character_id.as_bytes())
                    .map_err(debug)?,
                actual.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
            .ok_or("native map USE capabilities")?;
    let target = crate::map::overlay::TilePos {
        x: 33303,
        y: 32067,
        floor: -7,
    };
    let key = crate::map::view::placement_key(target, 1).ok_or("real source cake placement")?;
    let mut placement = map.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let intent = UseIntent {
        target: UseTarget::WorldObject(WorldObjectTarget {
            placement,
            expected_revision: 0,
        }),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent).map_err(debug)?;
    let (next, result, _) = super::item_craft::exchange(owner, admitted, 1, &payload, None).await?;
    assert_eq!(
        result,
        UseDisposition::NotPossible,
        "USE cannot open an unclaimed fixed clock window"
    );
    admitted = next;
    let state: serde_json::Value =
        sqlx::query_scalar("SELECT state FROM game_seasonal_cake_worlds")
            .fetch_one(&h.pool)
            .await?;
    assert!(state["window_start"].is_null());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_seasonal_cake_bite_receipts")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    owner
        .initialize_cake_world()
        .await
        .map_err(|_| "actual scheduled window refresh")?;
    let (next, result, _) = super::item_craft::exchange(owner, admitted, 2, &payload, None).await?;
    admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert!(
        (230_000..=240_000)
            .contains(&food_remaining(owner, actor, admitted.game_session_id).await?)
    );
    let facts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT (state->>'layer_progress')::bigint FROM game_seasonal_cake_worlds WHERE world_id=encode($1,'hex')::uuid),(SELECT state_item_id FROM game_terrain_tool_transforms LIMIT 1),(SELECT count(*) FROM game_seasonal_cake_bite_receipts)").bind(id(42).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        facts,
        (1, 0, 1),
        "actual Native USE atomically binds one physical tombstone and World bite"
    );
    let (_, result, _) = super::item_craft::exchange(owner, admitted, 2, &payload, None).await?;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_seasonal_cake_bite_receipts")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    eprintln!(
        "SRV7_NATIVE_CAKE_BITE_POSTGRES_ASSERTION_EXECUTED real_map=true fixed_clock_required=true actual_food=true atomic_cas=true replay=true"
    );
    Ok(())
}

#[test]
fn srv7_cake_actual_edible_footprint_goal_completes_without_eating_corners() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-cake-final-goal", || {
        scenario_mode(false, true)
    })
}
async fn native_final_goal(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    h: &super::fixture::Harness,
    map: &crate::map::boot::BundleWorld,
    digests: ([u8; 32], [u8; 32], [u8; 32]),
) -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use oteryn_protocol_oteryn::world_object::{
        UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
    };
    const START: i64 = 1_771_650_000;
    const NOW: i64 = START + 3601;
    let _clock = crate::gameplay_transport::seasonal_cake::test_clock::set(NOW);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(digests.0.as_slice()).bind(digests.1.as_slice()).bind(digests.2.as_slice()).execute(&h.pool).await?;
    owner
        .initialize_cake_world()
        .await
        .map_err(|_| "actual World owner initialize")?;
    let mut tx = h.pool.begin().await?;
    // Controlled predecessor only; every tested actor request uses actual runtime fences.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_seasonal_cake_worlds SET state=jsonb_set(jsonb_set(jsonb_set(jsonb_set(state,'{stage}','2'),'{substances}','200'),'{golems}','200'),'{revision}','2') WHERE world_id=encode($1,'hex')::uuid").bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_seasonal_cake_worlds SET state=state || jsonb_build_object('layer_progress',1065,'window_start',$2::bigint,'window_end',$2::bigint+1800) WHERE world_id=encode($1,'hex')::uuid").bind(id(42).as_slice()).bind(NOW-1).execute(&mut *tx).await?;
    let source_cells: Vec<(i32, i32, i32)> =
        serde_json::from_str(include_str!("seasonal_cake_goal_cells.json"))?;
    assert_eq!(source_cells.len(), 1066);
    for (x, y, appearance) in source_cells {
        if (x, y) == (33303, 32067) {
            continue;
        }
        sqlx::query("INSERT INTO game_seasonal_cake_tiles(world_id,event_start,layer,x,y,floor,appearance,window_start,removed) VALUES(encode($1,'hex')::uuid,$2,1,$3,$4,7,$5,$6,true)").bind(id(42).as_slice()).bind(START).bind(x).bind(y).bind(appearance).bind(NOW-1).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_seasonal_cake_stage_polls(world_id,event_start,poll_slot,observed_at,opened_bridge) VALUES(encode($1,'hex')::uuid,$2,($3-$2)/600,$3,true)").bind(id(42).as_slice()).bind(START).bind(NOW-1801).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_seasonal_cake_windows(world_id,event_start,window_start,layer,observed_at) VALUES(encode($1,'hex')::uuid,$2,$3,1,$3)").bind(id(42).as_slice()).bind(START).bind(NOW-1).execute(&mut *tx).await?;
    tx.commit().await?;
    admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
    admitted.runtime_actor = Some(actor);
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "current item fence")?
            .ok_or("current item fence")?
            .item_fence(),
    );
    let actual = admitted.item_fence.ok_or("actual source actor fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(actual.character_id.as_bytes())
                    .map_err(debug)?,
                actual.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
            .ok_or("native map USE capabilities")?;
    let target = crate::map::overlay::TilePos {
        x: 33303,
        y: 32067,
        floor: -7,
    };
    let key = crate::map::view::placement_key(target, 1).ok_or("real source cake placement")?;
    let mut placement = map.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let intent = UseIntent {
        target: UseTarget::WorldObject(WorldObjectTarget {
            placement,
            expected_revision: 0,
        }),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent).map_err(debug)?;
    let (_, result, _) = super::item_craft::exchange(owner, admitted, 1, &payload, None).await?;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "actual final existing edible square must be reachable while corners stay untouched"
    );
    let state: serde_json::Value = sqlx::query_scalar(
        "SELECT state FROM game_seasonal_cake_worlds WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(id(42).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(state["layer"], 1);
    assert_eq!(
        state["layer_progress"], 1066,
        "the actual final bite must reach the real edible footprint goal"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_seasonal_cake_tiles WHERE removed AND layer=1"
        )
        .fetch_one(&h.pool)
        .await?,
        1066
    );
    owner
        .initialize_cake_world()
        .await
        .map_err(|_| "following source World clock turn")?;
    let next: serde_json::Value = sqlx::query_scalar(
        "SELECT state FROM game_seasonal_cake_worlds WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(id(42).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(next["layer"], 2);
    assert_eq!(next["layer_progress"], 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_seasonal_cake_tiles WHERE removed AND layer=1"
        )
        .fetch_one(&h.pool)
        .await?,
        0,
        "nonfinal successful round restores the edible floor"
    );
    eprintln!(
        "SRV7_CAKE_ACTUAL_FOOTPRINT_NATIVE_GOAL_POSTGRES_ASSERTION_EXECUTED controlled_predecessor1065=true actual_final_food_native_USE=true"
    );
    Ok(())
}

#[test]
fn srv7_cake_current_tibia_four_immediate_food_bites_then_full_and_one_minute_hunger() -> TestResult
{
    super::kill_reward_e2e::scenario_thread::run("srv7-cake-native-food", || {
        scenario_flags(false, false, true, false)
    })
}
#[test]
fn srv7_cake_legacy_chat7_projects_source_text_as_say_without_386_or_288() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-cake-legacy-speech", || {
        scenario_flags(false, false, true, true)
    })
}
async fn native_food(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    legacy_speech: bool,
    actor: crate::foundation::ExactActorRef,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    h: &super::fixture::Harness,
    map: &crate::map::boot::BundleWorld,
    digests: ([u8; 32], [u8; 32], [u8; 32]),
) -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use oteryn_protocol_oteryn::world_object::{
        UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
    };
    const START: i64 = 1_771_650_000;
    const NOW: i64 = START + 7201;
    let _clock = crate::gameplay_transport::seasonal_cake::test_clock::set(NOW);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(digests.0.as_slice()).bind(digests.1.as_slice()).bind(digests.2.as_slice()).execute(&h.pool).await?;
    owner
        .initialize_cake_world()
        .await
        .map_err(|_| "actual World owner initialize")?;
    let mut tx = h.pool.begin().await?;
    // Controlled predecessor only; every tested actor request uses actual runtime fences.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_seasonal_cake_worlds SET state=jsonb_set(jsonb_set(jsonb_set(jsonb_set(state,'{stage}','2'),'{substances}','200'),'{golems}','200'),'{revision}','2') WHERE world_id=encode($1,'hex')::uuid").bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_seasonal_cake_worlds SET state=state || jsonb_build_object('window_start',$2::bigint,'window_end',$2::bigint+1800) WHERE world_id=encode($1,'hex')::uuid").bind(id(42).as_slice()).bind(NOW-1).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_seasonal_cake_stage_polls(world_id,event_start,poll_slot,observed_at,opened_bridge) VALUES(encode($1,'hex')::uuid,$2,($3-$2)/600,$3,true)").bind(id(42).as_slice()).bind(START).bind(NOW-1801).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_seasonal_cake_windows(world_id,event_start,window_start,layer,observed_at) VALUES(encode($1,'hex')::uuid,$2,$3,1,$3)").bind(id(42).as_slice()).bind(START).bind(NOW-1).execute(&mut *tx).await?;
    tx.commit().await?;
    admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
    admitted.runtime_actor = Some(actor);
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "current item fence")?
            .ok_or("current item fence")?
            .item_fence(),
    );
    let actual = admitted.item_fence.ok_or("actual source actor fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(actual.character_id.as_bytes())
                    .map_err(debug)?,
                actual.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let actual_default_offer = owner.offered_capabilities();
    for capability in [
        oteryn_protocol_oteryn::presentation::CAPABILITY_ITEM_EFFECTS_V1,
        oteryn_protocol_oteryn::chat::CAPABILITY_SCRIPTED_ITEM_SPEECH_V1,
    ] {
        assert!(
            actual_default_offer
                .iter()
                .any(|entry| entry.id == capability),
            "real default admission must offer {capability}"
        );
        assert!(
            !SelectedCapabilities::select(actual_default_offer, &[capability])
                .ok_or("bounded prerequisite selection")?
                .contains(capability),
            "missing prerequisites must remove {capability}"
        );
    }
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        if legacy_speech {
            &[4, 6, 7, 11, 15, 18][..]
        } else {
            &[
                4,
                6,
                7,
                11,
                15,
                18,
                oteryn_protocol_oteryn::chat::CAPABILITY_SCRIPTED_ITEM_SPEECH_V1,
            ][..]
        },
    )
    .ok_or("native map USE and ordinary chat7 capabilities")?;
    assert!(admitted.continuity.selected_capabilities.contains(7));
    assert!(!admitted.continuity.selected_capabilities.contains(288));
    assert_eq!(
        admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::chat::CAPABILITY_SCRIPTED_ITEM_SPEECH_V1),
        !legacy_speech
    );
    owner.states.chat.admit_as(
        admitted.game_session_id,
        "Citizen",
        Some(*actual.character_id.as_bytes()),
        owner.owner_now(),
    );
    owner
        .states
        .chat
        .select_mount_sounds(admitted.game_session_id, false);
    let targets = [
        (33303, 32067),
        (33302, 32067),
        (33304, 32067),
        (33303, 32068),
        (33304, 32068),
    ];
    let intent_for = |x, y| -> TestResult<Vec<u8>> {
        let pos = crate::map::overlay::TilePos { x, y, floor: -7 };
        let key = crate::map::view::placement_key(pos, 1).ok_or("actual source edible square")?;
        let mut placement = map.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        encode_item_use_intent(&UseIntent {
            target: UseTarget::WorldObject(WorldObjectTarget {
                placement,
                expected_revision: 0,
            }),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)
    };
    // Exact-instant hunger rule uses the mature kernel. Real PostgreSQL calls
    // advance wall time, so the Wiki's one-minute example is not a new cooldown.
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let mut exact = states
            .read_owned_player_state_test_snapshot(&runtime, actor, admitted.game_session_id)
            .ok_or("actual native Food predecessor")?
            .clone();
        let now_ms = owner.owner_now().get() / 1000;
        for _ in 0..4 {
            crate::spell::actor_conditions::eat_food(&mut exact, 240, now_ms)
                .map_err(|_| "exact-instant first four bites")?;
        }
        assert!(
            crate::spell::actor_conditions::eat_food(&mut exact, 240, now_ms).is_err(),
            "same-instant fifth reaches mature food cap"
        );
        crate::spell::actor_conditions::eat_food(&mut exact, 240, now_ms + 60_000)
            .map_err(|_| "minute of hunger permits fifth")?;
    }
    for (index, (x, y)) in targets[..4].iter().copied().enumerate() {
        let payload = intent_for(x, y)?;
        let (next, result, _, frames) =
            food_wire::exchange(owner, admitted, u64::try_from(index)? + 1, &payload, None).await?;
        if index == 0 {
            let mut chat_lines = Vec::new();
            for raw in &frames {
                let envelope = crate::foundation::decode_wire_envelope(raw)?;
                if envelope.message_type() == crate::foundation::MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id == oteryn_protocol_oteryn::chat::STATE_DOMAIN_CHAT {
                        chat_lines.push(delta.payload.to_vec());
                    }
                }
            }
            let expected_mode = if legacy_speech {
                oteryn_protocol_oteryn::chat::ChatSpeechMode::Say
            } else {
                oteryn_protocol_oteryn::chat::ChatSpeechMode::MonsterSay
            };
            let count=chat_lines.iter().filter(|bytes|matches!(oteryn_protocol_oteryn::chat::decode_chat_line(bytes),Ok(oteryn_protocol_oteryn::chat::ChatLine::Local{mode,text,..})if mode==expected_mode && text=="Mmmm!")).count();
            if let Some(path) = std::env::var_os(if legacy_speech {
                "OTERYN_SRV7_CAKE_FOOD_LEGACY_CAPTURE"
            } else {
                "OTERYN_SRV7_CAKE_FOOD_CAPTURE"
            }) {
                use sha2::{Digest, Sha256};
                let hex =
                    |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
                let actual_source_b3 = std::fs::read(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x130-y125.b3"
                ))?;
                let actual_source_b3_sha = Sha256::digest(&actual_source_b3);
                let mut transcript = Vec::new();
                let accepted = crate::foundation::encode_server_accepted(
                    &crate::foundation::ServerAcceptedValue {
                        game_session_id: admitted.game_session_id,
                        world_id: admitted.world_id,
                        channel_id: admitted.channel_id,
                        connection_generation: admitted.continuity.connection_generation,
                        current_server_sequence: admitted.continuity.server_sequence,
                        next_command_id: admitted.continuity.next_command_id,
                        schema_revision:
                            crate::gameplay_transport::connection::SERVER_SCHEMA_REVISION,
                        selected_capabilities: admitted.continuity.selected_capabilities.as_slice(),
                    },
                )
                .map_err(debug)?;
                crate::gameplay_transport::tcp_tls::write_frame(&mut transcript, &accepted).await?;
                let client_generation = owner
                    .runtime
                    .lock()
                    .await
                    .content_pin()
                    .client_artifact_digest();
                for raw in &frames {
                    crate::gameplay_transport::tcp_tls::write_frame(&mut transcript, raw).await?;
                }
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(
                        &serde_json::json!({"schema":"OTERYN_CAKE_FOOD_NATIVE_CHAT_CAPTURE/v1","source":{"producer_test":if legacy_speech { "srv7_cake_legacy_chat7_projects_source_text_as_say_without_386_or_288" } else { "srv7_cake_current_tibia_four_immediate_food_bites_then_full_and_one_minute_hunger" },"actor_identity_hex":hex(&actor.placement_identity()),"world_id_hex":hex(admitted.world_id.as_bytes()),"character_id_hex":hex(actual.character_id.as_bytes()),"observer_runtime_cell":[33303,32066,7],"b3_sha256":hex(&actual_source_b3_sha),"bundle_base_digest":hex(&map.overlay().base().digest()),"chat7":true,"scripted386":!legacy_speech,"ambient288":false,"client_content_generation":hex(&client_generation),"command_id":1,"command_type":2,"command_payload_hex":hex(&payload)},"frames_hex":hex(&transcript),"frames_sha256":hex(&Sha256::digest(&transcript)),"mmm_count":count}),
                    )?,
                )?;
            }
            assert_eq!(
                count, 1,
                "fresh real native Cake USE must publish source text in the selected scripted or legacy speech mode without ambient288"
            );
        }
        admitted = next;
        assert_eq!(
            result,
            UseDisposition::Committed,
            "current Tibia allows four immediate bites"
        );
        if legacy_speech {
            return Ok(());
        }
        let remaining = food_remaining(owner, actor, admitted.game_session_id).await?;
        let expected = i32::try_from(index + 1)? * 240_000;
        assert!(
            (expected - 5000..=expected).contains(&remaining),
            "native food must add exactly four minutes per bite: {remaining} vs {expected}"
        );
        if index == 0 {
            let (step, _) = owner
                .ordinary_paced_step(
                    actor,
                    admitted.game_session_id,
                    crate::gameplay_transport::world_spatial::StepDirection::South,
                    None,
                    None,
                )
                .await;
            assert_eq!(
                step.disposition,
                crate::gameplay_transport::world_spatial::StepDisposition::Moved,
                "the first eaten real cake square becomes walkable before the next three bites"
            );
        }
    }
    let payload = intent_for(targets[4].0, targets[4].1)?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(60))
        .ok_or("native food clock")?;
    let _later = crate::gameplay_transport::seasonal_cake::test_clock::set(NOW + 60);
    let (next, result, _) = super::item_craft::exchange(owner, admitted, 5, &payload, None).await?;
    admitted = next;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "source one-minute sufficient hunger example"
    );
    let remaining = food_remaining(owner, actor, admitted.game_session_id).await?;
    assert!(
        (1_130_000..=1_140_000).contains(&remaining),
        "after a minute the fifth bite leaves nineteen minutes: {remaining}"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_seasonal_cake_bite_receipts")
            .fetch_one(&h.pool)
            .await?,
        5
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_seasonal_cake_food_uses")
            .fetch_one(&h.pool)
            .await?,
        5
    );
    let full_payload = intent_for(33302, 32068)?;
    let (next, full, _) =
        super::item_craft::exchange(owner, admitted, 6, &full_payload, None).await?;
    admitted = next;
    assert_eq!(
        full,
        UseDisposition::Full,
        "native near-nineteen-minute state cannot add four more minutes"
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(240))
        .ok_or("native hunger advance")?;
    let _hungry = crate::gameplay_transport::seasonal_cake::test_clock::set(NOW + 300);
    let (next, replayed, _) =
        super::item_craft::exchange(owner, admitted, 6, &full_payload, None).await?;
    admitted = next;
    assert_eq!(
        replayed,
        UseDisposition::Full,
        "retained Full result survives a changed hunger clock"
    );
    let (next, fresh, _) =
        super::item_craft::exchange(owner, admitted, 7, &full_payload, None).await?;
    admitted = next;
    assert_eq!(
        fresh,
        UseDisposition::Committed,
        "fresh command uses current hunger rather than replayed Full"
    );
    for table in [
        "game_seasonal_cake_bite_receipts",
        "game_seasonal_cake_food_uses",
    ] {
        let query = match table {
            "game_seasonal_cake_bite_receipts" => {
                "SELECT count(*) FROM game_seasonal_cake_bite_receipts"
            }
            _ => "SELECT count(*) FROM game_seasonal_cake_food_uses",
        };
        assert_eq!(
            sqlx::query_scalar::<_, i64>(query)
                .fetch_one(&h.pool)
                .await?,
            6,
            "Full and replay cannot add source receipts: {table}"
        );
    }
    // Stage the real successor without publishing it, then exercise normal
    // removed-actor housekeeping. Existing committed snapshots remain authority.
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(240))
        .ok_or("real pending food hunger clock")?;
    let (moved, _) = owner
        .ordinary_paced_step(
            actor,
            admitted.game_session_id,
            crate::gameplay_transport::world_spatial::StepDirection::South,
            None,
            None,
        )
        .await;
    assert_eq!(
        moved.disposition,
        crate::gameplay_transport::world_spatial::StepDisposition::Moved,
        "actual eaten source square admits the next paced step"
    );
    let saved_snapshot: serde_json::Value = sqlx::query_scalar(
        "SELECT snapshot FROM game_character_condition_snapshots WHERE character_id=encode($1,'hex')::uuid",
    ).bind(actual.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("actual native food source")?;
    let source = crate::map::overlay::TilePos {
        x: 33302,
        y: 32069,
        floor: -7,
    };
    let key = crate::map::view::placement_key(source, 1).ok_or("real source key")?;
    let tile = crate::map::view::compose_tile(map.overlay(), map.facts(), source)
        .map_err(debug)?
        .ok_or("real pending source tile")?;
    let before_id = tile
        .entries
        .iter()
        .find(|entry| entry.source.key() == crate::map::view::MapEntryKey::Base(key))
        .ok_or("qualified real pending source entry")?
        .facts
        .appearance_id;
    assert!(
        matches!(before_id, 12085 | 12086),
        "qualified real edible source appearance"
    );
    let mut placement = map.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let mut runtime = owner.runtime.lock().await;
    let position = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let spatial = |x: u16, y: u16, z: i8| {
        crate::durability::map_item_mint_audit::map_item_spatial_position(x, y, z)
    };
    let command = crate::gameplay_transport::connection::UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: admitted.continuity.next_command_id,
        item_fence: Some(actual),
    };
    let request = crate::durability::item_craft::MapHarvestRequest {
        command: crate::foundation::CommandRef::new(
            command.game_session_id,
            crate::foundation::CommandId::new(command.command_id).map_err(debug)?,
        ),
        event_unix_seconds: NOW + 540,
        intent: intent_for(source.x, source.y)?,
        operand: Some(crate::durability::item_craft::MapHarvestOperand {
            scope: crate::durability::terrain_tool::TerrainScope {
                map_revision: crate::map::overlay::map_revision(map.base()),
                content_generation: native.source_digest(),
                reset_epoch: i64::try_from(owner.reset_epoch)?,
            },
            mutation: crate::durability::terrain_tool::MapMutation {
                placement,
                expected_revision: 0,
                before: u32::from(before_id),
                after: 0,
                decay: Vec::new(),
            },
            actor_position: spatial(
                u16::try_from(position.x)?,
                u16::try_from(position.y)?,
                i8::try_from(-position.floor)?,
            ),
            source_position: spatial(source.x, source.y, source.floor),
        }),
    };
    let mut states = owner.states.spell_states.lock().await;
    let before = states
        .read_owned_player_state_test_snapshot(&runtime, actor, admitted.game_session_id)
        .ok_or("actual live food before")?
        .clone();
    assert!(
        states
            .prepare_cake_food_successor(
                &runtime,
                actor,
                (command, actual),
                &request,
                native,
                owner.owner_now().get()
            )
            .map_err(|_| "actual pending food stage")?
            .is_some()
    );
    assert_eq!(
        states.pending_cake_food.len(),
        1,
        "real production stager retains successor"
    );
    assert_eq!(
        states.read_owned_player_state_test_snapshot(&runtime, actor, admitted.game_session_id),
        Some(&before),
        "staging cannot publish food before a receipt"
    );
    runtime.remove_test_actor(actor).map_err(debug)?;
    states
        .reconcile_owner_bookkeeping_for_test(&runtime)
        .ok_or("normal removed-actor housekeeping")?;
    assert!(
        states.pending_cake_food.is_empty(),
        "removed actor cannot retain an uncommitted food successor"
    );
    drop(states);
    drop(runtime);
    let after_snapshot:serde_json::Value=sqlx::query_scalar("SELECT snapshot FROM game_character_condition_snapshots WHERE character_id=encode($1,'hex')::uuid")
        .bind(actual.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after_snapshot, saved_snapshot,
        "actor removal cannot write an uncommitted food snapshot"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_seasonal_cake_food_uses")
            .fetch_one(&h.pool)
            .await?,
        6
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_seasonal_cake_bite_receipts")
            .fetch_one(&h.pool)
            .await?,
        6
    );
    eprintln!(
        "SRV7_CAKE_CURRENT_TIBIA_FOOD_POSTGRES_ASSERTION_EXECUTED exact_instant_four_then_full=true actual_native_four_bites=true native_after60s_fifth_committed=true native_full_and_replay=true same_tx_food_and_bite=true"
    );
    Ok(())
}
async fn food_remaining(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> TestResult<i32> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states
        .read_owned_player_state_test_snapshot(&runtime, actor, session)
        .ok_or("actual native player state")?;
    state
        .owned_conditions()
        .get(oteryn_gameplay_domain::domain::condition::ConflictKey::FoodRegeneration)
        .and_then(|food| food.remaining_duration_ms(owner.owner_now().get()))
        .ok_or_else(|| {
            "actual Cake native USE did not publish the source four-minute food condition".into()
        })
}
