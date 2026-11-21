//! Inventory hover facts use current item identity and retained, server-confirmed templates.
use super::*;
use bevy::{picking::hover::Hovered, window::PrimaryWindow};
use oteryn_client_ui_bevy as source;
use std::time::Duration;

#[derive(Component)]
pub(super) struct Tooltip(Entity);

pub(super) fn attach(commands: &mut Commands, skins: &Skins, parent: Entity, item: ItemEntry) {
    commands.entity(parent).insert(Hovered::default());
    // Float outside the scrollable sidebar so its clip cannot hide the tooltip.
    commands
        .spawn((
            Tooltip(parent),
            source::text(summary(item, None), source::font(skins.font.clone(), false)),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                padding: UiRect::all(px(6.)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
            GlobalZIndex(100),
        ))
        .insert(TextLayout::default());
}
fn summary(
    item: ItemEntry,
    template: Option<&oteryn_client_ui::inspection::InspectedItem>,
) -> String {
    let mut text = String::new();
    if let Some(template) = template {
        text.push_str(&template.name);
        text.push('\n');
        for (name, value) in &template.attributes {
            text.push_str(name);
            text.push_str(": ");
            text.push_str(value);
            text.push('\n');
        }
    }
    text.push_str(&format!(
        "Count: {}\nRight-click for Look and item actions",
        item.count
    ));
    text
}

#[derive(Default)]
pub(super) struct State {
    target: Option<(Entity, ItemEntry)>,
    since: Duration,
}
#[derive(Resource, Default)]
pub(super) struct HoverTarget(Option<(Entity, ItemEntry)>);

pub(super) fn hover(
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    slots: Query<(Entity, &Slot, &Hovered)>,
    mut target: ResMut<HoverTarget>,
) {
    target.0 = slots.iter().find_map(|(entity, slot, hovered)| {
        (hovered.get() && guard.allows(entity))
            .then(|| slot.current_item(&owner))
            .flatten()
            .map(|item| (entity, item))
    });
}
// The modal hierarchy is read in hover, before tooltip nodes are mutated here.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    hover: Res<HoverTarget>,
    time: Res<Time<Real>>,
    scale: Res<UiScale>,
    windows: Query<&Window, With<PrimaryWindow>>,
    slots: Query<(), With<Slot>>,
    mut tooltips: Query<(Entity, &Tooltip, &ComputedNode, &mut Node, &mut Text)>,
    mut state: Local<State>,
) {
    let window = windows.single().ok().filter(|window| window.focused);
    let target = window
        .filter(|window| window.cursor_position().is_some())
        .filter(|_| owner.settings.preferences.tooltips_enabled)
        .and(hover.0);
    if target != state.target {
        state.target = target;
        state.since = time.elapsed();
    }
    let ready = time.elapsed().saturating_sub(state.since)
        >= Duration::from_millis(u64::from(owner.settings.preferences.tooltip_delay_ms));
    let catalogue = target
        .and_then(|_| owner.play.as_ref())
        .map(|play| play.link.inspection_panel());
    for (entity, tooltip, computed, mut node, mut text) in &mut tooltips {
        if slots.get(tooltip.0).is_err() {
            commands.entity(entity).despawn();
            continue;
        }
        node.display = Display::None;
        let Some((_, item)) = target.filter(|(entity, _)| *entity == tooltip.0) else {
            continue;
        };
        let next = summary(
            item,
            catalogue
                .as_ref()
                .and_then(|panel| panel.catalogue.get(&item.item_definition_ref.get())),
        );
        let fresh = text.0 != next;
        if fresh {
            text.0 = next;
        }
        if !ready {
            continue;
        }
        if let Some(window) = window {
            let viewport = Vec2::new(window.width(), window.height()) / scale.0;
            let width = (viewport.x - 8.).clamp(1., 170.);
            let factor = window.scale_factor() * scale.0;
            let height = if !fresh && computed.size().y > 0. {
                computed.size().y / factor
            } else {
                // Before layout, reserve a conservative wrapped line bound.
                text.0
                    .lines()
                    .map(|line| {
                        (line.chars().count() as f32 * 11. / (width - 12.).max(1.))
                            .ceil()
                            .max(1.)
                    })
                    .sum::<f32>()
                    * 13.
                    + 12.
            };
            let bound = Vec2::new(width, height.min(viewport.y));
            let pointer = window.cursor_position().unwrap_or_default() / scale.0;
            let position = (pointer + Vec2::new(12., 20.)).min((viewport - bound).max(Vec2::ZERO));
            node.left = px(position.x);
            node.top = px(position.y);
            node.width = px(width);
            node.max_height = px(viewport.y);
            node.display = Display::Flex;
        }
    }
}
