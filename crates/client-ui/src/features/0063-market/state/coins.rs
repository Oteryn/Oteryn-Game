//! Authoritative coin market views and explicit confirmation; no local balance changes.
use oteryn_client_runtime::{
    coin_market as wire,
    server_mechanic::{CommandStatus, ServerMechanicCommand, ServerMechanicResponse},
};
use serde_json::Value;
#[path = "coins/presentation.rs"]
mod presentation;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoinPanel {
    pub open: bool,
    pub pending: Option<wire::Request>,
    pub fields: [String; 3],
    pub reviewed: Option<wire::Request>,
    pub confirmed: Option<wire::Request>,
    pub view: Option<String>,
    pub quotes: Value,
    pub own_quotes: bool,
    pub book_view: bool,
    pub detail_rows: Vec<String>,
    pub cursor: String,
    pub offer_cursor: String,
    pub last_operation: String,
    pub feedback: String,
}
fn uuid(v: &[u8; 16]) -> String {
    let h = v.iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}
fn parse_uuid(v: &str) -> Option<[u8; 16]> {
    if v.len() != 36 || ![8, 13, 18, 23].iter().all(|i| v.as_bytes()[*i] == b'-') {
        return None;
    }
    let h = v.replace('-', "");
    let mut id = [0; 16];
    for (i, n) in id.iter_mut().enumerate() {
        *n = u8::from_str_radix(h.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    (id[6] >> 4 == 7 && id[8] >> 6 == 2 && uuid(&id) == v).then_some(id)
}
fn draft(p: &CoinPanel, action: usize) -> Option<wire::Request> {
    let mut r = wire::Request {
        action: match action {
            2 => 3,
            3 => 4,
            4 => 5,
            5 => 6,
            _ => return None,
        },
        ..Default::default()
    };
    if r.action <= 5 {
        r.amount = p.fields[0].trim().parse().ok()?;
    }
    if r.action <= 4 {
        r.price = p.fields[1].trim().parse().ok()?;
    }
    if matches!(r.action, 5 | 6) {
        if !p.book_view {
            return None;
        }
        let index = p.fields[2].trim().parse::<usize>().ok()?.checked_sub(1)?;
        let quote = p.quotes["offers"].as_array()?.get(index)?;
        r.offer = parse_uuid(quote["offer"].as_str()?)?;
        if r.action == 5 && r.amount > quote["remaining"].as_u64()? {
            return None;
        }
        if r.action == 6 && !p.own_quotes {
            return None;
        }
    }
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    r.operation = id;
    wire::encode(&r).ok()?;
    Some(r)
}
pub fn coin_control(
    p: &mut CoinPanel,
    selected: bool,
    action: usize,
    fields: [String; 3],
    operation: [u8; 16],
) -> Option<ServerMechanicCommand> {
    if !selected || p.pending.is_some() {
        return None;
    }
    p.fields = fields;
    if matches!(action, 2..=5) {
        if p.confirmed.is_some() {
            p.feedback = "Inspect or retry the unresolved confirmation first".into();
            return None;
        }
        p.reviewed = draft(p, action);
        p.feedback = if let Some(r) = &p.reviewed {
            let (price, total, fee) = if matches!(r.action, 3 | 4) {
                (
                    r.price,
                    r.amount.checked_mul(r.price)?,
                    (r.amount.checked_mul(r.price)? / 50).clamp(20, 1_000_000),
                )
            } else {
                let index = p.fields[2].trim().parse::<usize>().ok()?.checked_sub(1)?;
                let q = p.quotes["offers"].as_array()?.get(index)?;
                let price = q["price"].as_u64()?;
                (price, r.amount.checked_mul(price)?, 0)
            };
            let verb = match r.action {
                3 => "Sell",
                4 => "Buy",
                5 => {
                    let row = p.fields[2].trim().parse::<usize>().ok()?.checked_sub(1)?;
                    if p.quotes["offers"][row]["side"] == "sell" {
                        "Buy from sell offer"
                    } else {
                        "Sell into buy offer"
                    }
                }
                6 => "Cancel",
                _ => "Review",
            };
            if r.action == 6 {
                format!(
                    "REVIEW cancel remaining offer row {}. The opening fee is not refunded. Confirm cancellation.",
                    p.fields[2]
                )
            } else {
                format!(
                    "REVIEW {verb}: {} coins, {} gold each, total {} gold, fee {} gold; offer row {}. Confirm these exact terms.",
                    r.amount, price, total, fee, p.fields[2]
                )
            }
        } else {
            "Enter valid amount, price and an offer row; load My Offers before cancelling".into()
        };
        return None;
    }
    let r = match action {
        0 => wire::Request::default(),
        1 => wire::Request {
            action: 2,
            ..Default::default()
        },
        6 => {
            if p.confirmed.is_some() {
                p.feedback = "Retry the existing confirmation".into();
                return None;
            }
            let reviewed = p.reviewed.clone()?;
            let input_action = match reviewed.action {
                3 => 2,
                4 => 3,
                5 => 4,
                6 => 5,
                _ => return None,
            };
            if draft(p, input_action).as_ref() != Some(&reviewed) {
                p.reviewed = None;
                p.feedback = "Terms changed: review again".into();
                return None;
            }
            let mut confirmed = reviewed;
            confirmed.operation = operation;
            wire::encode(&confirmed).ok()?;
            p.confirmed = Some(confirmed.clone());
            p.last_operation = uuid(&confirmed.operation);
            confirmed
        }
        7 => p.confirmed.clone()?,
        8 => wire::Request {
            action: 7,
            ..Default::default()
        },
        9 => {
            if p.cursor.is_empty() {
                p.feedback = "No older history page".into();
                return None;
            }
            wire::Request {
                action: 7,
                cursor: p.cursor.clone(),
                ..Default::default()
            }
        }
        14 => {
            if p.offer_cursor.is_empty() {
                p.feedback = "No further offer page".into();
                return None;
            }
            wire::Request {
                action: if p.own_quotes { 2 } else { 1 },
                cursor: p.offer_cursor.clone(),
                ..Default::default()
            }
        }
        10 => wire::Request {
            action: 8,
            offer: parse_uuid(
                p.detail_rows
                    .get(p.fields[2].trim().parse::<usize>().ok()?.checked_sub(1)?)?,
            )?,
            ..Default::default()
        },
        11 => wire::Request {
            action: 9,
            ..Default::default()
        },
        12 => wire::Request {
            action: 8,
            offer: parse_uuid(&p.last_operation)?,
            ..Default::default()
        },
        13 => {
            p.reviewed = None;
            p.feedback =
                "Review dismissed; unresolved operations remain retained by the server".into();
            return None;
        }
        _ => return None,
    };
    let payload = wire::encode(&r).ok()?;
    p.pending = Some(r);
    p.feedback = "Waiting for the server".into();
    Some(ServerMechanicCommand {
        capability: wire::CAPABILITY,
        command_type: wire::COMMAND,
        payload,
    })
}
pub fn coin_result(p: &mut CoinPanel, r: &ServerMechanicResponse) {
    let pending = p.pending.take();
    if !matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = "Server refused the request; retry retains the confirmation key".into();
        return;
    }
    let Some(value) = wire::decode_reply(&r.payload)
        .ok()
        .and_then(|v| serde_json::from_str::<Value>(&v).ok())
    else {
        p.feedback = "Invalid coin market response; confirmation retained".into();
        return;
    };
    p.book_view = value["offers"].is_array();
    p.detail_rows = presentation::targets(&value);
    if value["offers"].is_array() {
        p.quotes = value.clone();
        p.offer_cursor = value["next_offer_cursor"].as_str().unwrap_or("").into();
        p.own_quotes = pending.as_ref().is_some_and(|r| r.action == 2);
    }
    if let Some(cursor) = value["next_cursor"].as_str() {
        p.cursor = cursor.into();
    } else if value.get("history").is_some() {
        p.cursor.clear();
    }
    if let Some(confirmed) = &p.confirmed {
        let completed = value["operation"].as_str() == Some(uuid(&confirmed.operation).as_str())
            && value["pending"] == false;
        let refused = pending.as_ref() == Some(confirmed)
            && matches!(
                value["result"].as_str(),
                Some(
                    "insufficient_gold"
                        | "offer_limit"
                        | "invalid_amount"
                        | "rejected"
                        | "offer_busy_or_closed"
                        | "not_found"
                        | "junior"
                        | "depot_required"
                        | "idempotency_conflict"
                )
            );
        if completed || refused {
            p.confirmed = None;
            p.reviewed = None;
        }
    }
    p.feedback = if value["pending"] == true {
        "Pending reconciliation; retry retains the same confirmation".into()
    } else if value["result"] == "ok" {
        "Server confirmed".into()
    } else {
        value["result"]
            .as_str()
            .unwrap_or("invalid response")
            .replace('_', " ")
    };
    p.view = Some(presentation::render(&value));
}
pub fn coin_text(p: &CoinPanel) -> String {
    format!(
        "Coin market — transferable coins only. Coins belong to your account; gold stays in this World. Pending trades retain their reserved coins or gold.\r\n{}\r\n{}\r\n{}",
        p.feedback,
        if p.confirmed.is_some() {
            "An unresolved confirmation is retained for retry."
        } else {
            ""
        },
        p.view
            .as_deref()
            .unwrap_or("Load the book, My Offers, history or balances")
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn market_confirmation_keeps_one_key_after_ambiguous_result_and_never_changes_balances()
    -> Result<(), &'static str> {
        let mut p = CoinPanel::default();
        let fields = ["25".into(), "100".into(), String::new()];
        assert!(coin_control(&mut p, true, 2, fields.clone(), [0; 16]).is_none());
        assert!(p.feedback.contains("2500 gold"));
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let first = coin_control(&mut p, true, 6, fields.clone(), id).ok_or("confirmation")?;
        assert!(p.view.is_none());
        p.pending = None;
        let retry = coin_control(&mut p, true, 7, fields, [0; 16]).ok_or("retry")?;
        assert_eq!(first, retry);
        assert!(p.view.is_none());
        Ok(())
    }
}
