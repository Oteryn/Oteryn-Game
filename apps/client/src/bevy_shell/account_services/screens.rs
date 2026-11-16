//! Worker-side screens. Each presenter turns its controller view into a Bevy screen and the
//! pressed button back into the controller's own action, as the native account forms did.
use super::bridge::{Ask, Button, Field, List, Reply, Screen};
use crate::{account_services as services, coin_shop, premium};
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::native_login::bazaar::AuctionView;

fn button(id: &'static str, label: &'static str, enabled: bool) -> Button {
    Button { id, label, enabled }
}
fn list(label: &'static str, rows: Vec<String>) -> List {
    List {
        label,
        selected: (!rows.is_empty()).then_some(0),
        rows,
        fill: None,
    }
}
/// Shows `screen` until a button maps to an action; a refused choice is explained in place.
fn run<A>(
    mut screen: Screen,
    ask: &Ask,
    cancel: &CancellationToken,
    back: A,
    mut choose: impl FnMut(&Reply) -> Result<A, String>,
) -> A {
    loop {
        let Some(reply) = ask(screen.clone(), cancel) else {
            return back;
        };
        match choose(&reply) {
            Ok(action) => return action,
            Err(notice) => screen.keep(&reply, notice),
        }
    }
}
/// The Yes/No question the native forms asked before a charged or locking operation.
fn confirm(ask: &Ask, cancel: &CancellationToken, text: String) -> bool {
    let screen = Screen {
        title: tr!("Confirm Oteryn account action"),
        text,
        buttons: vec![
            button("yes", tr!("Yes"), true),
            button("no", tr!("No"), true),
        ],
        escape: "no",
        ..Screen::default()
    };
    ask(screen, cancel).is_some_and(|reply| reply.button == "yes")
}
fn chosen<'a, T>(items: &'a [T], reply: &Reply, list: usize) -> Option<&'a T> {
    reply
        .selected
        .get(list)
        .copied()
        .flatten()
        .and_then(|at| items.get(at))
}
fn value<T: std::str::FromStr>(reply: &Reply, field: usize) -> Option<T> {
    reply.fields.get(field)?.trim().parse().ok()
}

const AUCTIONS: usize = 0;
const CHARACTERS: usize = 1;
const WORLDS: usize = 2;
const RECEIPTS: usize = 3;
const BID: usize = 0;
const MINIMUM: usize = 1;
const DURATION: usize = 2;

