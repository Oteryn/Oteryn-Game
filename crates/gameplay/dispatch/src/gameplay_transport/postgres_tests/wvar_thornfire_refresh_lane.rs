//! Regression: terrain publication under an already-owned native source lane.
use super::*;

#[test]
fn wvar_thornfire_bundle_terrain_refresh_reuses_held_lane() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(|| {
        run(async |admin| {
            let started=std::time::Instant::now();reset();
            let harness=Harness::create_with_admission_seed(admin,"wvar_thorn_refresh",true,false).await?;
            prepare_current_session(&harness).await?;
            let seal=harness.recovery.seal_current().map_err(debug)?;
            let authority=harness.root.open_character_authority(&seal).await.map_err(debug)?;
        let deps = Dependencies::new()?;
        let (controller, _pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let water = "oteryn:item.tibia.i4597";
        let bag = "oteryn:item.tibia.i2854";
        let rod = "oteryn:item.tibia.i3483";
        for key in [
            rod,
            water,
            "oteryn:item.tibia.i3492",
            "oteryn:item.tibia.i3578",
        ] {
            assert!(
                native.item_policy(key, "definition-r1").is_some(),
                "native fishing profile {key}"
            );
        }
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (water, reference(water).ok_or("water")?.get() - 1),
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
            let mut owner=deps.owner(&harness,&authority);
            owner.active_generation=Some(active);
            owner.movement_cells=&cells;
            assert!(owner.bundle_map().is_some(),"actual Bundle map prevents the fixture-map early return");
            let permit=owner.spell_lane_permit().await.ok_or("real held native source lane")?;
            tokio::time::timeout(std::time::Duration::from_secs(2),owner.refresh_terrain_tools_in_lane(&permit)).await.map_err(|_|"terrain refresh recursively waited for its already-held lane")?;
            drop(permit);drop(owner);drop(authority);drop(seal);
            harness.cleanup().await?;
            println!("WVAR_THORNFIRE_TERRAIN_LANE POSTGRES_CASE_COMPLETED actual-bundle,held-source-lane,terrain-snapshot,no-recursive-acquire; duration={:?}",started.elapsed());
            Ok(())
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"Thornfire held-lane PG thread")??;
    Ok(())
}
