use oteryn_client::play::{PlayView, UiSound, play_channel};
use oteryn_renderer::TileCoord;
use oteryn_session::{AppliedDelta, ChatLine, SessionEvent};
fn delta<T>(value: T) -> AppliedDelta<T> {
    AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value,
    }
}
#[test]
fn confirmed_ui_events_reach_filtered_native_audio_queue() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(0, 0), 7, &[])?;
    let events = || {
        vec![
            SessionEvent::ChatLine(delta(ChatLine::Party {
                speaker_name: "Leader".into(),
                text: "Ready".into(),
            })),
            SessionEvent::ChatLine(delta(ChatLine::Private {
                speaker_name: "Friend".into(),
                text: "Hello".into(),
            })),
            SessionEvent::OpenContainer(delta(Default::default())),
            SessionEvent::VipPresence(delta(oteryn_protocol_oteryn::vip_presence::PresenceBatch {
                entries: vec![oteryn_protocol_oteryn::vip_presence::PresenceEntry {
                    entry: 1,
                    name: "Friend".into(),
                    status: oteryn_protocol_oteryn::vip_presence::Status::Online,
                    announce: true,
                    confirmed_for_ms: 10_000,
                }],
            })),
        ]
    };
    outbox.push(events());
    assert!(view.tick(&link).is_ok());
    assert_eq!(
        view.take_ui_sounds(),
        vec![
            UiSound::Party,
            UiSound::Chat,
            UiSound::Interaction,
            UiSound::Vip
        ]
    );
    assert!(view.take_ui_sounds().is_empty());
    let mut preferences = view.preferences().clone();
    preferences.audio.ui_sounds = [false; 4];
    view.set_preferences(preferences)?;
    outbox.push(events());
    assert!(view.tick(&link).is_ok());
    assert!(view.take_ui_sounds().is_empty());
    Ok(())
}
