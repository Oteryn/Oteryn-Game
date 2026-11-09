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
    assert!(!link.archive_assign(0, 36));
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
    assert_eq!(archive.slots.len(), 36);
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
            vec!["{\"kind\":\"Empty\"}"; 37].join(",")
        ))
        .is_err()
    );
    Ok(())
}
