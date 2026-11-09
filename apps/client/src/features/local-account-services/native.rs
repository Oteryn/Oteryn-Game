use super::window::Window;
use super::*;

pub(super) fn present(view: &View, cancel: CancellationToken) -> Action {
    form(view, cancel).unwrap_or(Action::Back)
}
fn form(view: &View, cancel: CancellationToken) -> Option<Action> {
    let window = Window::new(
        tr!("Oteryn — Character Bazaar and world transfer"),
        900,
        790,
        cancel,
    )?;
    window.button(tr!("Browse auctions"), 1, [15, 15, 160, 30])?;
    window.button(tr!("Premium overview"), 14, [645, 15, 220, 30])?;
    window.button(tr!("My auctions"), 2, [185, 15, 150, 30])?;
    window.button(tr!("Auction history"), 3, [345, 15, 150, 30])?;
    let next = window.button(tr!("Next page"), 4, [505, 15, 120, 30])?;
    let list = window.list(100, [15, 60, 850, 190])?;
    window.label(tr!("Bid (Oteryn coins)"), [15, 265, 160, 25])?;
    let amount = window.edit("", 101, [180, 260, 200, 28])?;
    let bid = window.button(tr!("Place bid…"), 5, [400, 260, 160, 30])?;
    window.label(tr!("Your character"), [15, 310, 250, 20])?;
    window.label(tr!("Minimum bid"), [300, 310, 150, 20])?;
    window.label(tr!("Duration (hours)"), [490, 310, 160, 20])?;
    let characters = window.combo(102, [15, 335, 260, 250])?;
    let minimum = window.edit("1", 103, [300, 335, 150, 28])?;
    let duration = window.edit("24", 104, [490, 335, 140, 28])?;
    let sell = window.button(tr!("List for auction…"), 6, [655, 335, 210, 30])?;
    window.label(tr!("Destination world"), [15, 380, 250, 20])?;
    let worlds = window.combo(105, [15, 405, 260, 250])?;
    let quote = window.button(tr!("Check transfer"), 7, [300, 405, 180, 30])?;
    let transfer = window.button(tr!("Confirm transfer…"), 8, [500, 405, 200, 30])?;
    window.label(
        tr!("Your operation receipts (select to recover status)"),
        [15, 455, 650, 20],
    )?;
    let receipts = window.combo(106, [15, 480, 640, 240])?;
    window.button(tr!("Check receipt"), 9, [675, 480, 190, 30])?;
    let retry = window.button(tr!("Retry same request"), 10, [15, 535, 210, 30])?;
    let status = window.button(tr!("Check pending status"), 11, [240, 535, 230, 30])?;
    window.button(tr!("Get coins"), 13, [485, 535, 135, 30])?;
    window.button(tr!("Return to characters"), 99, [630, 535, 235, 30])?;
    window.output(&view.status_text(), [15, 585, 850, 145])?;
    if let Some(page) = &view.page {
        for a in &page.auctions {
            list.add(&lfmt!(
                "{} — level {} — {} — next {} coins — high {} — {:?} — ends {} UTC seconds",
                a.character_name,
                a.level,
                a.world_name,
                a.minimum_next_bid,
                a.highest_bid,
                a.state,
                a.ends_at
            ));
        }
        for c in &page.owned_characters {
            characters.add(&c.name);
        }
        for w in &page.worlds {
            worlds.add(&w.name);
        }
        for r in &page.recent_operations {
            receipts.add(&format!("{:?} — {} — {}", r.state, r.code, r.request_id));
        }
        list.select(0);
        characters.select(0);
        worlds.select(0);
        receipts.select(0);
        if let Some(q) = &view.quote {
            if let Some(i) = page
                .owned_characters
                .iter()
                .position(|c| c.id == q.character_id)
            {
                characters.select(i);
            }
            if let Some(i) = page
                .worlds
                .iter()
                .position(|w| w.id == q.destination_world_id)
            {
                worlds.select(i);
            }
        }
    }
    next.enable(view.page.as_ref().is_some_and(|p| p.next_cursor.is_some()));
    bid.enable(!view.pending());
    sell.enable(!view.pending());
    quote.enable(!view.pending());
    retry.enable(view.can_retry());
    status.enable(view.pending());
    let mut previous = None;
    loop {
        let event = window.event();
        let selected = list.selected();
        if selected != previous {
            if let Some(a) = view
                .page
                .as_ref()
                .and_then(|p| selected.and_then(|i| p.auctions.get(i)))
            {
                amount.set(&a.minimum_next_bid.to_string());
            }
            previous = selected;
        }
        let character = view.page.as_ref().and_then(|p| {
            characters
                .selected()
                .and_then(|i| p.owned_characters.get(i))
        });
        let quote_matches = view.quote.as_ref().is_some_and(|q| {
            character.is_some_and(|c| c.id == q.character_id)
                && view
                    .page
                    .as_ref()
                    .and_then(|p| worlds.selected().and_then(|i| p.worlds.get(i)))
                    .is_some_and(|w| w.id == q.destination_world_id)
        });
        transfer.enable(view.can_transfer() && quote_matches);
        match event {
            1 => return Some(Action::Refresh(AuctionView::All)),
            2 => return Some(Action::Refresh(AuctionView::Own)),
            3 => return Some(Action::Refresh(AuctionView::History)),
            4 => return Some(Action::Next),
            5 => {
                if let (Some(a), Ok(coins)) = (
                    view.page
                        .as_ref()
                        .and_then(|p| list.selected().and_then(|i| p.auctions.get(i))),
                    amount.text().parse::<u64>(),
                ) {
                    return Some(Action::Bid {auction_id:a.id.clone(),coins,confirmed:window.confirm(&lfmt!("Bid {coins} Oteryn coins for {}? The service reserves funds; settlement requires the Game ownership receipt.",a.character_name,coins=coins))});
                }
                window.explain(tr!("Choose an auction and enter a valid coin amount."));
            }
            6 => {
                if let (Some(c), Ok(minimum_bid), Ok(duration_hours)) = (
                    character,
                    minimum.text().parse::<u64>(),
                    duration.text().parse::<u32>(),
                ) {
                    return Some(Action::Sell {character_id:c.id.clone(),minimum_bid,duration_hours,confirmed:window.confirm(&lfmt!("List {} for {duration_hours} hours, minimum {minimum_bid} coins? The character must be offline and will stay locked. Items and gold remain with it.",c.name,duration_hours=duration_hours,minimum_bid=minimum_bid))});
                }
                window.explain(tr!(
                    "Choose your character, a positive minimum bid, and 1–168 hours."
                ));
            }
            7 => {
                if let (Some(c), Some(w)) = (
                    character,
                    view.page
                        .as_ref()
                        .and_then(|p| worlds.selected().and_then(|i| p.worlds.get(i))),
                ) {
                    return Some(Action::Quote {
                        character_id: c.id.clone(),
                        world_id: w.id.clone(),
                    });
                }
                window.explain(tr!("Choose a character and destination world."));
            }
            8 => {
                if view.can_transfer() && quote_matches {
                    return Some(Action::Transfer {confirmed:window.confirm(&lfmt!("{}\nGame rechecks offline, guild, house and market restrictions before committing.", view.transfer_description(view.quote.as_ref()?)))});
                }
            }
            9 => {
                if let Some(r) = view
                    .page
                    .as_ref()
                    .and_then(|p| receipts.selected().and_then(|i| p.recent_operations.get(i)))
                {
                    return Some(Action::Receipt(r.request_id.clone()));
                }
            }
            10 => return Some(Action::Retry),
            11 => return Some(Action::Status),
            13 => return Some(Action::CoinShop),
            14 => return Some(Action::Premium),
            99 => return Some(Action::Back),
            _ => {}
        }
    }
}
