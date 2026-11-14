use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup,
    ui::InteractionDisabled,
    ui_widgets::{Button, ScrollArea},
};
pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    dock: Entity,
    generation: u64,
    serial: u64,
    panel: &PartyPanel,
) -> Entity {
    // TibiaStyle.sidebarWidth176; Party is an ordinary battle-list sidebar, not a modal.
    let root = commands
        .spawn((
            Name::new("party.sidebar"),
            TabGroup::new(4),
            source::frame(skin.frame.clone(), [2., 17., 2., 2.]),
            Node {
                width: px(176.),
                flex_shrink: 0.,
                padding: UiRect {
                    top: px(20.),
                    bottom: px(4.),
                    left: px(4.),
                    right: px(4.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(3.),
                ..default()
            },
        ))
        .id();
    commands.entity(dock).add_child(root);
    source::dialog_caption(commands, root, "Party", skin.font.clone());
    let header = row(commands, root);
    button(
        commands,
        skin,
        header,
        Control {
            generation,
            serial,
            kind: Kind::Refresh,
        },
        "Refresh",
        68.,
        panel.selected && panel.pending.is_none(),
    );
    button(
        commands,
        skin,
        header,
        Control {
            generation,
            serial,
            kind: Kind::Close,
        },
        "Close",
        60.,
        true,
    );
    if !panel.selected {
        label(commands, skin, root, "Party unavailable in this session");
        return root;
    }
    let pending = panel.pending.is_some();
    if let Some(state) = &panel.state {
        label(
            commands,
            skin,
            root,
            if state.shared_xp_enabled {
                "Shared experience: enabled"
            } else {
                "Shared experience: disabled"
            },
        );
        label(commands, skin, root, "Members");
        let members = list(commands, root);
        for member in &state.members {
            let selected = panel.selected_member == Some(member.handle);
            let button = commands
                .spawn((
                    Button,
                    Control {
                        generation,
                        serial,
                        kind: Kind::Member(member.handle),
                    },
                    Name::new("party.member"),
                    bevy::input_focus::tab_navigation::TabIndex(0),
                    Node {
                        width: percent(100.),
                        min_height: px(20.),
                        flex_shrink: 0.,
                        padding: px(2.).all(),
                        overflow: Overflow::clip_x(),
                        ..default()
                    },
                    BackgroundColor(if selected {
                        Color::srgb_u8(88, 88, 88)
                    } else {
                        Color::NONE
                    }),
                ))
                .observe(activate)
                .id();
            if pending {
                commands.entity(button).insert(InteractionDisabled);
            }
            let name = label(commands, skin, button, &member.name);
            if member.leader {
                commands
                    .entity(name)
                    .insert(TextColor(Color::srgb_u8(248, 219, 56)));
            }
            commands.entity(members).add_child(button);
        }
        if state.members.is_empty() {
            label(commands, skin, members, "No members");
        }
        if !state.invitations.is_empty() {
            label(commands, skin, root, "Invitations");
            let invitations = list(commands, root);
            for invitation in &state.invitations {
                let entry = button(
                    commands,
                    skin,
                    invitations,
                    Control {
                        generation,
                        serial,
                        kind: Kind::Invitation(invitation.handle),
                    },
                    &invitation.name,
                    164.,
                    !pending,
                );
                if panel.selected_invitation == Some(invitation.handle) {
                    commands.entity(entry).insert(bevy::ui::Checked);
                }
            }
        }
        let actions = row(commands, root);
        button(
            commands,
            skin,
            actions,
            Control {
                generation,
                serial,
                kind: Kind::Join,
            },
            "Join",
            74.,
            !pending && panel.selected_invitation.is_some(),
        );
        button(
            commands,
            skin,
            actions,
            Control {
                generation,
                serial,
                kind: Kind::Leave,
            },
            "Leave",
            74.,
            !pending && !state.members.is_empty(),
        );
        let actions = row(commands, root);
        button(
            commands,
            skin,
            actions,
            Control {
                generation,
                serial,
                kind: Kind::Pass,
            },
            "Pass leader",
            90.,
            panel.can_manage_member(),
        );
        button(
            commands,
            skin,
            actions,
            Control {
                generation,
                serial,
                kind: Kind::Kick,
            },
            "Kick",
            60.,
            panel.can_manage_member(),
        );
        button(
            commands,
            skin,
            root,
            Control {
                generation,
                serial,
                kind: Kind::Shared,
            },
            "Shared experience",
            160.,
            !pending && panel.chat_selected && panel.is_leader(),
        );
    } else {
        label(commands, skin, root, "Refresh to load the current party.");
    }
    if pending {
        label(commands, skin, root, "Waiting for the server…");
    }
    if let Some(feedback) = panel.feedback {
        label(commands, skin, root, feedback);
    }
    if let Some(feedback) = &panel.server_feedback {
        label(commands, skin, root, feedback);
    }
    root
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            width: percent(100.),
            column_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn list(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                max_height: px(180.),
                min_height: px(0.),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                row_gap: px(1.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, text: &str) -> Entity {
    let entity = commands
        .spawn(source::text(text, source::font(skin.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                min_width: px(0.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    control: Control,
    label: &str,
    width: f32,
    enabled: bool,
) -> Entity {
    let entity = commands
        .spawn((source::button(skin.buttons.clone(), width), control))
        .observe(activate)
        .id();
    if !enabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(label, skin.font.clone()))
        .id();
    if matches!(control.kind, Kind::Invitation(_)) {
        commands.entity(entity).insert(Node {
            width: px(width),
            max_width: percent(100.),
            min_height: px(20.),
            padding: px(2.).all(),
            flex_shrink: 0.,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        });
        commands.entity(caption).insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                min_width: px(0.),
                ..default()
            },
        ));
    }
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
    entity
}
