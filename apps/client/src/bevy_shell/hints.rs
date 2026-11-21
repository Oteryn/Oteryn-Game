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
        .add_systems(
            PostUpdate,
            (projection::resolve, projection::present)
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

#[cfg(test)]
mod tests;
