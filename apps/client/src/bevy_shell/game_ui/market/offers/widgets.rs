use super::super::widgets::{button as navigation, label, row};
use super::*;
use bevy::{input_focus::tab_navigation::TabIndex, text::TextCursorStyle, ui_widgets::TextInput};
pub(in super::super) fn select_button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    index: usize,
) {
    button(
        commands,
        skin,
        parent,
        key,
        Operation::Select(index),
        "Select offer",
        &format!("market.offer.select.{index}"),
    );
}
pub(in super::super) fn list(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    panel: &MarketPanel,
) {
    label(
        commands,
        skin,
        parent,
        if panel.view == MarketView::Own {
            "My Offers"
        } else {
            "Offer History"
        },
    );
    let list = super::super::widgets::scroll(
        commands,
        parent,
        Val::Auto,
        if panel.view == MarketView::Own { 6 } else { 7 },
    );
    for index in oteryn_client_ui::market::visible_rows(panel) {
        let offer = &panel.rows[index];
        label(
            commands,
            skin,
            list,
            &format!(
                "{} · state {}",
                oteryn_client_ui::market::row_text(offer),
                offer.state
            ),
        );
        if panel.view == MarketView::Own && offer.state == 1 {
            select_button(commands, skin, list, key, index);
        }
    }
    if panel.rows.is_empty() {
        label(commands, skin, list, "No offers on this page");
    }
    label(commands, skin, parent, &format!("Page {}", panel.page + 1));
    let paging = row(commands, parent);
    for (intent, caption) in [
        (Intent::PreviousPage, "Previous"),
        (Intent::NextPage, "Next"),
        (Intent::Refresh, "Refresh"),
    ] {
        navigation(
            commands,
            skin,
            paging,
            key,
            Kind::Intent(intent),
            caption,
            90.,
        );
    }
}
// The form projects the same server panel, local item and selection as the enclosing window.
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn editor(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    panel: &MarketPanel,
    ware: Option<&Ware>,
    offer: Option<usize>,
    sell: bool,
    drafts: &mut Drafts,
) {
    if panel.view == MarketView::Own {
        label(
            commands,
            skin,
            parent,
            &offer.and_then(|index| panel.rows.get(index)).map_or_else(
                || "Select an own offer to cancel".into(),
                oteryn_client_ui::market::row_text,
            ),
        );
        button(
            commands,
            skin,
            parent,
            key,
            Operation::Cancel,
            "Cancel selected offer",
            "market.offer.cancel",
        );
        return;
    }
    let accept = row(commands, parent);
    let selected = offer.and_then(|index| panel.rows.get(index));
    label(
        commands,
        skin,
        accept,
        &selected.map_or_else(
            || "Select a buy or sell offer to accept".into(),
            |offer| {
                if limit(panel, offer) == 0 {
                    return "No items available to accept this buy offer".into();
                }
                format!(
                    "{} · Accept quantity: 1–{}",
                    oteryn_client_ui::market::row_text(offer),
                    limit(panel, offer)
                )
            },
        ),
    );
    let quantity = field(
        commands,
        skin,
        Field::Quantity,
        "1",
        "market.offer.quantity",
    );
    commands.entity(accept).add_child(quantity);
    button(
        commands,
        skin,
        accept,
        key,
        Operation::Accept,
        "Accept selected offer",
        "market.offer.accept",
    );
    label(
        commands,
        skin,
        parent,
        &format!(
            "Create offer: {}",
            ware.map_or("select an item", |ware| ware.name.as_str())
        ),
    );
    let types = row(commands, parent);
    button(
        commands,
        skin,
        types,
        key,
        Operation::Sell(false),
        "Buy",
        "market.create.buy",
    );
    button(
        commands,
        skin,
        types,
        key,
        Operation::Sell(true),
        "Sell",
        "market.create.sell",
    );
    let form = row(commands, parent);
    for (kind, caption, name, initial) in [
        (
            Field::Amount,
            "Amount",
            "market.create.amount",
            panel.amount.to_string(),
        ),
        (
            Field::Price,
            "Price per piece (gold)",
            "market.create.price",
            panel.price.to_string(),
        ),
    ] {
        label(commands, skin, form, caption);
        let editor = drafts.field(commands, DraftRole::Offer(kind), &initial, |commands| {
            field(commands, skin, kind, &initial, name)
        });
        commands.entity(form).add_child(editor);
    }
    let summary = commands
        .spawn((
            Summary,
            source::text(
                if sell { "Sell offer" } else { "Buy offer" },
                source::font(skin.font.clone(), false),
            ),
        ))
        .id();
    commands.entity(parent).add_child(summary);
    button(
        commands,
        skin,
        parent,
        key,
        Operation::Create,
        "Confirm create offer",
        "market.create.confirm",
    );
}
fn field(commands: &mut Commands, skin: &Skins, kind: Field, value: &str, name: &str) -> Entity {
    let mut text = EditableText::new(value);
    text.max_characters = Some(20);
    commands
        .spawn((
            kind,
            Name::new(name.to_owned()),
            TextInput,
            text,
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            Node {
                width: px(125.),
                height: px(25.),
                padding: px(4.).all(),
                flex_shrink: 0.,
                ..default()
            },
            source::frame(skin.inset.clone(), [1.; 4]),
        ))
        .id()
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    operation: Operation,
    caption: &str,
    name: &str,
) {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), 160.),
            Control { key, operation },
            Name::new(name.to_owned()),
            InteractionDisabled,
        ))
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption(caption, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}
