use super::super::game_ui::{GameRegions, modal::ModalGuard, viewport::WorldViewport};
use super::*;
use bevy::{
    ecs::system::SystemParam, picking::hover::Hovered, text::EditableText, window::CursorIcon,
    window::PrimaryWindow,
};
use std::time::{Duration, Instant};

pub(super) fn track_inputs(
    mut commands: Commands,
    fields: Query<Entity, (Added<EditableText>, Without<Hovered>)>,
) {
    for entity in &fields {
        commands.entity(entity).insert(Hovered::default());
    }
}

#[derive(SystemParam)]
pub(super) struct Sources<'w, 's> {
    owner: NonSend<'w, HostOwner>,
    viewport: Res<'w, WorldViewport>,
    regions: Res<'w, GameRegions>,
    guard: ModalGuard<'w, 's>,
    windows: Query<'w, 's, (Entity, &'static Window), With<PrimaryWindow>>,
    hovered: Query<'w, 's, (Entity, &'static Hovered, Has<EditableText>)>,
    parents: Query<'w, 's, &'static ChildOf>,
    time: Res<'w, Time<Real>>,
}
// The read-only modal tree and the mutable tooltip nodes run in separate systems.
pub(super) fn resolve(sources: Sources, mut state: ResMut<HintState>) {
    let scope = sources
        .owner
        .play
        .as_ref()
        .map(|_| sources.owner.generation());
    if state.generation != scope {
        state.hover = HoverTooltip::default();
        state.generation = scope;
    }
    state.pointer = None;
    state.cursor = SystemCursorIcon::Default;
    let mut target = None;
    if let Ok((_, window)) = sources.windows.single()
        && window.focused
    {
        let mut over_ui = false;
        let mut over_text = false;
        for (entity, hovered, text) in &sources.hovered {
            if !hovered.get() || !sources.guard.allows(entity) {
                continue;
            }
            over_text |= text;
            let world = sources.regions.world;
            let map_branch = entity == world
                || sources
                    .parents
                    .iter_ancestors::<ChildOf>(entity)
                    .any(|parent| parent == world)
                || sources
                    .parents
                    .iter_ancestors::<ChildOf>(world)
                    .any(|parent| parent == entity);
            over_ui |= !map_branch;
        }
        if over_text {
            state.cursor = SystemCursorIcon::Text;
        }
        if !over_ui
            && !over_text
            && !sources.owner.modal_active()
            && sources.guard.allows(sources.regions.world)
            && let (Some(play), Some(pointer)) =
                (&sources.owner.play, window.physical_cursor_position())
        {
            let (width, height) = play.view.scene().view().viewport_px();
            if let Some(point) = sources
                .viewport
                .scene_point(pointer, UVec2::new(width, height))
            {
                state.pointer = Some(pointer);
                state.cursor = if play.link.lens_active() {
                    SystemCursorIcon::Help
                } else if sources.owner.settings.preferences.cursor_crosshair {
                    SystemCursorIcon::Crosshair
                } else {
                    SystemCursorIcon::Default
                };
                if sources.owner.settings.preferences.tooltips_enabled {
                    target = play.view.tooltip_target([point.x, point.y]);
                }
            }
        }
    }
    let now = sources.time.last_update().unwrap_or_else(Instant::now);
    let delay = Duration::from_millis(u64::from(
        sources.owner.settings.preferences.tooltip_delay_ms,
    ));
    let label = state.hover.update(target, now, delay).map(str::to_owned);
    state.fresh = label != state.label;
    state.label = label;
}

pub(super) fn present(
    mut commands: Commands,
    state: Res<HintState>,
    scale: Res<UiScale>,
    windows: Query<(Entity, &Window, Option<&CursorIcon>), With<PrimaryWindow>>,
    mut roots: Query<(&mut Node, &ComputedNode), With<HintRoot>>,
    mut labels: Query<&mut Text, With<HintText>>,
) {
    let Ok((entity, window, current)) = windows.single() else {
        return;
    };
    let cursor = CursorIcon::System(state.cursor);
    if current != Some(&cursor) {
        commands.entity(entity).insert(cursor);
    }
    let total = window.scale_factor() * scale.0;
    let window_size = Vec2::new(
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    for (mut node, computed) in &mut roots {
        node.display = if state.label.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        node.max_width = px((window_size.x / total - 8.).clamp(1., 400.));
        node.max_height = px((window_size.y / total - 8.).clamp(1., 100.));
        if let Some(point) = state.pointer {
            // A fresh label may grow. Its explicit maximum is a safe first-layout bound.
            let bound = if state.fresh || computed.size().min_element() <= 0. {
                Vec2::new(
                    (window_size.x / total - 8.).clamp(1., 400.),
                    (window_size.y / total - 8.).clamp(1., 100.),
                ) * total
            } else {
                computed.size()
            };
            place(&mut node, point, window_size, bound, total);
        }
    }
    for mut text in &mut labels {
        let label = state.label.as_deref().unwrap_or("");
        if text.0 != label {
            text.0 = label.to_owned();
        }
    }
}
fn place(node: &mut Node, pointer: Vec2, window: Vec2, size: Vec2, scale: f32) {
    let point = (pointer + Vec2::new(16., 24.) * scale)
        .max(Vec2::ZERO)
        .min((window - size).max(Vec2::ZERO));
    node.left = px(point.x / scale);
    node.top = px(point.y / scale);
}
pub(super) fn clamp(
    state: Res<HintState>,
    scale: Res<UiScale>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut roots: Query<(&mut Node, &ComputedNode), With<HintRoot>>,
) {
    let (Ok(window), Some(pointer)) = (windows.single(), state.pointer) else {
        return;
    };
    for (mut node, computed) in &mut roots {
        if node.display != Display::None && computed.size().min_element() > 0. {
            place(
                &mut node,
                pointer,
                Vec2::new(
                    window.physical_width() as f32,
                    window.physical_height() as f32,
                ),
                computed.size(),
                window.scale_factor() * scale.0,
            );
        }
    }
}
