use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, item_look_fixtures, prepare_current_session};
use crate::foundation::{ChannelContentPin, WorldId};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_move::{ItemMoveStep, item_move, observe_inventory};
use crate::gameplay_transport::item_view::{ItemKey, ItemViewContinuity, SessionItemView};
use crate::map::boot::tests::{floor_items, floor_pins, teleport_bundle};
use oteryn_protocol_oteryn::item_view::{
    EquipmentSlot, ItemHandle, ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome,
    ItemMoveSelection, encode_item_move_intent_for,
};
use oteryn_protocol_oteryn::zones_nologout::ServerMessage;

const CLAW: &str = "oteryn:item.tibia.i9393";
// Crystal 201189f8 data/scripts/movements/claw_of_the_noxious_spawn.lua:3-18.
const REQUIRED_LEVEL: i64 = 100;
const EQUIP: ItemMoveSelection = ItemMoveSelection {
    equip_drop: true,
    container_tree: false,
    world_interaction: false,
    manual_sort: false,
};

#[test]
fn noxious_spawn_claw_postgres_dispatch_level_gate_and_equip_damage() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("noxious-spawn-claw", || {
        scenario(false, false)?;
        scenario(true, false)
    })
}

#[test]
fn noxious_spawn_claw_postgres_committed_equip_hits_before_failed_refresh() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("noxious-spawn-claw-refresh", || {
        scenario(false, true)
    })
}

