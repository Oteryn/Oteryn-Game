use super::*;
use oteryn_client_ui::chat::{CommunicationList, ComposerMode, ConsoleTab};

#[test]
fn console_controls_preserve_tab_filters_and_temporary_mode_reaches_session_once()
-> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        commands: commands.clone(),
                        rooms: ChatRoomSet::default(),
                        lines: vec![ChatLine::Private {
                            speaker_name: "Alice".into(),
                            text: "private hello".into(),
                        }],
                    },
                    incoming,
                    outbox,
                ));
                wait_for(&link, |panel| panel.selected && !panel.lines.is_empty()).await?;
                link.chat_console_tab(ConsoleTab::Private);
                link.chat_console_filter();
                assert!(!panel_text(&link.chat_panel()).contains("private hello"));
                link.chat_console_tab(ConsoleTab::Local);
                assert!(link.chat_panel().console.show_messages);
                link.chat_console_tab(ConsoleTab::Private);
                assert!(!link.chat_panel().console.show_messages);
                link.chat_console_mode();
                assert_eq!(link.chat_panel().console.mode, ComposerMode::Auto);
                link.chat_text("hello".into());
                link.chat_request(Mode::Say, "");
                assert!(
                    commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .is_empty()
                );
                link.chat_console_mode();
                assert_eq!(link.chat_panel().console.mode, ComposerMode::Off);
                link.chat_set_mode(ComposerMode::Temporary);
                link.chat_request(Mode::Say, "");
                wait_for(&link, |panel| !panel.pending).await?;
                assert_eq!(
                    commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .len(),
                    1
                );
                assert_eq!(link.chat_panel().console.mode, ComposerMode::Off);
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "console test thread failed")?
}

#[test]
fn channel_log_control_saves_appends_replaces_and_reports_io_errors() -> Result<(), Error> {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("oteryn-chat-log.txt");
    let (link, _, outbox) = play_channel();
    outbox.chat_initialize(true);
    crate::play::mailbox::lock(&outbox.0).chat.lines = vec![ChatLine::Private {
        speaker_name: "Alice".into(),
        text: "hëllo".into(),
    }];
    link.chat_save_log(&path, false)?;
    assert!(std::fs::read_to_string(&path)?.contains("hëllo"));
    link.chat_save_log(&path, true)?;
    assert_eq!(std::fs::read_to_string(&path)?.matches("hëllo").count(), 2);
    link.chat_save_log(&path, false)?;
    assert_eq!(std::fs::read_to_string(&path)?.matches("hëllo").count(), 1);
    assert!(link.chat_save_log(&path.join("child"), false).is_err());
    assert!(
        link.chat_panel()
            .feedback
            .as_deref()
            .is_some_and(|text| text.contains("Could not write"))
    );
    std::fs::remove_file(path)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn channel_log_is_created_with_owner_only_permissions() -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;

    const CHILD_PATH: &str = "OTERYN_TEST_RESTRICTIVE_EXPORT_PATH";
    if let Some(path) = std::env::var_os(CHILD_PATH) {
        rustix::process::umask(rustix::fs::Mode::RWXU);
        let path = std::path::Path::new(&path);
        let (link, _, _) = play_channel();
        link.chat_save_log(path, false)?;
        assert_eq!(std::fs::metadata(path)?.permissions().mode() & 0o777, 0o600);
        link.chat_save_log(path, true)?;
        return Ok(());
    }
    let fixture = tempfile::tempdir()?;
    let name = std::thread::current();
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", name.name().ok_or("unnamed test")?])
        .env(CHILD_PATH, fixture.path().join("chat.log"))
        .status()?;
    assert!(status.success());
    assert!(fixture.path().join("chat.log").is_file());
    Ok(())
}

#[cfg(unix)]
#[test]
fn channel_log_refuses_to_append_to_a_public_file() -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;

    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("chat.log");
    std::fs::write(&path, "existing log")?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
    let (link, _, _) = play_channel();
    let result = link.chat_save_log(&path, true);
    assert_eq!(
        result.err().map(|error| error.kind()),
        Some(std::io::ErrorKind::PermissionDenied)
    );
    assert_eq!(std::fs::read_to_string(path)?, "existing log");
    assert!(
        link.chat_panel()
            .feedback
            .is_some_and(|text| text.contains("Could not write"))
    );
    Ok(())
}

