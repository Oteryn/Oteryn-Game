//! Account commerce sends explicit choices to the authenticated controller.
use super::{
    bridge::{Reply, Request},
    *,
};
use crate::{account_services::Action as Account, coin_shop::Action as Coin};
use bevy::{input_focus::tab_navigation::TabIndex, text::EditableText, ui_widgets::TextInput};
use oteryn_platform_client::native_login::bazaar::{AuctionState, AuctionView};

#[derive(Component)]
struct Form {
    character: Option<String>,
    status: Entity,
    confirmation: Entity,
    confirming: Option<(Entity, Entity)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    Bid,
    MinimumBid,
    Duration,
}

#[derive(Component)]
struct Field(Entity, FieldKind);

#[derive(Clone)]
enum Action {
    Send(Reply),
    ChooseCharacter(String, String),
    Quote(String),
    Bid(String, String, u64),
    Sell,
    Prepare(String, Reply),
    Confirm(Reply),
    Cancel,
}

#[derive(Component)]
struct Button {
    request: u64,
    form: Entity,
    action: Action,
}

#[derive(Resource, Default)]
struct Pending(std::collections::VecDeque<Entity>);

pub(super) fn install(app: &mut App) {
    app.init_resource::<Pending>()
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}

pub(super) fn build(commands: &mut Commands, skin: &Skins, parent: Entity, request: &Request) {
    if matches!(&request.screen, Screen::Premium(_)) {
        return;
    }
    let form = commands.spawn(column()).id();
    let confirmation = commands.spawn(column()).id();
    let status = commands
        .spawn(source::text("", source::font(skin.font.clone(), false)))
        .id();
    commands
        .entity(form)
        .insert(Form {
            character: None,
            status,
            confirmation,
            confirming: None,
        })
        .add_children(&[confirmation, status]);
    commands.entity(parent).add_child(form);
    let mut ui = Builder {
        commands,
        skin,
        request: request.id,
        form,
        parent: form,
    };
    match &request.screen {
        Screen::Account(view) => account(&mut ui, view),
        Screen::Coin(view) => coins(&mut ui, view),
        Screen::Premium(_) => {}
    }
}

fn account(ui: &mut Builder<'_, '_, '_>, view: &crate::account_services::View) {
    for (name, label, filter) in [
        ("account.bazaar.all", "Browse auctions", AuctionView::All),
        ("account.bazaar.own", "My auctions", AuctionView::Own),
        (
            "account.bazaar.history",
            "Auction history",
            AuctionView::History,
        ),
    ] {
        ui.account(name, label, Account::Refresh(filter));
    }
    ui.account("account.coins", "Get Oteryn coins", Account::CoinShop);
    if view.can_retry() {
        ui.account("account.retry", "Retry same request", Account::Retry);
    }
    if view.pending() {
        ui.account("account.status", "Check pending operation", Account::Status);
    }
    if view.can_transfer()
        && let Some(quote) = &view.quote
    {
        ui.button(
            "account.transfer.confirm",
            "Review world transfer",
            Action::Prepare(
                format!(
                    "{}\nGame rechecks offline, guild, house and market restrictions before committing.",
                    view.transfer_description(quote)
                ),
                Reply::Account(Account::Transfer { confirmed: true }),
            ),
        );
    }
    let Some(page) = &view.page else { return };
    if page.next_cursor.is_some() {
        ui.account("account.bazaar.next", "Next page", Account::Next);
    }
    if !view.pending()
        && page
            .auctions
            .iter()
            .any(|auction| auction.state == AuctionState::Open)
    {
        ui.field("Bid amount (coins, blank for minimum)", FieldKind::Bid, "");
    }
    for auction in &page.auctions {
        ui.text(&format!(
            "{} · level {} · {} · {:?} · minimum {} coins · highest bid {} · ends {} UTC seconds",
            auction.character_name,
            auction.level,
            auction.world_name,
            auction.state,
            auction.minimum_next_bid,
            auction.highest_bid,
            auction.ends_at
        ));
        if !view.pending() && auction.state == AuctionState::Open {
            ui.button(
                &format!("account.auction.{}", auction.id),
                "Review bid",
                Action::Bid(
                    auction.id.clone(),
                    auction.character_name.clone(),
                    auction.minimum_next_bid,
                ),
            );
        }
    }
    if !view.pending() && !page.owned_characters.is_empty() {
        ui.text("Select your character for listing or transfer");
        for character in &page.owned_characters {
            ui.button(
                &format!("account.character.{}", character.id),
                &character.name,
                Action::ChooseCharacter(character.id.clone(), character.name.clone()),
            );
        }
        ui.field("Minimum listing bid (coins)", FieldKind::MinimumBid, "1");
        ui.field("Listing duration (hours)", FieldKind::Duration, "24");
        ui.button("account.sell", "Review auction listing", Action::Sell);
        for world in &page.worlds {
            ui.button(
                &format!("account.transfer.{}", world.id),
                &format!("Check transfer to {}", world.name),
                Action::Quote(world.id.clone()),
            );
        }
    }
    for receipt in &page.recent_operations {
        ui.account(
            &format!("account.receipt.{}", receipt.request_id),
            &format!(
                "Check operation {} · {:?} · {}",
                receipt.request_id, receipt.state, receipt.code
            ),
            Account::Receipt(receipt.request_id.clone()),
        );
    }
}

