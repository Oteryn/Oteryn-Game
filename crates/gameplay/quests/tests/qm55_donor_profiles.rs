use oteryn_gameplay_quests::quest::authored_bindings::{
    PersonalEventProfile as P, personal_event_profile,
};

#[test]
fn qm55_empty_donor_missions_are_typed_unavailable_instead_of_daily() -> Result<(), String> {
    for quest in [
        "bank_robbery_mini_world_change",
        "chakoya_iceberg_mini_world_change",
        "devovorga_s_essence_mini_world_change",
        "down_the_drain_mini_world_change",
        "fire_from_the_earth_mini_world_change",
        "hive_born_world_change",
        "hive_outpost_mini_world_change",
        "jungle_camp_mini_world_change",
        "kingsday_mini_world_change",
        "lumberjack_mini_world_change",
    ] {
        let P::Unavailable { source, line } =
            personal_event_profile(&format!("oteryn:quest.authored.{quest}"))
                .ok_or("missing donor profile")?
        else {
            return Err(format!("donor mission must not be offered: {quest}"));
        };
        assert!(source.starts_with("crystal:201189f871d23597b82caf88fd611454a2ddefb8:"));
        assert!(line > 0);
    }
    assert!(matches!(
        personal_event_profile("oteryn:quest.awash_world_change"),
        Some(P::SharedWorldOnly { .. })
    ));
    assert!(matches!(
        personal_event_profile("oteryn:quest.authored.demon_wars_world_change"),
        Some(P::SharedWorldOnly { .. })
    ));
    for quest in [
        "oteryn:quest.fury_gates_mini_world_change",
        "oteryn:quest.horse_station_world_change",
        "oteryn:quest.authored.insectoid_invasion_world_change",
    ] {
        assert!(matches!(
            personal_event_profile(quest),
            Some(P::SharedWorldOnly { .. })
        ));
    }
    for quest in [
        "oteryn:quest.bored_mini_world_change",
        "oteryn:quest.chyllfroest_mini_world_change",
    ] {
        assert!(matches!(
            personal_event_profile(quest),
            Some(P::Unavailable { .. })
        ));
    }
    assert!(matches!(
        personal_event_profile("oteryn:quest.deeplings_worldchange"),
        Some(P::Counter {
            helper: "oteryn:quest.deeplings_native_shared"
        })
    ));
    assert_eq!(personal_event_profile("oteryn:quest.unrelated"), None);
    Ok(())
}

#[test]
fn qm55_canonical_aliases_are_declared_in_the_production_catalogue() -> Result<(), String> {
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    for quest in [
        "oteryn:quest.authored.hive_born_world_change",
        "oteryn:quest.deeplings_worldchange",
    ] {
        assert!(
            loaded.catalogue().definition_hash(quest).is_some(),
            "canonical completion needs a production declaration: {quest}"
        );
    }
    Ok(())
}
