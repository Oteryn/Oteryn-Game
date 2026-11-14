use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub overlay: Entity,
}
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    key: Key,
    panel: &Panel,
    confirmation: bool,
) -> Widgets {
    let overlay = commands
        .spawn((
            GameModal,
            GlobalZIndex(165),
            TabGroup {
                order: 2,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let frame = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(560.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(24.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(frame);
    source::dialog_caption(commands, frame, "Furniture rotation", skins.font.clone());
    let body = commands
        .spawn((
            ScrollArea,
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                max_height: px(360.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(6.)),
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    commands.entity(frame).add_child(body);
    text(
        commands,
        skins,
        body,
        oteryn_client_ui::house_furniture_rotation::panel_text(panel),
    );
    let enabled = panel.selected && !panel.pending;
    if let Some(catalogue) = &panel.catalogue {
        for (index, entry) in catalogue.entries.iter().enumerate() {
            button(
                commands,
                skins,
                body,
                Control {
                    key,
                    kind: Kind::Select(index),
                },
                &format!(
                    "{}. {}{}",
                    index + 1,
                    entry.label,
                    if panel.choice == (index + 1).to_string() {
                        " (selected)"
                    } else {
                        ""
                    }
                ),
                enabled && !confirmation,
            );
        }
    }
    if confirmation {
        // The selected label is copied only from this bound server catalogue.
        if let Some(entry) = panel
            .choice
            .parse::<usize>()
            .ok()
            .and_then(|value| value.checked_sub(1))
            .and_then(|index| panel.catalogue.as_ref()?.entries.get(index))
        {
            text(commands, skins, frame, format!("Rotate {}?", entry.label));
        }
        button(
            commands,
            skins,
            frame,
            Control {
                key,
                kind: Kind::Confirm,
            },
            "Confirm rotation",
            enabled,
        );
        button(
            commands,
            skins,
            frame,
            Control {
                key,
                kind: Kind::Cancel,
            },
            "Cancel",
            true,
        );
    } else {
        let footer = commands
            .spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(5.),
                row_gap: px(4.),
                ..default()
            })
            .id();
        commands.entity(frame).add_child(footer);
        for (kind, label) in [
            (Kind::Request(Intent::Refresh), "Refresh"),
            (Kind::Request(Intent::Previous), "Previous"),
            (Kind::Request(Intent::Next), "Next"),
            (Kind::Review, "Rotate…"),
            (Kind::Close, "Close"),
        ] {
            let allowed = match kind {
                Kind::Close => true,
                Kind::Review => {
                    enabled
                        && panel
                            .choice
                            .parse::<usize>()
                            .ok()
                            .and_then(|value| value.checked_sub(1))
                            .and_then(|index| panel.catalogue.as_ref()?.entries.get(index))
                            .is_some()
                }
                Kind::Request(Intent::Next) => {
                    enabled
                        && panel
                            .catalogue
                            .as_ref()
                            .is_some_and(|catalogue| catalogue.more)
                }
                _ => enabled,
            };
            button(
                commands,
                skins,
                footer,
                Control { key, kind },
                label,
                allowed,
            );
        }
    }
    Widgets { overlay }
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    control: Control,
    label: &str,
    enabled: bool,
) {
    let mut entity = commands.spawn((source::button(skins.buttons.clone(), 140.), control));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
