//! Local hover/cursor projection over the existing confirmed play presentation.
use super::{owner::HostOwner, ui::Skins};
use crate::play::HoverTooltip;
use bevy::prelude::*;
use bevy::window::SystemCursorIcon;
use oteryn_client_ui_bevy as source;
mod projection;

#[derive(Component)]
struct HintRoot;
#[derive(Component)]
struct LensRoot;
#[derive(Component)]
struct HintText;
#[derive(Resource)]
struct HintState {
    hover: HoverTooltip,
    generation: Option<u64>,
    pointer: Option<Vec2>,
    label: Option<String>,
    fresh: bool,
    cursor: SystemCursorIcon,
}
impl Default for HintState {
    fn default() -> Self {
        Self {
            hover: HoverTooltip::default(),
            generation: None,
            pointer: None,
            label: None,
            fresh: false,
            cursor: SystemCursorIcon::Default,
        }
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<HintState>()
        .add_systems(Startup, setup.after(super::ui::setup))
        .add_systems(Update, projection::track_inputs)
        .add_systems(Update, lens_project.after(super::input::route))
        .add_systems(
            PostUpdate,
            (projection::resolve, lens_cursor, projection::present)
                .chain()
                .before(bevy::ui::UiSystems::Prepare),
        )
        .add_systems(
            PostUpdate,
            projection::clamp
                .after(bevy::ui::UiSystems::PostLayout)
                .before(bevy::ui::UiSystems::Stack),
        );
}
fn setup(mut commands: Commands, skins: Res<Skins>) {
    commands
        .spawn((
            LensRoot,
            Name::new("Help lens"),
            super::game_ui::GameModal,
            GlobalZIndex(150),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                display: Display::None,
                ..default()
            },
        ))
        .observe(lens_click);
    let root = commands
        .spawn((
            HintRoot,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                padding: UiRect {
                    left: px(8.),
                    right: px(8.),
                    top: px(21.),
                    bottom: px(8.),
                },
                max_width: px(400.),
                max_height: px(100.),
                overflow: Overflow::clip(),
                ..default()
            },
            GlobalZIndex(120),
            Pickable::IGNORE,
        ))
        .id();
    let text = commands
        .spawn((
            HintText,
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(root).add_child(text);
}

fn lens_project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    guard: super::game_ui::modal::ModalGuard,
    roots: Query<(Entity, &Node), With<LensRoot>>,
) {
    for (entity, node) in &roots {
        let link = owner.play.as_ref().map(|play| &play.link);
        if !owner.modal_active()
            && guard.allows(entity)
            && keys
                .as_ref()
                .is_some_and(|keys| keys.just_pressed(KeyCode::Escape))
            && let Some(link) = link.filter(|link| link.lens_active())
        {
            link.lens_click(None);
        }
        let display = if !owner.modal_active() && link.is_some_and(|link| link.lens_active()) {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            commands.entity(entity).insert(Node {
                display,
                ..node.clone()
            });
        }
    }
}
// Picking is captured above gameplay; geometry identifies the original region without
// delivering the help-selection click to the underlying action.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn lens_click(
    mut event: On<PointerClick>,
    owner: NonSend<HostOwner>,
    guard: super::game_ui::modal::ModalGuard,
    regions: Res<super::game_ui::GameRegions>,
    viewport: Res<super::game_ui::viewport::WorldViewport>,
    geometry: Query<(
        &ComputedNode,
        &UiGlobalTransform,
        Option<&bevy::ui::CalculatedClip>,
    )>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    use bevy::picking::pointer::PointerButton;
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let Some(play) = owner.play.as_ref().filter(|play| play.link.lens_active()) else {
        return;
    };
    if !matches!(
        event.button,
        PointerButton::Primary | PointerButton::Secondary
    ) {
        return;
    }
    event.propagate(false);
    let preferences = &owner.settings.preferences;
    let swapped = (preferences.controls.mouse == crate::play::MousePreset::LeftHanded)
        ^ preferences.swap_mouse_buttons;
    let primary = if swapped {
        PointerButton::Secondary
    } else {
        PointerButton::Primary
    };
    let point = event.pointer.position * windows.single().map_or(1., Window::scale_factor);
    let key = if event.button == primary {
        [
            (regions.world, "world"),
            (regions.minimap, "minimap"),
            (regions.vitals, "resources"),
            (regions.battle, "combat"),
        ]
        .into_iter()
        .find_map(|(entity, key)| {
            geometry
                .get(entity)
                .ok()
                .filter(|(node, transform, clip)| {
                    node.contains_point(**transform, point)
                        && clip.is_none_or(|clip| clip.contains_point(point))
                })
                .map(|_| key)
        })
    } else {
        None
    };
    let key = if key == Some("world") {
        let (w, h) = play.view.scene().view().viewport_px();
        let creature = viewport
            .scene_point(point, UVec2::new(w, h))
            .and_then(|point| play.view.click_tile(f64::from(point.x), f64::from(point.y)))
            .and_then(|tile| play.view.scene().target_at(tile))
            .is_some_and(|target| target.entity.is_some() && target.item.is_none());
        Some(if creature { "creature" } else { "world" })
    } else {
        key
    };
    play.link.lens_click(key);
}

#[cfg(test)]
mod tests;

// The lens captures picking above the world; its own modal must retain the help cursor.
fn lens_cursor(
    owner: NonSend<HostOwner>,
    guard: super::game_ui::modal::ModalGuard,
    roots: Query<Entity, With<LensRoot>>,
    viewport: Res<super::game_ui::viewport::WorldViewport>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut state: ResMut<HintState>,
) {
    let Some(play) = &owner.play else {
        return;
    };
    if owner.modal_active()
        || !play.link.lens_active()
        || !roots.iter().any(|root| guard.allows(root))
    {
        return;
    }
    if let Ok(window) = windows.single()
        && window.focused
        && let Some(pointer) = window.physical_cursor_position()
    {
        let (width, height) = play.view.scene().view().viewport_px();
        if viewport
            .scene_point(pointer, UVec2::new(width, height))
            .is_some()
        {
            state.cursor = SystemCursorIcon::Help;
        }
    }
}
