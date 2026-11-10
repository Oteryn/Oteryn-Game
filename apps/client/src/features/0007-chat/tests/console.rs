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
                link.chat_text("hello".into());
                link.chat_request(Mode::Say, "");
                assert!(
                    commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .is_empty()
                );
                link.chat_console_mode();
                assert_eq!(link.chat_panel().console.mode, ComposerMode::Temporary);
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
    static LOGS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "oteryn-chat-log-{}-{}.txt",
        std::process::id(),
        LOGS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
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
