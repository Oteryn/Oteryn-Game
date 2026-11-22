use crate::bestiary_postgres_harness::{TestResult, debug};
use crate::map::{boot::BundleWorld, overlay::TilePos, view::MapFacts};
use crate::node::serve::daily_world_changes::tests::{boot_physical, world};
use oteryn_gameplay_world_change::{daily::DailyWorldChanges, variants::Selected};

fn day(epoch: u64, enabled: &[&str], last: &[&str]) -> TestResult<DailyWorldChanges> {
    let mut day = DailyWorldChanges::roll(epoch, 1, 1, |_| u64::MAX);
    day.variants = Selected::select(|purpose| {
        let mut parts = purpose.split(':');
        let _ = parts.next();
        let definition = parts.next();
        let field = parts.next();
        match field {
            Some("chance") if definition.is_some_and(|d| enabled.contains(&d)) => 0,
            Some("variant") if !definition.is_some_and(|d| last.contains(&d)) => 0,
            _ => u64::MAX,
        }
    })?;
    Ok(day)
}

fn appearances(map: &BundleWorld, x: u16, y: u16, floor: i8) -> Vec<u16> {
    map.base().tile(x, y, floor).into_iter().flat_map(|tile| {
        tile.ids().iter().enumerate().filter_map(move |(index, id)| {
            map.facts().base_entry(TilePos { x, y, floor }, u8::try_from(index).ok()?, *id)
                .map(|entry| entry.appearance_id)
        })
    }).collect()
}

fn roster<'a>(map: &'a BundleWorld, key: &str) -> Vec<&'a str> {
    map.daily_variant_spawns().find(|(family, _)| *family == key)
        .into_iter().flat_map(|(_, spawns)| spawns)
        .map(|spawn| spawn.creature.as_str()).collect()
}

#[test]
fn daily_variant_reset_rotates_jungle_camp_and_nomads_and_removes_chip() -> TestResult {
    let enabled = ["jungle_camp", "lumberjack", "nomads"];
    let first = boot_physical(&day(41, &enabled, &["chakoya_iceberg", "spider_nest"])?);
    let rotated = boot_physical(&day(42, &["jungle_camp", "nomads"], &["jungle_camp", "nomads", "chakoya_iceberg", "spider_nest"])?);
    assert_eq!(roster(&first, "jungle_camp"), ["oteryn:creature.hunter"; 2]);
    assert_eq!(
        roster(&rotated, "jungle_camp"),
        ["oteryn:creature.dworc_fleshhunter", "oteryn:creature.dworc_venomsniper"]
    );
    assert_eq!(roster(&first, "nomads").len(), 6);
    assert_eq!(roster(&rotated, "nomads").len(), 3);
    assert!(first.owns_daily_variant_npc("oteryn:npc.chip"));
    assert!(rotated.owns_daily_variant_npc("oteryn:npc.chip"));
    assert_eq!(first.daily_variant_npcs().collect::<Vec<_>>(), [("oteryn:npc.chip", (32418, 31699, 7))]);
    assert_eq!(rotated.daily_variant_npcs().count(), 0);
    assert_ne!(appearances(&first, 32417, 31695, -7), appearances(&rotated, 32417, 31695, -7));
    Ok(())
}

#[test]
fn daily_variant_reset_rotates_spirit_grounds_and_retains_the_return() -> TestResult {
    let first = boot_physical(&day(41, &[], &["chakoya_iceberg", "spider_nest"])?);
    let rotated = boot_physical(&day(42, &[], &["chakoya_iceberg", "spider_nest", "spirit_grounds"])?);
    assert!(first.facts().daily_variant_passage((33142, 32545, 1)));
    assert!(!rotated.facts().daily_variant_passage((33142, 32545, 1)));
    assert!(rotated.facts().daily_variant_passage((32179, 31778, 7)));
    assert!(first.facts().daily_variant_passage((32857, 32426, 10)));
    assert!(rotated.facts().daily_variant_passage((32857, 32426, 10)));
    assert_eq!(roster(&first, "spirit_grounds"), ["oteryn:creature.nightmare"; 4]);
    assert_eq!(roster(&rotated, "spirit_grounds"), ["oteryn:creature.phantasm"; 4]);
    Ok(())
}

#[test]
fn daily_variant_reset_restores_iceberg_drain_and_hive_projection() -> TestResult {
    let first = boot_physical(&day(41, &["down_the_drain", "hive_outpost"], &["spider_nest"])?);
    let closed = boot_physical(&day(42, &[], &["chakoya_iceberg", "spider_nest"])?);
    assert_eq!(roster(&first, "chakoya_iceberg").len(), 63);
    assert!(roster(&closed, "chakoya_iceberg").is_empty());
    assert!(first.base().tile(32619, 32549, -5).is_some());
    assert!(closed.base().tile(32619, 32549, -5).is_none());
    assert_eq!(roster(&first, "down_the_drain"), ["oteryn:creature.water_elemental"; 11]);
    assert!(roster(&closed, "down_the_drain").is_empty());
    assert!(first.facts().daily_variant_passage((32588, 32303, 7)));
    assert!(!closed.facts().daily_variant_passage((32588, 32303, 7)));
    assert!(closed.facts().daily_variant_passage((32589, 32303, 8)));
    assert_eq!(roster(&first, "hive_outpost").len(), 6);
    assert!(roster(&closed, "hive_outpost").is_empty());
    Ok(())
}

