//! Explored-world overview. F10 opens at the player; arrows pan.
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
    owner: NonSend<HostOwner>,
    keys: Res<ButtonInput<KeyCode>>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
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
    if super::super::input::satellite_shortcut(&keys) && !owner.modal_active() {
        if state.open {
            state.open = false;
        } else {
            *state = State {
                generation: owner.generation(),
                ..State::at(play.view.satellite_player())
            };
        }
    }
    if !state.open {
        if let Some(panel) = state.panel.take() {
            commands.entity(panel).despawn();
        }
        state.rendered = None;
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.open = false;
        return;
    }
    let step = 16 / state.scale.max(1);
    if keys.pressed(KeyCode::ArrowLeft) {
        state.x = state.x.saturating_sub(step);
    }
    if keys.pressed(KeyCode::ArrowRight) {
        state.x = state.x.saturating_add(step);
    }
    if keys.pressed(KeyCode::ArrowUp) {
        state.y = state.y.saturating_sub(step);
    }
    if keys.pressed(KeyCode::ArrowDown) {
        state.y = state.y.saturating_add(step);
    }
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    play.view.satellite_map(|cells, markers| {
        cells.hash(&mut hash);
        for marker in markers {
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
        for (&(floor, x, y), &color) in cells {
            if let Some((left, top)) = state.point(x, y, floor) {
                let shade = f32::from(color) / 255.;
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
                        BackgroundColor(Color::srgb(shade, shade, shade)),
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
}
