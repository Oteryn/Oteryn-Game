use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_runtime::arena::{ArenaAction, DromeAction};
use oteryn_client_ui::arena::{ArenaPanel, DROME_REWARDS};
use oteryn_client_ui_bevy as source;

pub(super) fn actions_for_rank(rank: u8) -> Vec<(Intent, String)> {
    let mut actions = vec![
        (Intent::Catalogue, "Catalogue".into()),
        (Intent::Action(ArenaAction::Status), "Refresh status".into()),
        (Intent::Action(ArenaAction::Enter), "Enter".into()),
        (Intent::Action(ArenaAction::Advance), "Next pit".into()),
        (Intent::Action(ArenaAction::Leave), "Leave".into()),
    ];
    {
        for (category, choice, label) in [
            (1, 1, "Present"),
            (2, 1, "Supplies"),
            (3, 1, "Club"),
            (3, 2, "Axe"),
            (3, 3, "Sword"),
            (4, 1, "Trophy"),
        ] {
            actions.push((
                Intent::Action(ArenaAction::ClaimReward {
                    rank,
                    category,
                    choice,
                }),
                label.into(),
            ));
        }
    }
    for (action, label) in [
        (DromeAction::Status, "Drome status"),
        (DromeAction::Enter, "Drome enter"),
        (DromeAction::Leave, "Drome leave"),
        (DromeAction::LowerStart, "Lower start -5"),
        (DromeAction::ClaimPotion, "Claim potion"),
    ] {
        actions.push((Intent::Action(ArenaAction::Drome(action)), label.into()));
    }
    for (index, label) in DROME_REWARDS.iter().enumerate() {
        actions.push((
            Intent::Action(ArenaAction::Drome(DromeAction::Buy(index as u8 + 1))),
            format!("Buy {label}"),
        ));
    }
    actions
}
pub(super) const RANKS: [&str; 3] = ["Greenhorn rewards", "Scrapper rewards", "Warlord rewards"];
#[cfg(test)]
pub(super) fn actions() -> Vec<(Intent, String)> {
    (1..=3)
        .flat_map(actions_for_rank)
        .filter(|(intent, _)| matches!(intent, Intent::Action(ArenaAction::ClaimReward { .. })))
        .chain(actions_for_rank(1).into_iter().filter(|(intent, _)| {
            !matches!(intent, Intent::Action(ArenaAction::ClaimReward { .. }))
        }))
        .collect()
}
pub(super) fn build(commands: &mut Commands, skins: &Skins, state: &State) -> Entity {
    let root = commands
        .spawn((
            Name::new("arena.window"),
            Pickable::IGNORE,
            GlobalZIndex(166),
            TabGroup {
                order: 2,
                modal: state.confirmation.is_some(),
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(if state.confirmation.is_some() {
                Color::srgba(0., 0., 0., 0.45)
            } else {
                Color::NONE
            }),
        ))
        .id();
    if state.confirmation.is_some() {
        commands.entity(root).insert(super::super::GameModal);
    }
    let frame = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            ScrollArea,
            Node {
                width: px(720.),
                max_width: percent(95.),
                max_height: percent(95.),
                overflow: Overflow::scroll_y(),
                padding: UiRect {
                    top: px(25.),
                    bottom: px(12.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(frame);
    source::dialog_caption(commands, frame, "Arena — Ctrl+Shift+Z", skins.font.clone());
    if let Some(snapshot) = &state.snapshot {
        text(commands, skins, frame, &state.text);
        if let Some((_, label)) = &state.confirmation {
            text(commands, skins, frame, &format!("Confirm {label}?"));
            button(
                commands,
                skins,
                frame,
                state,
                Action::Confirm,
                "Confirm",
                true,
            );
            button(
                commands,
                skins,
                frame,
                state,
                Action::Cancel,
                "Cancel",
                true,
            );
        } else {
            let selector = commands
                .spawn((
                    Name::new("arena.rank.selector"),
                    Node {
                        position_type: PositionType::Relative,
                        width: px(210.),
                        min_height: px(24.),
                        flex_shrink: 0.,
                        ..default()
                    },
                ))
                .id();
            commands.entity(frame).add_child(selector);
            button(
                commands,
                skins,
                selector,
                state,
                Action::RankMenu,
                RANKS[usize::from(state.rank)],
                ready(&snapshot.panel),
            );
            if state.rank_menu {
                let popup = commands
                    .spawn((
                        Name::new("arena.rank.options"),
                        source::frame(skins.inset.clone(), [1.; 4]),
                        Node {
                            position_type: PositionType::Absolute,
                            top: px(24.),
                            left: px(0.),
                            width: px(210.),
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 0.,
                            ..default()
                        },
                        ZIndex(20),
                        BackgroundColor(Color::srgb(0.18, 0.18, 0.18)),
                    ))
                    .id();
                commands.entity(selector).add_child(popup);
                for (index, label) in RANKS.iter().enumerate() {
                    button(
                        commands,
                        skins,
                        popup,
                        state,
                        Action::Rank(index as u8 + 1),
                        label,
                        ready(&snapshot.panel),
                    );
                }
            }
            let row = commands
                .spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(5.),
                    row_gap: px(5.),
                    flex_shrink: 0.,
                    ..default()
                })
                .id();
            commands.entity(frame).add_child(row);
            for (intent, label) in actions_for_rank(state.rank + 1) {
                button(
                    commands,
                    skins,
                    row,
                    state,
                    Action::Request(intent, label.clone()),
                    &label,
                    ready(&snapshot.panel),
                );
            }
        }
    }
    button(commands, skins, frame, state, Action::Close, "Close", true);
    root
}
fn ready(panel: &ArenaPanel) -> bool {
    panel.selected && panel.pending.is_none()
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) {
    let entity = commands
        .spawn((
            source::text(
                value.replace("\r\n", "\n"),
                source::font(skins.font.clone(), false),
            ),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    state: &State,
    action: Action,
    label: &str,
    enabled: bool,
) {
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), 210.),
        Control {
            generation: state.generation,
            revision: state.revision,
            action,
        },
    ));
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
