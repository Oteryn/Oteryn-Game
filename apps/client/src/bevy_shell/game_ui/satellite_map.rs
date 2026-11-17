//! Explored-world overview. Ctrl+Alt+Z opens at the player; arrows pan.
use super::super::{owner::HostOwner, ui::Skins};
use bevy::prelude::*;
use oteryn_client_ui_bevy as source;

#[derive(Resource, Default)]
struct State {
    x: i32,
    y: i32,
    floor: i16,
    scale: i32,
    open: bool,
    panel: Option<Entity>,
    generation: u64,
    rendered: Option<(i32, i32, i16, i32, bool, u64)>,
}
impl State {
    fn at(own: (i32, i32, i16)) -> Self {
        Self {
            x: own.0,
            y: own.1,
            floor: own.2,
            scale: 4,
            open: true,
            ..default()
        }
    }
    fn zoom(&mut self, inward: bool) {
        self.scale = if inward {
            self.scale.saturating_mul(2).min(16)
        } else {
            (self.scale / 2).max(1)
        };
    }
    fn change_floor(&mut self, up: bool) {
        self.floor = if up {
            oteryn_client_ui::minimap::floor_up(self.floor)
        } else {
            oteryn_client_ui::minimap::floor_down(self.floor)
        };
    }
    // Exploration is ordered by (floor, x, y). Seek the visible y interval
    // separately for each column so distant cells never enter the render cache.
    fn visible_cells<'a>(
        &self,
        cells: &'a std::collections::BTreeMap<(i16, i32, i32), u8>,
    ) -> impl Iterator<Item = (&'a (i16, i32, i32), &'a u8)> {
        let floor = self.floor;
        let left = self.x.saturating_sub(240 / self.scale);
        let right = self.x.saturating_add(239 / self.scale);
        let top = self.y.saturating_sub(180 / self.scale);
        let bottom = self.y.saturating_add(179 / self.scale);
        (left..=right).flat_map(move |x| cells.range((floor, x, top)..=(floor, x, bottom)))
    }
    fn point(&self, x: i32, y: i32, floor: i16) -> Option<(f32, f32)> {
        let px = (i64::from(x) - i64::from(self.x)) as f32 * self.scale as f32 + 240.;
        let py = (i64::from(y) - i64::from(self.y)) as f32 * self.scale as f32 + 180.;
        (floor == self.floor && (0.0..480.).contains(&px) && (0.0..360.).contains(&py))
            .then_some((px, py))
    }
}
#[derive(Component, Clone, Copy)]
enum Action {
    Close,
    In,
    Out,
    Up,
    Down,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
fn buttons(
    event: On<PointerClick>,
    mut state: ResMut<State>,
    actions: Query<&Action>,
    guard: super::modal::ModalGuard,
) {
    if event.button != bevy::picking::pointer::PointerButton::Primary || !guard.allows(event.entity)
    {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    match action {
        Action::Close => state.open = false,
        Action::In => state.zoom(true),
        Action::Out => state.zoom(false),
        Action::Up => state.change_floor(true),
        Action::Down => state.change_floor(false),
    }
}
fn project(
    mut commands: Commands,
    mut owner: NonSendMut<HostOwner>,
    keys: Res<ButtonInput<KeyCode>>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    guard: super::modal::ModalGuard,
) {
    let requested = owner.take_satellite_request();
    let Some(play) = owner.play.as_ref() else {
        state.open = false;
        if let Some(panel) = state.panel.take() {
            commands.entity(panel).despawn();
        }
        state.rendered = None;
        return;
    };
    if state.generation != owner.generation() {
        state.open = false;
        state.generation = owner.generation();
    }
    if requested && !state.open {
        // Replacing State would lose the old panel entity, which keeps blocking input.
        if let Some(panel) = state.panel.take() {
            commands.entity(panel).despawn();
        }
        *state = State {
            generation: owner.generation(),
            ..State::at(play.view.satellite_player())
        };
    }
    let foreground = !owner.modal_active() && state.panel.is_some_and(|panel| guard.allows(panel));
    if state.open && foreground && keys.just_pressed(KeyCode::Escape) {
        state.open = false;
    }
    if !state.open {
        if let Some(panel) = state.panel.take() {
            commands.entity(panel).despawn();
        }
        state.rendered = None;
        return;
    }
    let step = 16 / state.scale.max(1);
    if foreground && keys.pressed(KeyCode::ArrowLeft) {
        state.x = state.x.saturating_sub(step);
    }
    if foreground && keys.pressed(KeyCode::ArrowRight) {
        state.x = state.x.saturating_add(step);
    }
    if foreground && keys.pressed(KeyCode::ArrowUp) {
        state.y = state.y.saturating_sub(step);
    }
    if foreground && keys.pressed(KeyCode::ArrowDown) {
        state.y = state.y.saturating_add(step);
    }
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    play.view.satellite_map(|cells, markers| {
        for cell in state.visible_cells(cells) {
            cell.hash(&mut hash);
        }
        for marker in markers
            .iter()
            .filter(|m| state.point(m.x, m.y, m.floor).is_some())
        {
            (
                marker.x,
                marker.y,
                marker.floor,
                &marker.label,
                marker.color,
            )
                .hash(&mut hash);
        }
    });
    play.view.satellite_player().hash(&mut hash);
    let rendered = (
        state.x,
        state.y,
        state.floor,
        state.scale,
        state.open,
        hash.finish(),
    );
    if state.rendered == Some(rendered) {
        return;
    }
    state.rendered = Some(rendered);
    if let Some(panel) = state.panel.take() {
        commands.entity(panel).despawn();
    }
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(180.),
                top: px(80.),
                width: px(500.),
                padding: UiRect::all(px(10.)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
            GlobalZIndex(20),
            super::GameModal,
            crate::bevy_shell::ui::window_layout::WindowLayer("satellite_map"),
        ))
        .id();
    let title = commands
        .spawn(source::text(
            format!("Satellite — floor {} — arrows pan", state.floor),
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(root).add_child(title);
    let bar = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            ..default()
        })
        .id();
    commands.entity(root).add_child(bar);
    for (label, action) in [
        ("Close", Action::Close),
        ("+", Action::In),
        ("-", Action::Out),
        ("Floor up", Action::Up),
        ("Floor down", Action::Down),
    ] {
        let button = commands
            .spawn((source::button(skins.buttons.clone(), 90.), action))
            .observe(buttons)
            .id();
        let text = commands
            .spawn(source::text(label, source::font(skins.font.clone(), false)))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(bar).add_child(button);
    }
    let canvas = commands
        .spawn((
            Node {
                width: px(480.),
                height: px(360.),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .id();
    commands.entity(root).add_child(canvas);
    play.view.satellite_map(|cells, markers| {
        for (&(floor, x, y), &color) in state.visible_cells(cells) {
            if let Some((left, top)) = state.point(x, y, floor) {
                let cell = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(left),
                            top: px(top),
                            width: px(state.scale as f32),
                            height: px(state.scale as f32),
                            ..default()
                        },
                        BackgroundColor(super::minimap::palette_color(color)),
                    ))
                    .id();
                commands.entity(canvas).add_child(cell);
            }
        }
        for marker in markers {
            if let Some((left, top)) = state.point(marker.x, marker.y, marker.floor) {
                let label = commands
                    .spawn((
                        source::text(
                            format!("◆ {}", marker.label),
                            source::font(skins.font.clone(), false),
                        ),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(left),
                            top: px(top),
                            ..default()
                        },
                    ))
                    .id();
                let color = [180, 5, 30, 210]
                    .get(usize::from(marker.color))
                    .copied()
                    .unwrap_or(0);
                commands
                    .entity(label)
                    .insert(TextColor(super::minimap::palette_color(color)));
                commands.entity(canvas).add_child(label);
            }
        }
    });
    let own = play.view.satellite_player();
    if let Some((left, top)) = state.point(own.0, own.1, own.2) {
        let player = commands
            .spawn((
                source::text("@", source::font(skins.font.clone(), false)),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(left),
                    top: px(top),
                    ..default()
                },
            ))
            .id();
        commands.entity(canvas).add_child(player);
    }
    state.panel = Some(root);
}

#[cfg(test)]
mod tests {
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
    fn satellite_projects_terrain_palette_and_marker_colours()
    -> Result<(), Box<dyn std::error::Error>> {
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
}

#[cfg(test)]
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
