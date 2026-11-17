//! M41 exercises the real craft owner, immutable corpse provenance and source NPC finales.
use super::*;
#[path = "m41/finales.rs"]
mod finales;
#[path = "m41/quest_fixture.rs"]
mod quest_fixture;
#[path = "m41/skinning.rs"]
mod skinning;
#[path = "m41/story.rs"]
mod story;

#[test]
fn m41_stag_self_skinning_provenance_and_npc_achievement_finales_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M41 provenance PG panic")??;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "M41_PG_EXECUTED stag_skinning_and_two_source_npc_finales duration={:?}",
        started.elapsed()
    );
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "m41_stag_beaks", true, false).await?;
        super::super::reset();
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
        let station = "oteryn:item.tibia.i3600";
        let bag = "oteryn:item.tibia.i2854";
        let tool = "oteryn:item.tibia.i5908";
        for key in [
            tool,
            station,
            "oteryn:item.tibia.i5969",
            "oteryn:item.tibia.i4012",
            "oteryn:item.tibia.i5878",
        ] {
            assert!(
                native.item_policy(key, "definition-r1").is_some(),
                "native skinning profile {key}"
            );
        }
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (station, reference(station).ok_or("water")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let boot_pins = crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 6,
                y: 0,
                floor: -7,
            },
        };
        // The production boot policy: an unknown fact blocks and is not pickupable.
        let definitions = |key: &str| {
            let policy = native.item_policy(key, "definition-r1").map(|p| p.record());
            Some(crate::map::facts::ItemDefinition {
                reference: reference(key)?,
                solid: policy.and_then(|p| p.attributes.blocks_movement),
                blocks_projectile: policy.and_then(|p| p.attributes.blocks_projectile).unwrap_or(true),
                pickupable: policy.is_some_and(|p| matches!(&p.semantics.physical, crate::content::ReferenceItemField::Known(physical) if physical.pickupable == crate::content::ReferenceItemField::Known(true))),
                door: false,
            })
        };
        let map = crate::map::boot::boot(
            &bytes,
            &boot_pins,
            deps.admitted.world_id,
            deps.admitted.channel_id,
            definitions,
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
        super::super::item_look::seed_main_backpack(
            &harness,
            &[
                (tool, 1),
                ("oteryn:item.tibia.i5942", 1),
                ("oteryn:item.tibia.i52632", 20),
            ],
        )
        .await?;

        story::exercise(&owner, &harness, native, fence, deps.actor).await?;
        Ok(())
    })
}
