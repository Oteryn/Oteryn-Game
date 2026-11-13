use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui_widgets::{ScrollArea, TextInput},
};
pub(super) fn build(commands: &mut Commands, skin: &Skins, parent: Entity) -> Entity {
    let root = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            TabGroup {
                order: 3,
                modal: true,
            },
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(1000.),
                height: px(650.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(10.),
                    right: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(root);
    source::dialog_caption(commands, root, "Market", skin.font.clone());
    root
}
pub(super) fn render(
    commands: &mut Commands,
    skin: &Skins,
    frame: Entity,
    key: Key,
    panel: &MarketPanel,
    selected: Option<&Ware>,
) -> Entity {
    let root = commands
        .spawn(Node {
            width: percent(100.),
            flex_grow: 1.,
            min_height: px(0.),
            flex_direction: FlexDirection::Column,
            row_gap: px(8.),
            ..default()
        })
        .id();
    commands.entity(frame).add_child(root);
    let content = commands
        .spawn(Node {
            width: percent(100.),
            flex_grow: 1.,
            min_height: px(0.),
            column_gap: px(10.),
            ..default()
        })
        .id();
    commands.entity(root).add_child(content);
    let left = pane(commands, skin, content, px(160.));
    label(commands, skin, left, "Categories");
    let categories = scroll(commands, left, px(137.));
    button(
        commands,
        skin,
        categories,
        key,
        Kind::Category(0),
        "All",
        140.,
    );
    for (index, category) in panel.details.categories.iter().enumerate() {
        button(
            commands,
            skin,
            categories,
            key,
            Kind::Category(index + 1),
            category,
            140.,
        );
    }
    label(commands, skin, left, "Search catalogue");
    let mut text = EditableText::new(&panel.details.filter.search);
    text.max_characters = Some(80);
    let field = commands
        .spawn((
            Search,
            TextInput,
            text,
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            Node {
                width: percent(100.),
                height: px(25.),
                flex_shrink: 0.,
                padding: px(4.).all(),
                ..default()
            },
            source::frame(skin.inset.clone(), [1.; 4]),
        ))
        .id();
    commands.entity(left).add_child(field);
    button(commands, skin, left, key, Kind::Search, "Search", 140.);
    let items = scroll(commands, left, Val::Auto);
    for (index, ware) in panel.details.items.iter().enumerate() {
        button(
            commands,
            skin,
            items,
            key,
            Kind::Item(index),
            &ware.name,
            140.,
        );
    }
    if panel.details.items.is_empty() {
        label(commands, skin, items, "No catalogue items loaded");
    }
    label(
        commands,
        skin,
        left,
        &format!("Catalogue page {}", panel.details.page + 1),
    );
    let navigation = row(commands, left);
    button(
        commands,
        skin,
        navigation,
        key,
        Kind::Catalogue(-1),
        "Previous",
        80.,
    );
    button(
        commands,
        skin,
        navigation,
        key,
        Kind::Catalogue(1),
        "Next",
        70.,
    );
    let right = pane(commands, skin, content, Val::Auto);
    label(
        commands,
        skin,
        right,
        selected.map_or("Select an item", |ware| ware.name.as_str()),
    );
    if let Some(ware) = selected {
        // The server definition reference is not an appearance identifier.
        label(commands, skin, right, "Item artwork unavailable");
        if panel.definition == ware.definition {
            let books = row(commands, right);
            for (side, title) in [(1, "Buy offers"), (2, "Sell offers")] {
                let book = pane(commands, skin, books, percent(49.));
                label(commands, skin, book, title);
                label(commands, skin, book, "Amount       Price       Total");
                let offers = scroll(commands, book, px(190.));
                let mut count = 0;
                for offer in panel
                    .rows
                    .iter()
                    .filter(|offer| offer.definition == ware.definition && offer.side == side)
                {
                    label(
                        commands,
                        skin,
                        offers,
                        &format!(
                            "{}       {}       {}",
                            offer.amount,
                            offer.price,
                            u128::from(offer.amount) * u128::from(offer.price)
                        ),
                    );
                    count += 1;
                }
                if count == 0 {
                    label(commands, skin, offers, "No offers on this page");
                }
            }
            label(
                commands,
                skin,
                right,
                &format!("Offer page {}", panel.page + 1),
            );
            let navigation = row(commands, right);
            button(
                commands,
                skin,
                navigation,
                key,
                Kind::Book(-1),
                "Previous",
                90.,
            );
            button(commands, skin, navigation, key, Kind::Book(1), "Next", 70.);
            button(
                commands,
                skin,
                navigation,
                key,
                Kind::Book(0),
                "Refresh offers",
                110.,
            );
            let stats = scroll(commands, right, px(130.));
            label(
                commands,
                skin,
                stats,
                &oteryn_client_ui::market::details::statistics_text(panel),
            );
            button(
                commands,
                skin,
                right,
                key,
                Kind::Statistics,
                "Refresh statistics",
                150.,
            );
        }
    }
    if !panel.selected {
        label(
            commands,
            skin,
            root,
            "Market is unavailable in this session",
        );
    }
    if panel.details.pending.is_some() || panel.pending.is_some() {
        label(commands, skin, root, "Waiting for the server…");
    }
    if let Some(feedback) = panel.details.feedback.or(panel.feedback) {
        label(commands, skin, root, feedback);
    }
    let footer = row(commands, root);
    button(commands, skin, footer, key, Kind::Close, "Close", 80.);
    root
}
fn pane(commands: &mut Commands, skin: &Skins, parent: Entity, width: Val) -> Entity {
    let entity = commands
        .spawn((
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width,
                flex_grow: if width == Val::Auto { 1. } else { 0. },
                min_width: px(0.),
                min_height: px(0.),
                padding: px(5.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            width: percent(100.),
            column_gap: px(5.),
            row_gap: px(5.),
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn scroll(commands: &mut Commands, parent: Entity, height: Val) -> Entity {
    let entity = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                height,
                min_height: px(0.),
                flex_grow: if height == Val::Auto { 1. } else { 0. },
                flex_shrink: 1.,
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, value: &str) {
    let entity = commands
        .spawn(source::text(value, source::font(skin.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
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
    key: Key,
    kind: Kind,
    value: &str,
    width: f32,
) {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), width),
            Action { key, kind },
        ))
        .insert(Node {
            width: px(width),
            max_width: percent(100.),
            min_height: px(22.),
            height: Val::Auto,
            flex_shrink: 0.,
            padding: px(3.).all(),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption(value, skin.font.clone()))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}
