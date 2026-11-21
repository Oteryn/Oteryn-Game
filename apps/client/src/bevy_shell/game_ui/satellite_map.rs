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
#[path = "satellite_map_tests.rs"]
mod tests;
