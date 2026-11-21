//! Expandable explanations remain in the scrolling page, beside their controls.
use super::*;

#[derive(Component)]
struct Help(Entity);

pub(super) fn wrap(commands: &mut Commands, skins: &Skins, field: Row, editor: Entity) -> Entity {
    let Some(explanation) = field.help() else {
        return editor;
    };
    let container = commands
        .spawn(Node {
            width: percent(100.),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            row_gap: px(3.),
            ..default()
        })
        .id();
    let line = commands
        .spawn(Node {
            display: Display::Grid,
            width: percent(100.),
            flex_shrink: 0.,
            column_gap: px(5.),
            grid_template_columns: vec![GridTrack::flex(1.), GridTrack::px(22.)],
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    let detail = commands
        .spawn((
            Name::new(format!("{}.help.text", field.id)),
            source::text(explanation, source::font(skins.font.clone(), false)),
            Node {
                display: Display::None,
                width: percent(100.),
                min_width: px(0.),
                flex_shrink: 0.,
                padding: UiRect::all(px(4.)),
                ..default()
            },
        ))
        .insert(TextLayout::default())
        .id();
    let button = button(
        commands,
        skins,
        "?",
        22.,
        Control::Help,
        &format!("{}.help", field.id),
    );
    commands
        .entity(button)
        .insert(Help(detail))
        .observe(activate);
    commands.entity(line).add_children(&[editor, button]);
    commands.entity(container).add_children(&[line, detail]);
    container
}

fn activate(
    event: On<bevy::ui_widgets::Activate>,
    targets: Query<&Help>,
    nodes: Query<&Node>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut commands: Commands,
) {
    if !guard.allows(event.entity)
        || owner.preferences_draft().is_none()
        || top_server_dialog(&owner)
    {
        return;
    }
    let Ok(target) = targets.get(event.entity) else {
        return;
    };
    let Ok(node) = nodes.get(target.0) else {
        return;
    };
    let mut node = node.clone();
    node.display = if node.display == Display::None {
        Display::Flex
    } else {
        Display::None
    };
    commands.entity(target.0).insert(node);
}