pub(super) fn account(
    view: &services::View,
    ask: &Ask,
    cancel: &CancellationToken,
) -> services::Action {
    let page = view.page.as_ref();
    let auctions = page.map_or(&[][..], |p| &p.auctions);
    let characters = page.map_or(&[][..], |p| &p.owned_characters);
    let worlds = page.map_or(&[][..], |p| &p.worlds);
    let receipts = page.map_or(&[][..], |p| &p.recent_operations);
    let mut lists = vec![
        list(
            "Auctions",
            auctions
                .iter()
                .map(|a| {
                    lfmt!(
                        "{} — level {} — {} — next {} coins — high {} — {:?} — ends {} UTC seconds",
                        a.character_name,
                        a.level,
                        a.world_name,
                        a.minimum_next_bid,
                        a.highest_bid,
                        a.state,
                        a.ends_at
                    )
                })
                .collect(),
        ),
        list(
            tr!("Your character"),
            characters.iter().map(|c| c.name.clone()).collect(),
        ),
        list(
            tr!("Destination world"),
            worlds.iter().map(|w| w.name.clone()).collect(),
        ),
        list(
            tr!("Your operation receipts (select to recover status)"),
            receipts
                .iter()
                .map(|r| format!("{:?} — {} — {}", r.state, r.code, r.request_id))
                .collect(),
        ),
    ];
    lists[AUCTIONS].fill = Some((
        BID,
        auctions
            .iter()
            .map(|a| a.minimum_next_bid.to_string())
            .collect(),
    ));
    if let Some(quote) = &view.quote {
        if let Some(at) = characters.iter().position(|c| c.id == quote.character_id) {
            lists[CHARACTERS].selected = Some(at);
        }
        if let Some(at) = worlds
            .iter()
            .position(|w| w.id == quote.destination_world_id)
        {
            lists[WORLDS].selected = Some(at);
        }
    }
    let idle = !view.pending();
    let screen = Screen {
        title: tr!("Oteryn — Character Bazaar and world transfer"),
        text: view.status_text(),
        lists,
        fields: vec![
            Field {
                label: tr!("Bid (Oteryn coins)"),
                value: auctions
                    .first()
                    .map(|a| a.minimum_next_bid.to_string())
                    .unwrap_or_default(),
            },
            Field {
                label: tr!("Minimum bid"),
                value: "1".into(),
            },
            Field {
                label: tr!("Duration (hours)"),
                value: "24".into(),
            },
        ],
        buttons: vec![
            button("all", tr!("Browse auctions"), true),
            button("own", tr!("My auctions"), true),
            button("history", tr!("Auction history"), true),
            button(
                "next",
                tr!("Next page"),
                page.is_some_and(|p| p.next_cursor.is_some()),
            ),
            button("bid", tr!("Place bid…"), idle),
            button("sell", tr!("List for auction…"), idle),
            button("quote", tr!("Check transfer"), idle),
            button("transfer", tr!("Confirm transfer…"), view.can_transfer()),
            button("receipt", tr!("Check receipt"), !receipts.is_empty()),
            button("retry", tr!("Retry same request"), view.can_retry()),
            button("status", tr!("Check pending status"), view.pending()),
            button("coins", tr!("Get coins"), true),
            button("premium", tr!("Premium overview"), true),
            button("back", tr!("Return to characters"), true),
        ],
        escape: "back",
        ..Screen::default()
    };
    run(screen, ask, cancel, services::Action::Back, |reply| {
        Ok(match reply.button {
            "all" => services::Action::Refresh(AuctionView::All),
            "own" => services::Action::Refresh(AuctionView::Own),
            "history" => services::Action::Refresh(AuctionView::History),
            "next" => services::Action::Next,
            "retry" => services::Action::Retry,
            "status" => services::Action::Status,
            "coins" => services::Action::CoinShop,
            "premium" => services::Action::Premium,
            "bid" => {
                let (Some(a), Some(coins)) =
                    (chosen(auctions, reply, AUCTIONS), value::<u64>(reply, BID))
                else {
                    return Err(tr!("Choose an auction and enter a valid coin amount.").into());
                };
                services::Action::Bid {
                    auction_id: a.id.clone(),
                    coins,
                    confirmed: confirm(
                        ask,
                        cancel,
                        lfmt!(
                            "Bid {coins} Oteryn coins for {}? The service reserves funds; settlement requires the Game ownership receipt.",
                            a.character_name,
                            coins = coins
                        ),
                    ),
                }
            }
            "sell" => {
                let (Some(c), Some(minimum_bid), Some(duration_hours)) = (
                    chosen(characters, reply, CHARACTERS),
                    value::<u64>(reply, MINIMUM),
                    value::<u32>(reply, DURATION),
                ) else {
                    return Err(tr!(
                        "Choose your character, a positive minimum bid, and 1–168 hours."
                    )
                    .into());
                };
                services::Action::Sell {
                    character_id: c.id.clone(),
                    minimum_bid,
                    duration_hours,
                    confirmed: confirm(
                        ask,
                        cancel,
                        lfmt!(
                            "List {} for {duration_hours} hours, minimum {minimum_bid} coins? The character must be offline and will stay locked. Items and gold remain with it.",
                            c.name,
                            duration_hours = duration_hours,
                            minimum_bid = minimum_bid
                        ),
                    ),
                }
            }
            "quote" => {
                let (Some(c), Some(w)) = (
                    chosen(characters, reply, CHARACTERS),
                    chosen(worlds, reply, WORLDS),
                ) else {
                    return Err(tr!("Choose a character and destination world.").into());
                };
                services::Action::Quote {
                    character_id: c.id.clone(),
                    world_id: w.id.clone(),
                }
            }
            "transfer" => {
                // The confirmation names the quote, so it must be for the chosen pair.
                let Some(quote) = view.quote.as_ref().filter(|q| {
                    chosen(characters, reply, CHARACTERS).is_some_and(|c| c.id == q.character_id)
                        && chosen(worlds, reply, WORLDS)
                            .is_some_and(|w| w.id == q.destination_world_id)
                }) else {
                    return Err(tr!("Choose a character and destination world.").into());
                };
                services::Action::Transfer {
                    confirmed: confirm(
                        ask,
                        cancel,
                        lfmt!(
                            "{}\nGame rechecks offline, guild, house and market restrictions before committing.",
                            view.transfer_description(quote)
                        ),
                    ),
                }
            }
            "receipt" => {
                let Some(r) = chosen(receipts, reply, RECEIPTS) else {
                    return Err(tr!("Your operation receipts (select to recover status)").into());
                };
                services::Action::Receipt(r.request_id.clone())
            }
            _ => services::Action::Back,
        })
    })
}

