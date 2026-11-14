use crate::play::{mailbox::lock, play_channel};

#[test]
fn analyser_tools_window_shows_owner_data_only() {
    let (link, _, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.analyser_initialize(true);
    let snapshot = link.analyser_tools_snapshot();
    assert_eq!(snapshot.text, link.analyser_text());
    assert_eq!(
        snapshot.fields,
        [String::new(), String::new(), String::new(), String::new()]
    );
    assert_eq!(super::super::input::ANALYSER_TOOLS_LABELS.len(), 37);
}

#[test]
fn analyser_tools_window_registered() {
    let controls = include_str!("../../../bevy_shell/controls.rs");
    let toolbar = include_str!("../../../bevy_shell/ui.rs");
    assert!(controls.contains("oteryn_client_ui::analyser::open_key(routed)"));
    assert!(toolbar.contains("Analysers (Ctrl+Shift+Y)"));
    assert!(include_str!("../../../bevy_shell/ui/toolbar.rs").contains("Control::AnalyserTools"));
}

#[test]
fn analyser_tools_actions_are_bound_and_confirmed() {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.analyser_initialize(true);
    assert!(link.analyser_tools_open_bound());
    let expected = link.analyser_tools_snapshot();
    assert!(!link.analyser_tools_submit_bound(&expected, 14, Default::default(), false));
    assert!(link.analyser_tools_submit_bound(
        &expected,
        15,
        ["42".into(), String::new(), String::new(), String::new()],
        false
    ));
    assert_eq!(link.analyser_panel().goal_xp, Some(42));
    assert!(!link.analyser_tools_submit_bound(&expected, 15, Default::default(), false));
    lock(&outbox.0).analyser.kills = 7;
    let expected = link.analyser_tools_snapshot();
    assert!(link.analyser_tools_submit_bound(&expected, 14, Default::default(), true));
    assert_eq!(link.analyser_panel().goal_xp, Some(42));
    assert_eq!(link.analyser_panel().kills, 0);
    assert!(commands.try_recv().is_err());
}

#[test]
fn analyser_tools_rejected_when_session_replaced_or_target_changed() {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.analyser_initialize(true);
    assert!(link.analyser_tools_open_bound());
    let expected = link.analyser_tools_snapshot();
    link.analyser_select("loot");
    assert!(!link.analyser_tools_submit_bound(&expected, 15, Default::default(), false));
    let (replacement, _replacement_commands, replacement_outbox) = play_channel();
    replacement.use_shared_frontend();
    replacement_outbox.analyser_initialize(true);
    assert!(replacement.analyser_tools_open_bound());
    assert!(!replacement.analyser_tools_submit_bound(&expected, 15, Default::default(), false));
    let expected = link.analyser_tools_snapshot();
    {
        let mut mailbox = lock(&outbox.0);
        mailbox.analyser.party_pending = true;
        mailbox.analyser.selected = false;
        mailbox.death.dead = true;
    }
    assert!(link.analyser_tools_close_bound(&expected));
    assert!(!link.analyser_panel().open);
    assert!(link.analyser_panel().party_pending);
    assert!(commands.try_recv().is_err());
}

#[test]
fn analyser_tools_server_reply_settles_after_close_and_reopen()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::analyser::party::{Member, encode_reply};
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.analyser_initialize(true);
    outbox.party_initialize(true, false);
    assert!(link.analyser_tools_open_bound());
    let expected = link.analyser_tools_snapshot();
    assert!(link.analyser_tools_submit_bound(&expected, 19, Default::default(), false));
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("party command".into());
    };
    assert_eq!((command.capability, command.kind), (10, 735));
    assert!(link.analyser_panel().party.is_empty());
    assert!(link.analyser_panel().party_pending);
    let pending = link.analyser_tools_snapshot();
    assert!(!link.analyser_tools_submit_bound(&pending, 19, Default::default(), false));
    assert!(link.analyser_tools_close_bound(&pending));
    assert!(link.analyser_tools_open_bound());
    assert!(link.analyser_panel().party_pending);
    let rows = vec![Member {
        handle: 1,
        name: "Alice".into(),
        available: true,
        loot_value: 1000,
        ..Default::default()
    }];
    outbox.analyser_result(
        &command,
        &oteryn_session::GatedCommandResult {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 1,
            payload: encode_reply(&rows)?,
        },
    );
    assert!(!link.analyser_panel().party_pending);
    assert_eq!(link.analyser_panel().party, rows);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn analyser_tools_server_modal_rejects_actions_but_allows_close() {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.analyser_initialize(true);
    assert!(link.analyser_tools_open_bound());
    let expected = link.analyser_tools_snapshot();
    let _modal = link.bevy_server_modal_fixture();
    assert!(!link.analyser_tools_submit_bound(
        &expected,
        15,
        ["99".into(), String::new(), String::new(), String::new()],
        false
    ));
    assert!(expected.same(&link.analyser_tools_snapshot()));
    assert!(link.analyser_tools_close_bound(&expected));
    assert!(commands.try_recv().is_err());
}

#[test]
fn analyser_tools_all_native_actions_use_existing_owner() -> Result<(), Box<dyn std::error::Error>>
{
    let path = std::env::temp_dir().join(format!("analyser-tools-{}.txt", std::process::id()));
    for action in 0..37 {
        let (link, mut commands, outbox) = play_channel();
        link.use_shared_frontend();
        let (native, mut native_commands, native_outbox) = play_channel();
        native.use_shared_frontend();
        outbox.analyser_initialize(true);
        native_outbox.analyser_initialize(true);
        assert!(link.analyser_tools_open_bound());
        assert!(native.analyser_tools_open_bound());
        let fields = [
            "120".into(),
            "1".into(),
            "7".into(),
            path.to_string_lossy().into_owned(),
        ];
        let expected = link.analyser_tools_snapshot();
        assert!(
            link.analyser_tools_submit_bound(&expected, action, fields.clone(), true),
            "action {action}"
        );
        super::super::input::submit(
            &mut lock(&native.mailbox),
            &native.commands,
            action,
            fields.into(),
        );
        assert_eq!(
            link.analyser_text(),
            native.analyser_text(),
            "action {action}"
        );
        assert_eq!(
            format!("{:?}", commands.try_recv()),
            format!("{:?}", native_commands.try_recv()),
            "action {action}"
        );
        assert!(!link.analyser_tools_submit_bound(&expected, action, Default::default(), true));
        assert!(commands.try_recv().is_err());
    }
    std::fs::remove_file(path)?;
    Ok(())
}
