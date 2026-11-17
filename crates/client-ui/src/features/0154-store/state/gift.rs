use super::{GiftDraft, StorePanel};
use oteryn_client_runtime::{
    coin_transfer as wire,
    server_mechanic::{ServerMechanicCommand, ServerMechanicResponse},
};
use serde_json::Value;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GiftPanel {
    pub quote: Option<wire::Request>,
    pub pending: Option<wire::Request>,
    pub confirmed: Option<wire::Request>,
    pub receipt: String,
    pub last_operation: Option<[u8; 16]>,
}
fn id(s: &str) -> Option<[u8; 16]> {
    let h = s.replace('-', "");
    if h.len() != 32 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut v = [0; 16];
    for (i, b) in v.iter_mut().enumerate() {
        *b = u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(v)
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
pub fn gift_control(
    p: &mut StorePanel,
    action: usize,
    fields: [String; 2],
    operation: [u8; 16],
) -> Option<ServerMechanicCommand> {
    if action == 2 {
        p.gift_open = false;
        return None;
    }
    if !p.selected || p.gift.pending.is_some() {
        return None;
    }
    p.gift_fields = fields.clone();
    let r = match action {
        0 => {
            if p.gift.confirmed.is_some() {
                p.feedback = Some("A gift is unresolved; retry or inspect its receipt".into());
                return None;
            }
            p.gift.quote = None;
            p.gift_reviewed = None;
            wire::Request {
                action: 1,
                recipient: fields[0].trim().into(),
                amount: fields[1].trim().parse().ok()?,
                ..Default::default()
            }
        }
        1 => {
            if p.gift.confirmed.is_some() {
                p.feedback =
                    Some("Retry the unresolved confirmation; do not create another gift".into());
                return None;
            }
            let Some(mut r) = p.gift.quote.clone() else {
                p.feedback = Some("Review the recipient and amount with the server first".into());
                return None;
            };
            if r.recipient != fields[0].trim()
                || fields[1].trim().parse::<u64>().ok() != Some(r.amount)
            {
                p.gift.quote = None;
                p.feedback = Some("Recipient or amount changed; review again".into());
                return None;
            }
            r.action = 2;
            r.operation = operation;
            p.gift.confirmed = Some(r.clone());
            r
        }
        3 => p.gift.confirmed.clone()?,
        4 => wire::Request {
            action: 3,
            operation: p
                .gift
                .confirmed
                .as_ref()
                .map(|r| r.operation)
                .or(p.gift.last_operation)?,
            ..Default::default()
        },
        _ => return None,
    };
    let payload = match wire::encode(&r) {
        Ok(v) => v,
        Err(_) => {
            p.feedback =
                Some("Use a full character name and a positive multiple of 25 coins".into());
            return None;
        }
    };
    p.gift.pending = Some(r);
    p.feedback = Some("Waiting for the server; no local coin debit".into());
    Some(ServerMechanicCommand {
        capability: wire::CAPABILITY,
        command_type: wire::COMMAND,
        payload,
    })
}
fn balance(v: &Value, transferable: &str, non_transferable: &str) -> String {
    let coins = |key: &str| v[key].as_str().and_then(|s| s.parse::<u64>().ok());
    match (coins(transferable), coins(non_transferable)) {
        (Some(a), Some(b)) => a.saturating_add(b).to_string(),
        _ => "?".into(),
    }
}
pub fn gift_result(p: &mut StorePanel, response: &ServerMechanicResponse) {
    let Some(request) = p.gift.pending.take() else {
        return;
    };
    let value = wire::decode_reply(&response.payload)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok());
    let Some(v) = value.filter(|_| {
        matches!(
            response.status,
            oteryn_client_runtime::CommandStatus::Accepted
                | oteryn_client_runtime::CommandStatus::DuplicateReplay
        )
    }) else {
        p.feedback = Some("Gift response unavailable; retry keeps the same confirmation".into());
        return;
    };
    if request.action == 1 && v["result"] == "ok" {
        let Some(quote) = v["quote"].as_str().and_then(id) else {
            return;
        };
        let Some(name) = v["recipient"].as_str() else {
            return;
        };
        let Some(amount) = v["amount"].as_str().and_then(|s| s.parse::<u64>().ok()) else {
            return;
        };
        if !name.eq_ignore_ascii_case(&request.recipient) || amount != request.amount {
            return;
        }
        let r = wire::Request {
            action: 1,
            recipient: name.into(),
            amount,
            quote,
            ..Default::default()
        };
        p.gift_fields = [name.into(), amount.to_string()];
        p.gift_reviewed = Some(GiftDraft {
            recipient: name.into(),
            amount,
        });
        p.gift.quote = Some(r);
        p.feedback = Some(format!(
            "Review gift to {name}: {amount} coins. Balance: {}. Confirm within 60 seconds. Gifts cannot be recalled.",
            balance(&v, "transferable", "non_transferable")
        ));
        return;
    }
    if matches!(request.action, 2 | 3)
        && v["operation"].as_str() == Some(uuid(&request.operation).as_str())
    {
        if v["pending"] == false {
            p.gift.confirmed = None;
            p.gift.quote = None;
            p.gift.last_operation = Some(request.operation);
            p.gift_reviewed = None;
            let receipt = &v["receipt"]["receipt"];
            p.gift.receipt = format!(
                "Gift receipt: {}\r\nAmount: {}\r\nBalance after: {}\r\nTime: {}",
                uuid(&request.operation),
                receipt["amount"].as_str().unwrap_or("?"),
                balance(receipt, "transferable_after", "non_transferable_after"),
                receipt["occurred_at"].as_str().unwrap_or("?")
            );
            p.feedback = Some(p.gift.receipt.clone());
            return;
        }
        p.feedback = Some(format!(
            "Gift pending: {}. Retry uses the same confirmation. {}",
            uuid(&request.operation),
            v["error"].as_str().unwrap_or("")
        ));
        return;
    }
    let result = v["result"].as_str().unwrap_or("invalid_response");
    if matches!(
        result,
        "quote_expired" | "quote_changed_or_used" | "recipient_changed" | "idempotency_conflict"
    ) {
        p.gift.confirmed = None;
        p.gift.quote = None;
        p.gift_reviewed = None;
    }
    p.feedback = Some(format!(
        "Gift: {result}. No local coin balance was changed."
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coin1_gift_review_uses_one_balance_and_allows_confirmation()
    -> Result<(), Box<dyn std::error::Error>> {
        for (transferable, non_transferable, total) in [
            ("95", "0", "95"),
            ("90", "5", "95"),
            ("0", "95", "95"),
            ("18446744073709551615", "1", "18446744073709551615"),
        ] {
            let mut panel = StorePanel {
                selected: true,
                ..Default::default()
            };
            let fields = ["Alice".into(), "75".into()];
            assert!(gift_control(&mut panel, 0, fields.clone(), [0; 16]).is_some());
            let payload = wire::encode_reply(
                &serde_json::json!({
                    "result": "ok", "quote": "019a65f0-0000-7000-8000-000000000001",
                    "recipient": "Alice", "amount": "75",
                    "transferable": transferable, "non_transferable": non_transferable
                })
                .to_string(),
            )
            .map_err(|e| format!("{e:?}"))?;
            gift_result(
                &mut panel,
                &ServerMechanicResponse {
                    command_id: 1,
                    server_sequence: 1,
                    status: oteryn_client_runtime::CommandStatus::Accepted,
                    payload,
                },
            );
            let text = panel.feedback.as_deref().ok_or("feedback")?;
            assert!(
                text.contains(&format!("75 coins. Balance: {total}.")),
                "{text}"
            );
            assert!(!text.to_lowercase().contains("transferable"), "{text}");
            let mut operation = [1; 16];
            operation[6] = 0x70;
            operation[8] = 0x80;
            assert!(gift_control(&mut panel, 1, fields, operation).is_some());
        }
        Ok(())
    }
}
