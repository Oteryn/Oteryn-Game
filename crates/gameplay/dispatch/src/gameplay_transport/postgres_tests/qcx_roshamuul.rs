//! Roshamuul mortar preparation through the production carried craft owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_craft;
use crate::gameplay_transport::item_view::{ItemKey, SessionItemView, inventory_from_backpack};
use oteryn_protocol_oteryn::world_object::MapUseTarget;
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

#[test]
fn gravel_and_chalk_make_one_mixture_and_return_one_empty_bucket() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .unwrap_or_else(|_| Err("Roshamuul craft worker panicked".into()));
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx_roshamuul_mixture duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "qcx_roshamuul_mixture", true, false)
                .await?;
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
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
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
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;

        super::item_look::seed_main_backpack(
            &harness,
            &[
                ("oteryn:item.tibia.i20053", 1),
                ("oteryn:item.tibia.i20054", 1),
            ],
        )
        .await?;
        let inventory = owner
            .root
            .read_item_use_inventory(owner.character, owner.holder, fence)
            .await
            .map_err(debug)?;
        let mut view = SessionItemView::resume(Default::default());
        view.snapshot(
            inventory_from_backpack(inventory.backpack.as_ref(), |definition| {
                index.definition_ref(
                    &definition.family,
                    &definition.production_key,
                    &definition.revision_ref,
                )
            })
            .ok_or("recipe inventory")?,
            None,
        )
        .map_err(debug)?;
        let gravel = view
            .table()
            .handle(&ItemKey::Instance(id(71)))
            .ok_or("gravel handle")?;
        let chalk = view
            .table()
            .handle(&ItemKey::Instance(id(72)))
            .ok_or("chalk handle")?;
        let intent = UseIntent {
            target: UseTarget::Item(gravel),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(chalk)),
            use_with_position: None,
        };
        let command = UseCommand {
            game_session_id: fence.game_session_id,
            command_id: 1,
            item_fence: Some(fence),
        };
        let result = item_craft::dispatch(&owner, deps.actor, command, intent.clone(), Some(&view))
            .await
            .ok_or("missing Roshamuul gravel and chalk craft route")?;
        assert_eq!(result.disposition, UseDisposition::Committed);
        let replay = item_craft::dispatch(&owner, deps.actor, command, intent, None)
            .await
            .ok_or("missing Roshamuul craft replay")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        // TibiaWiki spoiler 66033/1104388, Delivering Mortar.
        // Crystal actions_mixture.lua:2-4,13-15 supplies item IDs and the empty bucket.
        let items: Vec<(String, i64)> = sqlx::query_as(
            "SELECT definition_production_key, quantity FROM game_item_instances
             WHERE lifecycle=1 AND definition_production_key IN
                ('oteryn:item.tibia.i20053','oteryn:item.tibia.i20054',
                 'oteryn:item.tibia.i20169','oteryn:item.tibia.i2873')
             ORDER BY definition_production_key",
        )
        .fetch_all(&harness.pool)
        .await?;
        assert_eq!(
            items,
            vec![
                ("oteryn:item.tibia.i20169".into(), 1),
                ("oteryn:item.tibia.i2873".into(), 1)
            ]
        );
        Ok(())
    })
}
