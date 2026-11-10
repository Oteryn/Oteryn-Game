//! A later migration cannot silently drop an admitted reset proof arm (D1453).

#[test]
fn world_reset_migration_preserves_the_latest_guard_definitions()
-> Result<(), Box<dyn std::error::Error>> {
    let reset = oteryn_gameplay_durability::GAME_MIGRATOR
        .iter()
        .find(|migration| migration.version == 148)
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
            .filter(|migration| migration.version > 148)
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

#[test]
fn death_tree_retirement_adds_private_source_proofs_without_replacing_authority_checks()
-> Result<(), Box<dyn std::error::Error>> {
    let migration = oteryn_gameplay_durability::GAME_MIGRATOR
        .iter()
        .find(|migration| migration.version == 2731)
        .ok_or("death tree retirement migration")?;
    let sql = migration.sql.as_str();
    for guard in [
        "game_world_reset_item_proven",
        "game_activity_retirement_item_proven",
    ] {
        assert!(sql.contains(&format!("'{guard}(uuid)'")));
        assert!(!sql.contains(&format!("CREATE OR REPLACE FUNCTION {guard}")));
    }
    for invariant in [
        "pg_get_functiondef(to_regprocedure(f))",
        "NOT game_player_death_retirement_entry_proven(p_item)",
        "parent.created_xact_id=pg_current_xact_id()",
        "root.created_xact_id=pg_current_xact_id()",
        "root.fence_scope_ownership_generation",
        "root.activity_handoff_id,root.activity_ownership_generation",
        "root.reset_epoch,root.retirement_binding,root.target_digest",
        "CASE d.source_slot WHEN 9 THEN 8 WHEN 0 THEN 7 ELSE 1 END",
        "CASE d.source_slot WHEN 9 THEN 500 WHEN 0 THEN 499 ELSE 20 END",
        "d.source_slot NOT IN(6,10) OR root_item.definition_production_key IN(",
        "REVOKE ALL ON game_player_death_retirement_edges FROM PUBLIC,oteryn_game_runtime,oteryn_game_control",
        "SECURITY DEFINER SET search_path=pg_catalog,%I,pg_temp",
    ] {
        assert!(
            sql.contains(invariant),
            "missing retirement proof invariant: {invariant}"
        );
    }
    assert!(!sql.contains("GRANT INSERT ON game_player_death_retirement_edges"));
    assert!(!sql.contains("DISABLE TRIGGER"));
    for key in super::ground_step::EQUIPPED_QUIVERS {
        assert!(sql.contains(key), "missing admitted equipped quiver: {key}");
    }
    Ok(())
}

#[test]
fn death_tree_retirement_preserves_the_existing_corpse_capacity()
-> Result<(), Box<dyn std::error::Error>> {
    let corpse_max = crate::foundation::loot_data::CORPSE_CONTAINER_ENTRIES_MAX;
    let corpse_capacity = oteryn_gameplay_durability::GAME_MIGRATOR
        .iter()
        .find(|migration| migration.version == 242)
        .ok_or("corpse capacity migration")?;
    assert_eq!(
        corpse_capacity
            .sql
            .as_str()
            .matches(&format!(
                "CHECK (placement_ordinal BETWEEN 1 AND {corpse_max})"
            ))
            .count(),
        2,
        "the reservation and receipt ceilings must match the admitted corpse capacity"
    );
    let retirement = oteryn_gameplay_durability::GAME_MIGRATOR
        .iter()
        .find(|migration| migration.version == 2731)
        .ok_or("death tree retirement migration")?;
    assert!(retirement.sql.as_str().contains(&format!(
        "AND (cause_kind<>''CORPSE_DECAY'' OR placement_ordinal<={corpse_max})"
    )));
    assert!(
        retirement
            .sql
            .as_str()
            .contains(&format!("placement_ordinal BETWEEN 1 AND {}", u64::MAX))
    );
    Ok(())
}
