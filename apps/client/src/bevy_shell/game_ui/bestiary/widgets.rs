use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui_bevy as source;

pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    control: Control,
    panel: &Panel,
    confirm: Option<Pending>,
) -> Entity {
    let root = commands
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
                width: px(940.),
                height: px(650.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect::all(px(20.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(window);
    source::dialog_caption(commands, window, "Bestiary", skin.font.clone());
    text(
        commands,
        skin,
        window,
        if !panel.selected {
            "Bestiary unavailable in this session"
        } else if panel.pending {
            "Waiting for the server"
        } else {
            &panel.feedback
        },
    );
    let category = panel
        .catalogue
        .category
        .checked_sub(1)
        .and_then(|i| panel.catalogue.page.categories.get(usize::from(i)))
        .map_or("All categories", String::as_str);
    text(commands, skin, window, category);
    let navigation = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(window).add_child(navigation);
    let categories = !panel.catalogue.page.categories.is_empty();
    for (action, label, available) in [
        (0, "First page / Refresh", true),
        (1, "Next page", panel.catalogue.page.next.is_some()),
        (4, "Previous category", categories),
        (5, "Next category", categories),
        (2, "Creature details", !panel.fields[1].is_empty()),
        (3, "More loot", panel.catalogue.next_loot.is_some()),
    ] {
        button(
            commands,
            skin,
            navigation,
            Control {
                kind: Kind::Catalogue(action),
                ..control
            },
            label,
            panel.pending || !panel.selected || !available,
        );
    }
    button(
        commands,
        skin,
        navigation,
        Control {
            kind: Kind::Progress,
            ..control
        },
        "Refresh progress",
        panel.pending || !panel.selected,
    );
    button(commands, skin, navigation, control, "Close", false);
    let body = commands
        .spawn((
            ScrollArea,
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.,
                min_height: px(0.),
                overflow: Overflow::scroll(),
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(window).add_child(body);
    if panel.catalogue.page.cards.is_empty() && !panel.pending && panel.selected {
        text(commands, skin, body, "No creatures on this page");
    }
    for card in &panel.catalogue.page.cards {
        let progress = panel.reply.as_ref().and_then(|reply| {
            reply
                .bestiary
                .iter()
                .find(|p| p.race.get() == u32::from(card.race))
        });
        let label = format!(
            "{}{} [{}] — {}",
            if panel.fields[1] == card.race.to_string() {
                "> "
            } else {
                ""
            },
            card.name,
            card.category,
            progress.map_or_else(
                || "Progress not reported".into(),
                |p| format!("{} kills | Stage {}/3", p.kill_count, p.unlocked_stage())
            )
        );
        button(
            commands,
            skin,
            body,
            Control {
                kind: Kind::Select(card.race),
                ..control
            },
            &label,
            panel.pending || !panel.selected,
        );
    }
    if let Some(card) = panel.catalogue.selected_card(&panel.fields[1]) {
        if let Some(progress) = panel.reply.as_ref().and_then(|reply| {
            reply
                .bestiary
                .iter()
                .find(|p| p.race.get() == u32::from(card.race))
        }) {
            for (index, threshold) in progress.kill_thresholds.iter().enumerate() {
                let stage = index + 1;
                let kills = progress.kill_count.min(*threshold);
                let status = if kills == *threshold {
                    "complete"
                } else {
                    "in progress"
                };
                let label = format!("Stage {stage}: {kills}/{threshold} {status}");
                text(commands, skin, body, label);
            }
        }
        if let Some(detail) = panel
            .catalogue
            .detail_card
            .as_ref()
            .filter(|detail| detail.race == card.race)
        {
            text(commands, skin, body, detail.detail.as_str());
        }
    }
    charms(commands, skin, body, control, panel, confirm);
    root
}
/// Charm runes from the last progression reply, with the actions on the chosen charm.
fn charms(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    control: Control,
    panel: &Panel,
    confirm: Option<Pending>,
) {
    let Some(reply) = &panel.reply else {
        return;
    };
    let view = &reply.charms;
    text(
        commands,
        skin,
        parent,
        format!(
            "Charms — points: {} | minor echoes: {} | assignment slots: {}",
            view.charm_points_available,
            view.minor_charm_echoes_available,
            view.assignment_slot_limit
                .map_or_else(|| "unlimited".into(), |limit| limit.to_string())
        ),
    );
    let busy = panel.pending || !panel.selected;
    let chosen = view
        .charms
        .iter()
        .find(|charm| panel.fields[0] == charm.charm.get().to_string());
    for charm in &view.charms {
        let name = u16::try_from(charm.charm.get())
            .ok()
            .and_then(|id| panel.charm_browser.cards.get(&id))
            .map_or_else(
                || format!("Charm {}", charm.charm),
                |card| card.name.clone(),
            );
        let race = charm.assigned_race.map_or_else(
            || "unassigned".into(),
            |race| {
                u16::try_from(race.get())
                    .ok()
                    .and_then(|race| panel.catalogue.names.get(&race))
                    .map_or_else(|| format!("race {race}"), Clone::clone)
            },
        );
        let label = format!(
            "{}{name} ({:?}) — stage {}/3, next cost {}, {race}{}",
            if chosen == Some(charm) { "> " } else { "" },
            charm.kind,
            charm.unlocked_stage,
            charm.next_stage_cost,
            if charm.effect_active {
                ""
            } else {
                ", not yet active"
            }
        );
        button(
            commands,
            skin,
            parent,
            Control {
                kind: Kind::Charm(charm.charm.get()),
                ..control
            },
            &label,
            busy,
        );
    }
    if let Some(card) = chosen
        .and_then(|charm| u16::try_from(charm.charm.get()).ok())
        .and_then(|id| panel.charm_browser.cards.get(&id))
    {
        text(
            commands,
            skin,
            parent,
            format!("{} — {}", card.name, card.category),
        );
        text(commands, skin, parent, card.detail.as_str());
    }
    let creature = panel.catalogue.selected_card(&panel.fields[1]).is_some();
    let actions = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(actions);
    for (intent, label, available) in [
        (
            Intent::Unlock,
            "Unlock next stage",
            chosen.is_some_and(|charm| charm.next_stage_cost != 0),
        ),
        (
            Intent::Assign,
            "Assign to selected creature",
            creature && chosen.is_some_and(|charm| charm.assigned_race.is_none()),
        ),
        (
            Intent::Remove,
            tr!("Remove from creature"),
            chosen.is_some_and(|charm| charm.assigned_race.is_some()),
        ),
        (
            Intent::ResetAll,
            tr!("Reset all charms"),
            // The reset refunds unlocked stages too, assigned or not.
            view.charms.iter().any(|charm| charm.unlocked_stage > 0),
        ),
    ] {
        button(
            commands,
            skin,
            actions,
            Control {
                kind: Kind::CharmAction(intent),
                ..control
            },
            label,
            busy || !available,
        );
    }
    let Some((intent, fee)) = confirm else {
        return;
    };
    let action = if intent == Intent::ResetAll {
        tr!("Reset all charms").to_string()
    } else {
        chosen.map_or_else(
            || tr!("Remove the charm").into(),
            |charm| {
                let name = u16::try_from(charm.charm.get())
                    .ok()
                    .and_then(|id| panel.charm_browser.cards.get(&id))
                    .map_or_else(
                        || format!("Charm {}", charm.charm),
                        |card| card.name.clone(),
                    );
                lfmt!("Remove {name} from its creature", name = name)
            },
        )
    };
    text(
        commands,
        skin,
        parent,
        fee.map_or_else(
            || {
                lfmt!(
                    "{action}? The gold fee depends on your level, which is not reported yet.",
                    action = action
                )
            },
            |fee| lfmt!("{action} for {fee} gold?", action = action, fee = fee),
        ),
    );
    let answer = commands
        .spawn(Node {
            column_gap: px(4.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(answer);
    button(
        commands,
        skin,
        answer,
        Control {
            kind: Kind::Confirm(intent),
            ..control
        },
        &fee.map_or_else(
            || tr!("Confirm").into(),
            |fee| lfmt!("Confirm ({fee} gold)", fee = fee),
        ),
        busy || fee.is_none(),
    );
    button(
        commands,
        skin,
        answer,
        Control {
            kind: Kind::Cancel,
            ..control
        },
        tr!("Cancel"),
        false,
    );
}
fn text(commands: &mut Commands, skin: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn((
            source::text(value, source::font(skin.font.clone(), false)),
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
    skin: &Skins,
    parent: Entity,
    control: Control,
    label: &str,
    disabled: bool,
) {
    let entity = commands
        .spawn((source::button(skin.buttons.clone(), 200.), control))
        .observe(activate)
        .id();
    if matches!(control.kind, Kind::Select(_)) {
        commands.entity(entity).insert(Node {
            width: percent(100.),
            min_height: px(24.),
            border: UiRect::all(px(1.)),
            padding: UiRect::all(px(3.)),
            flex_shrink: 0.,
            align_items: AlignItems::Center,
            ..default()
        });
    }
    if disabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(label, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
