use super::*;

/// North-up entrance map of the selected row's floor: one marker per catalogue entrance on
/// that floor, placed where `map_houses` fits the page into the view.
pub(super) fn entrance_map(
    commands: &mut Commands,
    skin: &Skins,
    generation: u64,
    root: Entity,
    houses: &[oteryn_client_ui::house::MapHouse],
) {
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(680.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(
        commands,
        panel,
        "House entrances — north up",
        skin.font.clone(),
    );
    let floor = houses.first().map_or(0, |house| house.position.2);
    label(
        commands,
        skin,
        panel,
        &format!("Floor {floor} · selected entrance ◆ · other entrances ⌂ · page fitted to view"),
    );
    let area = commands
        .spawn(Node {
            width: px(656.),
            height: px(400.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(panel).add_child(area);
    for house in houses {
        let (x, y, floor) = house.position;
        let marker = commands
            .spawn((
                source::text(
                    format!(
                        "{} {}\n{x},{y},{floor}",
                        if house.selected { "◆" } else { "⌂" },
                        house.label
                    ),
                    source::font(skin.font.clone(), false),
                ),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(house.pixel.0 as f32),
                    top: px((house.pixel.1 - 40) as f32),
                    ..default()
                },
            ))
            .id();
        commands.entity(area).add_child(marker);
    }
    label(
        commands,
        skin,
        panel,
        "Entrances on the selected floor. Query another page or row in Houses to change the map.",
    );
    let button = commands
        .spawn((
            source::button(skin.buttons.clone(), 120.),
            Control {
                generation,
                root,
                selection: Some(CLOSE_MAP),
            },
        ))
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption("Close map", skin.font.clone()))
        .id();
    commands.entity(button).add_child(text);
    commands.entity(panel).add_child(button);
}
