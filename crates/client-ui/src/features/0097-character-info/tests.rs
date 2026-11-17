use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
#[test]
fn pc03_character_info_shortcut_and_request_gate() -> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(13)?;
    let modifiers = Modifiers::one(Modifier::Control).with(Modifier::Alt);
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    assert!(open_key(&[event(ButtonState::Pressed, modifiers, false)]));
    assert!(!open_key(&[event(ButtonState::Pressed, modifiers, true)]));
    assert!(!open_key(&[event(ButtonState::Released, modifiers, false)]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        modifiers.with(Modifier::Shift),
        false
    )]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        Modifiers::NONE,
        false
    )]));
    let mut panel = Panel::default();
    assert!(!request(&mut panel, 1));
    assert!(panel_text(&panel).contains("Unavailable"));
    panel.selected = true;
    assert!(request(&mut panel, 1));
    assert!(!request(&mut panel, 2));
    assert_eq!(panel.pending_kind, 1);
    assert!(panel_text(&panel).contains("Waiting"));
    panel.failed("The session has ended");
    assert!(!panel.pending);
    assert!(panel_text(&panel).contains("ended"));
    Ok(())
}

#[test]
fn pc03_character_info_view_keeps_confirmed_result_during_refusal() {
    let mut panel = Panel {
        selected: true,
        summary: "Confirmed server result".into(),
        ..Default::default()
    };
    let idle = panel_text(&panel);
    assert!(idle.starts_with("Character information"));
    assert!(idle.contains("Confirmed server result"));
    assert!(request(&mut panel, 1));
    assert!(panel_text(&panel).contains("Waiting"));
    panel.failed("The server refused the request");
    let refused = panel_text(&panel);
    assert!(refused.contains("refused"));
    assert!(refused.contains("Confirmed server result"));
    assert!(!panel.pending);
    let fresh = Panel::default();
    assert!(fresh.fields.iter().all(String::is_empty));
    assert!(fresh.summary.is_empty());
    assert!(panel_text(&fresh).contains("Unavailable"));
}

#[test]
fn title_selector_filters_owned_titles_and_foreign_profiles_never_replace_ownership()
-> Result<(), &'static str> {
    use oteryn_client_runtime::character_info_panel::{
        CharacterInfoOperation as Op, CharacterTitle, PageRequest,
    };
    let mut p = Panel::default();
    let query = Op::QuerySelf(PageRequest::new(0, 1).ok_or("page")?);
    let first = CharacterTitle::ALL[0];
    let second = CharacterTitle::ALL[1];
    let reply = |title: CharacterTitle| {
        let mut b = vec![0; 72];
        b[69] = 1;
        b[70] = title.id();
        b
    };
    assert!(apply_title_reply(&mut p, query, &reply(first)));
    p.title_filter = TitleFilter::Earned;
    assert_eq!(title_rows(&p).len(), 1);
    assert_eq!(title_rows(&p)[0].title, first);
    assert!(apply_title_reply(
        &mut p,
        Op::QueryVisible {
            character: [7; 16],
            page: PageRequest::new(0, 1).ok_or("page")?
        },
        &reply(second)
    ));
    assert_eq!(title_rows(&p)[0].title, first);
    p.title_filter = TitleFilter::Locked;
    assert!(title_rows(&p).iter().all(|r| r.title != first && !r.earned));
    assert!(apply_title_reply(
        &mut p,
        Op::Unlock(second),
        &[2, second.id()]
    ));
    p.title_filter = TitleFilter::Earned;
    assert_eq!(title_rows(&p).len(), 2);
    let mut selected = vec![3];
    selected.extend_from_slice(&5u64.to_le_bytes());
    assert!(apply_title_reply(
        &mut p,
        Op::Select(Some(second)),
        &selected
    ));
    assert!(
        title_rows(&p)
            .iter()
            .any(|r| r.title == second && r.selected)
    );
    let before = p.own_titles.clone();
    assert!(!apply_title_reply(&mut p, query, &[0]));
    assert_eq!(p.own_titles, before);
    p.title_search = "no such title".into();
    assert!(title_rows(&p).is_empty());
    Ok(())
}