#[test]
fn daily_variant_reset_drains_fishing_water_and_seals_spider_nest() -> TestResult {
    let first = boot_physical(&day(41, &["river_runs_deep"], &["chakoya_iceberg"])?);
    let closed = boot_physical(&day(42, &[], &["chakoya_iceberg", "spider_nest"])?);
    assert!(appearances(&first, 33432, 31523, -11).contains(&13989));
    assert!(appearances(&closed, 33432, 31523, -11).contains(&231));
    assert!(!appearances(&closed, 33432, 31523, -11).contains(&13989));
    assert_eq!(roster(&first, "spider_nest"), ["oteryn:creature.mamma_longlegs"]);
    assert!(first.facts().daily_variant_passage((32817, 32280, 7)));
    assert!(!closed.facts().daily_variant_passage((32817, 32280, 7)));
    assert!(closed.facts().daily_variant_passage((32818, 32280, 8)));
    assert!(roster(&closed, "spider_nest").is_empty());
    Ok(())
}

#[test]
fn daily_variant_warpath_spawns_bibby_only_after_six_guard_deaths() -> TestResult {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        warpath().map_err(debug)
    })?.join().map_err(|_| "Warpath spawn test thread failed")??;
    Ok(())
}

fn warpath() -> TestResult {
    use crate::foundation::{ChannelContentPin, ChannelId, ChannelRuntimeV1, NodeId};
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    let map = boot_physical(&day(41, &["warpath"], &["chakoya_iceberg", "spider_nest"])?);
    let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(world(), None);
    let active = controller.active().ok_or("active runtime content")?;
    let native = active.native_gameplay().ok_or("native runtime content")?;
    let channel_pin = ChannelContentPin::from_activation(
        world(), pin.activation_sequence(), pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(), map.frame_binding_digest(world()),
        map.map_revision_digest(), (33330, 31670, 7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world(),
        ChannelId::decode(&crate::bestiary_postgres_harness::id(2)).map_err(debug)?,
        NodeId::decode(&crate::bestiary_postgres_harness::id(3)).map_err(debug)?,
        1, 1, 1, "runtime-scope-assignment:1", 128, channel_pin,
    ).map_err(debug)?;
    native.install_companion_policies(&mut runtime).map_err(debug)?;
    crate::bestiary_postgres_harness::runtime()?.block_on(crate::node::serve::realize_bundle_spawns(
        &mut runtime, &map, &oteryn_foundation::CancellationToken::new(),
    )).map_err(debug)?.ok_or("completed activation")?;
    let guards = runtime.spawn_point_creatures().into_iter().filter_map(|(_, _, actor)| {
        actor.filter(|actor| runtime.world_spawn_cohort(*actor).is_some_and(|(key, _, _)| key == "warpath"))
    }).collect::<Vec<_>>();
    assert_eq!(guards.len(), 6);
    let boss = runtime.spawn_point_creatures().into_iter()
        .find(|(_, cell, _)| (cell.x, cell.y, cell.floor) == (32622, 31826, 7))
        .ok_or("dormant Bibby source")?.0;
    for (ordinal, actor) in guards.into_iter().enumerate() {
        let health = runtime.companion_snapshot(actor).map_err(debug)?.health;
        runtime.commit_monster_lab_damage(actor, format!("guard-{ordinal}").as_bytes(), health).map_err(debug)?;
        let mut owner = runtime.borrow_combat_death();
        let receipt = owner.committed_lethal_receipt(actor).map_err(debug)?;
        owner.project_committed_lethal(receipt).map_err(debug)?;
        crate::gameplay_transport::drive_owner_spawns(&mut runtime, ordinal as u64 * 50_000);
        if ordinal < 5 {
            assert!(runtime.pending_respawn(boss).is_none());
        }
    }
    crate::gameplay_transport::drive_owner_spawns(&mut runtime, 5_000_000);
    let spawned = runtime.spawn_point_creatures().into_iter()
        .find(|(source, _, _)| *source == boss).and_then(|(_, _, actor)| actor)
        .ok_or("Bibby admitted after the last guard")?;
    assert_eq!(runtime.companion_snapshot(spawned).map_err(debug)?.state.policy.definition_key, "oteryn:creature.bibby_bloodbath");
    Ok(())
}
