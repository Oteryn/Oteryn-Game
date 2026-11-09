use crate::play::{SettingsPage, SettingsState};
use oteryn_client_ui::localization::{Language, with_language};

#[test]
fn language_selection_persists_and_localizes_real_chat_view_without_changing_messages()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::play_channel;
    use oteryn_session::{ChatLine, ChatRoomSet};
    let path = std::env::temp_dir().join(format!("o005-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    outbox.chat_public_state(
        ChatRoomSet::default(),
        vec![ChatLine::Private {
            speaker_name: "Settings".into(),
            text: "Apply console settings".into(),
        }],
    );
    for language in Language::ALL {
        let mut preferences = settings.preferences.clone();
        preferences.set_language(language);
        settings.apply(preferences);
        let restored = SettingsState::new(Some(path.clone()));
        assert_eq!(restored.preferences.language(), language);
        let title = SettingsPage::Graphics.localized_label(language);
        assert!(!title.is_empty());
        if language != Language::En {
            assert_ne!(title, "Graphics");
        }
        let text = with_language(language, || {
            oteryn_client_ui::chat::panel_text(&link.chat_panel())
        });
        with_language(language, || {
            link.chat_text("Apply console settings".into());
            link.chat_request(oteryn_client_ui::chat::Mode::Private, "Settings");
        });
        assert!(
            matches!(commands.try_recv()?, crate::combat_input::PlayCommand::ChatPublic(oteryn_session::ChatIntent::Private { recipient_name, text }) if recipient_name == "Settings" && text == "Apply console settings")
        );
        outbox.local_chat_result(&oteryn_session::ChatOutcome {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            disposition: oteryn_session::ChatDisposition::Ok,
            wait_seconds: 0,
            result_server_sequence: 1,
        });
        assert!(text.contains("Settings: Apply console settings"));
        if language != Language::En {
            assert!(!text.starts_with("Local Chat"));
        }
    }
    assert!(Language::from_code("unknown").is_none());
    std::fs::remove_file(path)?;
    Ok(())
}
