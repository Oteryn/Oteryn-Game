use super::*;
use crate::bevy_shell::window_drag::DraggableDialog;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    picking::Pickable,
    text::TextCursorStyle,
    ui_widgets::{Button, ScrollArea, TextInput},
};
#[derive(Clone, Copy)]
pub(super) struct Widgets {
    pub rows: Entity,
    pub status: Entity,
    pub price: Entity,
    pub balance: Entity,
    pub trade_caption: Entity,
}
#[derive(Component)]
pub(super) struct OptionCaption(pub Kind);

pub(super) fn build(commands: &mut Commands, skin: &Skins, generation: u64) -> Widgets {
    // NpcDialog.qml: width600/minHeight350; NpcTrade.qml: addonWidth180 and35px offer rows.
    // Only the trade slice is mounted here; the wider left area holds real Look/refusal text.
    let panel = commands
        .spawn((
            Panel,
            DraggableDialog,
            Name::new("npc-shop.dialog"),
            TabGroup {
                order: 3,
                modal: false,
            },
            GlobalZIndex(60),
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                right: px(20.),
                top: px(60.),
                width: px(600.),
                height: px(500.),
                max_width: percent(96.),
                max_height: percent(90.),
                padding: UiRect {
                    top: px(25.),
                    bottom: px(10.),
                    left: px(10.),
                    right: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    source::dialog_caption(commands, panel, "NPC Trade", skin.font.clone());
    let content = commands
        .spawn(Node {
            width: percent(100.),
            min_height: px(0.),
            flex_grow: 1.,
            column_gap: px(10.),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(content);
    let details = commands
        .spawn(Node {
            flex_grow: 1.,
            min_width: px(0.),
            flex_direction: FlexDirection::Column,
            row_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(content).add_child(details);
    label(commands, skin, details, "NPC response");
    let details_scroll = commands
        .spawn((
            ScrollArea,
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_grow: 1.,
                padding: px(5.).all(),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(details).add_child(details_scroll);
    let status = label(commands, skin, details_scroll, "");
    commands.entity(status).insert((
        TextLayout::default(),
        Node {
            width: percent(100.),
            ..default()
        },
    ));
    for (kind, caption) in [
        (Kind::IgnoreCapacity, "Ignore capacity"),
        (Kind::IgnoreEquipped, "Ignore equipped"),
        (Kind::OnlyOwned, "Only owned"),
    ] {
        let button = action(commands, skin, details, generation, kind, caption, 180.);
        commands.entity(button).with_children(|parent| {
            parent.spawn((
                OptionCaption(kind),
                source::text("", source::font(skin.font.clone(), false)),
            ));
        });
    }
    action(
        commands,
        skin,
        details,
        generation,
        Kind::OpenBackpack,
        "Open backpack",
        130.,
    );
    let trade = commands
        .spawn(Node {
            width: px(180.),
            min_width: px(150.),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            row_gap: px(5.),
            min_height: px(0.),
            ..default()
        })
        .id();
    commands.entity(content).add_child(trade);
    let tabs = row(commands, trade);
    action(
        commands,
        skin,
        tabs,
        generation,
        Kind::Side(NpcTradeSide::Buy),
        "Buy",
        83.,
    );
    action(
        commands,
        skin,
        tabs,
        generation,
        Kind::Side(NpcTradeSide::Sell),
        "Sell",
        83.,
    );
    let rows = commands
        .spawn((
            ScrollArea,
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                min_height: px(60.),
                flex_grow: 1.,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(trade).add_child(rows);
    label(commands, skin, trade, "Search");
    field(commands, skin, trade, Field::Search, "", 80);
    label(commands, skin, trade, "Amount");
    field(commands, skin, trade, Field::Amount, "1", 3);
    label(commands, skin, trade, "Price");
    let price = label(commands, skin, trade, "");
    label(commands, skin, trade, "Gold");
    let balance = label(commands, skin, trade, "Unknown");
    let actions = row(commands, trade);
    let trade_button = action(commands, skin, actions, generation, Kind::Trade, "", 82.);
    let trade_caption = commands
        .spawn((source::button_caption("Buy", skin.font.clone()),))
        .id();
    commands.entity(trade_button).add_child(trade_caption);
    action(commands, skin, actions, generation, Kind::Look, "Look", 82.);
    action(
        commands,
        skin,
        trade,
        generation,
        Kind::SellAll,
        "Sell all",
        100.,
    );
    let footer = row(commands, panel);
    action(
        commands,
        skin,
        footer,
        generation,
        Kind::Close,
        "Close",
        60.,
    );
    Widgets {
        rows,
        status,
        price,
        balance,
        trade_caption,
    }
}
pub(super) fn rows(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    generation: u64,
    rows: &[Quote],
    selection: Option<Quote>,
) {
    for quote in rows {
        let button = commands
            .spawn((
                Button,
                Action {
                    generation,
                    kind: Kind::Select(*quote),
                },
                TabIndex(0),
                Name::new("npc-shop.offer"),
                Node {
                    width: percent(100.),
                    height: px(35.),
                    min_height: px(35.),
                    flex_shrink: 0.,
                    align_items: AlignItems::Center,
                    column_gap: px(3.),
                    ..default()
                },
                BackgroundColor(if selection == Some(*quote) {
                    Color::srgb_u8(88, 88, 88)
                } else {
                    Color::NONE
                }),
            ))
            .observe(activate)
            .id();
        commands.entity(parent).add_child(button);
        let icon = commands
            .spawn((
                source::frame(skin.inset.clone(), [1.; 4]),
                Name::new("Item appearance unavailable"),
                Node {
                    width: px(32.),
                    height: px(32.),
                    flex_shrink: 0.,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        label(commands, skin, icon, "?");
        commands.entity(button).add_child(icon);
        let caption = commands
            .spawn((
                Node {
                    min_width: px(0.),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        label(commands, skin, caption, "Item details unavailable");
        label(
            commands,
            skin,
            caption,
            &format!("{} gold / {} items", quote.price, quote.count),
        );
        commands.entity(button).add_child(caption);
    }
    if rows.is_empty() {
        label(commands, skin, parent, "No matching offers");
    }
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let row = commands
        .spawn(Node {
            column_gap: px(5.),
            flex_shrink: 0.,
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(row);
    row
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, text: &str) -> Entity {
    let label = commands
        .spawn((
            source::text(text, source::font(skin.font.clone(), false)),
            Node {
                max_width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(label);
    label
}
fn action(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    generation: u64,
    kind: Kind,
    label: &str,
    width: f32,
) -> Entity {
    let button = commands
        .spawn((
            source::button(skin.buttons.clone(), width),
            Action { generation, kind },
        ))
        .insert(Node {
            width: px(width),
            max_width: percent(100.),
            height: px(20.),
            flex_shrink: 0.,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .observe(activate)
        .id();
    let caption = commands
        .spawn((source::button_caption(label, skin.font.clone()),))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
    button
}
fn field(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    field: Field,
    value: &str,
    limit: usize,
) {
    let frame = commands
        .spawn((
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                height: px(24.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    let mut text = EditableText::new(value);
    text.max_characters = Some(limit);
    let editor = commands
        .spawn((
            field,
            TextInput,
            text,
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            Node {
                width: percent(100.),
                height: px(24.),
                padding: px(3.).all(),
                ..default()
            },
        ))
        .id();
    commands.entity(frame).add_child(editor);
    commands.entity(parent).add_child(frame);
}
