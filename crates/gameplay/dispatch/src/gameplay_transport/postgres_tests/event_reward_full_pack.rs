//! Blue Present uses the carried reward route even when its gift lands on Ground.
use super::fixture::{Harness, debug, id, run};
use super::item_craft_fixtures::exchange;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_items::reward_bag::{reward_bag_conversion, reward_bag_draw};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

/// Minted item key, quantity, ground position, channel and audit count.
type PlacementRow = (String, i64, Vec<u8>, Vec<u8>, i64);

const BLUE_PRESENT: &str = "oteryn:item.tibia.i6570";

#[test]
fn blue_present_full_backpack_drops_one_reward_and_consumes_one_present_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, 2, PackSetup::Full).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "full backpack reward scenario thread")??;
    Ok(())
}

#[test]
fn blue_present_last_unit_in_full_backpack_drops_before_freeing_source_slot() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, 1, PackSetup::Full).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "last unit full backpack reward scenario thread")??;
    Ok(())
}

#[test]
fn blue_present_full_backpack_merges_the_gift_without_a_public_drop() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, 2, PackSetup::CompatibleStack).map_err(|error| error.to_string()))?
        .join().map_err(|_| "compatible-stack scenario thread")??;
    Ok(())
}

#[test]
fn blue_present_with_a_free_slot_and_no_capacity_uses_ground() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, 1, PackSetup::Overweight).map_err(|error| error.to_string()))?
        .join().map_err(|_| "overweight present scenario thread")??;
    Ok(())
}

#[test]
fn blue_present_legacy_receipt_uses_the_retained_original_handle_binding() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, 1, PackSetup::LegacyReceipt).map_err(|error| error.to_string()))?
        .join().map_err(|_| "legacy present replay scenario thread")??;
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PackSetup {
    Full,
    CompatibleStack,
    Overweight,
    LegacyReceipt,
}

