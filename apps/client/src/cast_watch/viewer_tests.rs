use super::*;
use crate::cast_watch::{Input, channel};
use bevy::input::keyboard::KeyCode;
use oteryn_session::cast::{DisplayTile, Message, PublicMedia, Scene};

fn key(logical_key: Key, text: Option<&str>) -> KeyboardInput {
    KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key,
        state: ButtonState::Pressed,
        text: text.map(Into::into),
        repeat: false,
        window: Entity::PLACEHOLDER,
    }
}

fn canvas_pixels(app: &App) -> Result<Vec<u8>, &'static str> {
    let handle = app.world().non_send::<Viewer>().image.clone();
    let image = app
        .world()
        .resource::<Assets<Image>>()
        .get(&handle)
        .ok_or("watch canvas")?;
    image.data.clone().ok_or("canvas pixels")
}

fn shown_text(app: &mut App) -> Result<String, Box<dyn std::error::Error>> {
    Ok(app
        .world_mut()
        .query_filtered::<&Text, With<WatchText>>()
        .single(app.world())?
        .0
        .clone())
}

#[test]
fn watch_window_draws_the_cast_and_sends_chat_then_leaves_on_escape()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands) = channel();
    for message in [
        Message::Ready {
            epoch: 1,
            chat: true,
            delay: 0,
        },
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
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .add_message::<KeyboardInput>();
    install(&mut app, link.clone());
    app.update();

    let pixels = canvas_pixels(&app)?;
    assert_eq!(pixels.len(), (WIDTH * HEIGHT * 4) as usize);
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[..3] != [0, 0, 0]),
        "the map tile and the creature are drawn into the window"
    );
    let (_, panel) = link.snapshot().ok_or("watch panel")?;
    assert!(shown_text(&mut app)?.starts_with(&panel_text(&panel)));

    for (logical, text) in [
        (Key::Character("h".into()), Some("h")),
        (Key::Character("i".into()), Some("i")),
    ] {
        app.world_mut().write_message(key(logical, text));
    }
    app.update();
    assert!(shown_text(&mut app)?.ends_with("Cast chat: hi"));
    app.world_mut().write_message(key(Key::Enter, None));
    app.update();
    assert!(matches!(commands.try_recv(), Ok(Input::Chat(text)) if text == "hi"));
    assert!(shown_text(&mut app)?.ends_with("Cast chat: "));

    app.world_mut().write_message(key(Key::Escape, None));
    app.update();
    assert!(matches!(commands.try_recv(), Ok(Input::Leave)));
    assert_eq!(app.should_exit(), Some(AppExit::Success));

    link.fail();
    app.update();
    assert!(
        canvas_pixels(&app)?
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[..3] == [0, 0, 0]),
        "an ended cast blanks the picture"
    );
    assert!(shown_text(&mut app)?.contains("Cast unavailable"));
    Ok(())
}
