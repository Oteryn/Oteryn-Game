use super::*;

#[test]
fn guided_bug_report_requires_review_and_sends_only_opted_in_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::feature_0076_player_report::view::bug::activate;
    use oteryn_client_runtime::player_report as wire;
    let (link, mut receiver, _) = play_channel();
    assert!(
        link.player_report_input(&[oteryn_input_actions::NormalizedInputEvent::Key {
            code: oteryn_input_actions::KeyCode::new(69)?,
            state: oteryn_input_actions::ButtonState::Pressed,
            modifiers: oteryn_input_actions::Modifiers::one(
                oteryn_input_actions::Modifier::Control
            )
            .with(oteryn_input_actions::Modifier::Shift),
            repeat: false,
        }])
    );
    assert!(link.player_report_panel().bug.open);
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.properties.player_report.selected = true;
        mailbox.properties.player_report.bug.fields = [
            "100,200,7".into(),
            "The tile cannot be reached".into(),
            String::new(),
        ];
        activate(&mut mailbox, &link.commands, 0);
        activate(&mut mailbox, &link.commands, 5);
        assert!(receiver.try_recv().is_err());
        activate(&mut mailbox, &link.commands, 4);
        mailbox.properties.player_report.bug.fields[1].push('!');
        activate(&mut mailbox, &link.commands, 5);
        assert!(receiver.try_recv().is_err());
        activate(&mut mailbox, &link.commands, 4);
        activate(&mut mailbox, &link.commands, 5);
    }
    let crate::combat_input::PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing bug report".into());
    };
    let request =
        wire::decode_player_report_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (
            request.category,
            request.map_x,
            request.map_y,
            request.map_floor
        ),
        (0, 100, 200, 7)
    );
    assert_eq!(request.comment, "The tile cannot be reached!");
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.properties.player_report.pending = false;
        mailbox.properties.player_report.bug.fields[2] = "YES".into();
        activate(&mut mailbox, &link.commands, 2);
        activate(&mut mailbox, &link.commands, 4);
        activate(&mut mailbox, &link.commands, 5);
    }
    let crate::combat_input::PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing technical report".into());
    };
    let request =
        wire::decode_player_report_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(request.category, 2);
    assert_eq!((request.map_x, request.map_y, request.map_floor), (0, 0, 0));
    assert!(request.comment.contains(crate::CLIENT_BUILD));
    assert!(!request.comment.contains("100,200,7"));
    Ok(())
}

#[test]
fn bug_categories_show_specific_reproduction_guidance() {
    use crate::play::feature_0076_player_report::view::bug::activate;
    let (link, _receiver, _) = play_channel();
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    mailbox.properties.player_report.selected = true;
    for (category, expected) in [
        (0, "Coordinates"),
        (1, "original text"),
        (2, "operating system"),
        (3, "expected result"),
    ] {
        activate(&mut mailbox, &link.commands, category);
        assert!(
            mailbox
                .properties
                .player_report
                .bug
                .preview
                .contains(expected)
        );
    }
}

#[test]
fn violation_review_is_invalidated_by_edits() -> Result<(), Error> {
    use crate::play::feature_0076_player_report::view::violation::activate;
    let (link, mut receiver, _) = play_channel();
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    let panel = &mut mailbox.properties.player_report;
    panel.selected = true;
    panel.fields = [
        "0".into(),
        "1".into(),
        "Other Player".into(),
        String::new(),
        "Original statement".into(),
    ];
    activate(&mut mailbox, &link.commands, false);
    assert!(receiver.try_recv().is_err());
    activate(&mut mailbox, &link.commands, true);
    mailbox.properties.player_report.fields[4].push('!');
    activate(&mut mailbox, &link.commands, false);
    assert!(receiver.try_recv().is_err());
    activate(&mut mailbox, &link.commands, true);
    activate(&mut mailbox, &link.commands, false);
    assert!(receiver.try_recv().is_ok());
    Ok(())
}
