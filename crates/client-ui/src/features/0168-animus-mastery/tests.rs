use super::*;
use oteryn_client_runtime::animus_mastery::encode_page;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn animus_pages_require_selection_and_advance_only_after_confirmed_replies()
-> Result<(), &'static str> {
    let mut panel = AnimusPanel::default();
    assert!(request(&mut panel, Intent::Refresh).is_none());
    panel.selected = true;
    assert!(request(&mut panel, Intent::Next).is_none());
    assert_eq!(
        request(&mut panel, Intent::Refresh),
        Some(vec![0, 0, 0, 50])
    );
    assert!(request(&mut panel, Intent::Refresh).is_none());
    let first = (0..50)
        .map(|index| format!("oteryn:creature.{index:02}"))
        .collect::<Vec<_>>();
    assert!(panel.finish(true, &encode_page(51, &first).ok_or("first page")?));
    assert_eq!(panel.keys, first);
    assert!(!panel.has_previous());
    assert!(panel.has_next());
    assert_eq!(request(&mut panel, Intent::Next), Some(vec![0, 50, 0, 50]));
    let last = vec!["oteryn:creature.dragon".into()];
    assert!(panel.finish(true, &encode_page(51, &last).ok_or("last page")?));
    assert_eq!(panel.offset, 50);
    assert_eq!(panel.keys, last);
    assert!(!panel.has_next());
    assert!(request(&mut panel, Intent::Next).is_none());
    assert!(panel_text(&panel).contains("Species 51-51 of 51"));
    assert!(panel_text(&panel).contains("dragon"));
    assert_eq!(
        request(&mut panel, Intent::Previous),
        Some(vec![0, 0, 0, 50])
    );
    assert!(panel.finish(true, &encode_page(51, &first).ok_or("previous page")?));
    assert_eq!(panel.offset, 0);
    Ok(())
}

#[test]
fn animus_refusal_or_malformed_reply_preserves_confirmed_page_and_allows_refresh()
-> Result<(), &'static str> {
    let mut panel = AnimusPanel {
        selected: true,
        ..AnimusPanel::default()
    };
    assert!(request(&mut panel, Intent::Refresh).is_some());
    let bytes = encode_page(1, &["oteryn:creature.rat".into()]).ok_or("page")?;
    assert!(panel.finish(true, &bytes));
    for (accepted, reply) in [(false, bytes.as_slice()), (true, &[1, 0, 1][..])] {
        assert!(request(&mut panel, Intent::Refresh).is_some());
        assert!(!panel.finish(accepted, reply));
        assert_eq!(panel.keys, ["oteryn:creature.rat"]);
        assert_eq!(panel.total, 1);
        assert!(panel.pending.is_none());
        assert!(panel.error.is_some());
    }
    assert!(!panel.finish(true, &bytes));
    assert!(request(&mut panel, Intent::Refresh).is_some());
    assert!(panel.finish(true, &bytes));
    assert!(panel.error.is_none());
    Ok(())
}

#[test]
fn animus_empty_page_and_requested_offset_are_bounded() {
    let mut panel = AnimusPanel {
        selected: true,
        ..AnimusPanel::default()
    };
    assert!(request(&mut panel, Intent::Refresh).is_some());
    assert!(panel.finish(true, &[0, 0, 0]));
    assert!(panel_text(&panel).contains("No completed species"));
    assert!(!panel.has_next());
    assert!(!panel.has_previous());
    panel.pending = Some(AnimusPage {
        offset: u16::MAX,
        limit: 50,
    });
    assert!(!panel.finish(true, &[1, 0, 1, 1, b'a']));
    assert_eq!(panel.total, 0);
}

#[test]
fn animus_hotkey_requires_the_exact_nonrepeating_control_a_chord()
-> Result<(), Box<dyn std::error::Error>> {
    let event = |code, state, repeat, modifiers| -> Result<_, Box<dyn std::error::Error>> {
        Ok(NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state,
            modifiers,
            repeat,
        })
    };
    assert!(open_key(&[event(
        4,
        ButtonState::Pressed,
        false,
        Modifiers::one(Modifier::Control)
    )?]));
    assert!(!open_key(&[event(
        4,
        ButtonState::Pressed,
        true,
        Modifiers::one(Modifier::Control)
    )?]));
    assert!(!open_key(&[event(
        4,
        ButtonState::Released,
        false,
        Modifiers::one(Modifier::Control)
    )?]));
    assert!(!open_key(&[event(
        5,
        ButtonState::Pressed,
        false,
        Modifiers::one(Modifier::Control)
    )?]));
    assert!(!open_key(&[event(
        4,
        ButtonState::Pressed,
        false,
        Modifiers::one(Modifier::Alt)
    )?]));
    Ok(())
}
