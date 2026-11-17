//! Compile the actual generated provider, rather than a hand-built elemental policy.
#[test]
fn engenc_authored_elementals_activate_with_exact_combat_facts() -> Result<(), String> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| -> Result<(), String> {
        let world = crate::foundation::WorldId::decode(&[
            1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 90,
        ]).map_err(|e| format!("{e:?}"))?;
        let (controller, _) = oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let native = controller.active().and_then(|g| g.native_gameplay()).ok_or("native content")?;
        let policies = native.policies().policies();
        for (source, target, name) in [
            ("massive_fire_elemental", "greater_fire_elemental", "Greater Fire Elemental"),
            ("massive_energy_elemental", "greater_energy_elemental", "Greater Energy Elemental"),
        ] {
            let find = |slug: &str| policies.iter().find(|p| p.definition_key == format!("oteryn:creature.{slug}"));
            let base = find(source).ok_or("baseline policy")?;
            let greater = find(target).ok_or("authored policy")?;
            assert_eq!(greater.display_name, name);
            assert_eq!(greater.maximum_health, base.maximum_health);
            assert_eq!(greater.resistances, base.resistances);
            assert_eq!(greater.armor, base.armor);
            assert_eq!(greater.outfit_look_type, base.outfit_look_type);
            assert!(greater.flags.attackable);
            assert_eq!(greater.forge_difficulty, None);
        }
        Ok(())
    }).map_err(|e| e.to_string())?.join().map_err(|_| "elemental activation thread failed".to_string())?
}
