use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui::item_forge::{self, resource_dialog::Content};
use oteryn_client_ui_bevy as source;
use std::cmp::Reverse;

pub(super) struct Widgets {
    pub overlay: Entity,
    pub panel: Entity,
    pub content: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skins: &Skins) -> Widgets {
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
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(760.),
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
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Exaltation Forge", skins.font.clone());
    Widgets {
        overlay,
        panel,
        content: None,
    }
}
pub(super) fn render(
    commands: &mut Commands,
    skins: &Skins,
    widgets: &mut Widgets,
    key: Key,
    panel: &Panel,
) {
    if let Some(content) = widgets.content.take() {
        commands.entity(content).despawn();
    }
    let content = column(commands, widgets.panel);
    widgets.content = Some(content);
    let tabs = row(commands, content);
    for tab in Tab::ALL {
        let label = if key.tab == tab {
            format!("[{}]", tab.label())
        } else {
            tab.label().into()
        };
        button(
            commands,
            skins,
            tabs,
            key,
            Kind::Tab(tab),
            &label,
            panel.selected && !panel.pending,
        );
    }
    let body = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                height: px(440.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(content).add_child(body);
    if !panel.selected {
        text(
            commands,
            skins,
            body,
            "This server has not enabled the Exaltation Forge.",
        );
    }
    match key.tab {
        Tab::Fusion | Tab::Transfer => pair(commands, skins, body, key, panel),
        Tab::Conversion => {
            for operation in [
                Operation::DustToSlivers,
                Operation::SliversToCores,
                Operation::DustLimit,
            ] {
                text(
                    commands,
                    skins,
                    body,
                    item_forge::resource_dialog::operation_name(operation),
                );
                button(
                    commands,
                    skins,
                    body,
                    key,
                    Kind::Convert(operation),
                    "Review conversion",
                    panel.editing() && panel.preview.pending.is_none(),
                );
            }
            dialog(commands, skins, body, key, panel);
        }
        Tab::History => {
            button(
                commands,
                skins,
                body,
                key,
                Kind::Convert(Operation::QueryHistory),
                "Refresh history",
                panel.selected && !panel.pending,
            );
            match &panel.decoded {
                Some(item_forge::reply::Reply::History(rows)) => {
                    if rows.is_empty() {
                        text(commands, skins, body, "No forge operations yet");
                    }
                    for (ordinal, receipt) in rows {
                        text(
                            commands,
                            skins,
                            body,
                            format!("History #{ordinal}\n{}", item_forge::reply::render(receipt)),
                        );
                    }
                }
                _ => text(
                    commands,
                    skins,
                    body,
                    "Request forge history to view recorded operations.",
                ),
            }
        }
    }
    if panel.pending || panel.preview.pending.is_some() {
        text(commands, skins, content, "Waiting for server…");
    }
    if let Some(feedback) = &panel.feedback {
        text(commands, skins, content, feedback);
    }
    if let Some(feedback) = &panel.preview.feedback {
        text(commands, skins, content, feedback);
    }
    let footer = row(commands, content);
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Refresh,
        "Refresh items",
        panel.editing() && panel.preview.selected,
    );
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Next,
        "More items",
        panel.editing()
            && panel.preview.pending.is_none()
            && panel
                .preview
                .page
                .as_ref()
                .is_some_and(|page| page.next.is_some()),
    );
    button(commands, skins, footer, key, Kind::Close, "Close", true);
}
fn pair(commands: &mut Commands, skins: &Skins, parent: Entity, key: Key, panel: &Panel) {
    let convergence = matches!(
        panel.operation,
        Operation::ConvergenceFusion | Operation::ConvergenceTransfer
    );
    button(
        commands,
        skins,
        parent,
        key,
        Kind::Convergence,
        if convergence {
            "Convergence: on"
        } else {
            "Convergence: off"
        },
        panel.editing(),
    );
    let mut items: Vec<_> = panel
        .preview
        .items
        .iter()
        .filter(|item| !item.container)
        .collect();
    // OTClient handleFusionItems: tier descending, item id ascending. Handles break ties only.
    items.sort_by_key(|item| (Reverse(item.tier), item.definition_ref, item.handle));
    text(
        commands,
        skins,
        parent,
        "Receiver / source (tier donor for Transfer)",
    );
    for item in items {
        let entry = column(commands, parent);
        text(
            commands,
            skins,
            entry,
            format!(
                "{} — tier {}, item {}",
                item.name, item.tier, item.definition_ref
            ),
        );
        let choices = row(commands, entry);
        let id = item.handle.get();
        button(
            commands,
            skins,
            choices,
            key,
            Kind::First(id),
            if panel.first_item == id {
                "Receiver selected"
            } else {
                "Select receiver"
            },
            panel.editing(),
        );
        button(
            commands,
            skins,
            choices,
            key,
            Kind::Second(id),
            if panel.second_item == id {
                "Source selected"
            } else {
                "Select source"
            },
            panel.editing(),
        );
    }
    if !panel.preview.selected {
        text(
            commands,
            skins,
            parent,
            "Pair costs and tiers are unavailable on this server. A current quote is required to forge.",
        );
        for item in &panel.preview.live {
            text(
                commands,
                skins,
                parent,
                format!("Item {} × {}", item.item_definition_ref, item.count),
            );
        }
    }
    if panel.operation == Operation::Fusion {
        let cores = row(commands, parent);
        button(
            commands,
            skins,
            cores,
            key,
            Kind::Boost,
            if panel.boost {
                "Boost: on"
            } else {
                "Boost: off"
            },
            panel.editing(),
        );
        button(
            commands,
            skins,
            cores,
            key,
            Kind::Protect,
            if panel.protect {
                "Protect: on"
            } else {
                "Protect: off"
            },
            panel.editing(),
        );
    }
    let name = |handle| {
        panel
            .preview
            .item(handle)
            .map_or("Not selected", |item| item.name.as_str())
    };
    text(
        commands,
        skins,
        parent,
        format!(
            "Receiver: {}\nSource: {}",
            name(panel.first_item),
            name(panel.second_item)
        ),
    );
    if let Some(quote) = &panel.preview.quote {
        text(
            commands,
            skins,
            parent,
            format!(
                "Cost: {} gold, {} dust, {} cores\nAvailable: {} carried gold; {} bank gold{}; {} / {} dust; {} cores\nTarget tier: {}; success: {}%; protection: {}%",
                quote.cost.gold,
                quote.cost.dust,
                quote.cost.cores,
                quote.resources.carried_gold,
                quote.resources.bank_gold,
                if quote.resources.bank_spendable {
                    ""
                } else {
                    " (unavailable)"
                },
                quote.resources.dust,
                quote.resources.dust_limit,
                quote.resources.cores,
                quote.target_tier,
                quote.success_percent,
                quote.protection_percent,
            ),
        );
    } else if panel.preview.selected {
        text(
            commands,
            skins,
            parent,
            "Choose two different items, then request a current quote.",
        );
    }
    let actions = row(commands, parent);
    let pair_valid = panel.first_item != panel.second_item && panel.selection().is_some();
    button(
        commands,
        skins,
        actions,
        key,
        Kind::Quote,
        "Request quote",
        panel.editing() && panel.preview.selected && panel.preview.pending.is_none() && pair_valid,
    );
    button(
        commands,
        skins,
        actions,
        key,
        Kind::Submit,
        "Forge selected pair",
        forge_action_enabled(panel),
    );
    dialog(commands, skins, parent, key, panel);
}
fn dialog(commands: &mut Commands, skins: &Skins, parent: Entity, key: Key, panel: &Panel) {
    if let Some(content) = &panel.resource_dialog.content {
        text(
            commands,
            skins,
            parent,
            item_forge::resource_dialog::dialog_text(content, 0),
        );
        let actions = row(commands, parent);
        if matches!(content, Content::Confirm { .. }) {
            button(
                commands,
                skins,
                actions,
                key,
                Kind::Confirm(panel.resource_dialog.serial),
                "Confirm conversion",
                panel.selected
                    && !panel.pending
                    && panel.preview.pending.is_none()
                    && panel.retry.is_none(),
            );
        }
        button(
            commands,
            skins,
            actions,
            key,
            Kind::Dismiss(panel.resource_dialog.serial),
            "Dismiss",
            !panel.pending,
        );
    }
}
fn column(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            width: percent(100.),
            flex_direction: FlexDirection::Column,
            row_gap: px(5.),
            min_height: px(0.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            width: percent(100.),
            column_gap: px(6.),
            flex_shrink: 0.,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                max_width: percent(100.),
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
    key: Key,
    kind: Kind,
    label: &str,
    enabled: bool,
) {
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), 150.),
        Control { key, kind },
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
