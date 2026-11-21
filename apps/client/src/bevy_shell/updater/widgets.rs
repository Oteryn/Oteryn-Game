use super::*;
use bevy::input_focus::tab_navigation::TabGroup;
use oteryn_client_ui_bevy as source;
pub(super) fn setup(mut commands: Commands, skins: Res<Skins>) {
    let entry = button(&mut commands, &skins, Action::Open, "Updates", 90.);
    commands.entity(entry).insert((
        GlobalZIndex(11),
        Node {
            position_type: PositionType::Absolute,
            right: px(8.),
            top: px(8.),
            width: px(90.),
            height: px(22.),
            ..default()
        },
    ));
    let root = commands
        .spawn((
            Dialog,
            super::super::game_ui::GameModal,
            GlobalZIndex(200),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.65)),
        ))
        .id();
    let panel = commands
        .spawn((
            TabGroup {
                order: 200,
                modal: true,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(640.),
                max_width: percent(98.),
                padding: px(24.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.),
                ..default()
            },
        ))
        .id();
    source::dialog_caption(
        &mut commands,
        panel,
        "Updates and recovery — stable",
        skins.font.clone(),
    );
    let status = commands
        .spawn((
            Status,
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .id();
    commands.entity(root).add_child(panel);
    commands.entity(panel).add_child(status);
    for (action, label) in [
        (Action::Check, "Check / Retry"),
        (Action::Download, "Download verified package"),
        (
            Action::Maintain,
            "Close client and maintain (update / verify / repair)",
        ),
        (Action::Close, "Hide (background work continues)"),
    ] {
        let entity = button(&mut commands, &skins, action, label, 570.);
        commands.entity(panel).add_child(entity);
    }
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    action: Action,
    label: &str,
    width: f32,
) -> Entity {
    let entity = commands
        .spawn((action, source::button(skins.buttons.clone(), width)))
        .id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    entity
}
