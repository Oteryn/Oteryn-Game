use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
pub(super) fn build(commands: &mut Commands, skins: &Skins, state: &State) -> Entity {
    let root = commands
        .spawn((
            Overlay,
            GameModal,
            GlobalZIndex(164),
            TabGroup {
                order: 3,
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
                width: px(740.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    top: px(24.),
                    left: px(12.),
                    right: px(12.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(commands, panel, "Game Store", skins.font.clone());
    let tabs = row(commands, panel);
    for (action, caption) in [
        (Action::Refresh, "Catalogue / Refresh"),
        (Action::History, "Purchase history"),
        (Action::Ledger, "Coin ledger"),
        (Action::Gift, "Gift coins"),
        (Action::Close, "Close"),
    ] {
        button(commands, skins, tabs, state, action, caption, false);
    }
    let Some(snapshot) = &state.seen else {
        return root;
    };
    let store = &snapshot.panel;
    let pending = store.pending || snapshot.history.pending || store.gift.pending.is_some();
    if pending {
        text(commands, skins, panel, "Waiting for the server…");
    }
    if let Some(result) = &store.purchase {
        text(
            commands,
            skins,
            panel,
            &format!("Purchase: {:?}", result.code),
        );
    }
    if let Some(feedback) = &store.feedback {
        text(commands, skins, panel, feedback);
    }
    let body = commands
        .spawn((
            ScrollArea,
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                max_height: px(420.),
                min_height: px(80.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                padding: px(5.).all(),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(body);
    if store.gift_open {
        text(
            commands,
            skins,
            body,
            "Gift transferable COINS. Review the exact recipient and amount with the server before confirming. Gifts cannot be recalled.",
        );
        field(
            commands,
            skins,
            body,
            state,
            Field::Recipient,
            "Recipient name",
            pending,
        );
        field(
            commands,
            skins,
            body,
            state,
            Field::Amount,
            "Transferable coins (multiple of 25)",
            pending,
        );
        if let Some(quote) = &store.gift.quote {
            text(
                commands,
                skins,
                body,
                &format!(
                    "Confirm gift: {} transferable COINS to {}",
                    quote.amount, quote.recipient
                ),
            );
        }
        text(commands, skins, body, &store.gift.receipt);
        for (action, caption) in [
            (Action::ReviewGift, "Review gift"),
            (Action::ConfirmGift, "Confirm reviewed gift"),
            (Action::RetryGift, "Retry same gift"),
            (Action::Receipt, "Refresh receipt"),
        ] {
            button(commands, skins, body, state, action, caption, pending);
        }
    } else {
        match store.page {
            StorePage::Catalogue | StorePage::Inbox => {
                field(
                    commands,
                    skins,
                    body,
                    state,
                    Field::Search,
                    "Server catalogue search",
                    pending,
                );
                if let Some(page) = &store.browser.page {
                    text(
                        commands,
                        skins,
                        body,
                        &format!(
                            "Balance: {} COINS; transferable: {}",
                            page.total,
                            if page.transferable_known {
                                page.transferable.to_string()
                            } else {
                                "unknown".into()
                            }
                        ),
                    );
                    for (i, category) in page.categories.iter().enumerate() {
                        button(
                            commands,
                            skins,
                            body,
                            state,
                            Action::Category(i),
                            &category.title,
                            pending,
                        );
                    }
                    for (i, product) in page.products.iter().enumerate() {
                        let e = button(
                            commands,
                            skins,
                            body,
                            state,
                            Action::Select(i),
                            &format!(
                                "{} — {} {}",
                                product.offer.title, product.offer.price, product.offer.currency
                            ),
                            pending,
                        );
                        commands.entity(e).insert(ProductRow);
                    }
                }
                if let Some((quote, custom)) = &state.consent {
                    if let Some(offer) = quote
                        .panel
                        .catalogue
                        .as_ref()
                        .and_then(|p| p.offers.get(quote.panel.selected_offer))
                    {
                        text(
                            commands,
                            skins,
                            body,
                            &format!(
                                "Confirm purchase: {} for {} {}\nRevision: {}\nName: {}; sex: {}; target: {}",
                                offer.title,
                                offer.price,
                                offer.currency,
                                quote
                                    .panel
                                    .catalogue
                                    .as_ref()
                                    .map_or("", |p| p.catalog_revision.as_str()),
                                custom.name,
                                custom.sex,
                                custom.identity.map_or_else(
                                    || "none".into(),
                                    |id| id.iter().map(|b| format!("{b:02x}")).collect::<String>()
                                )
                            ),
                        );
                    }
                    button(
                        commands,
                        skins,
                        body,
                        state,
                        Action::CancelPurchase,
                        "Cancel review",
                        pending,
                    );
                } else {
                    field(
                        commands,
                        skins,
                        body,
                        state,
                        Field::Name,
                        "Hireling name / new name",
                        pending,
                    );
                    field(
                        commands,
                        skins,
                        body,
                        state,
                        Field::Sex,
                        "Hireling sex (1 male, 2 female)",
                        pending,
                    );
                    field(
                        commands,
                        skins,
                        body,
                        state,
                        Field::Identity,
                        "Hireling target UUID (required for rename / sex change)",
                        pending,
                    );
                }
                button(
                    commands,
                    skins,
                    body,
                    state,
                    Action::ReviewPurchase,
                    "Review selected purchase",
                    pending,
                );
                button(
                    commands,
                    skins,
                    body,
                    state,
                    Action::ConfirmPurchase,
                    "Confirm reviewed purchase",
                    pending || state.consent.is_none(),
                );
            }
            StorePage::History | StorePage::CoinHistory => {
                text(
                    commands,
                    skins,
                    body,
                    "Filters cover this server page. Times are inclusive Unix milliseconds.",
                );
                field(
                    commands,
                    skins,
                    body,
                    state,
                    Field::Filter,
                    if store.page == StorePage::History {
                        "Offer filter"
                    } else {
                        "Transaction kind"
                    },
                    pending,
                );
                field(commands, skins, body, state, Field::From, "From", pending);
                field(
                    commands,
                    skins,
                    body,
                    state,
                    Field::Until,
                    "Through",
                    pending,
                );
                button(
                    commands,
                    skins,
                    body,
                    state,
                    Action::Filter,
                    "Apply page filters",
                    pending,
                );
                text(
                    commands,
                    skins,
                    body,
                    &if store.page == StorePage::History {
                        oteryn_client_ui::purchase_history::panel_text(&snapshot.history)
                    } else {
                        oteryn_client_ui::store::coin_history_text(&store.coin_history)
                    },
                );
            }
        }
    }
    let paging = row(commands, panel);
    button(
        commands,
        skins,
        paging,
        state,
        Action::Previous,
        "Previous catalogue page",
        pending || store.page != StorePage::Catalogue || store.browser.cursors.len() < 2,
    );
    let has_next = match store.page {
        StorePage::Catalogue | StorePage::Inbox => store
            .browser
            .page
            .as_ref()
            .is_some_and(|p| !p.next.is_empty()),
        StorePage::History => snapshot
            .history
            .page
            .as_ref()
            .is_some_and(|p| !p.next_page_cursor.is_empty()),
        StorePage::CoinHistory => store
            .coin_history
            .page
            .as_ref()
            .is_some_and(|p| !p.next_page_cursor.is_empty()),
    };
    button(
        commands,
        skins,
        paging,
        state,
        Action::Next,
        "Next server page",
        pending || !has_next,
    );
    root
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let e = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(6.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(e);
    e
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) {
    let e = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(e);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    state: &State,
    action: Action,
    caption: &str,
    disabled: bool,
) -> Entity {
    let e = commands
        .spawn((
            source::button(skins.buttons.clone(), 180.),
            Control {
                generation: state.generation,
                revision: state.revision,
                action,
            },
        ))
        .observe(activate)
        .id();
    if disabled {
        commands.entity(e).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(e).add_child(caption);
    commands.entity(parent).add_child(e);
    e
}
fn field(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    state: &State,
    kind: Field,
    label: &str,
    disabled: bool,
) {
    text(commands, skins, parent, label);
    let mut editable = EditableText::new(state.value(kind));
    editable.max_characters = Some(256);
    let e = commands
        .spawn((
            kind,
            TextInput,
            editable,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                height: px(25.),
                padding: px(4.).all(),
                ..default()
            },
        ))
        .id();
    if disabled {
        commands.entity(e).insert(InteractionDisabled);
    }
    commands.entity(parent).add_child(e);
}
