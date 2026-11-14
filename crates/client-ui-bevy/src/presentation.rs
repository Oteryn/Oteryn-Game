use bevy::prelude::*;

/// TibiaDialogFrameWithCaption's header, distinct from sidebar widget titles.
#[derive(Component)]
pub struct DialogCaption;

/// Center ordinary 11px Caption text within the source 17px header and 10px
/// horizontal margins. The caller retains the dialog, actions and focus scope.
pub fn dialog_caption(
    commands: &mut Commands,
    parent: Entity,
    value: &str,
    bold_face: Handle<Font>,
) -> Entity {
    let header = commands
        .spawn((
            DialogCaption,
            Node {
                position_type: PositionType::Absolute,
                left: px(10.),
                right: px(10.),
                top: px(0.),
                height: px(17.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let caption = commands
        .spawn(super::text(value, super::font(bold_face, false)))
        .insert((
            TextColor(Color::srgb_u8(144, 144, 144)),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(header).add_child(caption);
    commands.entity(parent).add_child(header);
    header
}

/// Bounded RowLayout/ColumnLayout sizing for source-derived children. A full-
/// width child in a row shares remaining space; literal preferred widths stay
/// intact. Fill-height columns/viewport wrappers consume the available height.
/// This maps layout policy only, not QML expressions or application state.
pub fn layout_child(
    node: &mut Node,
    parent_axis: FlexDirection,
    fill_width: bool,
    fill_height: bool,
) {
    let row = matches!(parent_axis, FlexDirection::Row | FlexDirection::RowReverse);
    if row && (fill_width || node.width == percent(100.)) {
        if !matches!(node.width, Val::Px(_)) {
            node.width = Val::Auto;
            node.flex_basis = px(0.);
        }
        node.min_width = px(0.);
        node.flex_grow = 1.;
        node.flex_shrink = 1.;
    } else if fill_width {
        if !matches!(node.width, Val::Px(_)) {
            node.width = percent(100.);
        }
        node.min_width = px(0.);
    }
    if fill_height {
        node.min_height = px(0.);
        if row {
            if node.height == Val::Auto {
                node.height = percent(100.);
            }
        } else {
            node.flex_grow = 1.;
            node.flex_shrink = 1.;
            if node.height == Val::Auto {
                node.flex_basis = px(0.);
            }
        }
    }
}
