use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui::daily_reward::{
    CYCLE_DAYS, cycle_day, daily_reward::details::Tier, details_text,
};
use oteryn_client_ui_bevy as source;
pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    panel: &DailyRewardPanel,
    history: bool,
    control: Control,
) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            Name::new("reward_wall"),
            GlobalZIndex(166),
            TabGroup {
                order: 2,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let window = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(760.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(window);
    source::dialog_caption(commands, window, "Daily Reward", skin.font.clone());
    if let Some(wall) = panel.wall {
        label(
            commands,
            skin,
            window,
            &format!(
                "Streak: {}    Jokers: {}    Day {} of {}{}",
                wall.streak,
                wall.jokers,
                cycle_day(wall.lane_day),
                CYCLE_DAYS,
                if wall.claimed { " — claimed" } else { "" }
            ),
        );
    }
    let tabs = row(commands, window);
    for (kind, text) in [(Kind::Wall, "Rewards"), (Kind::History, "History")] {
        button(
            commands,
            skin,
            tabs,
            Control { kind, ..control },
            text,
            true,
        );
    }
    let body = commands
        .spawn((
            ScrollArea,
            crate::bevy_shell::game_ui::install::RememberScroll("reward_wall", 0),
            Node {
                width: percent(100.),
                max_height: px(420.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    commands.entity(window).add_child(body);
    if history {
        if panel.details.is_some() {
            label(commands, skin, body, &details_text(panel));
        } else {
            label(
                commands,
                skin,
                body,
                "Refresh to load account/World history.",
            );
        }
    } else if let Some(details) = &panel.details {
        let header = row(commands, body);
        for text in ["Day", "Free", "Premium"] {
            label(commands, skin, header, text);
        }
        for (index, day) in details
            .days
            .iter()
            .take(usize::from(CYCLE_DAYS))
            .enumerate()
        {
            let cell = row(commands, body);
            commands.entity(cell).insert(DayCell);
            let today = panel
                .wall
                .is_some_and(|wall| usize::from(cycle_day(wall.lane_day)) == index + 1);
            if today {
                commands
                    .entity(cell)
                    .insert(BackgroundColor(Color::srgb_u8(65, 65, 45)));
            }
            label(
                commands,
                skin,
                cell,
                &format!(
                    "Day {}{}",
                    index + 1,
                    if today {
                        if panel.wall.is_some_and(|wall| wall.claimed) {
                            " — claimed"
                        } else {
                            " — today"
                        }
                    } else {
                        ""
                    }
                ),
            );
            label(commands, skin, cell, &tier_text(day.free));
            label(commands, skin, cell, &tier_text(day.premium));
        }
    } else {
        label(
            commands,
            skin,
            body,
            "Reward catalogue has not loaded. Refresh to request it.",
        );
    }
    if !panel.selected {
        label(
            commands,
            skin,
            window,
            "Daily rewards are unavailable in this session.",
        );
    }
    if panel.pending {
        label(commands, skin, window, "Waiting for the server…");
    }
    if let Some(feedback) = panel.feedback {
        label(commands, skin, window, feedback);
    }
    if panel
        .wall
        .is_some_and(|wall| !wall.claimed && !wall.claim_enabled)
    {
        label(
            commands,
            skin,
            window,
            "Collect unavailable. Open a reward shrine within reach, then refresh.",
        );
    }
    let actions = row(commands, window);
    button(
        commands,
        skin,
        actions,
        Control {
            kind: Kind::Collect,
            ..control
        },
        "Collect",
        collectable(panel),
    );
    button(
        commands,
        skin,
        actions,
        Control {
            kind: Kind::Refresh,
            ..control
        },
        "Refresh",
        panel.selected && !panel.pending,
    );
    button(commands, skin, actions, control, "Close", true);
    root
}
fn tier_text(tier: Tier) -> String {
    let mut parts = Vec::new();
    for (count, name) in [
        (tier.cosmetic_tokens, "Cosmetic Tokens"),
        (tier.prey_wildcards, "Prey Wildcards"),
        (tier.temple_scrolls, "Temple Teleport Scroll"),
        (tier.bounty_tokens, "Bounty Reroll Token"),
    ] {
        if count > 0 {
            parts.push(format!("{count} {name}"));
        }
    }
    if tier.xp_percent > 0 {
        parts.push(format!(
            "{} minutes XP Boost {}%",
            tier.xp_minutes, tier.xp_percent
        ));
    }
    if parts.is_empty() {
        "No reward".into()
    } else {
        parts.join("\n")
    }
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            width: percent(100.),
            column_gap: px(6.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, text: &str) {
    let entity = commands
        .spawn(source::text(text, source::font(skin.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                flex_basis: px(0.),
                flex_grow: 1.,
                min_width: px(0.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    control: Control,
    text: &str,
    enabled: bool,
) {
    let entity = commands
        .spawn((source::button(skin.buttons.clone(), 90.), control))
        .observe(activate)
        .id();
    if !enabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(text, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
