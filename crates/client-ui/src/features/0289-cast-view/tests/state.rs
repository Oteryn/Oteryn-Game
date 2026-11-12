use super::*;
use oteryn_client_runtime::cast::{DisplayTile, Message, PublicMedia, Scene};
#[test]
fn s17b_public_picture_commits_atomically_and_slate_clears_private_free_view()
-> Result<(), &'static str> {
    let now = std::time::Instant::now();
    let mut panel = Panel::default();
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
                last: false,
                tiles: vec![DisplayTile {
                    x: 16,
                    y: 16,
                    floor: -7,
                    appearances: vec![100],
                }],
            },
        },
    ] {
        protocol::apply(&mut panel, message, now)?;
    }
    assert!(panel.picture.tiles.is_empty());
    assert!(panel.picture.actors.is_empty());
    protocol::apply(
        &mut panel,
        Message::Media {
            epoch: 1,
            sequence: 4,
            media: PublicMedia::Map {
                reset: false,
                last: true,
                tiles: vec![],
            },
        },
        now,
    )?;
    assert_eq!(panel.picture.tiles.get(&(-7, 16, 16)), Some(&vec![100]));
    assert_eq!(panel.picture.actors[0].4, 75);
    protocol::apply(
        &mut panel,
        Message::Media {
            epoch: 1,
            sequence: 5,
            media: PublicMedia::Speech {
                position: oteryn_client_runtime::cast_view::Position {
                    x: 16,
                    y: 16,
                    floor: -7,
                },
                mode: oteryn_client_runtime::cast_view::SpeechMode::Say,
                text: "Hello spectators".into(),
            },
        },
        now,
    )?;
    assert!(panel_text(&panel).contains("Hello spectators"));
    protocol::apply(
        &mut panel,
        Message::Slate {
            epoch: 1,
            sequence: 6,
        },
        now,
    )?;
    assert!(panel.picture.tiles.is_empty());
    assert!(panel.picture.actors.is_empty());
    assert!(panel.picture.speech.is_empty());
    assert!(panel_text(&panel).contains("consent"));
    assert!(protocol::apply(&mut panel, Message::Reply("wrong route".into()), now).is_err());
    assert!(panel.picture.ended);
    Ok(())
}
#[test]
fn s17b_chat_draft_is_unicode_bounded_and_has_no_gameplay_intent() {
    let mut draft = ChatDraft::default();
    draft.append(&"ą".repeat(200));
    assert_eq!(draft.0.len(), 256);
    draft.backspace();
    assert_eq!(draft.0.len(), 254);
    draft.append("\n");
    assert!(!draft.0.contains('\n'));
}