#[test]
fn communication_control_filters_live_lines_and_propagates_whitelist_exceptions() {
    let (link, _, outbox) = play_channel();
    outbox.chat_initialize(true);
    crate::play::mailbox::lock(&outbox.0).chat.lines = vec![ChatLine::Private {
        speaker_name: "Alice".into(),
        text: "private hello".into(),
    }];
    assert!(link.chat_communication(Some(CommunicationList::Ignore), " Alice ", true));
    assert!(!panel_text(&link.chat_panel()).contains("private hello"));
    assert!(link.chat_communication(Some(CommunicationList::Blacklist), "ALICE", true));
    assert!(link.chat_communication(Some(CommunicationList::Whitelist), "alice", true));
    assert!(panel_text(&link.chat_panel()).contains("private hello"));
    {
        let mailbox = crate::play::mailbox::lock(&outbox.0);
        assert!(mailbox.chat_private.communication.allows("Alice"));
        assert!(mailbox.chat_party_room.communication.allows("Alice"));
    }
    assert!(link.chat_communication(None, "Alice", false));
    assert!(link.chat_panel().console.communication.ignored.is_empty());
    assert!(link.chat_panel().console.communication.whitelist.is_empty());
    assert!(link.chat_panel().console.communication.blacklist.is_empty());
    assert!(!link.chat_communication(Some(CommunicationList::Ignore), "", true));
}

#[test]
fn dragged_channel_mirror_tracks_live_history_and_closes_with_original() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        commands: commands.clone(),
                        rooms: ChatRoomSet::default(),
                        lines: vec![ChatLine::Private {
                            speaker_name: "Alice".into(),
                            text: "existing history".into(),
                        }],
                    },
                    incoming,
                    outbox,
                ));
                wait_for(&link, |panel| panel.selected && !panel.lines.is_empty()).await?;
                link.open_chat();
                link.chat_console_tab(ConsoleTab::Private);
                assert!(!link.chat_console_drop(ConsoleTab::Private, 400, 1100));
                assert!(oteryn_client_ui::chat::mirror_text(&link.chat_panel()).is_none());
                assert!(link.chat_console_drop(ConsoleTab::Private, 1000, 1100));
                assert_eq!(
                    oteryn_client_ui::chat::mirror_text(&link.chat_panel()).as_deref(),
                    Some(panel_text(&link.chat_panel()).as_str())
                );
                // The copy stays on Private while the original changes tabs and sends.
                link.chat_console_tab(ConsoleTab::Local);
                link.chat_text("new live message".into());
                link.chat_request(Mode::Private, "Player One");
                wait_for(&link, |panel| panel.lines.len() == 2 && !panel.pending).await?;
                let mirror = oteryn_client_ui::chat::mirror_text(&link.chat_panel())
                    .ok_or("mirror missing")?;
                assert!(mirror.contains("existing history"));
                assert!(mirror.contains("new live message"));
                assert!(!panel_text(&link.chat_panel()).contains("new live message"));
                assert_eq!(
                    commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .len(),
                    1
                );
                link.chat_console_tab(ConsoleTab::Private);
                link.chat_console_filter();
                assert!(
                    !oteryn_client_ui::chat::mirror_text(&link.chat_panel())
                        .ok_or("mirror missing")?
                        .contains("new live message")
                );
                assert!(link.window_toggle("0007-chat-keybindings", &[]));
                assert!(oteryn_client_ui::chat::mirror_text(&link.chat_panel()).is_none());
                link.open_chat();
                assert!(oteryn_client_ui::chat::mirror_text(&link.chat_panel()).is_none());
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "mirror console test thread failed")?
}

#[cfg(windows)]
#[test]
fn channel_log_refuses_an_intermediate_directory_junction() -> Result<(), Error> {
    let fixture = tempfile::tempdir()?;
    std::fs::create_dir_all(fixture.path().join("target").join("nested"))?;
    let status = std::process::Command::new("cmd.exe")
        .current_dir(fixture.path())
        .args(["/d", "/c", "mklink", "/J", "junction", "target"])
        .status()?;
    assert!(status.success());
    let (link, _, _) = play_channel();
    for append in [false, true] {
        let result = link.chat_save_log(&fixture.path().join("junction/nested/chat.log"), append);
        assert_eq!(
            result.err().map(|error| error.kind()),
            Some(std::io::ErrorKind::PermissionDenied)
        );
        assert!(!fixture.path().join("target/nested/chat.log").try_exists()?);
    }
    Ok(())
}

