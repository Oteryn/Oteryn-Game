use crate::play::{BarAction, BarDock, BarGesture, SettingsState, play_channel};

#[test]
fn docked_row_viewport_stays_within_configured_buttons_at_every_offset() {
    let mut layout = crate::play::BarLayout::default();
    for row in &mut layout.rows {
        for count in 1..=50 {
            row.count = count;
            for offset in 0..count {
                row.offset = offset;
                for capacity in [1, 5, 12] {
                    let visible = row.visible_slots(capacity);
                    assert_eq!(visible.len(), capacity.min(usize::from(count)));
                    assert!(visible.end <= usize::from(count));
                    assert!(visible.contains(&usize::from(offset)));
                }
            }
        }
    }
}

#[test]
fn docked_bars_lock_swaps_scroll_and_persist_all_rows() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, _) = play_channel();
    assert!(link.assign_bar_action(
        0,
        BarAction::Text {
            text: "Saved action".into(),
            send_automatically: true
        }
    ));
    assert!(!link.bar_gesture(BarGesture::Swap { from: 0, to: 35 }));
    for (bar, dock) in [
        (0, BarDock::Bottom),
        (1, BarDock::Left),
        (2, BarDock::Right),
    ] {
        assert!(link.bar_gesture(BarGesture::Configure {
            bar,
            dock,
            enabled: true,
            locked: false
        }));
    }
    assert!(link.bar_gesture(BarGesture::Swap { from: 0, to: 35 }));
    assert!(
        commands.try_recv().is_err(),
        "layout edits must never trigger actions"
    );
    assert!(link.bar_gesture(BarGesture::Scroll { bar: 2, by: 1 }));
    let path = std::env::temp_dir().join(format!("bar-layout-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.sync_bar_actions(&link);
    settings.sync_archive_bindings(&link);
    let loaded = SettingsState::new(Some(path.clone()));
    assert_eq!(
        loaded.preferences.bar_layout,
        settings.preferences.bar_layout
    );
    assert_eq!(
        loaded.preferences.bar_actions.0.get(&35),
        Some(&BarAction::Text {
            text: "Saved action".into(),
            send_automatically: true
        })
    );
    assert_eq!(loaded.preferences.bar_layout.rows[2].offset, 1);
    assert!(!link.bar_gesture(BarGesture::Swap { from: 0, to: 150 }));
    assert!(!link.bar_gesture(BarGesture::Configure {
        bar: 3,
        dock: BarDock::Bottom,
        enabled: true,
        locked: true
    }));
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn third_bar_last_slot_uses_the_same_real_cast_caller() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, _) = play_channel();
    assert!(link.bar_gesture(BarGesture::Configure {
        bar: 2,
        dock: BarDock::Right,
        enabled: true,
        locked: true
    }));
    let mut fields = vec![String::new(); 150];
    fields[149] = "71".into();
    assert!(link.spell_bar_bind(&fields));
    assert!(link.bar_gesture(BarGesture::Cast { slot: 149 }));
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get()==71)
    );
    assert_eq!(link.spell_slot_labels().len(), 150);
    Ok(())
}

#[test]
fn action_amount_labels_follow_server_inventory_replacements()
-> Result<(), Box<dyn std::error::Error>> {
    use std::num::{NonZeroU32, NonZeroU64};
    let (link, _, outbox) = play_channel();
    outbox.hotkey_equip_items(vec![256]);
    assert!(link.assign_bar_action(
        0,
        BarAction::Equip {
            definition: NonZeroU32::new(256).ok_or("item")?,
            upgrade_tier: 0
        }
    ));
    assert!(link.spell_slot_labels()[0].contains("Visible: ?"));
    for count in [7, 2] {
        outbox.inventory_initialize(
            Some(oteryn_session::CharacterInventory {
                entries: vec![oteryn_session::ItemEntry {
                    handle: NonZeroU64::MIN,
                    item_definition_ref: NonZeroU32::new(256).ok_or("item")?,
                    count: NonZeroU32::new(count).ok_or("count")?,
                    sub_type: 0,
                }],
                ..Default::default()
            }),
            true,
        );
        assert!(link.spell_slot_labels()[0].contains(&format!("Visible: {count}")));
    }
    Ok(())
}

#[test]
fn fifty_button_count_is_configurable_persisted_and_keeps_hidden_assignments()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, _) = play_channel();
    assert!(link.assign_bar_action(
        49,
        BarAction::Text {
            text: "saved".into(),
            send_automatically: true
        }
    ));
    assert!(link.bar_gesture(BarGesture::Count { bar: 0, count: 12 }));
    assert!(!link.bar_gesture(BarGesture::Cast { slot: 49 }));
    assert!(!link.bar_gesture(BarGesture::Count { bar: 0, count: 51 }));
    assert!(!link.bar_gesture(BarGesture::Count { bar: 0, count: 0 }));
    let path = std::env::temp_dir().join(format!("bar-count-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.sync_bar_actions(&link);
    let saved = SettingsState::new(Some(path.clone()));
    assert_eq!(saved.preferences.bar_layout.rows[0].count, 12);
    assert!(saved.preferences.bar_actions.0.contains_key(&49));
    assert!(link.bar_gesture(BarGesture::Count { bar: 0, count: 50 }));
    assert!(link.spell_slot_labels()[49].contains("saved"));
    assert!(commands.try_recv().is_err());
    std::fs::remove_file(path)?;
    Ok(())
}
