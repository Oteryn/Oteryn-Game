//! M06: carried key custody through production USE wire, replay and reconnect.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::map::view::MapFacts;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority,
};
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
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M06 wire panic")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(admin, "m06_keys", true, false).await?;
        eprintln!("START REVM06 real USE wire PostgreSQL");
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
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let door = "oteryn:item.tibia.i1628";
        let key_item = "oteryn:item.tibia.i2967";
        let wrong_key = "oteryn:item.tibia.i2968";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            (door, reference(door).ok_or("door")?.get() - 1),
            ("oteryn:item.tibia.i3031", reference("oteryn:item.tibia.i3031").ok_or("coin")?.get() - 1),
        );
        let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
        let entry = data.sectors.iter_mut().flat_map(|s| &mut s.tiles)
            .find(|tile| tile.x == 6).ok_or("door tile")?
            .items.last_mut().ok_or("door entry")?;
        entry.attrs.action = Some(4321);
        let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
            &data.manifest, &data.sectors, &data.spawns, &data.npcs)?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 7,
                    y: 0,
                    floor: -7,
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
            (7, 0, 7),
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
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        super::item_look::seed_main_backpack(
            &harness,
            &[(key_item, 1), (wrong_key, 1)],
        )
        .await?;
        // Fixture-only immutable key custody rows; production reads run as runtime role.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        for (item, number) in [(71, 4321), (72, 4322)] {
            sqlx::query("INSERT INTO game_item_keys(item_instance_id,key_number,minted_transaction_id) SELECT item_instance_id,$2,game_character_uuid_v7() FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(item).as_slice()).bind(number).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        let mut placement = map.base().digest().to_vec();
        let station_pos = crate::map::overlay::TilePos {
            x: 6,
            y: 0,
            floor: -7,
        };
        placement.extend_from_slice(
            &crate::map::view::placement_key(station_pos, 1)
                .ok_or("station placement")?
                .to_be_bytes(),
        );
        let mut station = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };

        let intent = |station: WorldObjectTarget| UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(station)),
            use_with_position: None,
        };
        let appearance = || map.facts().base_entry(station_pos, 1, reference(door).ok_or("door ref")?.get() - 1).map(|entry| entry.appearance_id).ok_or("door facts");
        assert_eq!(appearance()?, 1628);
        let payload = encode_item_use_intent(&intent(station.clone())).map_err(debug)?;
        let (next, result, _) = super::item_craft::exchange(&owner, admitted, 1, &payload, reference(wrong_key)).await?;
        assert_eq!(result, UseDisposition::KeyMismatch);
        assert_eq!(map.facts().doors().revision(crate::map::boot::tests::door_key()), 0);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let (next, result, replay) = super::item_craft::exchange(&owner, next, 2, &payload, reference(key_item)).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(map.facts().doors().revision(crate::map::boot::tests::door_key()), 1);
        assert!(map.enterable(station_pos));
        assert_eq!(appearance()?, 1630);
        // Exact original command bytes and connection continuity replay the prior result once.
        let retry = next;
        let (next, result, _) = super::item_craft::exchange(&owner, retry, 2, &replay, None).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(map.facts().doors().revision(crate::map::boot::tests::door_key()), 1);
        let states = std::mem::take(&mut owner.states);
        let clock_origin = owner.clock_origin;
        drop(owner);
        // Reopen the durable Character authority and reconstruct the connection owner. The
        // Channel's scope-ephemeral door book survives this session reconnect, not a reboot.
        drop(authority);
        let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut owner = deps.owner(&harness, &authority);
        owner.states = states;
        owner.clock_origin = clock_origin;
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        assert!(map.enterable(station_pos));
        assert_eq!(appearance()?, 1630);
        let inventory = harness.root.read_item_use_inventory(&authority, &harness.node, fence).await.map_err(debug)?;
        assert_eq!(inventory.key_action(id(71)), Some(4321));
        assert_eq!(inventory.key_action(id(72)), Some(4322));
        station.expected_revision = 1;
        let payload = encode_item_use_intent(&intent(station.clone())).map_err(debug)?;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let (next, result, _) = super::item_craft::exchange(&owner, next, 3, &payload, reference(key_item)).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert!(!map.enterable(station_pos));
        assert_eq!(appearance()?, 1628);
        assert_eq!(map.facts().doors().revision(crate::map::boot::tests::door_key()), 2);
        // Direct USE is locked again; the key can reopen it after reconnect.
        station.expected_revision = 2;
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&station).map_err(debug)?;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let (next, result, _) = super::item_craft::exchange(&owner, next, 4, &payload, None).await?;
        assert_eq!(result, UseDisposition::Locked);
        let payload = encode_item_use_intent(&intent(station)).map_err(debug)?;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let (_, result, _) = super::item_craft::exchange(&owner, next, 5, &payload, reference(key_item)).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert!(map.enterable(station_pos));
        assert_eq!(appearance()?, 1630);
        assert_eq!(map.facts().doors().revision(crate::map::boot::tests::door_key()), 3);
        eprintln!("REVM06 executed 6 wire commands, 3 transitions, elapsed {:?}", started.elapsed());
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
