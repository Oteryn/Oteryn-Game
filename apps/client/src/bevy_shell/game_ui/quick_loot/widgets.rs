use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub overlay: Entity,
    pub panel: Entity,
    pub content: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skins: &Skins) -> Widgets {
    let overlay = commands
        .spawn((
            Overlay,
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
                width: px(700.),
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
    source::dialog_caption(
        commands,
        panel,
        "Manage Loot Containers",
        skins.font.clone(),
    );
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
    text(
        commands,
        skins,
        content,
        "Saved settings cannot be read back. Changes shown here were confirmed during this session.",
    );
    let body = commands
        .spawn(Node {
            width: percent(100.),
            height: px(400.),
            min_height: px(0.),
            column_gap: px(8.),
            ..default()
        })
        .id();
    commands.entity(content).add_child(body);
    let destinations = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            ScrollArea,
            Node {
                flex_grow: 1.,
                min_width: px(0.),
                padding: UiRect::all(px(5.)),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(body).add_child(destinations);
    text(commands, skins, destinations, "Loot containers");
    for (category, label) in [
        (None, "Default"),
        (Some(Category::Gold), "Gold"),
        (Some(Category::CreatureProduct), "Creature products"),
        (Some(Category::Equipment), "Equipment"),
        (Some(Category::Consumable), "Consumables"),
        (Some(Category::Other), "Other"),
    ] {
        let row = column(commands, destinations);
        text(commands, skins, row, label);
        let confirmed = panel
            .destinations
            .iter()
            .find(|action| matches!(action,Action::Destination {category:c,..} if *c==category));
        if let Some(Action::Destination {
            recipient,
            fallback,
            ..
        }) = confirmed
        {
            let recipient = match recipient {
                Recipient::MainContainer => "Main backpack",
                Recipient::Container => "Container",
                Recipient::GoldPouch => "Gold pouch",
            };
            text(
                commands,
                skins,
                row,
                format!(
                    "Confirmed: {recipient}; fallback {}",
                    if *fallback { "on" } else { "off" }
                ),
            );
        } else {
            text(commands, skins, row, "Saved destination not available");
        }
        let buttons = row_node(commands, row);
        for (fallback, label) in [(false, "Use main"), (true, "Main + fallback")] {
            button(
                commands,
                skins,
                buttons,
                key,
                Kind::Request(Action::Destination {
                    category,
                    recipient: Recipient::MainContainer,
                    id: None,
                    fallback,
                }),
                label,
                102.,
                panel,
            );
        }
        if let Some(Action::Destination {
            category,
            recipient,
            id,
            fallback,
        }) = confirmed
        {
            button(
                commands,
                skins,
                row,
                key,
                Kind::Request(Action::Destination {
                    category: *category,
                    recipient: *recipient,
                    id: *id,
                    fallback: !*fallback,
                }),
                "Toggle confirmed fallback",
                190.,
                panel,
            );
        }
    }
    let filters = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(234.),
                max_width: percent(45.),
                padding: UiRect::all(px(6.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(body).add_child(filters);
    text(commands, skins, filters, "QuickLoot filter");
    let selected = panel.confirmed.iter().rev().find_map(|action| {
        if let Action::Select(mode) = action {
            Some(*mode)
        } else {
            None
        }
    });
    text(
        commands,
        skins,
        filters,
        match selected {
            Some(Mode::Skip) => "Confirmed: skip listed items",
            Some(Mode::Accept) => "Confirmed: only listed items",
            None => "Saved filter mode not available",
        },
    );
    for (mode, label, clear) in [
        (
            Mode::Skip,
            "Skip listed items (blacklist)",
            "Clear blacklist",
        ),
        (
            Mode::Accept,
            "Only listed items (whitelist)",
            "Clear whitelist",
        ),
    ] {
        button(
            commands,
            skins,
            filters,
            key,
            Kind::Request(Action::Select(mode)),
            label,
            210.,
            panel,
        );
        button(
            commands,
            skins,
            filters,
            key,
            Kind::Request(Action::Clear(mode)),
            clear,
            130.,
            panel,
        );
    }
    let known = panel.skipped.len() + panel.accepted.len();
    text(
        commands,
        skins,
        filters,
        format!("{known} filter entries confirmed this session"),
    );
    if let Some(result) = &panel.result {
        text(commands, skins, filters, "Last confirmed result:");
        text(
            commands,
            skins,
            filters,
            if result.items.is_empty() {
                "No items reported".to_owned()
            } else {
                format!(
                    "Looted {} of {} items",
                    result.items.iter().filter(|item| item.accepted).count(),
                    result.items.len()
                )
            },
        );
    }
    if !panel.selected {
        text(
            commands,
            skins,
            content,
            "Quick loot is unavailable in this session",
        );
    }
    if !panel.feedback.is_empty() {
        text(commands, skins, content, &panel.feedback);
    }
    let footer = row_node(commands, content);
    commands.entity(footer).insert(Node {
        justify_content: JustifyContent::End,
        column_gap: px(8.),
        ..default()
    });
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Request(Action::LootAll),
        "Loot nearby",
        100.,
        panel,
    );
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Close,
        "Close",
        70.,
        panel,
    );
}
fn column(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            width: percent(100.),
            min_height: px(0.),
            flex_direction: FlexDirection::Column,
            row_gap: px(4.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn row_node(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            column_gap: px(4.),
            flex_wrap: FlexWrap::Wrap,
            row_gap: px(3.),
            flex_shrink: 0.,
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
// The button receives the canonical snapshot key and the source control's size/caption.
#[allow(clippy::too_many_arguments)]
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    label: &str,
    width: f32,
    panel: &Panel,
) {
    let enabled = kind == Kind::Close || panel.selected && panel.pending.is_none();
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), width),
        Control { key, kind },
    ));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let text = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}
