use super::*;

pub(super) fn spawn(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    row: &WorldSpatialEntity,
    disabled: bool,
    panel: &oteryn_client_ui::battle::BattlePanel,
    parent: Entity,
) {
    let entry = commands
        .spawn((
            Pickable::default(),
            bevy::picking::hover::Hovered::default(),
            Chord::default(),
            bevy::input_focus::tab_navigation::TabIndex(0),
            HudAction {
                generation,
                command: Command::Attack(row.entity),
            },
            Node {
                width: percent(100.),
                height: px(22.),
                flex_shrink: 0.,
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .observe(click)
        .observe(press)
        .observe(keyboard)
        .id();
    if disabled {
        commands.entity(entry).insert(InteractionDisabled);
    }
    let target = panel
        .combat
        .is_some_and(|combat| combat.target == Some(row.entity));
    let icon = commands
        .spawn((
            TargetFrame(row.entity),
            portrait::Shown::default(),
            Node {
                width: px(CREATURE_ICON),
                height: px(CREATURE_ICON),
                margin: UiRect::all(px(1.)),
                border: UiRect::all(px(1.)),
                ..default()
            },
            BorderColor::all(if target {
                Color::srgb_u8(220, 40, 40)
            } else {
                Color::NONE
            }),
            Pickable::IGNORE,
        ))
        .id();
    let caption = commands
        .spawn((
            RowCaption(row.entity),
            source::text(name(row), source::font(skins.font.clone(), false)),
            Node {
                position_type: PositionType::Absolute,
                left: px(22.),
                right: px(0.),
                top: px(0.),
                ..default()
            },
        ))
        .id();
    let track = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(22.),
                right: px(0.),
                height: px(5.),
                bottom: px(2.),
                border: UiRect::all(px(1.)),
                ..default()
            },
            BorderColor::all(Color::BLACK),
            Pickable::IGNORE,
        ))
        .id();
    let health = crate::play::battle_row_health(row);
    let fill = commands
        .spawn((
            HealthFill(row.entity),
            Node {
                width: percent(f32::from(health.min(100))),
                height: percent(100.),
                ..default()
            },
            BackgroundColor(crate::bevy_shell::health_circle::health_colour(
                u32::from(health),
                100,
            )),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_child(fill);
    commands.entity(entry).add_children(&[icon, caption, track]);
    commands.entity(parent).add_child(entry);
}
