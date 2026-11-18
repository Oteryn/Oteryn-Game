use super::*;
#[test]
fn satellite_centres_on_player() {
    let state = State::at((100, 200, -7));
    assert_eq!((state.x, state.y, state.floor), (100, 200, -7));
}
#[test]
fn satellite_zoom_bounds() {
    let mut state = State::at((0, 0, -7));
    for _ in 0..20 {
        state.zoom(true);
    }
    assert_eq!(state.scale, 16);
    for _ in 0..20 {
        state.zoom(false);
    }
    assert_eq!(state.scale, 1);
}
#[test]
fn satellite_floor_change() {
    let mut state = State::at((0, 0, -7));
    state.change_floor(true);
    assert_eq!(state.floor, -6);
    state.change_floor(false);
    assert_eq!(state.floor, -7);
}
#[test]
fn satellite_shows_markers() {
    let state = State::at((100, 200, -7));
    assert_eq!(state.point(100, 200, -7), Some((240., 180.)));
    assert_eq!(state.point(100, 200, -8), None);
}
use crate::{
    bevy_shell::owner::{ActivePlay, Phase},
    play::{PlayView, play_channel},
};
use oteryn_input_actions::{
    ButtonState, KeyCode as Key, Modifier, Modifiers, NormalizedInputEvent,
};
fn key(
    code: u16,
    modifiers: Modifiers,
) -> Result<NormalizedInputEvent, Box<dyn std::error::Error>> {
    Ok(NormalizedInputEvent::Key {
        code: Key::new(code)?,
        state: ButtonState::Pressed,
        modifiers,
        repeat: false,
    })
}
fn chord() -> Result<NormalizedInputEvent, Box<dyn std::error::Error>> {
    key(29, Modifiers::one(Modifier::Control).with(Modifier::Alt))
}
fn host() -> Result<HostOwner, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    let (link, _commands, _outbox) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    Ok(owner)
}
#[test]
fn satellite_chord_reaches_only_unblocked_play_and_f10_stays_audio()
-> Result<(), Box<dyn std::error::Error>> {
    let mut owner = host()?;
    let chord = chord()?;
    owner.route_input(std::slice::from_ref(&chord), true);
    assert!(
        !owner.take_satellite_request(),
        "blocked input must not open it"
    );
    owner.open_graphics();
    owner.route_input(std::slice::from_ref(&chord), false);
    assert!(
        !owner.take_satellite_request(),
        "a modal fences the shortcut"
    );
    owner.cancel();
    owner.route_input(&[key(67, Modifiers::NONE)?], false);
    assert!(
        !owner.take_satellite_request(),
        "plain F10 belongs to audio mute"
    );
    owner.route_input(
        &[key(
            67,
            Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
        )?],
        false,
    );
    assert!(!owner.take_satellite_request());
    owner.route_input(&[chord], false);
    assert!(owner.take_satellite_request());
    assert!(!owner.take_satellite_request(), "one request per press");
    Ok(())
}
#[test]
fn satellite_escape_despawns_the_panel_so_reopening_leaves_no_orphan()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(host()?)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    let modals = |app: &mut App| {
        app.world_mut()
            .query::<&super::super::GameModal>()
            .iter(app.world())
            .count()
    };
    let open = |app: &mut App| -> Result<(), Box<dyn std::error::Error>> {
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .route_input(&[chord()?], false);
        app.update();
        Ok(())
    };
    app.update();
    let base = modals(&mut app);
    open(&mut app)?;
    assert_eq!(modals(&mut app), base + 1);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    assert_eq!(
        modals(&mut app),
        base,
        "Escape must remove the blocking panel"
    );
    open(&mut app)?;
    assert_eq!(modals(&mut app), base + 1);
    Ok(())
}
#[test]
fn satellite_keyboard_waits_for_top_modal() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(host()?)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[chord()?], false);
    app.update();
    let x = app.world().resource::<State>().x;
    app.world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(300), Node::default()));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.update();
    assert!(app.world().resource::<State>().open);
    assert_eq!(app.world().resource::<State>().x, x);
    Ok(())
}