const PACKAGES: usize = 0;
const ORDERS: usize = 1;

pub(super) fn coins(
    view: &coin_shop::View,
    ask: &Ask,
    cancel: &CancellationToken,
) -> coin_shop::Action {
    let catalogue = view.catalogue.as_ref();
    let packages = catalogue.map_or(&[][..], |c| &c.packages);
    let orders = catalogue.map_or(&[][..], |c| &c.recent_orders);
    let screen = Screen {
        title: tr!("Oteryn — Get coins (sandbox)"),
        text: view.status_text(),
        lists: vec![
            list(
                tr!("Choose a server-priced package. Sandbox checkout uses no real money."),
                packages
                    .iter()
                    .map(|p| {
                        lfmt!(
                            "{} — {} coins — {}",
                            p.title,
                            p.coins,
                            coin_shop::price_text(&p.price)
                        )
                    })
                    .collect(),
            ),
            list(
                tr!("Your recent orders (choose one to inspect or resume)"),
                orders
                    .iter()
                    .map(|o| {
                        lfmt!(
                            "{:?} — {} coins — {} — {}",
                            o.state,
                            o.coins,
                            coin_shop::price_text(&o.price),
                            o.order_id
                        )
                    })
                    .collect(),
            ),
        ],
        buttons: vec![
            button(
                "refresh",
                tr!("Refresh packages / balance"),
                !view.unavailable,
            ),
            button(
                "buy",
                tr!("Buy selected package…"),
                packages.iter().any(|p| view.can_purchase(&p.id)),
            ),
            button("checkout", tr!("Open checkout"), view.can_checkout()),
            button(
                "status",
                tr!("Check purchase status"),
                view.pending() || view.order.is_some(),
            ),
            button(
                "inspect",
                tr!("Inspect selected order"),
                !view.pending() && !orders.is_empty(),
            ),
            button("retry", tr!("Retry same order request"), view.pending()),
            button("back", tr!("Back"), true),
        ],
        escape: "back",
        ..Screen::default()
    };
    run(screen, ask, cancel, coin_shop::Action::Back, |reply| {
        Ok(match reply.button {
            "refresh" => coin_shop::Action::Refresh,
            "checkout" => coin_shop::Action::OpenCheckout,
            "status" => coin_shop::Action::Status,
            "retry" => coin_shop::Action::Retry,
            "buy" => {
                let Some(p) =
                    chosen(packages, reply, PACKAGES).filter(|p| view.can_purchase(&p.id))
                else {
                    return Err(tr!(
                        "Choose a server-priced package. Sandbox checkout uses no real money."
                    )
                    .into());
                };
                coin_shop::Action::Purchase {
                    package_id: p.id.clone(),
                    confirmed: confirm(
                        ask,
                        cancel,
                        lfmt!(
                            "Buy {} Oteryn coins for {} in sandbox checkout? No real money is charged. Coins appear only after payment verification.",
                            p.coins,
                            coin_shop::price_text(&p.price)
                        ),
                    ),
                }
            }
            "inspect" => {
                let Some(o) = chosen(orders, reply, ORDERS) else {
                    return Err(tr!("Your recent orders (choose one to inspect or resume)").into());
                };
                coin_shop::Action::Inspect(o.request_id.clone())
            }
            _ => coin_shop::Action::Back,
        })
    })
}

pub(super) fn premium(
    view: &premium::View,
    ask: &Ask,
    cancel: &CancellationToken,
) -> premium::Action {
    let screen = Screen {
        title: tr!("Premium overview"),
        premium: Some(view.clone()),
        buttons: vec![
            button("refresh", tr!("Refresh Premium status"), true),
            button("back", tr!("Back"), true),
        ],
        escape: "back",
        ..Screen::default()
    };
    run(screen, ask, cancel, premium::Action::Back, |reply| {
        Ok(match reply.button {
            "refresh" => premium::Action::Refresh,
            _ => premium::Action::Back,
        })
    })
}
