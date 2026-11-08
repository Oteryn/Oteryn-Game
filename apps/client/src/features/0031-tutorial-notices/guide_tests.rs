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
