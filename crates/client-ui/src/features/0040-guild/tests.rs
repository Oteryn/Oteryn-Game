use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn guild_entry_requires_ctrl_g_pressed_without_repeat()
-> Result<(), oteryn_input_actions::InputError> {
    let key = KeyCode::new(10)?;
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code: key,
        state,
        modifiers,
        repeat,
    };
    let ctrl = Modifiers::one(Modifier::Control);
    assert!(open_key(&[event(ButtonState::Pressed, ctrl, false)]));
    for input in [
        event(ButtonState::Released, ctrl, false),
        event(ButtonState::Pressed, ctrl, true),
        event(ButtonState::Pressed, Modifiers::NONE, false),
    ] {
        assert!(!open_key(&[input]));
    }
    Ok(())
}

#[test]
fn no_next_page_does_not_repeat_the_first_page() {
    let mut panel = GuildPanel {
        selected: true,
        own: Some(guild::GuildSummary {
            guild: [1; 16],
            name: "Knights".into(),
            state: 1,
            revision: 3,
            own_rank: 0,
            title: String::new(),
            message: String::new(),
            ranks: vec!["Leader".into()],
        }),
        ..Default::default()
    };
    assert_eq!(request(&mut panel, Intent::MoreRoster), None);
    panel.roster.push(guild::GuildMember {
        character: [2; 16],
        name: "Alice".into(),
        level: 2,
        title: String::new(),
    });
    apply_result(
        &mut panel,
        &GuildReply::Query {
            status: oteryn_client_runtime::guild::CommandStatus::Accepted,
            result: Some(guild::GuildQueryResult::Own(None)),
        },
    );
    assert!(panel.roster.is_empty());
    assert!(!panel_text(&panel).contains("Alice"));
}
