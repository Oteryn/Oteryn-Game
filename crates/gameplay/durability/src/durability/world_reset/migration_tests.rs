//! A later migration cannot silently drop an admitted reset proof arm (D1453).

#[test]
fn world_reset_migration_preserves_the_latest_guard_definitions()
-> Result<(), Box<dyn std::error::Error>> {
    let reset = oteryn_gameplay_durability::GAME_MIGRATOR
        .iter()
        .find(|migration| migration.version == 137)
        .ok_or("allocated reset migration")?;
    for guard in [
        "game_item_decay_retire_consistency_guard",
        "game_item_instance_change_proven",
        "game_item_ground_removal_proven",
        "game_item_corpse_entry_removal_proven",
    ] {
        assert!(reset.sql.as_str().contains(guard));
        for later in oteryn_gameplay_durability::GAME_MIGRATOR
            .iter()
            .filter(|migration| migration.version > 137)
        {
            if later.sql.as_str().contains(&format!("FUNCTION {guard}(")) {
                assert!(
                    later.sql.as_str().contains("game_world_reset_item_proven"),
                    "migration {} replaces {guard} without the reset arm",
                    later.version
                );
            }
        }
    }
    assert!(reset.sql.as_str().contains("pg_get_functiondef"));
    assert!(
        reset
            .sql
            .as_str()
            .contains("SECURITY DEFINER SET search_path")
    );
    Ok(())
}