fn scenario(protected: bool, fail_refresh: bool) -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "noxious_spawn_claw", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world,
                "content-1",
            )?;
        let active = controller.active().ok_or("active content")?;
        let room = pin.qualified_room().clone();
        let (map_bytes, map_pins) = teleport_bundle();
        let map = crate::map::boot::boot(
            &map_bytes,
            &floor_pins(map_pins),
            world,
            crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            floor_items,
        )
        .map_err(debug)?;
        let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
        // The fixture marks (5, 1, 8) as protection zone and (6, 1, 8) as open ground.
        let start = if protected { (5, 1, 8) } else { (6, 1, 8) };
        let frame_digest = map.frame_binding_digest(world);
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            frame_digest,
            map.map_revision_digest(),
            start,
        );
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        let session = deps.admitted.game_session_id;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            assert_eq!(
                crate::gameplay_transport::actor_spell::qualified_protection_zone(
                    &mut runtime,
                    &cells,
                    deps.actor
                ),
                Some(protected)
            );
        }
        assert!(owner.states.server_messages.join(session, 1));
        let effects = &owner.bundle_map().ok_or("bundle map")?.facts;
        let fence = super::death_ml_loss_composed_fixtures::current_fence(&owner, session).await?;
        {
            let runtime = owner.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    deps.actor,
                    session,
                    crate::spell::cast::CharacterCastFacts {
                        level: u32::try_from(REQUIRED_LEVEL)?,
                        max_health: 1000,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("spell state")?;
        }
        item_look_fixtures::seed_main_backpack(&h, &[(CLAW, 1)]).await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(frame_digest.as_slice()).execute(&h.pool).await?;
        let mut view = SessionItemView::resume(ItemViewContinuity::default()).with_equip_drop();
        let inventory = observe_inventory(&owner, deps.actor, session, &view)
            .await
            .ok_or("inventory")?;
        view.snapshot(inventory, None).map_err(debug)?;
        let source = ItemHandle::new(2).ok_or("handle")?;
        assert_eq!(view.resolve(source), Some(ItemKey::Instance(id(71))));
        let equip = encode_item_move_intent_for(
            &ItemMoveIntent {
                source,
                destination: ItemMoveDestination::Equipment(EquipmentSlot::Ring),
            },
            EQUIP,
        )
        .map_err(debug)?;
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence.item_fence()),
        };
        let before = health(&owner, &deps).await?;

        set_level(&h, REQUIRED_LEVEL).await?;
        eprintln!("POSTGRES_STARTED NOXIOUS-SPAWN-CLAW");
        if fail_refresh {
            crate::gameplay_transport::equipment_move::fail_post_equip_refresh_once();
        }
        let moved = item_move(
            &owner,
            deps.actor,
            command(1),
            &equip,
            EQUIP,
            false,
            &mut view,
        )
        .await;
        if fail_refresh {
            assert!(matches!(moved, ItemMoveStep::Disconnect), "{moved:?}");
        } else {
            assert!(
                matches!(moved, ItemMoveStep::Result(ItemMoveOutcome::Moved, _)),
                "level 100 equip: {moved:?}"
            );
        }
        let worn = observe_inventory(&owner, deps.actor, session, &view)
            .await
            .ok_or("worn inventory")?;
        assert_eq!(worn.equipment.len(), 1);
        assert_eq!(worn.equipment[0].0, EquipmentSlot::Ring);
        assert_eq!(worn.equipment[0].1.key, ItemKey::Instance(id(71)));
        let after = health(&owner, &deps).await?;
        if protected {
            assert_eq!(before, after);
            assert_eq!(owner.states.server_messages.take(session, 1), None);
            assert!(effects.movement_effects_since(0).is_empty());
        } else {
            assert!(
                before
                    .checked_sub(after)
                    .is_some_and(|loss| (150..=200).contains(&loss)),
                "equip hit: {before} -> {after}"
            );
            assert!(matches!(
                owner.states.server_messages.take(session, 1),
                Some(ServerMessage::Advance(_))
            ));
            let impacts = effects.movement_effects_since(0);
            assert_eq!(impacts.len(), 1);
            assert_eq!(impacts[0].2, 1, "draw blood");
        }
        let previous_impacts = effects.movement_effects_since(0);
        let replayed = item_move(
            &owner,
            deps.actor,
            command(1),
            &equip,
            EQUIP,
            false,
            &mut view,
        )
        .await;
        assert!(
            matches!(replayed, ItemMoveStep::Result(ItemMoveOutcome::Moved, _)),
            "{replayed:?}"
        );
        assert_eq!(health(&owner, &deps).await?, after);
        assert_eq!(owner.states.server_messages.take(session, 1), None);
        assert_eq!(effects.movement_effects_since(0), previous_impacts);

        let unequip = encode_item_move_intent_for(
            &ItemMoveIntent {
                source,
                destination: ItemMoveDestination::MainBackpack,
            },
            EQUIP,
        )
        .map_err(debug)?;
        let moved = item_move(
            &owner,
            deps.actor,
            command(2),
            &unequip,
            EQUIP,
            false,
            &mut view,
        )
        .await;
        assert!(
            matches!(moved, ItemMoveStep::Result(ItemMoveOutcome::Moved, _)),
            "{moved:?}"
        );
        set_level(&h, REQUIRED_LEVEL - 1).await?;
        assert_eq!(
            item_move(
                &owner,
                deps.actor,
                command(3),
                &equip,
                EQUIP,
                false,
                &mut view
            )
            .await,
            ItemMoveStep::Result(ItemMoveOutcome::RequirementNotMet, Vec::new()),
        );
        assert_eq!(health(&owner, &deps).await?, after);
        let carried = observe_inventory(&owner, deps.actor, session, &view)
            .await
            .ok_or("carried inventory")?;
        assert!(carried.equipment.is_empty());
        assert!(
            carried
                .entries
                .iter()
                .any(|entry| entry.key == ItemKey::Instance(id(71)))
        );

        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn set_level(h: &Harness, level: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query("UPDATE game_character_progression_state SET level=$2 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(level).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(result.rows_affected(), 1);
    Ok(())
}

async fn health(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
) -> TestResult<u32> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    Ok(states
        .get(&runtime, deps.actor, deps.admitted.game_session_id)
        .ok_or("spell state")?
        .vitals()
        .health)
}
