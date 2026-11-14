use super::*;
use bevy::{input_focus::tab_navigation::TabGroup, ui::InteractionDisabled};
use oteryn_client_ui_bevy as source;
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    revision: u64,
    panel: &HirelingPanel,
    confirmation: Option<Intent>,
) -> Entity {
    let root = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            GlobalZIndex(165),
            TabGroup::new(2),
            Node {
                position_type: PositionType::Absolute,
                left: percent(25.),
                top: percent(25.),
                width: px(440.),
                max_width: percent(95.),
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
    source::dialog_caption(commands, root, "Hireling services", skins.font.clone());
    text(
        commands,
        skins,
        root,
        oteryn_client_ui::hireling::panel_text(panel),
    );
    if let Some((x, y, floor)) = panel.target {
        text(
            commands,
            skins,
            root,
            format!("Target: {x}, {y}, floor {floor}"),
        );
    }
    let choices = if let Some(intent) = confirmation {
        commands.entity(root).insert((
            super::super::GameModal,
            TabGroup {
                order: 2,
                modal: true,
            },
        ));
        text(
            commands,
            skins,
            root,
            match intent {
                Intent::Summon => "Summon banker at the displayed target?",
                Intent::Dismiss => "Dismiss the banker hireling?",
                Intent::Refresh => "Refresh hireling status?",
            },
        );
        vec![(Kind::Confirm(intent), "Confirm"), (Kind::Cancel, "Cancel")]
    } else {
        vec![
            (Kind::Request(Intent::Summon), "Summon banker"),
            (Kind::Request(Intent::Dismiss), "Dismiss"),
            (Kind::Request(Intent::Refresh), "Refresh"),
            (Kind::Close, "Close"),
        ]
    };
    for (kind, caption) in choices {
        let enabled = matches!(kind, Kind::Close | Kind::Cancel)
            || panel.selected
                && panel.pending.is_none()
                && (!matches!(kind, Kind::Request(Intent::Summon)) || panel.target.is_some());
        let mut button = commands.spawn((
            source::button(skins.buttons.clone(), 140.),
            Control {
                generation,
                revision,
                kind,
            },
        ));
        if !enabled {
            button.insert(InteractionDisabled);
        }
        let button = button.observe(activate).id();
        let label = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(label);
        commands.entity(root).add_child(button);
    }
    root
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, text: impl Into<String>) {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(child);
}