fn coins(ui: &mut Builder<'_, '_, '_>, view: &crate::coin_shop::View) {
    ui.coin("coin.refresh", "Refresh prices and balance", Coin::Refresh);
    if view.pending() {
        ui.coin("coin.retry", "Retry same request", Coin::Retry);
    }
    if view.pending() || view.order.is_some() {
        ui.coin("coin.status", "Check order status", Coin::Status);
    }
    if view.can_checkout() {
        ui.coin("coin.checkout", "Open sandbox checkout", Coin::OpenCheckout);
    }
    if let Some(catalogue) = &view.catalogue {
        for package in &catalogue.packages {
            let price = crate::coin_shop::price_text(&package.price);
            let label = format!("{} · {} coins · {price}", package.title, package.coins);
            if view.can_purchase(&package.id) {
                ui.button(&format!("coin.package.{}", package.id), &label, Action::Prepare(
                    format!("Buy {} coins for {price} in sandbox checkout? No real money is charged.", package.coins),
                    Reply::Coin(Coin::Purchase { package_id: package.id.clone(), confirmed: true }),
                ));
            } else {
                ui.text(&label);
            }
        }
        if !catalogue.recent_orders.is_empty() {
            ui.text("Recent orders");
        }
        for order in &catalogue.recent_orders {
            let price = crate::coin_shop::price_text(&order.price);
            let label = format!("{:?} · {} coins · {price}", order.state, order.coins);
            ui.text(&format!("{label} · {}", order.order_id));
            if !view.pending() {
                ui.coin(
                    &format!("coin.order.{}", order.order_id),
                    "Inspect order",
                    Coin::Inspect(order.request_id.clone()),
                );
            }
        }
    }
}

struct Builder<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    skin: &'a Skins,
    request: u64,
    form: Entity,
    parent: Entity,
}

impl Builder<'_, '_, '_> {
    fn text(&mut self, value: &str) {
        let entity = self
            .commands
            .spawn(source::text(
                value,
                source::font(self.skin.font.clone(), false),
            ))
            .id();
        self.commands.entity(self.parent).add_child(entity);
    }
    fn field(&mut self, label: &str, kind: FieldKind, initial: &str) {
        self.text(label);
        let mut text = EditableText::new(initial);
        text.max_characters = Some(20);
        let entity = self
            .commands
            .spawn((
                Field(self.form, kind),
                TextInput,
                TabIndex(0),
                text,
                source::font(self.skin.font.clone(), false),
                bevy::text::TextCursorStyle::default(),
                Node {
                    min_height: px(24.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            ))
            .id();
        self.commands.entity(self.parent).add_child(entity);
    }
    fn button(&mut self, name: &str, label: &str, action: Action) -> Entity {
        let button = self
            .commands
            .spawn((
                source::button(self.skin.buttons.clone(), 440.),
                Name::new(name.to_owned()),
                Button {
                    request: self.request,
                    form: self.form,
                    action,
                },
            ))
            .observe(queue)
            .id();
        let text = self
            .commands
            .spawn(source::button_caption(label, self.skin.font.clone()))
            .id();
        self.commands.entity(button).add_child(text);
        self.commands.entity(self.parent).add_child(button);
        button
    }
    fn account(&mut self, name: &str, label: &str, action: Account) {
        self.button(name, label, Action::Send(Reply::Account(action)));
    }
    fn coin(&mut self, name: &str, label: &str, action: Coin) {
        self.button(name, label, Action::Send(Reply::Coin(action)));
    }
}

fn column() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        flex_shrink: 0.,
        row_gap: px(4.),
        ..default()
    }
}

