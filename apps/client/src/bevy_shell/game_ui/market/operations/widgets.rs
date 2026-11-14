use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui_widgets::{ScrollArea, TextInput},
};
pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    state: &Operations,
    panel: &MarketPanel,
) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(159),
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
    let frame = commands
        .spawn((
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            TabGroup {
                order: 5,
                modal: true,
            },
            Node {
                width: px(760.),
                max_width: percent(95.),
                max_height: percent(92.),
                padding: px(24.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(frame);
    source::dialog_caption(commands, frame, "Market operations", skin.font.clone());
    let body = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(frame).add_child(body);
    if let Some(review) = &state.review {
        let caption = match review {
            Review::Create {
                fields,
                sell,
                anonymous,
            } => format!(
                "Confirm {} {} × item {} at {} gold each (anonymous: {})?",
                if *sell { "sell" } else { "buy" },
                fields[0],
                panel.definition,
                fields[1],
                anonymous
            ),
            Review::Cancel(row) => format!(
                "Cancel own offer: {} × item {} at {} gold each?",
                row.amount, row.definition, row.price
            ),
        };
        text(
            commands,
            skin,
            body,
            &caption,
            "market.operations.confirmation",
        );
        button(
            commands,
            skin,
            body,
            state.revision,
            ControlKind::Confirm,
            "Confirm",
            "market.operations.confirm",
        );
        button(
            commands,
            skin,
            body,
            state.revision,
            ControlKind::Dismiss,
            "Back",
            "market.operations.dismiss",
        );
    } else {
        let name = panel
            .details
            .items
            .iter()
            .find(|ware| ware.definition == panel.definition)
            .map_or_else(
                || format!("Item {}", panel.definition),
                |ware| ware.name.clone(),
            );
        text(
            commands,
            skin,
            body,
            &format!(
                "Create offer for {name}. Select the item in the catalogue before opening this form."
            ),
            "market.operations.item",
        );
        for (index, caption) in ["amount", "price"].into_iter().enumerate() {
            text(
                commands,
                skin,
                body,
                if index == 0 {
                    "Amount (1–64000)"
                } else {
                    "Gold per item"
                },
                "market.operations.field-label",
            );
            let entity = commands
                .spawn((
                    Field(index),
                    Name::new(format!("market.operations.{caption}")),
                    TextInput,
                    EditableText::new(&state.fields[index]),
                    source::font(skin.font.clone(), false),
                    TextColor(source::TEXT_COLOR),
                    TextLayout::no_wrap(),
                    TextCursorStyle::default(),
                    TabIndex(index as i32),
                    source::frame(skin.inset.clone(), [1.; 4]),
                    Node {
                        width: px(160.),
                        height: px(26.),
                        padding: px(4.).all(),
                        ..default()
                    },
                ))
                .id();
            commands.entity(body).add_child(entity);
        }
        for (kind, caption, name) in [
            (
                ControlKind::Anonymous,
                if state.anonymous {
                    "Anonymous: yes"
                } else {
                    "Anonymous: no"
                },
                "anonymous",
            ),
            (ControlKind::Sell, "Review sell", "sell"),
            (ControlKind::Buy, "Review buy", "buy"),
            (ControlKind::Own, "My offers", "own"),
            (ControlKind::History, "My transactions", "history"),
            (ControlKind::Previous, "Previous page", "previous"),
            (ControlKind::Next, "Next page", "next"),
            (ControlKind::Coins, "Coin market", "coins"),
        ] {
            button(
                commands,
                skin,
                body,
                state.revision,
                kind,
                caption,
                &format!("market.operations.{name}"),
            );
        }
        if matches!(panel.view, MarketView::Own | MarketView::History) {
            text(
                commands,
                skin,
                body,
                &format!(
                    "{} — page {}",
                    if panel.view == MarketView::Own {
                        "My offers"
                    } else {
                        "My transactions"
                    },
                    panel.page + 1
                ),
                "market.operations.view",
            );
            for (index, row) in panel.rows.iter().enumerate() {
                text(
                    commands,
                    skin,
                    body,
                    &oteryn_client_ui::market::row_text(row),
                    &format!(
                        "market.operations.{}.{}",
                        if panel.view == MarketView::History {
                            "history"
                        } else {
                            "own"
                        },
                        index
                    ),
                );
                if panel.view == MarketView::Own && row.state == 1 && row.amount > 0 {
                    button(
                        commands,
                        skin,
                        body,
                        state.revision,
                        ControlKind::Cancel(index),
                        "Review cancellation",
                        &format!("market.operations.cancel.{index}"),
                    );
                }
            }
        }
    }
    if panel.pending.is_some() {
        text(
            commands,
            skin,
            body,
            "Waiting for the server…",
            "market.operations.pending",
        );
    }
    if let Some(feedback) = panel.feedback {
        text(commands, skin, body, feedback, "market.operations.feedback");
    }
    button(
        commands,
        skin,
        body,
        state.revision,
        ControlKind::Close,
        "Close",
        "market.operations.close",
    );
    root
}
fn text(commands: &mut Commands, skin: &Skins, parent: Entity, value: &str, name: &str) {
    let entity = commands
        .spawn((
            source::text(value, source::font(skin.font.clone(), false)),
            Name::new(name.to_owned()),
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    revision: u64,
    kind: ControlKind,
    caption: &str,
    name: &str,
) {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), 180.),
            Control { revision, kind },
            Name::new(name.to_owned()),
        ))
        .observe(activate)
        .id();
    let label = commands
        .spawn(source::button_caption(caption, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(label);
    commands.entity(parent).add_child(entity);
}
