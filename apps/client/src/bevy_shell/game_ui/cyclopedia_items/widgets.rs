use super::*;

pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    panel: &Panel,
    details: &str,
    (search_value, own_draft): (&str, Option<&str>),
    control: Control,
    (can_continue_drops, loot, filters): (bool, LootControls, Filters),
) -> Entity {
    let overlay = commands
        .spawn((
            GameModal,
            crate::bevy_shell::ui::window_layout::WindowLayer("cyclopedia_items"),
            GlobalZIndex(170),
            bevy::input_focus::tab_navigation::TabGroup {
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
        ))
        .id();
    let window = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(820.),
                height: px(620.),
                max_width: percent(95.),
                max_height: percent(94.),
                padding: UiRect::all(px(22.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(window);
    source::dialog_caption(commands, window, "Cyclopedia Items", skin.font.clone());
    use crate::bevy_shell::game_ui::cyclopedia_navigation::{Subject, bar};
    bar(commands, skin, window, control.generation, Subject::Items);
    label(
        commands,
        skin,
        window,
        "Search item name; choose a category, then Browse. Prices include market offers and completed trades.",
    );
    let search = commands
        .spawn((
            Search,
            TextInput,
            bevy::input_focus::tab_navigation::TabIndex(0),
            EditableText::new(search_value),
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            Node {
                width: percent(100.),
                min_height: px(24.),
                ..default()
            },
        ))
        .id();
    if panel.pending || !panel.selected {
        commands.entity(search).insert(InteractionDisabled);
    }
    commands.entity(window).add_child(search);
    label(
        commands,
        skin,
        window,
        &format!(
            "Category: {}",
            if panel.category.is_empty() {
                "All"
            } else {
                &panel.category
            }
        ),
    );
    let buttons = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(window).add_child(buttons);
    let selected = panel
        .catalogue_definition
        .parse::<u32>()
        .ok()
        .is_some_and(|id| panel.catalogue.contains_key(&id));
    if selected && loot.analyser {
        label(
            commands,
            skin,
            window,
            &loot.source.zip(loot.value).map_or_else(
                || "Loot unit value".into(),
                |(source, value)| format!("Loot unit value: {value} gold ({source:?})"),
            ),
        );
        commands.spawn((
            ChildOf(window),
            OwnPrice,
            TextInput,
            bevy::input_focus::tab_navigation::TabIndex(1),
            EditableText::new(
                own_draft
                    .or(loot.value.map(|v| v.to_string()).as_deref())
                    .unwrap_or(""),
            ),
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            Node {
                width: px(160.),
                min_height: px(24.),
                ..default()
            },
        ));
    }
    for (action, caption, enabled) in [
        (Action::Browse, "Browse", true),
        (Action::Next, "Next page", panel.catalogue_next.is_some()),
        (
            Action::Category(true),
            "Previous category",
            !panel.categories().is_empty(),
        ),
        (
            Action::Category(false),
            "Next category",
            !panel.categories().is_empty(),
        ),
        (Action::Details, "Item details", selected),
        (Action::DropSources, "Drop sources", selected),
        (
            Action::NextDropSources,
            "Next drop sources",
            can_continue_drops,
        ),
        (
            Action::Track,
            if loot.tracked {
                "Stop tracking drops"
            } else {
                "Track drops"
            },
            selected && loot.analyser,
        ),
        (
            Action::ExcludeSale,
            if loot.excluded {
                "Allow quick selling"
            } else {
                "Exclude from quick selling"
            },
            selected,
        ),
        (
            Action::MarketValue,
            "Use market average",
            selected && loot.analyser && loot.market.is_some(),
        ),
        (
            Action::MerchantValue,
            "Use merchant buy price",
            selected && loot.analyser && loot.merchant.is_some(),
        ),
        (Action::OwnValue, "Use own value", selected && loot.analyser),
        (Action::Prices, "Market prices", selected),
        (
            Action::Hands,
            ["Hands: all", "One-handed weapons", "Two-handed weapons"][usize::from(filters.hands)],
            true,
        ),
        (
            Action::Class,
            filters.class.map_or("Class: all", |c| {
                ["Unclassified", "Class 1", "Class 2", "Class 3", "Class 4"][usize::from(c)]
            }),
            true,
        ),
        (
            Action::Level,
            if filters.level {
                "Own level only"
            } else {
                "All levels"
            },
            true,
        ),
        (
            Action::Vocation,
            if filters.vocation {
                "Own vocation only"
            } else {
                "All vocations"
            },
            true,
        ),
        (
            Action::ManageLoot,
            "Manage loot containers",
            selected && loot.available,
        ),
        (
            Action::LootFilter(Mode::Skip),
            if loot.skipped {
                "Remove from skip list"
            } else {
                "Add to skip list"
            },
            selected && loot.ready,
        ),
        (
            Action::LootFilter(Mode::Accept),
            if loot.accepted {
                "Remove from accept list"
            } else {
                "Add to accept list"
            },
            selected && loot.ready,
        ),
        (Action::Close, "Close", true),
    ] {
        button(
            commands,
            skin,
            buttons,
            Control { action, ..control },
            caption,
            action != Action::Close && (!enabled || panel.pending || !panel.selected),
        );
    }
    let scroll = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                flex_grow: 1.,
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    commands.entity(window).add_child(scroll);
    for item in panel.catalogue.values() {
        button(
            commands,
            skin,
            scroll,
            Control {
                action: Action::Select(item.definition_ref),
                ..control
            },
            &item.name,
            panel.pending || !panel.selected,
        );
    }
    label(commands, skin, scroll, &details.replace("\r\n", "\n"));
    if panel.pending {
        label(commands, skin, scroll, "Waiting for the server");
    }
    overlay
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, text: &str) {
    let child = commands
        .spawn((
            source::text(text, source::font(skin.font.clone(), false)),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(child);
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    control: Control,
    caption: &str,
    disabled: bool,
) {
    let button = commands
        .spawn((source::button(skin.buttons.clone(), 130.), control))
        .observe(activate)
        .id();
    if disabled {
        commands.entity(button).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(caption, skin.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
}