fn scenario(admin: String, quantity: i64, setup: PackSetup) -> TestResult {
    run(async move |_| {
        let harness =
            Harness::create_with_admission_seed(admin, "w19a_full_pack", true, false).await?;
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
        let oven = "oteryn:item.tibia.i2535";
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let bundle_room = super::rust_lottery_fixtures::bundle_action_room(
            pin.qualified_room(),
            &map,
            deps.admitted.world_id,
            &cells,
        )?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            bundle_room.frame_binding().digest(),
            bundle_room.map_revision_digest(),
            (6, 0, 7),
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
                    x: 6,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&bundle_room));
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
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(bundle_room.frame_binding().digest().as_slice()).execute(&harness.pool).await?;

        let source_ref = reference(BLUE_PRESENT).ok_or("Blue Present reference")?;
        let definition = crate::gameplay_transport::item_look::typed_definition(index, source_ref)
            .ok_or("Blue Present definition")?;
        let command_id = 1;
        let source = if setup == PackSetup::CompatibleStack {
            (4096_u32..8192).map(|candidate| {
                let mut source = id(71);
                source[12..].copy_from_slice(&candidate.to_be_bytes());
                source
            }).find(|source| {
                let draw = reward_bag_draw(*fence.game_session_id.as_bytes(), command_id, *source);
                reward_bag_conversion(native, &definition, draw).is_some_and(|conversion|
                    conversion.outputs.iter().any(|(reward, _)| reward.production_key == "oteryn:item.tibia.i3598"))
            }).ok_or("stackable Blue Present draw")?
        } else {
            id(71)
        };
        let draw = reward_bag_draw(*fence.game_session_id.as_bytes(), command_id, source);
        let expected = reward_bag_conversion(native, &definition, draw).ok_or("Blue Present reward")?;
        assert_eq!(expected.recipe, "oteryn:reward-bag.event_6570");
        let [(reward, amount)] = expected.outputs.as_slice() else {
            return Err("Blue Present must select one reward".into());
        };
        let mut entries = vec![(BLUE_PRESENT, quantity)];
        if setup == PackSetup::CompatibleStack {
            entries.push((reward.production_key.as_str(), 1));
        }
        if setup == PackSetup::Overweight {
            entries.resize(19, ("oteryn:item.tibia.i43872", 1));
        } else {
            entries.resize(20, ("oteryn:item.tibia.i3588", 1));
        }
        super::item_look_fixtures::seed_main_backpack(&harness, &entries).await?;

        if source != id(71) {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_item_instances SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid")
                .bind(source.as_slice()).bind(id(71).as_slice()).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_item_container_entries SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid")
                .bind(source.as_slice()).bind(id(71).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }

        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("source handle")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.next_command_id = command_id;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("reward use capabilities")?;
        let command = UseCommand {
            game_session_id: fence.game_session_id,
            command_id,
            item_fence: Some(fence),
        };
        if setup == PackSetup::CompatibleStack {
            let (_, result, _) = exchange(&owner, admitted, command_id, &payload, Some(source_ref)).await?;
            assert_eq!(result, UseDisposition::Committed);
            let state: (i64, bool, i64, i64, i64) = sqlx::query_as(
                "SELECT i.quantity, i.state_revision=2,
                        (SELECT count(*) FROM game_item_ground_locations),
                        (SELECT count(*) FROM game_item_craft_records),
                        (SELECT count(*) FROM game_item_audit_outbox)
                 FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id)
                 WHERE i.item_instance_id=encode($1,'hex')::uuid",
            ).bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(state, (1 + i64::from(*amount), true, 0, 1, 1));
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if setup == PackSetup::LegacyReceipt {
            use crate::gameplay_transport::item_view::{ItemViewContinuity, SessionItemView};
            // Record a binding from the original server view. The replacement
            // connection rebuilds the view after the last present is consumed.
            let mut original = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
            let handle = original.map_view(&[ItemKey::Instance(source)], |table| {
                table.handle(&ItemKey::Instance(source)).ok_or(crate::gameplay_transport::item_view::ItemViewError::Encode)
            }).map_err(debug)?;
            original.remember_plain_use(command_id, handle);
            admitted.continuity.item_view = original.continuity();
            let use_result = owner.use_reward_bag(deps.actor, command, ItemKey::Instance(source)).await
                .flatten().ok_or("legacy commit")?;
            assert_eq!(use_result.disposition, UseDisposition::Committed);
            let no_binding = owner.use_reward_bag_intent(deps.actor, command, &intent, None, None).await
                .map_err(debug)?.ok_or("unproved legacy replay")?;
            assert_eq!(no_binding.disposition, UseDisposition::Rejected);
            let raw = encode_item_use_intent(&UseIntent { target: UseTarget::Item(handle), ..intent })
                .map_err(debug)?;
            let (_, replayed, _) = exchange(&owner, admitted, command_id, &raw, None).await?;
            assert_eq!(replayed, UseDisposition::Committed);
            let counts: (i64, i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM game_item_craft_records),
                        (SELECT count(*) FROM game_item_ground_locations),
                        (SELECT count(*) FROM game_item_audit_outbox)",
            ).fetch_one(&harness.pool).await?;
            assert_eq!(counts, (1, 1, 1));
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        // Fail after the source debit, when the transaction inserts the reward's custody.
        sqlx::raw_sql(
            "CREATE SEQUENCE test_present_drop_reached;
             GRANT USAGE ON SEQUENCE test_present_drop_reached TO oteryn_game_runtime;
             CREATE FUNCTION refuse_test_present_drop() RETURNS trigger LANGUAGE plpgsql AS $$
                 BEGIN
                     PERFORM nextval('test_present_drop_reached');
                     RAISE EXCEPTION 'injected Blue Present Ground insert failure';
                 END $$;
             CREATE TRIGGER refuse_test_present_drop AFTER INSERT ON game_item_ground_locations
                 FOR EACH ROW EXECUTE FUNCTION refuse_test_present_drop();",
        )
        .execute(&harness.pool)
        .await?;
        let failed = owner
            .use_reward_bag(deps.actor, command, ItemKey::Instance(source))
            .await;
        assert!(matches!(failed, Some(None)), "{failed:?}");
        assert!(
            sqlx::query_scalar::<_, bool>("SELECT is_called FROM test_present_drop_reached")
                .fetch_one(&harness.pool)
                .await?,
            "the injected failure must occur at the Ground insert"
        );
        let rolled_back: (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT i.quantity,
             (SELECT count(*) FROM game_item_craft_records),
             (SELECT count(*) FROM game_item_ground_locations),
             (SELECT count(*) FROM game_item_audit_outbox)
             FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(source.as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(rolled_back, (quantity, 0, 0, 0));
        sqlx::query("DROP TRIGGER refuse_test_present_drop ON game_item_ground_locations")
            .execute(&harness.pool)
            .await?;

        let (_, result, actual) = exchange(&owner, admitted, command_id, &payload, Some(source_ref)).await?;
        assert_eq!(result, UseDisposition::Committed);

        let check = async || -> TestResult {
            let source_state: (i64, i16, bool) = sqlx::query_as(
                "SELECT i.quantity,i.lifecycle,EXISTS(SELECT 1 FROM game_item_container_entries e
                 WHERE e.item_instance_id=i.item_instance_id AND e.placement_ordinal=1
                  AND e.parent_item_instance_id=encode($2,'hex')::uuid)
                 FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(source.as_slice())
            .bind(id(70).as_slice())
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(source_state, if quantity == 1 { (0, 2, false) } else { (1, 1, true) });

            let placements: Vec<PlacementRow> = sqlx::query_as(
                "SELECT i.definition_production_key,i.quantity,g.spatial_position,uuid_send(g.channel_id),
                 (SELECT count(*) FROM game_item_audit_outbox a WHERE a.transaction_id=r.transaction_id)
                 FROM game_item_craft_records r JOIN game_item_instances i ON i.minted_transaction_id=r.transaction_id
                 JOIN game_item_ground_locations g USING(item_instance_id)
                 WHERE r.command_id=$2::text::numeric AND r.source_item_instance_id=encode($1,'hex')::uuid",
            ).bind(source.as_slice()).bind(command_id.to_string()).fetch_all(&harness.pool).await?;
            let position = [
                6_i32.to_be_bytes().as_slice(),
                &0_i32.to_be_bytes(),
                &(-7_i16).to_be_bytes(),
            ]
            .concat();
            assert_eq!(
                placements,
                [(
                    reward.production_key.clone(),
                    i64::from(*amount),
                    position,
                    id(43).to_vec(),
                    1
                )]
            );
            let counts: (i64, i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM game_item_craft_records),
                 (SELECT count(*) FROM game_item_ground_locations),
                 (SELECT count(*) FROM game_item_container_entries)",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(counts, (1, 1, i64::try_from(entries.len())? - i64::from(quantity == 1)));
            Ok(())
        };
        check().await?;

        let intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
            admitted.continuity.selected_capabilities.as_slice(),
            &actual,
        ).map_err(debug)?;
        let replay = owner
            .use_reward_bag_intent(deps.actor, command, &intent, None, None)
            .await.map_err(|()| "reward replay unavailable")?
            .ok_or("reward replay")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        check().await?;

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