fn queue(event: On<Activate>, guard: ModalGuard, mut pending: ResMut<Pending>) {
    if guard.allows(event.entity) {
        pending.0.push_back(event.entity);
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "The active request, editor and confirmation controls are scoped independently"
)]
fn finish(
    mut commands: Commands,
    mut owner: NonSendMut<HostOwner>,
    skin: Res<Skins>,
    guard: ModalGuard,
    mut pending: ResMut<Pending>,
    buttons: Query<&Button>,
    mut forms: Query<(&mut Form, &ChildOf)>,
    fields: Query<(&Field, &EditableText)>,
    mut statuses: Query<&mut Text>,
) {
    while let Some(entity) = pending.0.pop_front() {
        let Ok(button) = buttons.get(entity) else {
            continue;
        };
        if !guard.allows(entity)
            || !owner
                .services
                .pending()
                .is_some_and(|r| r.id == button.request)
        {
            continue;
        }
        let Ok((mut form, scroll)) = forms.get_mut(button.form) else {
            continue;
        };
        let current = match button.action {
            Action::Confirm(_) => form
                .confirming
                .is_some_and(|(confirm, _)| confirm == entity),
            Action::Cancel => form.confirming.is_some_and(|(_, cancel)| cancel == entity),
            _ => form.confirming.is_none(),
        };
        if !current {
            continue;
        }
        if fields.iter().any(|(field, text)| {
            field.0 == button.form
                && (text.pending_paste.is_some()
                    || text
                        .pending_edits
                        .iter()
                        .any(bevy::text::TextEdit::is_destructive))
        }) {
            pending.0.push_front(entity);
            break;
        }
        let value = |kind| {
            fields
                .iter()
                .find(|(field, _)| field.0 == button.form && field.1 == kind)
                .map(|(_, text)| text.value().to_string().trim().to_owned())
        };
        let status_entity = form.status;
        let mut status = |message: String| {
            if let Ok(mut text) = statuses.get_mut(status_entity) {
                text.0 = message;
            }
        };
        let (caption, reply) = match &button.action {
            Action::Send(reply) | Action::Confirm(reply) => {
                form.confirming = None;
                owner.services.reply(button.request, reply.clone());
                continue;
            }
            Action::ChooseCharacter(id, name) => {
                status(format!("Selected character: {name}"));
                form.character = Some(id.clone());
                continue;
            }
            Action::Quote(world_id) => {
                let Some(character_id) = &form.character else {
                    status("Select a character first".into());
                    continue;
                };
                owner.services.reply(
                    button.request,
                    Reply::Account(Account::Quote {
                        character_id: character_id.clone(),
                        world_id: world_id.clone(),
                    }),
                );
                continue;
            }
            Action::Bid(id, name, minimum) => {
                let Some(coins) = value(FieldKind::Bid)
                    .and_then(|text| {
                        if text.is_empty() {
                            Some(*minimum)
                        } else {
                            text.parse::<u64>().ok()
                        }
                    })
                    .filter(|coins| coins >= minimum)
                else {
                    status(format!("Enter a bid of at least {minimum} coins"));
                    continue;
                };
                (
                    format!(
                        "Bid {coins} Oteryn coins for {name}? The service reserves funds; settlement requires the Game ownership receipt."
                    ),
                    Reply::Account(Account::Bid {
                        auction_id: id.clone(),
                        coins,
                        confirmed: true,
                    }),
                )
            }
            Action::Sell => {
                let minimum = value(FieldKind::MinimumBid)
                    .and_then(|text| text.parse::<u64>().ok())
                    .filter(|n| *n > 0);
                let hours = value(FieldKind::Duration)
                    .and_then(|text| text.parse::<u32>().ok())
                    .filter(|n| (1..=168).contains(n));
                let (Some(character_id), Some(minimum_bid), Some(duration_hours)) =
                    (&form.character, minimum, hours)
                else {
                    status(
                        "Select a character and enter a minimum bid and a duration of 1–168 hours"
                            .into(),
                    );
                    continue;
                };
                let name = owner
                    .services
                    .pending()
                    .and_then(|request| {
                        let Screen::Account(view) = &request.screen else {
                            return None;
                        };
                        view.page
                            .as_ref()?
                            .owned_characters
                            .iter()
                            .find(|row| &row.id == character_id)
                    })
                    .map_or(character_id.as_str(), |row| row.name.as_str());
                (
                    format!(
                        "List {name} for {duration_hours} hours with a minimum bid of {minimum_bid} coins? The offline character will be locked. Items and gold remain with it."
                    ),
                    Reply::Account(Account::Sell {
                        character_id: character_id.clone(),
                        minimum_bid,
                        duration_hours,
                        confirmed: true,
                    }),
                )
            }
            Action::Prepare(caption, reply) => (caption.clone(), reply.clone()),
            Action::Cancel => {
                commands.entity(form.confirmation).despawn_children();
                form.confirming = None;
                continue;
            }
        };
        commands
            .entity(scroll.parent())
            .insert(ScrollPosition::default());
        let mut ui = Builder {
            commands: &mut commands,
            skin: &skin,
            request: button.request,
            form: button.form,
            parent: form.confirmation,
        };
        ui.text(&caption);
        let confirm = ui.button("commerce.confirm", "Confirm", Action::Confirm(reply));
        let cancel = ui.button("commerce.cancel", "Cancel", Action::Cancel);
        form.confirming = Some((confirm, cancel));
    }
}
