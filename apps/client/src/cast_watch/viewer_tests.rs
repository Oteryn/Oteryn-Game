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
                actors: vec![(1, 12, 272, 1, 75)],
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
                    x: 21,
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
    let lit_pixels = |left: bool| {
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .any(|(index, pixel)| {
                ((index % WIDTH as usize) < WIDTH as usize / 2) == left && pixel[..3] != [0, 0, 0]
            })
    };
    assert!(lit_pixels(true), "the creature is drawn on the left");
    assert!(lit_pixels(false), "the map tile is drawn on the right");
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

#[test]
fn watch_layout_keeps_long_chat_and_draft_reachable_after_resize()
-> Result<(), Box<dyn std::error::Error>> {
    use bevy::{
        app::{HierarchyPropagatePlugin, PropagateSet},
        camera::{ComputedCameraValues, RenderTargetInfo},
        text::TextPlugin,
        ui::{
            ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiSystems, ui_layout_system,
            ui_surface::UiSurface, update::propagate_ui_target_cameras,
        },
    };
    let (link, _) = channel();
    link.apply(Message::Ready {
        epoch: 1,
        chat: true,
        delay: 0,
    })?;
    for sequence in 1..=8 {
        link.apply(Message::ChatLine {
            epoch: 1,
            sequence,
            handle: [1; 16],
            text: "Long spectator message. ".repeat(8),
        })?;
    }
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        TextPlugin,
        HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(PostUpdate),
        HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(PostUpdate),
    ))
    .init_asset::<Image>()
    .init_resource::<Assets<TextureAtlasLayout>>()
    .init_resource::<UiScale>()
    .init_resource::<UiSurface>()
    .add_message::<KeyboardInput>()
    .configure_sets(
        PostUpdate,
        (
            UiSystems::Prepare,
            UiSystems::Propagate,
            UiSystems::Content,
            UiSystems::Layout,
        )
            .chain(),
    )
    .configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiTargetCamera>::default().in_set(UiSystems::Propagate),
    )
    .configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiRenderTargetInfo>::default().in_set(UiSystems::Propagate),
    )
    .add_systems(
        PostUpdate,
        (
            propagate_ui_target_cameras.in_set(UiSystems::Prepare),
            bevy::ui::widget::update_image_content_size_system.in_set(UiSystems::Content),
            bevy::ui::widget::measure_text_system
                .after(bevy::text::detect_text_needs_rerender)
                .after(bevy::text::load_font_assets_into_font_collection)
                .in_set(UiSystems::Content),
            ui_layout_system.in_set(UiSystems::Layout),
        ),
    );
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (WIDTH, 700).into(),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    install(&mut app, link);
    app.update();
    for mut camera in app
        .world_mut()
        .query::<&mut Camera>()
        .iter_mut(app.world_mut())
    {
        camera.computed = ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(360, 260),
                scale_factor: 1.,
            }),
            ..default()
        };
    }
    let text = app
        .world_mut()
        .query_filtered::<Entity, With<WatchText>>()
        .single(app.world())?;
    let font = Font::from_bytes(include_bytes!("../../tests/assets/FiraMono-subset.ttf").to_vec());
    let font = app.world_mut().resource_mut::<Assets<Font>>().add(font);
    app.world_mut()
        .get_mut::<TextFont>(text)
        .ok_or("chat font")?
        .font = font.into();
    app.world_mut()
        .get_mut::<Window>(window)
        .ok_or("watch window")?
        .resolution
        .set(360., 260.);
    app.world_mut()
        .write_message(key(Key::Character("h".into()), Some("h")));
    app.update();
    app.update();

    let picture = app
        .world_mut()
        .query_filtered::<&ComputedNode, With<ImageNode>>()
        .single(app.world())?;
    assert!(picture.size().y < 260. && picture.size().x <= 360.);
    let parent = app
        .world()
        .get::<ChildOf>(text)
        .ok_or("chat viewport")?
        .parent();
    let viewport = app
        .world()
        .get::<ComputedNode>(parent)
        .ok_or("chat viewport bounds")?;
    let text_bounds = app
        .world()
        .get::<ComputedNode>(text)
        .ok_or("chat content bounds")?;
    assert!(viewport.size().y > 0. && text_bounds.size().y > viewport.size().y);
    assert!(
        (viewport.scroll_position.y + viewport.size().y - viewport.content_size().y).abs() <= 1.
    );
    assert!(shown_text(&mut app)?.ends_with("Cast chat: h"));
    Ok(())
}

#[test]
fn watch_chat_uses_the_bundled_font_when_the_external_font_is_absent()
-> Result<(), Box<dyn std::error::Error>> {
    let folder = tempfile::tempdir()?;
    let (link, _) = channel();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .add_message::<KeyboardInput>();
    crate::bevy_shell::install_font_fallback(&mut app, folder.path());
    install(&mut app, link);
    app.update();
    let world = app.world_mut();
    let font = world
        .query_filtered::<&TextFont, With<WatchText>>()
        .single(world)?;
    assert_eq!(font.font, Handle::<Font>::default().into());
    Ok(())
}
