use super::*;
use oteryn_session::cast::{DisplayTile, PublicMedia, Scene};
#[test]
fn s17b_watch_entry_needs_account_ticket_and_never_character_configuration() {
    let config = WatchConfig::from_env(|name| match name {
        "OTERYN_WATCH_ADDRESS" => Some("127.0.0.1:7172".into()),
        "OTERYN_WATCH_TLS_NAME" => Some("game.test".into()),
        "OTERYN_WATCH_ROOT_CERT" => Some("root.pem".into()),
        "OTERYN_WATCH_TICKET" => Some(format!("ocv1.{}", "a".repeat(64))),
        _ => None,
    });
    assert!(config.is_ok());
    assert!(WatchConfig::from_env(|_| None).is_err());
}
#[test]
fn s17b_native_picture_draws_map_and_creatures_then_blanks_on_terminal()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _) = channel();
    for message in [
        Message::Clear { epoch: 1 },
        Message::Scene {
            epoch: 1,
            sequence: 1,
            scene: Scene {
                tiles: vec![],
                actors: vec![(1, 16, 272, 1, 75)],
            },
        },
        Message::Media {
            epoch: 1,
            sequence: 2,
            media: PublicMedia::Camera { floor: -7 },
        },
        Message::Media {
            epoch: 1,
            sequence: 3,
            media: PublicMedia::Map {
                reset: true,
                last: true,
                tiles: vec![DisplayTile {
                    x: 16,
                    y: 16,
                    floor: -7,
                    appearances: vec![100],
                }],
            },
        },
    ] {
        link.apply(message)?;
    }
    let (_, panel) = link.snapshot().ok_or("watch panel")?;
    let mut assets = picture::Assets::new();
    let drawn = assets.draw(&panel.picture, Instant::now())?;
    assert!(!drawn.sprites.is_empty());
    assert!(!drawn.tiles.instances().is_empty());
    assert!(drawn.atlas.width() > 0);
    let now = Instant::now();
    link.apply(Message::Media {
        epoch: 1,
        sequence: 4,
        media: PublicMedia::Effects(vec![oteryn_session::PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: oteryn_session::PresentationEventKind::MagicEffect {
                position: oteryn_session::ActorPosition {
                    x: 16,
                    y: 16,
                    floor: 7,
                },
                effect_id: 1,
                source: oteryn_session::PresentationSource::Global,
            },
        }]),
    })?;
    let (_, effects) = link.snapshot().ok_or("effects panel")?;
    let visible = assets.draw(&effects.picture, now)?;
    assert_eq!(visible.sprites.len(), drawn.sprites.len() + 1);
    assert_eq!(
        visible
            .sprites
            .instances()
            .last()
            .ok_or("effect cue")?
            .position,
        drawn
            .sprites
            .instances()
            .first()
            .ok_or("map tile")?
            .position
    );
    link.fail();
    let (_, panel) = link.snapshot().ok_or("terminal panel")?;
    let blank = assets.draw(&panel.picture, Instant::now())?;
    assert!(blank.sprites.is_empty());
    assert!(panel.picture.ended);
    Ok(())
}

#[test]
fn s17b_unselected_chat_is_disabled_before_any_observer_input_is_sent() -> Result<(), &'static str>
{
    let (link, mut rx) = channel();
    link.apply(Message::Ready {
        epoch: 1,
        chat: false,
        delay: 300,
    })?;
    link.chat("not selected".into());
    assert!(rx.try_recv().is_err());
    let (_, panel) = link.snapshot().ok_or("viewer panel")?;
    assert!(!panel.picture.ended);
    assert!(panel.feedback.contains("disabled"));
    assert_eq!(panel.picture.delay, 300);
    Ok(())
}
