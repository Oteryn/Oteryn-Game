use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Browse,
    OwnOffers,
    History,
    CreateOffer,
    CancelOffer,
    AcceptOffer,
    Refresh,
}

pub fn request(panel: &mut MarketPanel, intent: Intent) -> Option<MarketCommand> {
    let result = prepare(panel, intent);
    match result {
        Ok(command) => {
            panel.pending = Some(command.clone());
            panel.feedback = None;
            Some(command)
        }
        Err(message) => {
            panel.feedback = Some(message);
            None
        }
    }
}

fn prepare(panel: &MarketPanel, intent: Intent) -> Result<MarketCommand, &'static str> {
    if !panel.selected {
        return Err("Market is unavailable in this session");
    }
    if panel.pending.is_some() {
        return Err("Waiting for the server");
    }
    let zero = [0; 16];
    let (action, definition, offer, amount, price, anonymous) = match intent {
        Intent::Browse => (2, panel.definition, zero, 0, 0, false),
        Intent::OwnOffers => (3, 0, zero, 0, 0, false),
        Intent::History => (4, 0, zero, 0, 0, false),
        Intent::CreateOffer => (
            if panel.sell { 6 } else { 5 },
            panel.definition,
            zero,
            panel.amount,
            panel.price,
            panel.anonymous,
        ),
        Intent::CancelOffer => (
            7,
            0,
            panel.selected_offer.ok_or("Select an offer to cancel")?,
            0,
            0,
            false,
        ),
        Intent::AcceptOffer => (
            8,
            0,
            panel.selected_offer.ok_or("Select an offer to accept")?,
            panel.amount,
            0,
            false,
        ),
        Intent::Refresh => match panel.view {
            MarketView::Browse => (2, panel.definition, zero, 0, 0, false),
            MarketView::Own => (3, 0, zero, 0, 0, false),
            MarketView::History => (4, 0, zero, 0, 0, false),
        },
    };
    let request = market::MarketRequest {
        action,
        definition,
        offer,
        amount,
        price,
        anonymous,
        page: if action <= 4 { panel.page } else { 0 },
    };
    market::encode_request(request).map_err(|_| "Enter valid market offer details")?;
    Ok(MarketCommand(request))
}

pub fn apply_result(panel: &mut MarketPanel, response: &MarketResponse) -> bool {
    let Some(pending) = panel.pending.take() else {
        return false;
    };
    let Some(reply) = &response.reply else {
        panel.feedback = Some("The server refused the market request");
        return false;
    };
    if reply.action != pending.0.action {
        panel.feedback = Some("The market response did not match the request");
        return false;
    }
    if reply.code != 1 {
        panel.feedback = Some(refusal(reply.code));
        return false;
    }
    panel.view = match reply.action {
        2 => MarketView::Browse,
        3 => MarketView::Own,
        4 => MarketView::History,
        _ => panel.view,
    };
    if matches!(reply.action, 1..=4) {
        panel.rows.clone_from(&reply.rows);
    }
    panel.selected_offer = panel.rows.first().map(|row| row.offer);
    panel.feedback = Some("The server confirmed the market request");
    true
}

fn refusal(code: u8) -> &'static str {
    match code {
        2 => "Market refused: insufficient funds",
        3 => "Market refused: insufficient items",
        4 => "Market refused: inbox full",
        5 => "Market refused: offer limit reached",
        6 => "Market refused: offer is no longer available",
        7 => "Market refused: this is your offer",
        8 => "Market refused: touched-item limit reached",
        10 => "Market refused: balance limit reached",
        11 => "Market refused: no depot locker",
        12 => "Market refused: Premium required",
        13 => "Market refused: junior account",
        14 => "Market refused: item is not marketable",
        15 => "Market refused: order book busy",
        _ => "Market request refused by the server",
    }
}