#[test]
fn communication_form_switches_filter_private_lines_and_preserves_membership() -> Result<(), Error>
{
    let (link, _, outbox) = play_channel();
    outbox.chat_initialize(true);
    crate::play::mailbox::lock(&outbox.0).chat.lines = vec![ChatLine::Private {
        speaker_name: "Alice".into(),
        text: "letter".into(),
    }];
    link.chat_lists_open(None);
    let form = crate::play::FeatureForm::all()
        .iter()
        .copied()
        .find(|form| form.title(&link) == "Communication lists")
        .ok_or("communication form")?;
    let toggle = |label| -> Result<(), Error> {
        let index = link
            .feature_form_view(form)
            .buttons
            .iter()
            .position(|button| *button == label)
            .ok_or("missing preference button")?;
        link.feature_form_submit(form, index, vec![]);
        Ok(())
    };
    let visible = || panel_text(&link.chat_panel()).contains("letter");
    toggle("Block private messages")?;
    assert!(!visible());
    {
        let mut mailbox = crate::play::mailbox::lock(&outbox.0);
        mailbox.chat.recipients = vec![Some("Bob".into())];
    }
    assert!(
        visible(),
        "the sender still sees their own outgoing private echo"
    );
    {
        let mut mailbox = crate::play::mailbox::lock(&outbox.0);
        mailbox.chat.recipients.clear();
        mailbox.chat.console.sync_conversations(
            &[ChatLine::Private {
                speaker_name: "Hidden".into(),
                text: "blocked".into(),
            }],
            &[Some(1)],
            &[None],
        );
        assert!(!mailbox.chat.console.channels.contains(
            &oteryn_client_ui::chat::ChannelTab::Conversation("Hidden".into())
        ));
    }
    toggle("Block private messages")?;
    assert!(visible());
    link.chat_communication(Some(CommunicationList::Ignore), "Alice", true);
    toggle("Suspend ignore list")?;
    assert!(visible());
    toggle("Suspend ignore list")?;
    assert!(!visible());
    link.chat_communication(Some(CommunicationList::Whitelist), "Alice", true);
    assert!(visible());
    toggle("Suspend whitelist")?;
    assert!(!visible());
    toggle("Suspend whitelist")?;
    assert!(visible());
    Ok(())
}

#[test]
fn communication_form_exempts_vip_names_and_filters_yells_only() -> Result<(), Error> {
    use oteryn_client_runtime::chat::{ActorPosition, ChatSpeaker, ChatSpeechMode};
    let (link, _, outbox) = play_channel();
    outbox.chat_initialize(true);
    let line = |name: &str, mode| ChatLine::Local {
        speaker: ChatSpeaker {
            identity: [1; 16],
            generation: std::num::NonZeroU64::MIN,
        },
        speaker_name: name.into(),
        mode,
        text: format!("message from {name}"),
        position: ActorPosition {
            x: 1,
            y: 2,
            floor: 7,
        },
    };
    outbox.chat_public_state(
        ChatRoomSet::default(),
        vec![
            line("Alice", ChatSpeechMode::Yell),
            line("Bob", ChatSpeechMode::Yell),
            line("Carol", ChatSpeechMode::Say),
        ],
    );
    link.chat_communication(Some(CommunicationList::Ignore), "Alice", true);
    link.chat_lists_open(None);
    let form = crate::play::FeatureForm::all()
        .iter()
        .copied()
        .find(|form| form.title(&link) == "Communication lists")
        .ok_or("form")?;
    for label in ["Block yells", "Allow VIP speakers"] {
        let index = link
            .feature_form_view(form)
            .buttons
            .iter()
            .position(|button| *button == label)
            .ok_or("filter button")?;
        link.feature_form_submit(form, index, vec![]);
    }
    outbox.vip_result(&oteryn_client_runtime::vip::VipResponse {
        status: oteryn_session::CommandStatus::Accepted,
        result: oteryn_client_runtime::vip::vip::ResultView {
            code: 1,
            revision: 1,
            number: 0,
            wait: 0,
            view: r#"{"entries":[{"number":1,"name":"ALICE"}],"next_entry":0}"#.into(),
        },
    });
    let text = panel_text(&link.chat_panel());
    assert!(text.contains("message from Alice") && text.contains("message from Carol"));
    assert!(!text.contains("message from Bob"));
    outbox.vip_presence_initialize(true, false);
    outbox.push(vec![oteryn_session::SessionEvent::VipPresence(
        oteryn_session::AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: oteryn_session::VipPresenceBatch {
                entries: vec![oteryn_session::VipPresenceEntry {
                    entry: 2,
                    name: "Bob".into(),
                    status: oteryn_session::VipPresenceStatus::Offline,
                    announce: false,
                    confirmed_for_ms: 0,
                    channel: None,
                }],
            },
        },
    )]);
    let text = panel_text(&link.chat_panel());
    assert!(text.contains("message from Bob") && !text.contains("message from Alice"));
    Ok(())
}
