use crate::play::{SettingsState, play_channel};
use oteryn_client_runtime::spell_catalog::spellbook_details::{DetailedSpell, Page};
use oteryn_session::{CommandStatus, GatedCommandResult, spell_catalog::SpellCatalogEntry};

#[test]
fn magical_archive_assignments_persist_and_resolve_current_server_indices()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    let mut spell = SpellCatalogEntry {
        cast_index: 71,
        name: "Mending".into(),
        words: "mend".into(),
        level: 20,
        mana: 15,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![2],
        rune_item: None,
    };
    outbox.spellbook_details_initialize(true);
    outbox.spell_catalog_initialize(
        true,
        Some(
            oteryn_session::spell_catalog::encode_catalog(&[spell.clone()])
                .map_err(|_| "catalog")?,
        ),
    )?;
    assert!(!link.archive_assign(0, 0));
    assert!(link.spellbook_request(Default::default(), 0));
    let _ = commands.try_recv()?;
    outbox.spellbook_details_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: Page {
            page: 0,
            total: 1,
            entries: vec![DetailedSpell {
                spell: spell.clone(),
                vocations: 1,
                premium: false,
                magic_level: 0,
            }],
        }
        .encode()
        .ok_or("page")?,
    });
    assert!(link.archive_assign(0, 0));
    assert!(!link.archive_assign(1, 0));
    assert!(link.archive_assign(0, 11));
    assert!(link.archive_assign(0, oteryn_client_runtime::spell::ACTION_BAR_SLOTS - 1));
    assert!(!link.archive_assign(0, oteryn_client_runtime::spell::ACTION_BAR_SLOTS));
    let path = std::env::temp_dir().join(format!("archive-bindings-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.sync_archive_bindings(&link);
    let saved = settings.preferences.archive_bindings.clone();
    assert_eq!(
        SettingsState::new(Some(path.clone()))
            .preferences
            .archive_bindings,
        saved
    );
    spell.cast_index = 88;
    outbox.spell_catalog_initialize(
        true,
        Some(
            oteryn_session::spell_catalog::encode_catalog(&[spell.clone()])
                .map_err(|_| "catalog")?,
        ),
    )?;
    crate::play::feature_local_spell_action_bar::input::send_cast(
        &link,
        std::num::NonZeroU32::new(1).ok_or("slot")?,
    );
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get()==88)
    );
    spell.name = "Different spell".into();
    outbox.spell_catalog_initialize(
        true,
        Some(oteryn_session::spell_catalog::encode_catalog(&[spell]).map_err(|_| "catalog")?),
    )?;
    crate::play::feature_local_spell_action_bar::input::send_cast(
        &link,
        std::num::NonZeroU32::new(1).ok_or("slot")?,
    );
    assert!(
        commands.try_recv().is_err(),
        "a reused cast index must never cast a different spell"
    );
    assert!(link.archive_clear(0));
    settings.sync_archive_bindings(&link);
    assert_ne!(settings.preferences.archive_bindings, saved);
    assert!(link.spellbook_request(Default::default(), 0));
    assert!(
        !link.archive_assign(0, 0),
        "pending pages cannot be assigned"
    );
    let mut restored = settings.preferences.clone();
    restored.archive_bindings = saved;
    settings.apply(restored);
    settings.sync_archive_bindings(&link);
    settings.reset_defaults()?;
    settings.sync_archive_bindings(&link);
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox)
            .spell_bar
            .bar
            .fields()[0],
        "1"
    );
    assert!(link.spell_bar_bind(&["73".into(), "".into(), "3".into(), "4".into()]));
    settings.sync_archive_bindings(&link);
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox)
            .spell_bar
            .bar
            .fields()[0],
        "73"
    );
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn four_slot_saved_archive_migrates_to_the_full_row_without_losing_bindings()
-> Result<(), Box<dyn std::error::Error>> {
    let saved = r#"{"slots":[{"kind":"Empty"},{"kind":"Unmanaged"},{"kind":"Empty"},{"kind":"Unmanaged"}]}"#;
    let archive: super::archive::ArchiveBindings = serde_json::from_str(saved)?;
    assert_eq!(
        archive.slots.len(),
        oteryn_client_runtime::spell::ACTION_BAR_SLOTS
    );
    assert!(matches!(
        archive.slots[0],
        super::archive::ArchiveSlot::Empty
    ));
    assert!(matches!(
        archive.slots[11],
        super::archive::ArchiveSlot::Unmanaged
    ));
    assert!(
        serde_json::from_str::<super::archive::ArchiveBindings>(&format!(
            "{{\"slots\":[{}]}}",
            vec!["{\"kind\":\"Empty\"}"; oteryn_client_runtime::spell::ACTION_BAR_SLOTS + 1]
                .join(",")
        ))
        .is_err()
    );
    Ok(())
}

