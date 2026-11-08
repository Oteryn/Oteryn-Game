use crate::play::{Outbox, PlayLink, play_channel};
use oteryn_session::{AppliedDelta, GatedSnapshot, SessionEvent, spell_catalog::*};

#[test]
fn catalog_replacement_reaches_panel_and_disconnect_clears_it()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox): (PlayLink, _, Outbox) = play_channel();
    outbox.spell_catalog_initialize(true, Some(Vec::new()))?;
    let entry = SpellCatalogEntry {
        cast_index: 19,
        name: "Healing".into(),
        words: "exura".into(),
        level: 8,
        mana: 20,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![1],
        rune_item: None,
    };
    outbox.push(vec![SessionEvent::GatedSnapshotReplaced(AppliedDelta {
        server_sequence: 9,
        base_revision: 1,
        new_revision: 2,
        value: GatedSnapshot {
            domain_id: 100,
            revision: 2,
            snapshot_type: 1,
            payload: encode_catalog(&[entry]).map_err(|e| format!("{e:?}"))?,
        },
    })]);
    assert!(
        oteryn_client_ui::spell_catalog::panel_text(&link.spell_catalog_panel())
            .contains("19: Healing")
    );
    outbox.logged_out();
    assert!(!link.spell_catalog_panel().selected);
    assert!(link.spell_catalog_panel().catalog.entries().is_empty());
    Ok(())
}

#[test]
fn t033_selected_spell_assigns_slot_without_sending_a_cast()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    let entry = SpellCatalogEntry {
        cast_index: 19,
        name: "Healing".into(),
        words: "exura".into(),
        level: 8,
        mana: 20,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![1],
        rune_item: None,
    };
    outbox.spell_catalog_initialize(
        true,
        Some(encode_catalog(&[entry]).map_err(|e| format!("{e:?}"))?),
    )?;
    assert!(link.spell_catalog_assign(2));
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox)
            .spell_bar
            .bar
            .fields()[2],
        "19"
    );
    assert!(commands.try_recv().is_err());
    assert!(!link.spell_catalog_assign(4));
    outbox.logged_out();
    assert!(!link.spell_catalog_assign(2));
    Ok(())
}
