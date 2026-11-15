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
                order: 5,
                modal: true,
            },
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(940.),
                height: px(620.),
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
    source::dialog_caption(commands, root, "Coin Store", skin.font.clone());
    root
}
pub(super) fn render(
    commands: &mut Commands,
    skin: &Skins,
    frame: Entity,
    key: Key,
    state: &State,
    snapshot: &Snapshot,
) -> Entity {
    let root = column(commands, frame, Val::Auto, 0);
    let panel = &snapshot.store;
    if let Some(quote) = &state.confirmation {
        label(commands, skin, root, "Confirm purchase");
        for offer in &quote.quote.offers {
            label(
                commands,
                skin,
                root,
                &format!(
                    "Buy {} for {} {}?\nOffer: {}\nCatalogue: {}",
                    offer.title,
                    offer.price,
                    offer.currency,
                    offer.offer_key,
                    quote.quote.catalog_revision
                ),
            );
        }
        if !quote.fields[0].is_empty() {
            label(
                commands,
                skin,
                root,
                &format!(
                    "Name: {}; sex: {}; owned hireling: {}",
                    quote.fields[0], quote.fields[1], quote.fields[2]
                ),
            );
        }
        let controls = row(commands, root);
        button(
            commands,
            skin,
            controls,
            key,
            Kind::Confirm,
            "Confirm",
            true,
        );
        button(commands, skin, controls, key, Kind::Cancel, "Cancel", true);
        return root;
    }
    if let Some(quote) = &panel.catalogue {
        label(
            commands,
            skin,
            root,
            &format!("Coin balance: {}", quote.total_coins),
        );
    } else if let Some(page) = &panel.browser.page {
        label(
            commands,
            skin,
            root,
            &format!("Coin balance: {}", page.total),
        );
    } else {
        label(commands, skin, root, "Coin balance not received");
    }
    if let Some(page) = &panel.browser.page
        && page.transferable_known
    {
        label(
            commands,
            skin,
            root,
            &format!("Transferable coins: {}", page.transferable),
        );
    }
    if let Some(tokens) = panel.cosmetic_tokens {
        label(
            commands,
            skin,
            root,
            &format!("Cosmetic Tokens (this character): {tokens}"),
        );
    }
    let ready = panel.selected && !panel.pending && panel.gift.pending.is_none();
    let tabs = row(commands, root);
    for (kind, title) in [
        (Kind::Products, "Products"),
        (Kind::Gift, "Gift coins"),
        (Kind::Transfer, "Transfer coins"),
        (Kind::History(false), "Purchase history"),
        (Kind::Ledger(false), "Coin history"),
    ] {
        button(
            commands,
            skin,
            tabs,
            key,
            kind,
            title,
            ready
                && (!matches!(kind, Kind::History(_))
                    || snapshot.history.selected && !snapshot.history.pending),
        );
    }
    let body = column(commands, root, Val::Auto, 1);
    match state.page {
        Page::Products => products(commands, skin, body, key, panel, &state.fields),
        Page::Gift | Page::Transfer => gift(commands, skin, body, key, state, panel),
        Page::History => {
            label(
                commands,
                skin,
                body,
                &purchase_history::panel_text(&snapshot.history),
            );
            let controls = row(commands, body);
            button(
                commands,
                skin,
                controls,
                key,
                Kind::History(false),
                "Refresh",
                snapshot.history.selected && !snapshot.history.pending,
            );
            button(
                commands,
                skin,
                controls,
                key,
                Kind::History(true),
                "Next page",
                !snapshot.history.pending
                    && snapshot
                        .history
                        .page
                        .as_ref()
                        .is_some_and(|p| !p.next_page_cursor.is_empty()),
            );
        }
        Page::Ledger => {
            label(
                commands,
                skin,
                body,
                &store::coin_history_text(&panel.coin_history),
            );
            let controls = row(commands, body);
            button(
                commands,
                skin,
                controls,
                key,
                Kind::Ledger(false),
                "Refresh",
                ready,
            );
            button(
                commands,
                skin,
                controls,
                key,
                Kind::Ledger(true),
                "Next page",
                ready
                    && panel
                        .coin_history
                        .page
                        .as_ref()
                        .is_some_and(|p| !p.next_page_cursor.is_empty()),
            );
        }
    }
    if !panel.selected {
        label(commands, skin, root, "Store unavailable in this session");
    }
    if panel.pending || panel.gift.pending.is_some() {
        label(commands, skin, root, "Waiting for the server…");
    }
    if let Some(result) = &panel.purchase {
        label(
            commands,
            skin,
            root,
            &format!("Purchase: {:?}", result.code),
        );
    }
    if let Some(feedback) = &panel.feedback {
        label(commands, skin, root, feedback);
    }
    button(commands, skin, root, key, Kind::Close, "Close", true);
    root
}
fn products(
    commands: &mut Commands,
    skin: &Skins,
    root: Entity,
    key: Key,
    panel: &StorePanel,
    fields: &[String; 3],
) {
    let content = row(commands, root);
    commands.entity(content).insert(Node {
        flex_grow: 1.,
        min_height: px(0.),
        column_gap: px(8.),
        ..default()
    });
    let categories = column(commands, content, px(170.), 2);
    label(commands, skin, categories, "Categories");
    let list = column(commands, content, px(270.), 3);
    label(commands, skin, list, "Products");
    let details = column(commands, content, Val::Auto, 4);
    label(commands, skin, details, "Description / offer details");
    let ready = panel.selected && !panel.pending && panel.confirmed_purchase.is_none();
    if let Some(page) = &panel.browser.page {
        for (index, category) in page.categories.iter().enumerate() {
            let title = if category.parent.is_empty() {
                category.title.clone()
            } else {
                format!("  {}", category.title)
            };
            button(
                commands,
                skin,
                categories,
                key,
                Kind::Category(index),
                &title,
                ready,
            );
        }
        for (index, product) in page.products.iter().enumerate() {
            let selected = index == panel.browser.selected;
            button(
                commands,
                skin,
                list,
                key,
                Kind::Product(index),
                &format!(
                    "{}{} — {} {}",
                    if selected { "› " } else { "" },
                    product.offer.title,
                    product.offer.price,
                    product.offer.currency
                ),
                ready,
            );
        }
        if page.products.is_empty() {
            label(commands, skin, list, "No products on this page");
        }
        let pages = row(commands, root);
        button(
            commands,
            skin,
            pages,
            key,
            Kind::Page(-1),
            "Previous page",
            ready && panel.browser.cursors.len() > 1,
        );
        button(
            commands,
            skin,
            pages,
            key,
            Kind::Page(1),
            "Next page",
            ready && !page.next.is_empty(),
        );
    }
    if let Some(quote) = &panel.catalogue
        && let Some(offer) = quote.offers.first()
    {
        label(
            commands,
            skin,
            details,
            &format!(
                "{}\nOffer: {}\nPrice: {} {}\nCatalogue: {}",
                offer.title, offer.offer_key, offer.price, offer.currency, quote.catalog_revision
            ),
        );
        if matches!(
            offer.offer_key.as_str(),
            "store:hireling:25440" | "store:hireling:25438" | "store:hireling:25437"
        ) {
            field(commands, skin, details, 0, "Hireling name", &fields[0]);
            field(commands, skin, details, 1, "Sex (0 or 1)", &fields[1]);
            field(
                commands,
                skin,
                details,
                2,
                "Owned hireling UUID (optional)",
                &fields[2],
            );
        }
        if panel.token_supported {
            button(
                commands,
                skin,
                details,
                key,
                Kind::TokenPayment,
                "Switch coins / tokens",
                ready,
            );
        }
        button(
            commands,
            skin,
            details,
            key,
            Kind::Buy,
            if panel.confirmed_purchase.is_some() {
                "Retry purchase…"
            } else {
                "Buy…"
            },
            buy_enabled(panel),
        );
        if !buy_enabled(panel) && !panel.pending {
            label(
                commands,
                skin,
                details,
                "Purchase unavailable: check balance and payment currency",
            );
        }
    }
}
fn gift(
    commands: &mut Commands,
    skin: &Skins,
    root: Entity,
    key: Key,
    state: &State,
    panel: &StorePanel,
) {
    label(
        commands,
        skin,
        root,
        if state.page == Page::Gift {
            "Gift coins"
        } else {
            "Transfer coins"
        },
    );
    label(
        commands,
        skin,
        root,
        "Review the exact recipient and amount before confirming. Gifts and transfers cannot be recalled. Amount must be a positive multiple of 25.",
    );
    field(commands, skin, root, 0, "Recipient name", &state.fields[0]);
    field(commands, skin, root, 1, "Coins", &state.fields[1]);
    if let Some(review) = &panel.gift_reviewed {
        label(
            commands,
            skin,
            root,
            &format!(
                "Reviewed recipient: {}; coins: {}",
                review.recipient, review.amount
            ),
        );
    }
    if !panel.gift.receipt.is_empty() {
        label(commands, skin, root, &panel.gift.receipt);
    }
    let ready = panel.selected && panel.gift.pending.is_none();
    let controls = row(commands, root);
    for (control, title, enabled) in [
        (
            0,
            "Review recipient and amount",
            ready && panel.gift.confirmed.is_none(),
        ),
        (
            1,
            "Confirm reviewed transfer",
            ready && panel.gift.quote.is_some() && panel.gift.confirmed.is_none(),
        ),
        (
            3,
            "Retry confirmation",
            ready && panel.gift.confirmed.is_some(),
        ),
        (
            4,
            "Refresh receipt",
            ready && (panel.gift.confirmed.is_some() || panel.gift.last_operation.is_some()),
        ),
    ] {
        button(
            commands,
            skin,
            controls,
            key,
            Kind::GiftControl(control),
            title,
            enabled,
        );
    }
}
fn column(commands: &mut Commands, parent: Entity, width: Val, role: u32) -> Entity {
    let entity = commands
        .spawn((
            ScrollArea,
            crate::bevy_shell::game_ui::install::RememberScroll("store", role),
            Node {
                width,
                min_width: px(0.),
                min_height: px(0.),
                flex_grow: if width == Val::Auto { 1. } else { 0. },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                overflow: Overflow::scroll_y(),
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
            column_gap: px(6.),
            row_gap: px(6.),
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.,
            ..default()
        })
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
fn field(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    index: usize,
    title: &str,
    value: &str,
) {
    label(commands, skin, parent, title);
    let mut text = EditableText::new(value);
    text.max_characters = Some(64);
    let entity = commands
        .spawn((
            Field(index),
            TextInput,
            text,
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(index as i32),
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                height: px(25.),
                flex_shrink: 0.,
                padding: px(4.).all(),
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
    title: &str,
    enabled: bool,
) {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), 160.),
            Action { key, kind },
        ))
        .insert(Node {
            min_width: px(80.),
            max_width: percent(100.),
            min_height: px(24.),
            height: Val::Auto,
            flex_shrink: 0.,
            padding: px(3.).all(),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .observe(activate)
        .id();
    if !enabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(title, skin.font.clone()))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