#[test]
fn satellite_projects_terrain_palette_and_marker_colours() -> Result<(), Box<dyn std::error::Error>>
{
    let mut owner = host()?;
    owner
        .play
        .as_mut()
        .ok_or("play")?
        .view
        .bevy_minimap_colours_fixture();
    owner.route_input(&[chord()?], false);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    assert!(
        app.world_mut()
            .query::<&BackgroundColor>()
            .iter(app.world())
            .any(|color| color.0 == Color::srgb_u8(255, 0, 0)),
        "terrain must use the sidebar palette"
    );
    assert!(
        app.world_mut()
            .query::<(&Text, &TextColor)>()
            .iter(app.world())
            .any(|(text, color)| text.0.contains("Blue marker")
                && color.0 == Color::srgb_u8(0, 0, 255))
    );
    Ok(())
}
#[test]
fn offscreen_exploration_keeps_the_rendered_satellite_panel()
-> Result<(), Box<dyn std::error::Error>> {
    let mut owner = host()?;
    owner.route_input(&[chord()?], false);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    app.world_mut().resource_mut::<State>().x = 9000;
    app.update();
    let panel = app.world().resource::<State>().panel.ok_or("open map")?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view
        .bevy_minimap_colours_fixture();
    app.update();
    assert_eq!(app.world().resource::<State>().panel, Some(panel));
    Ok(())
}
#[test]
fn stationary_satellite_frame_cost_is_below_a_history_scan()
-> Result<(), Box<dyn std::error::Error>> {
    use std::{hint::black_box, time::Instant};
    let mut owner = host()?;
    let path = std::env::temp_dir().join(format!("satellite-frame-{}", std::process::id()));
    std::fs::create_dir_all(&path)?;
    let view = &mut owner.play.as_mut().ok_or("play")?.view;
    view.bind_test_minimap(path.clone());
    let portable = path.join("history.json");
    let cells: Vec<_> = (0..200_000)
        .map(|i| [-7, 1000 + i / 60_000, i % 60_000, 1])
        .collect();
    std::fs::write(
        &portable,
        serde_json::to_vec(
            &serde_json::json!({"world":"11111111-1111-4111-8111-111111111111","binding":[vec![1;32],vec![2;32],1],"saved":{"version":1,"cells":cells,"markers":[]}}),
        )?,
    )?;
    owner.settings.request_minimap_transfer(true, portable);
    owner.settings.apply_minimap_transfer(view)?;
    owner.route_input(&[chord()?], false);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    assert!(app.world().resource::<State>().panel.is_some());
    let view = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .view;
    let scan = view.satellite_map(|cells, _| {
        assert_eq!(cells.len(), 200_000);
        (0..5)
            .map(|_| {
                let start = Instant::now();
                black_box(cells.keys().filter(|key| black_box(key.2) < 45).count());
                start.elapsed()
            })
            .min()
            .unwrap_or_default()
    });
    let frame = (0..5)
        .map(|_| {
            let start = Instant::now();
            app.update();
            start.elapsed()
        })
        .min()
        .ok_or("frame")?;
    std::fs::remove_dir_all(path)?;
    assert!(
        frame < scan / 4,
        "stationary frame {frame:?}, history scan {scan:?}"
    );
    Ok(())
}
#[test]
fn satellite_keyboard_respects_foreground_modal() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(host()?)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[chord()?], false);
    app.update();
    let before = app.world().resource::<State>().x;
    app.world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(999)));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.update();
    assert_eq!(app.world().resource::<State>().x, before);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert!(app.world().resource::<State>().open);
    Ok(())
}

mod range_cost {
    use super::*;
    use std::{collections::BTreeMap, hint::black_box, time::Instant};

    #[test]
    fn visible_lookup_cost_stays_below_a_full_history_scan() {
        let state = State::at((0, 0, -7));
        let cells: BTreeMap<_, _> = (0..200_000).map(|y| ((-7, 0, y), 1)).collect();
        // Calibrate against a traversal of the same tree on this machine.
        // Best of several samples discounts scheduler pauses in either path.
        let measure = |f: &mut dyn FnMut()| {
            (0..5)
                .map(|_| {
                    let start = Instant::now();
                    f();
                    start.elapsed()
                })
                .min()
                .unwrap_or_default()
        };
        let scan = measure(&mut || {
            black_box(cells.keys().filter(|key| black_box(key.2) < 45).count());
        });
        let lookup = measure(&mut || {
            assert_eq!(state.visible_cells(black_box(&cells)).count(), 45);
        });
        assert!(lookup < scan / 4, "viewport {lookup:?}, full scan {scan:?}");
    }
}
