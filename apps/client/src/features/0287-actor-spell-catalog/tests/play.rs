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
    let capacity = crate::play::mailbox::lock(&link.mailbox)
        .spell_bar
        .bar
        .fields()
        .len();
    assert!(link.spell_catalog_assign(capacity - 1));
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox)
            .spell_bar
            .bar
            .fields()[capacity - 1],
        "19"
    );
    assert!(!link.spell_catalog_assign(capacity));
    assert!(commands.try_recv().is_err());
    outbox.logged_out();
    assert!(!link.spell_catalog_assign(2));
    Ok(())
}

#[test]
fn o059_detail_card_consumes_server_cooldown_observation() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, _, outbox) = play_channel();
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
    outbox.push(vec![SessionEvent::ActorCooldowns(AppliedDelta {
        server_sequence: 3,
        base_revision: 1,
        new_revision: 2,
        value: oteryn_session::ActorCooldowns {
            server_now_ms: 100,
            rtt_ms: None,
            entries: vec![oteryn_session::ActorCooldown {
                kind: oteryn_session::ActorCooldownKind::Group,
                id: 1,
                expires_at_ms: 350,
            }],
        },
    })]);
    let mailbox = crate::play::mailbox::lock(&link.mailbox);
    let text = oteryn_client_ui::spell_catalog::detail_text(
        &mailbox.spell_catalog,
        &mailbox.spell_bar.bar,
    );
    assert!(text.contains("Cooldown: 250 ms (last server observation)"));
    Ok(())
}

#[test]
fn o080_archive_has_its_own_window_lifetime_and_menu_request()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox) = play_channel();
    outbox.spell_catalog_initialize(true, Some(Vec::new()))?;
    let mut settings = crate::play::SettingsState::new(None);
    settings.request_magical_archive();
    assert!(settings.take_magical_archive_request());
    assert!(!settings.take_magical_archive_request());
    link.magical_archive_open();
    assert!(link.spell_catalog_panel().archive_open);
    assert!(!link.spell_catalog_panel().open);
    outbox.logged_out();
    assert!(!link.spell_catalog_panel().archive_open);
    Ok(())
}

#[path = "profession.rs"]
mod profession;
