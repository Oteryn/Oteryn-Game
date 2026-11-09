use super::compendium_text;
use crate::play::play_channel;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{AppliedDelta, SessionEvent};

#[test]
fn play_compendium_opens_guide_recommendations_and_live_notices()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox) = play_channel();
    assert!(compendium_text(&link.notices_panel(), 1).is_none());
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(17)?,
        state: ButtonState::Pressed,
        repeat: false,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
    };
    assert!(link.notices_input(&[key]));
    let panel = link.notices_panel();
    assert!(
        compendium_text(&panel, 1)
            .ok_or("guide")?
            .contains("arrow keys")
    );
    assert!(
        compendium_text(&panel, 2)
            .ok_or("recommendations")?
            .contains("supplies")
    );
    outbox.notices_initialize(true, true);
    outbox.push(vec![SessionEvent::ServerSaveCountdown(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: oteryn_session::ServerSaveCountdown {
            ending_epoch: 3,
            minutes_remaining: 5,
        },
    })]);
    assert!(
        compendium_text(&link.notices_panel(), 0)
            .ok_or("notices")?
            .contains("5 minutes")
    );
    assert!(
        compendium_text(&link.notices_panel(), 1)
            .ok_or("guide")?
            .contains("server confirms")
    );
    Ok(())
}

#[test]
fn compendium_caller_loads_package_updates_and_support_pages()
-> Result<(), Box<dyn std::error::Error>> {
    use super::{compendium_page_text, compendium_updates};
    let directory = std::env::temp_dir().join(format!("compendium-feed-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    std::fs::write(
        directory.join("bulletin.json"),
        r#"{"news":[{"title":"Release notes","body":"Improved client navigation."}],"events":[]}"#,
    )?;
    let (link, _, _) = play_channel();
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(17)?,
        state: ButtonState::Pressed,
        repeat: false,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
    };
    assert!(link.notices_input(&[key]));
    let updates = compendium_updates(&directory);
    let panel = link.notices_panel();
    let news = compendium_page_text(&panel, 3, &updates).ok_or("updates")?;
    assert!(news.contains("Release notes"));
    assert!(news.contains("Improved client navigation."));
    assert!(
        compendium_page_text(&panel, 4, &updates)
            .ok_or("features")?
            .contains("quests")
    );
    let help = compendium_page_text(&panel, 5, &updates).ok_or("help")?;
    assert!(help.contains("Print Screen"));
    assert!(help.contains("consent"));
    assert!(compendium_page_text(&panel, 6, &updates).is_none());
    std::fs::write(directory.join("bulletin.json"), "invalid feed")?;
    assert!(compendium_updates(&directory).contains("Updates unavailable"));
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
