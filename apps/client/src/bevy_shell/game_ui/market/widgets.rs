use super::offers::PendingFilter;
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
            Name::new("market.window"),
            TabGroup {
                order: 3,
                modal: true,
            },
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            // Tibia Global market: 1000 wide over a 600px content area.
            crate::bevy_shell::ui::window_layout::window(
                "market",
                Node {
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
            ),
        ))
        .id();
    commands.entity(parent).add_child(root);
    source::dialog_caption(commands, root, "Market", skin.font.clone());
    root
}
// The window projects its panel, item, offer selection and offer type together.
#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    commands: &mut Commands,
    skin: &Skins,
    frame: Entity,
    key: Key,
    panel: &MarketPanel,
    selected: Option<&Ware>,
    selected_offer: Option<usize>,
    sell: bool,
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
    let categories = scroll(commands, left, px(137.), 0);
    commands
        .entity(categories)
        .insert(Name::new("market.categories"));
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
        category_button(commands, skin, categories, key, index + 1, category);
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
    catalogue_filters(commands, skin, left, key, panel);
    let items = scroll(commands, left, Val::Auto, 1);
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
    let right = scroll(commands, right, Val::Auto, 2);
    label(
        commands,
        skin,
        right,
        selected.map_or("Select an item", |ware| ware.name.as_str()),
    );
    if panel.view != MarketView::Browse {
        offers::list(commands, skin, right, key, panel);
    }
    if let Some(ware) = selected.filter(|_| panel.view == MarketView::Browse) {
        // The server definition reference is not an appearance identifier.
        label(commands, skin, right, "Item artwork unavailable");
        label(
            commands,
            skin,
            right,
            &format!(
                "Required level: {} · Forge class: {} · In depot: {}",
                ware.level
                    .map_or_else(|| "Unknown".into(), |level| level.to_string()),
                ware.forge_class
                    .map_or_else(|| "Unknown".into(), |class| class.to_string()),
                ware.depot_units,
            ),
        );
        if panel.definition == ware.definition {
            for (side, title) in [(2, "Sell Offers"), (1, "Buy Offers")] {
                label(commands, skin, right, title);
                let table = pane(commands, skin, right, percent(100.));
                table_row(
                    commands,
                    skin,
                    table,
                    ["Name", "Amount", "Piece Price", "Total Price", "Ends at"].map(str::to_owned),
                );
                let offers = scroll(commands, table, px(190.), 2 + u32::from(side));
                let mut count = 0;
                for (index, offer) in
                    panel.rows.iter().enumerate().filter(|(_, offer)| {
                        offer.definition == ware.definition && offer.side == side
                    })
                {
                    // MarketRow carries no trader name or expiry: those cells stay empty.
                    let cells = table_row(
                        commands,
                        skin,
                        offers,
                        [
                            String::new(),
                            offer.amount.to_string(),
                            offer.price.to_string(),
                            (u128::from(offer.amount) * u128::from(offer.price)).to_string(),
                            String::new(),
                        ],
                    );
                    offers::select_button(commands, skin, cells, key, index);
                    if offer.side == 2 {
                        purchase::row(commands, skin, cells, key, offer);
                    }
                    count += 1;
                }
                if count == 0 {
                    label(commands, skin, offers, "No offers");
                }
            }
            // The server pages each item's offers; the list scrolls within a page.
            let navigation = row(commands, right);
            button(
                commands,
                skin,
                navigation,
                key,
                Kind::Book(-1),
                "Previous",
                75.,
            );
            button(commands, skin, navigation, key, Kind::Book(1), "Next", 75.);
            let stats = scroll(commands, right, px(180.), 5);
            statistics::render(commands, skin, stats, panel);
            label(
                commands,
                skin,
                stats,
                &oteryn_client_ui::market::details::statistics_text(panel),
            );
        }
    }
    if panel.view == MarketView::Browse {
        offers::anonymity(commands, skin, right, key);
    }
    if panel.view != MarketView::History {
        offers::editor(
            commands,
            skin,
            right,
            key,
            panel,
            selected,
            selected_offer,
            sell,
        );
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
    // Tibia Global's bottom row: 75px buttons, Close last.
    let footer = row(commands, root);
    commands.entity(footer).insert(Name::new("market.footer"));
    for (kind, title) in [
        (Kind::Browse, "Offers"),
        (Kind::Statistics, "Details"),
        (Kind::Intent(Intent::OwnOffers), "My Offers"),
        (Kind::Book(0), "Current Offers"),
        (Kind::Intent(Intent::History), "Offer History"),
        (Kind::Close, "Close"),
    ] {
        button(commands, skin, footer, key, kind, title, 75.);
    }
    root
}
/// Offer table widths from Tibia Global: Name 244, Amount, Piece Price, Total Price 130, Ends 150.
const OFFER_COLUMNS: [f32; 5] = [244., 130., 130., 130., 150.];
fn table_row(commands: &mut Commands, skin: &Skins, parent: Entity, cells: [String; 5]) -> Entity {
    let line = commands
        .spawn((
            Name::new("market.offer_row"),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                align_items: AlignItems::Center,
                column_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(line);
    for (index, (value, width)) in cells.iter().zip(OFFER_COLUMNS).enumerate() {
        // The first column fills the width the user gives the window.
        let fill = index == 0;
        let cell = commands
            .spawn(source::text(value, source::font(skin.font.clone(), false)))
            .insert((
                TextLayout::no_wrap(),
                crate::bevy_shell::ui::window_layout::Scaled {
                    width: Some(width),
                    ..default()
                },
                Node {
                    width: px(width),
                    flex_grow: if fill { 1. } else { 0. },
                    flex_shrink: if fill { 1. } else { 0. },
                    min_width: px(0.),
                    overflow: Overflow::clip(),
                    ..default()
                },
            ))
            .id();
        commands.entity(line).add_child(cell);
    }
    line
}
/// A category entry shows its Tibia Global icon; the name stays until the icon file loads.
fn category_button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    index: usize,
    category: &str,
) {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), 140.),
            Action {
                key,
                kind: Kind::Category(index),
            },
            Name::new("market.category"),
        ))
        .insert(Node {
            width: px(140.),
            min_height: px(22.),
            flex_shrink: 0.,
            padding: px(3.).all(),
            column_gap: px(4.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .observe(activate)
        .id();
    let icon = commands
        .spawn((
            crate::bevy_shell::ui::ArtSlot::MarketCategory(category.to_owned()),
            crate::bevy_shell::ui::window_layout::Scaled::square(32.),
            Node {
                width: px(32.),
                height: px(32.),
                flex_shrink: 0.,
                display: Display::None,
                ..default()
            },
            bevy::picking::Pickable::IGNORE,
        ))
        .id();
    let text = commands
        .spawn((
            source::button_caption(category, skin.font.clone()),
            crate::bevy_shell::ui::ArtFallback,
        ))
        .id();
    commands.entity(entity).add_children(&[icon, text]);
    commands.entity(parent).add_child(entity);
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
pub(super) fn row(commands: &mut Commands, parent: Entity) -> Entity {
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
pub(super) fn scroll(commands: &mut Commands, parent: Entity, height: Val, role: u32) -> Entity {
    let entity = commands
        .spawn((
            ScrollArea,
            crate::bevy_shell::game_ui::install::RememberScroll("market", role),
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
pub(super) fn label(commands: &mut Commands, skin: &Skins, parent: Entity, value: &str) {
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
pub(super) fn button(
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
            Name::new(match kind {
                Kind::Browse => "market.tab.browse",
                Kind::Intent(Intent::OwnOffers) => "market.tab.own",
                Kind::Intent(Intent::History) => "market.tab.history",
                _ => "market.action",
            }),
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

#[derive(Component, Clone, Copy)]
pub(super) enum FilterKind {
    Vocation,
    Minimum,
    Maximum,
    Forge,
}
#[derive(Component)]
pub(super) struct FilterEditor {
    pub key: Key,
    pub kind: FilterKind,
}
#[derive(Component)]
pub(super) struct FilterApply(pub Key);
#[derive(Component)]
pub(super) struct FilterDepot(pub Key);
mod filters;
use filters::catalogue_filters;