#[test]
fn saved_rune_display_names_restore_only_verified_legacy_identities()
-> Result<(), Box<dyn std::error::Error>> {
    for (
        saved_name,
        current_name,
        saved_item,
        current_item,
        saved_words,
        current_words,
        accepted,
    ) in [
        (
            "antidote rune",
            "Cure Poison Rune",
            Some(3153),
            Some(3153),
            "",
            "",
            true,
        ),
        (
            "desintegrate rune",
            "Disintegrate Rune",
            Some(3197),
            Some(3197),
            "",
            "",
            true,
        ),
        (
            "energybomb rune",
            "Energy Bomb Rune",
            Some(3149),
            Some(3149),
            "",
            "",
            true,
        ),
        (
            "firebomb rune",
            "Fire Bomb Rune",
            Some(3192),
            Some(3192),
            "",
            "",
            true,
        ),
        (
            "animate dead rune",
            "Animate Dead Rune",
            Some(3203),
            Some(3203),
            "",
            "",
            true,
        ),
        (
            "Cure Poison Rune",
            "antidote rune",
            Some(3153),
            Some(3153),
            "",
            "",
            true,
        ),
        (
            "antidote rune",
            "Unknown Rune",
            Some(3153),
            Some(3153),
            "",
            "",
            false,
        ),
        (
            "antidote rune",
            "Disintegrate Rune",
            Some(3153),
            Some(3153),
            "",
            "",
            false,
        ),
        (
            "antidote rune",
            "Cure Poison Rune",
            Some(3197),
            Some(3197),
            "",
            "",
            false,
        ),
        (
            "antidote rune",
            "Cure Poison Rune",
            Some(3153),
            Some(3154),
            "",
            "",
            false,
        ),
        (
            "antidote rune",
            "Cure Poison Rune",
            Some(3153),
            Some(3153),
            "changed words",
            "",
            false,
        ),
        (
            "Cure Poison Rune",
            "Cure Poison Rune",
            Some(3153),
            None,
            "",
            "test conjure",
            false,
        ),
        ("Mending", "mending", None, None, "mend", "mend", false),
        (
            "Mending",
            "Different Spell",
            None,
            None,
            "mend",
            "mend",
            false,
        ),
        (
            "antidote rune ",
            "Cure Poison Rune",
            Some(3153),
            Some(3153),
            "",
            "",
            false,
        ),
    ] {
        let (link, mut commands, outbox) = play_channel();
        let spell = SpellCatalogEntry {
            cast_index: 88,
            name: current_name.into(),
            words: current_words.into(),
            level: 8,
            mana: 0,
            mana_percent: false,
            soul: 0,
            learned: true,
            available: true,
            cooldown_groups: vec![1],
            rune_item: current_item,
        };
        let mut saved: super::archive::ArchiveBindings = serde_json::from_value(
            serde_json::json!({
                "slots": [{"kind":"Spell", "spell": {"name":saved_name, "words":saved_words, "rune_item":saved_item}}]
            }),
        )?;
        assert!(saved.valid());
        link.sync_archive_bindings(&mut saved, 1);
        outbox.spell_catalog_initialize(
            true,
            Some(oteryn_session::spell_catalog::encode_catalog(&[spell]).map_err(|_| "catalog")?),
        )?;
        crate::play::feature_local_spell_action_bar::input::send_cast(
            &link,
            std::num::NonZeroU32::new(1).ok_or("slot")?,
        );
        let command = commands.try_recv();
        if accepted {
            assert!(
                matches!(command?, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get()==88),
                "{saved_name} -> {current_name}"
            );
        } else {
            assert!(
                command.is_err(),
                "unexpected binding: {saved_name} -> {current_name}"
            );
        }
    }
    Ok(())
}

