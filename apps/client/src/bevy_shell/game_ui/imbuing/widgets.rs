use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui_bevy as source;
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    serial: u64,
    panel: &ImbuePanel,
    items: &[ItemEntry],
    selection: &Selection,
) -> Entity {
    let root = commands
        .spawn((
            super::super::GameModal,
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
            Name::new("imbuing.window"),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
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
    source::dialog_caption(commands, window, "Imbuing", skins.font.clone());
    let control = |kind| Control {
        generation,
        serial,
        kind,
    };
    button(commands, skins, window, control(Kind::Close), "Close", true);
    if !panel.selected {
        text(
            commands,
            skins,
            window,
            "Imbuing is unavailable in this session.",
        );
        return root;
    }
    button(
        commands,
        skins,
        window,
        control(Kind::Tracker),
        "Imbuement tracker",
        true,
    );
    text(
        commands,
        skins,
        window,
        &costs::summary(panel, selection.offer),
    );
    let body = commands
        .spawn(Node {
            width: percent(100.),
            max_height: px(450.),
            column_gap: px(10.),
            ..default()
        })
        .id();
    commands.entity(window).add_child(body);
    let left = column(commands, body, 0);
    text(
        commands,
        skins,
        left,
        "Carried items (definition id / count)",
    );
    for item in items {
        let label = format!(
            "{}Item #{} — {} × {}",
            if panel.item == Some(item.handle) {
                "> "
            } else {
                ""
            },
            item.handle,
            item.item_definition_ref,
            item.count
        );
        button(
            commands,
            skins,
            left,
            control(Kind::Item(item.handle)),
            &label,
            !panel.pending,
        );
    }
    if items.is_empty() {
        text(commands, skins, left, "No carried items reported.");
    }
    text(commands, skins, left, "Item slots");
    for (index, label) in slot_rows(panel).iter().enumerate() {
        if let Ok(slot) = u8::try_from(index) {
            let label = format!(
                "{}{label}",
                if selection.slot == Some(slot) {
                    "> "
                } else {
                    ""
                }
            );
            button(
                commands,
                skins,
                left,
                control(Kind::Slot(slot)),
                &label,
                !panel.pending,
            );
        }
    }
    let right = column(commands, body, 1);
    text(commands, skins, right, "Base type");
    if let Some(model) = &panel.window {
        let tiers: std::collections::BTreeSet<_> =
            model.offers.iter().map(|offer| offer.tier).collect();
        for tier in tiers {
            button(
                commands,
                skins,
                right,
                control(Kind::BaseType(tier)),
                &format!(
                    "{}Tier {tier}",
                    if selection.base_type == Some(tier) {
                        "> "
                    } else {
                        ""
                    }
                ),
                !panel.pending,
            );
        }
        text(commands, skins, right, "Imbuements");
        for (index, label) in offer_rows(panel, selection.base_type) {
            let label = format!(
                "{}{label}",
                if selection.offer == Some(index) {
                    "> "
                } else {
                    ""
                }
            );
            button(
                commands,
                skins,
                right,
                control(Kind::Offer(index)),
                &label,
                !panel.pending,
            );
        }
        text(
            commands,
            skins,
            right,
            &format!("Clear cost: {} gold", model.clear_gold),
        );
        if selection.offer == Some(panel.offer)
            && let Some(offer) = model.offers.get(panel.offer)
            && let Some(quote) = panel
                .details
                .as_ref()
                .filter(|details| Some(details.item) == panel.item)
                .and_then(|details| {
                    details
                        .quotes
                        .iter()
                        .find(|quote| quote.family == offer.family && quote.tier == offer.tier)
                })
        {
            if quote.recipe_available {
                for material in &quote.materials {
                    text(
                        commands,
                        skins,
                        right,
                        &format!("{} × {}", material.quantity, material.definition_key),
                    );
                }
            } else {
                text(
                    commands,
                    skins,
                    right,
                    "No available recipe for this offer.",
                );
            }
        }
    }
    if let Some(expected) = &selection.confirmation
        && let Some(quote) = &expected.window
    {
        text(
            commands,
            skins,
            window,
            &format!(
                "Clear {} for {} gold?",
                slot_label(expected, expected.slot),
                quote.clear_gold
            ),
        );
        button(
            commands,
            skins,
            window,
            control(Kind::Confirm),
            "Confirm clear",
            panel.can_clear(),
        );
        button(
            commands,
            skins,
            window,
            control(Kind::Cancel),
            "Cancel",
            true,
        );
    } else {
        button(
            commands,
            skins,
            window,
            control(Kind::Apply),
            "Imbue",
            selection.can_apply(panel),
        );
        button(
            commands,
            skins,
            window,
            control(Kind::Clear),
            "Clear slot…",
            selection.slot == Some(panel.slot) && panel.can_clear(),
        );
    }
    button(
        commands,
        skins,
        window,
        control(Kind::Refresh),
        "Refresh item",
        !panel.pending && panel.item.is_some(),
    );
    if panel.pending {
        text(commands, skins, window, "Waiting for the server…");
    }
    if panel.refresh_required && panel.feedback.is_none() {
        text(
            commands,
            skins,
            window,
            "Imbuement operation succeeded. Refresh the item to load its updated slots.",
        );
    }
    if let Some(feedback) = panel.feedback {
        text(commands, skins, window, feedback);
    }
    if let Some(feedback) = panel.details_feedback {
        text(commands, skins, window, feedback);
    }
    root
}
fn column(commands: &mut Commands, parent: Entity, role: u32) -> Entity {
    let entity = commands
        .spawn((
            ScrollArea,
            crate::bevy_shell::game_ui::install::RememberScroll("imbuing", role),
            Node {
                width: percent(50.),
                min_width: px(0.),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, label: &str) {
    let entity = commands
        .spawn(source::text(label, source::font(skins.font.clone(), false)))
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
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    control: Control,
    label: &str,
    enabled: bool,
) {
    let entity = commands
        .spawn((source::button(skins.buttons.clone(), 220.), control))
        .insert(Node {
            width: percent(100.),
            min_height: px(24.),
            flex_shrink: 0.,
            padding: px(3.).all(),
            ..default()
        })
        .observe(activate)
        .id();
    if !enabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .insert(TextLayout::default())
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
