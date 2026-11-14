use super::*;
use bevy::ui::InteractionDisabled;
use oteryn_client_ui_bevy as source;
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    key: Key,
    panel: &Panel,
    confirming: bool,
) -> Entity {
    let root = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            bevy::input_focus::tab_navigation::TabGroup::new(2),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            GlobalZIndex(160),
            Node {
                position_type: PositionType::Absolute,
                left: percent(25.),
                top: percent(20.),
                width: px(570.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    top: px(24.),
                    left: px(12.),
                    right: px(12.),
                    bottom: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
        ))
        .id();
    source::dialog_caption(commands, root, "Expert PvP modes", skins.font.clone());
    text(
        commands,
        skins,
        root,
        oteryn_client_ui::pvp_expert::panel_text(panel),
    );
    // These are the four fields and legal choices of the default presenter.
    for (field, label, choices) in [
        (
            0,
            "Fight",
            &[(1, "Offensive"), (2, "Balanced"), (3, "Defensive")][..],
        ),
        (1, "Chase", &[(1, "Stand"), (2, "Chase")][..]),
        (2, "Secure", &[(0, "Off"), (1, "On")][..]),
        (
            3,
            "Expert",
            &[
                (0, "Dove"),
                (1, "White hand"),
                (2, "Yellow hand"),
                (3, "Red fist"),
            ][..],
        ),
    ] {
        text(
            commands,
            skins,
            root,
            format!("{label}: {}", panel.fields[field]),
        );
        let row = commands
            .spawn(Node {
                column_gap: px(4.),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .id();
        commands.entity(root).add_child(row);
        for &(value, caption) in choices {
            button(
                commands,
                skins,
                row,
                key,
                Kind::Field(field, value),
                caption,
                panel.selected && !panel.pending && !confirming,
            );
        }
    }
    if confirming {
        text(
            commands,
            skins,
            root,
            "Apply these fight, chase, secure and expert settings?",
        );
        button(
            commands,
            skins,
            root,
            key,
            Kind::Confirm,
            "Confirm apply",
            panel.selected && !panel.pending,
        );
        button(commands, skins, root, key, Kind::Cancel, "Cancel", true);
    } else {
        button(
            commands,
            skins,
            root,
            key,
            Kind::Apply,
            "Apply",
            panel.selected && !panel.pending,
        );
    }
    button(commands, skins, root, key, Kind::Close, "Close", true);
    root
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(entity);
}
// A button needs both its displayed binding and its source styling.
#[allow(clippy::too_many_arguments)]
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    caption: &str,
    enabled: bool,
) {
    let mut button = commands.spawn((
        source::button(skins.buttons.clone(), 110.),
        Control { key, kind },
    ));
    if !enabled {
        button.insert(InteractionDisabled);
    }
    let entity = button.observe(activate).id();
    let caption = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
