//! M06: carried key custody through production USE wire, replay and reconnect.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[test]
fn m06_carried_keys_use_wire_lock_wrong_key_replay_and_relog() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(None).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M06 wire panic")??;
    Ok(())
}

pub(super) fn source_key_door(x: u16, y: u16, floor: i8, action: u16) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("fibula-source-key", move || {
        scenario(Some((x, y, floor, action)))
    })
}

fn scenario(source: Option<(u16, u16, i8, u16)>) -> TestResult {
    run(async |admin| {
        let level_gate = source.is_some_and(|(_, _, _, action)| action == 1050);
        let harness =
            Harness::create_with_admission_seed(admin, "m06_keys", !level_gate, false).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&harness.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED source_key_door_{source:?} server_version={version}");
        let started = std::time::Instant::now();
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = if level_gate {
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?
        } else {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            )
        };
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let (door_x, door_y, door_floor, key_action) = source.unwrap_or((6, 0, -7, 4321));
        let door_appearance = if level_gate {
            1648
        } else if source.is_some() {
            1631
        } else {
            1628
        };
        let door_key = format!("oteryn:item.tibia.i{door_appearance}");
        let door = door_key.as_str();
        let station_pos = crate::map::overlay::TilePos {
            x: door_x,
            y: door_y,
            floor: door_floor,
        };
        let placement_key =
            crate::map::view::placement_key(station_pos, 1).ok_or("door placement")?;
        let key_item = match key_action {
            3940 => "oteryn:item.tibia.i2968",
            3980 => "oteryn:item.tibia.i2972",
            _ => "oteryn:item.tibia.i2967",
        };
        let wrong_key = if key_action == 3940 {
            "oteryn:item.tibia.i2967"
        } else {
            "oteryn:item.tibia.i2968"
        };
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            (door, reference(door).ok_or("door")?.get() - 1),
            (
                "oteryn:item.tibia.i3031",
                reference("oteryn:item.tibia.i3031").ok_or("coin")?.get() - 1,
            ),
        );
        let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
        let entry = data
            .sectors
            .iter_mut()
            .flat_map(|s| &mut s.tiles)
            .find(|tile| tile.x == 6)
            .ok_or("door tile")?
            .items
            .last_mut()
            .ok_or("door entry")?;
        entry.attrs.action = source.is_none().then_some(4321);
        if source.is_some() {
            let mut sectors = std::collections::BTreeMap::new();
            for sector in std::mem::take(&mut data.sectors) {
                for mut tile in sector.tiles {
                    tile.x = tile.x + door_x - 6;
                    tile.y += door_y;
                    sectors
                        .entry((door_floor, tile.y / 32, tile.x / 32))
                        .or_insert_with(Vec::new)
                        .push(tile);
                }
            }
            data.sectors = sectors
                .into_iter()
                .map(
                    |((floor, sy, sx), tiles)| oteryn_world_bundle::bundle::Sector {
                        floor,
                        sx,
                        sy,
                        tiles,
                    },
                )
                .collect();
            data.manifest.world = oteryn_world_bundle::bundle::Extent {
                min_x: door_x - 6,
                min_y: door_y,
                max_x: door_x + 26,
                max_y: door_y + 32,
                floors: vec![door_floor],
            };
        }
        let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
            &data.manifest,
            &data.sectors,
            &data.spawns,
            &data.npcs,
        )?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: door_x + 1,
                    y: door_y,
                    floor: door_floor,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                use crate::content::{ReferenceItemField, ReferenceItemType};
                let policy = native.item_policy(key, "definition-r1")?.record();
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: matches!(&policy.semantics.physical, ReferenceItemField::Known(p) if p.pickupable == ReferenceItemField::Known(true)),
                    door: matches!(&policy.semantics.classification, ReferenceItemField::Known(c) if c.item_type == ReferenceItemField::Known(ReferenceItemType::Door)),
                })
            },
        )
        .map_err(debug)?;
        if source.is_some() {
            assert_eq!(
                map.facts().action_at(placement_key),
                Some(key_action),
                "the raw source door must acquire its exact startup key action"
            );
        }
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (
                i32::from(door_x + 1),
                i32::from(door_y),
                -i16::from(door_floor),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            content_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: i32::from(door_x + 1),
                    y: i32::from(door_y),
                    floor: -i16::from(door_floor),
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        if !level_gate {
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        }
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        if !level_gate {
            super::item_look::seed_main_backpack(&harness, &[(key_item, 1), (wrong_key, 1)])
                .await?;
            // Fixture-only immutable key custody rows; production reads run as runtime role.
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            for (item, number) in [(71, i32::from(key_action)), (72, i32::from(key_action) + 1)] {
                sqlx::query("INSERT INTO game_item_keys(item_instance_id,key_number,minted_transaction_id) SELECT item_instance_id,$2,game_character_uuid_v7() FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(item).as_slice()).bind(number).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        let mut placement = map.base().digest().to_vec();
        placement.extend_from_slice(
            &crate::map::view::placement_key(station_pos, 1)
                .ok_or("station placement")?
                .to_be_bytes(),
        );
        let mut station = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };

        if level_gate {
            let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
            harness
                .root
                .initialize_admission_equipment(
                    &authority,
                    &harness.node,
                    &fence,
                    active.identity().server_artifact_digest(),
                    fresh.operation(),
                )
                .await
                .map_err(debug)?;
            owner
                .runtime
                .lock()
                .await
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            owner.admit_progression_session(&mut admitted).await;
            for (level, expected) in [
                (49, UseDisposition::Locked),
                (50, UseDisposition::Committed),
            ] {
                super::questpc6_fibula_route::level(&owner, &mut admitted, level).await?;
                let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&station)
                    .map_err(debug)?;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let command = admitted.continuity.next_command_id;
                let (next, result, replay) =
                    super::item_craft::exchange(&owner, admitted, command, &payload, None).await?;
                assert_eq!(result, expected, "source level50 gate at level{level}");
                admitted = next;
                assert_eq!(map.enterable(station_pos), level == 50);
                if level == 50 {
                    assert_eq!(map.facts().doors().revision(placement_key), 1);
                    let (next, result, _) =
                        super::item_craft::exchange(&owner, admitted, command, &replay, None)
                            .await?;
                    admitted = next;
                    assert_eq!(result, UseDisposition::Committed);
                    assert_eq!(map.facts().doors().revision(placement_key), 1);
                }
            }
            eprintln!(
                "POSTGRES_CASE_COMPLETED fibula_source_level50_gate duration={:?}",
                started.elapsed()
            );
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        let intent = |station: WorldObjectTarget| UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(station)),
            use_with_position: None,
        };
        let appearance = || {
            map.facts()
                .base_entry(station_pos, 1, reference(door).ok_or("door ref")?.get() - 1)
                .map(|entry| entry.appearance_id)
                .ok_or("door facts")
        };
        assert_eq!(appearance()?, door_appearance);
        let payload = encode_item_use_intent(&intent(station.clone())).map_err(debug)?;
        let (next, result, _) =
            super::item_craft::exchange(&owner, admitted, 1, &payload, reference(wrong_key))
                .await?;
        assert_eq!(result, UseDisposition::KeyMismatch);
        assert_eq!(map.facts().doors().revision(placement_key), 0);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let (next, result, replay) =
            super::item_craft::exchange(&owner, next, 2, &payload, reference(key_item)).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(map.facts().doors().revision(placement_key), 1);
        assert!(map.enterable(station_pos));
        assert_eq!(appearance()?, door_appearance + 2);
        // Exact original command bytes and connection continuity replay the prior result once.
        let retry = next;
        let (next, result, _) =
            super::item_craft::exchange(&owner, retry, 2, &replay, None).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(map.facts().doors().revision(placement_key), 1);
        let states = std::mem::take(&mut owner.states);
        let clock_origin = owner.clock_origin;
        drop(owner);
        // Reopen the durable Character authority and reconstruct the connection owner. The
        // Channel's scope-ephemeral door book survives this session reconnect, not a reboot.
        drop(authority);
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&harness, &authority);
        owner.states = states;
        owner.clock_origin = clock_origin;
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        assert!(map.enterable(station_pos));
        assert_eq!(appearance()?, door_appearance + 2);
        let inventory = harness
            .root
            .read_item_use_inventory(&authority, &harness.node, fence)
            .await
            .map_err(debug)?;
        assert_eq!(inventory.key_action(id(71)), Some(key_action));
        assert_eq!(inventory.key_action(id(72)), Some(key_action + 1));
        station.expected_revision = 1;
        let payload = encode_item_use_intent(&intent(station.clone())).map_err(debug)?;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let (next, result, _) =
            super::item_craft::exchange(&owner, next, 3, &payload, reference(key_item)).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert!(!map.enterable(station_pos));
        assert_eq!(appearance()?, door_appearance);
        assert_eq!(map.facts().doors().revision(placement_key), 2);
        // Direct USE is locked again; the key can reopen it after reconnect.
        station.expected_revision = 2;
        let payload =
            oteryn_protocol_oteryn::world_object::encode_use_intent(&station).map_err(debug)?;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let (next, result, _) =
            super::item_craft::exchange(&owner, next, 4, &payload, None).await?;
        assert_eq!(result, UseDisposition::Locked);
        let payload = encode_item_use_intent(&intent(station)).map_err(debug)?;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let (_, result, _) =
            super::item_craft::exchange(&owner, next, 5, &payload, reference(key_item)).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert!(map.enterable(station_pos));
        assert_eq!(appearance()?, door_appearance + 2);
        assert_eq!(map.facts().doors().revision(placement_key), 3);
        eprintln!(
            "POSTGRES_CASE_COMPLETED source_key_door_{source:?} wire_commands=6 transitions=3 duration={:?}",
            started.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