#[test]
fn saved_rune_aliases_refuse_ambiguous_current_catalog() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    let mut saved: super::archive::ArchiveBindings = serde_json::from_value(serde_json::json!({
        "slots": [{"kind":"Spell", "spell": {"name":"antidote rune", "words":"", "rune_item":3153}}]
    }))?;
    link.sync_archive_bindings(&mut saved, 1);
    let spell = SpellCatalogEntry {
        cast_index: 71,
        name: "Cure Poison Rune".into(),
        words: String::new(),
        level: 8,
        mana: 0,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![1],
        rune_item: Some(3153),
    };
    let mut duplicate = spell.clone();
    duplicate.cast_index = 88;
    duplicate.name = "antidote rune".into();
    outbox.spell_catalog_initialize(
        true,
        Some(
            oteryn_session::spell_catalog::encode_catalog(&[spell, duplicate])
                .map_err(|_| "catalog")?,
        ),
    )?;
    crate::play::feature_local_spell_action_bar::input::send_cast(
        &link,
        std::num::NonZeroU32::new(1).ok_or("slot")?,
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn projected_rune_names_keep_source_professions_through_real_catalog_filter()
-> Result<(), Box<dyn std::error::Error>> {
    for (item, legacy, display) in [
        (3203, "animate dead rune", "Animate Dead Rune"),
        (3153, "antidote rune", "Cure Poison Rune"),
        (3161, "avalanche rune", "Avalanche Rune"),
        (3178, "chameleon rune", "Chameleon Rune"),
        (3177, "convince creature rune", "Convince Creature Rune"),
        (3197, "desintegrate rune", "Disintegrate Rune"),
        (3148, "destroy field rune", "Destroy Field Rune"),
        (3164, "energy field rune", "Energy Field Rune"),
        (3166, "energy wall rune", "Energy Wall Rune"),
        (3149, "energybomb rune", "Energy Bomb Rune"),
        (3200, "explosion rune", "Explosion Rune"),
        (3188, "fire field rune", "Fire Field Rune"),
        (3190, "fire wall rune", "Fire Wall Rune"),
        (3189, "fireball rune", "Fireball Rune"),
        (3192, "firebomb rune", "Fire Bomb Rune"),
        (3191, "great fireball rune", "Great Fireball Rune"),
        (3198, "heavy magic missile rune", "Heavy Magic Missile Rune"),
        (3182, "holy missile rune", "Holy Missile Rune"),
        (3158, "icicle rune", "Icicle Rune"),
        (3152, "intense healing rune", "Intense Healing Rune"),
        (3180, "Magic Wall Rune", "Magic Wall Rune"),
        (3165, "paralyze rune", "Paralyze Rune"),
        (3173, "poison bomb rune", "Poison Bomb Rune"),
        (3172, "poison field rune", "Poison Field Rune"),
        (3176, "poison wall rune", "Poison Wall Rune"),
        (3195, "soulfire rune", "Soulfire Rune"),
        (3179, "stalagmite rune", "Stalagmite Rune"),
        (3175, "stone shower rune", "Stone Shower Rune"),
        (3155, "sudden death rune", "Sudden Death Rune"),
        (3202, "thunderstorm rune", "Thunderstorm Rune"),
        (3160, "ultimate healing rune", "Ultimate Healing Rune"),
        (3156, "Wild Growth Rune", "Wild Growth Rune"),
    ] {
        let (link, _commands, outbox) = play_channel();
        let legacy_entry = SpellCatalogEntry {
            cast_index: 71,
            name: legacy.into(),
            words: String::new(),
            level: 8,
            mana: 0,
            mana_percent: false,
            soul: 0,
            learned: true,
            available: false,
            cooldown_groups: vec![1],
            rune_item: Some(item),
        };
        let mut displayed = legacy_entry.clone();
        displayed.cast_index = 88;
        displayed.name = display.into();
        let expected = oteryn_client_ui::spell_catalog::professions(&legacy_entry);
        assert!(!expected.is_empty(), "missing source professions: {legacy}");
        assert_eq!(
            oteryn_client_ui::spell_catalog::professions(&displayed),
            expected,
            "{legacy} -> {display}"
        );
        outbox.spell_catalog_initialize(
            true,
            Some(
                oteryn_session::spell_catalog::encode_catalog(&[legacy_entry, displayed])
                    .map_err(|_| "catalog")?,
            ),
        )?;
        for vocation in expected {
            assert!(link.spell_catalog_search("", "", "", false, vocation));
            let panel = link.spell_catalog_panel();
            assert!(
                oteryn_client_ui::spell_catalog::panel_text(&panel).contains("Spells: 2"),
                "{display}, {vocation}"
            );
            assert!(!panel.selected_spell().ok_or("filtered spell")?.available);
        }
    }
    Ok(())
}
