//! Confirmed party measurements and currency-conserving local split suggestions.
use super::Panel;
use oteryn_client_runtime::{
    analyser::party::{self, Member, Price},
    raw_feature::{FeatureCommand, GatedCommandResult},
};
pub fn request(panel: &mut Panel) -> Option<FeatureCommand> {
    if !panel.selected || panel.party_pending {
        return None;
    }
    let ids: std::collections::BTreeSet<_> = panel
        .loot_prices
        .keys()
        .chain(panel.supply_prices.keys())
        .copied()
        .collect();
    let prices: Vec<_> = ids
        .into_iter()
        .map(|item| Price {
            item,
            loot: panel.loot_prices.get(&item).copied(),
            supply: panel.supply_prices.get(&item).copied(),
        })
        .collect();
    let Ok(payload) = party::encode_request(&prices) else {
        panel.party_feedback = Some(tr!("Party accounting supports up to 2048 item prices"));
        return None;
    };
    panel.party_pending = true;
    panel.party_pending_revision = panel.party_revision;
    panel.party_feedback = Some(tr!("Reading confirmed current-party measurements"));
    Some(FeatureCommand {
        capability: 10,
        kind: party::COMMAND_TYPE_ANALYSER_PARTY_QUERY,
        payload,
    })
}
pub fn result(panel: &mut Panel, reply: &GatedCommandResult) {
    if !panel.party_pending {
        return;
    }
    panel.party_pending = false;
    if panel.party_pending_revision != panel.party_revision {
        panel.party_feedback = Some(tr!("Prices changed; refresh party accounting"));
        return;
    }
    if reply.status != oteryn_client_runtime::CommandStatus::Accepted {
        panel.party_feedback = Some(tr!(
            "Party accounting was refused; refresh after reconnecting or changing party"
        ));
        return;
    }
    match party::decode_reply(&reply.payload) {
        Ok(rows) => {
            panel.party = rows;
            panel.party_feedback = None;
        }
        Err(_) => {
            panel.party_feedback = Some(tr!(
                "Invalid party accounting; previous measurements retained"
            ))
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct Transfer {
    pub from: u64,
    pub to: u64,
    pub gold: u128,
}
/// Every member keeps an equal integer share; remainders go in character-name order.
pub fn split(rows: &[Member]) -> Option<Vec<Transfer>> {
    if rows.is_empty()
        || rows.iter().any(|r| {
            !r.available || r.incomplete || r.unpriced_loot != 0 || r.unpriced_supplies != 0
        })
    {
        return None;
    }
    let mut rows: Vec<_> = rows.iter().collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name).then(a.handle.cmp(&b.handle)));
    let total: i128 = rows
        .iter()
        .map(|r| i128::from(r.loot_value) - i128::from(r.supply_cost))
        .sum();
    let count = rows.len() as i128;
    let base = total.div_euclid(count);
    let extra = total.rem_euclid(count) as usize;
    let mut payers = Vec::new();
    let mut receivers = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let share = base + i128::from(i < extra);
        let excess = i128::from(row.loot_value) - i128::from(row.supply_cost) - share;
        if excess > 0 {
            payers.push((row.handle, excess.unsigned_abs()));
        } else if excess < 0 {
            receivers.push((row.handle, excess.unsigned_abs()));
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < payers.len() && j < receivers.len() {
        let gold = payers[i].1.min(receivers[j].1);
        out.push(Transfer {
            from: payers[i].0,
            to: receivers[j].0,
            gold,
        });
        payers[i].1 -= gold;
        receivers[j].1 -= gold;
        if payers[i].1 == 0 {
            i += 1;
        }
        if receivers[j].1 == 0 {
            j += 1;
        }
    }
    Some(out)
}
pub fn text(panel: &Panel) -> String {
    use std::fmt::Write;
    if let Some(imported) = &panel.party_import {
        let mut text = imported.text();
        if let Some(feedback) = panel.feedback {
            let _ = write!(text, "\r\n{feedback}");
        }
        return text;
    }
    let mut text = tr!("Party Hunt\r\nConfirmed current-party members · each current connection interval\r\nName | seconds | kills | XP | loot items/value | supplies/cost | damage/healing\r\n").to_owned();
    for row in &panel.party {
        if !row.available {
            let _ = lwriteln!(text, "{} | measurements unavailable", row.name);
            continue;
        }
        let _ = write!(
            text,
            "{} | {} | {} | {} | {}/{} | {}/{} | {}/{}\r\n",
            row.name,
            row.elapsed_seconds,
            row.kills,
            row.xp,
            row.loot_items,
            row.loot_value,
            row.supplies,
            row.supply_cost,
            row.damage,
            row.healing
        );
        if row.incomplete || row.unpriced_loot != 0 || row.unpriced_supplies != 0 {
            let _ = lwrite!(
                text,
                "  Incomplete: {} | unpriced loot {} / supplies {}\r\n",
                row.incomplete,
                row.unpriced_loot,
                row.unpriced_supplies
            );
        }
        for needed in &row.needed_prices {
            let _ = lwrite!(
                text,
                "  Price needed for item #{}: {}{}\r\n",
                needed.item,
                if needed.loot { "loot " } else { "" },
                if needed.supply { "supply" } else { "" }
            );
        }
    }
    if panel.party.is_empty() {
        text.push_str(tr!(
            "No party accounting received. Refresh to read the server.\r\n"
        ));
    }
    let transfers = if panel.party_pending || panel.party_feedback.is_some() {
        None
    } else {
        split(&panel.party)
    };
    if let Some(transfers) = transfers {
        text.push_str(tr!(
            "Equal balance split · suggestions only; no currency is moved\r\n"
        ));
        if transfers.is_empty() {
            text.push_str(tr!("Already balanced\r\n"));
        }
        for transfer in transfers {
            let name = |handle| {
                panel
                    .party
                    .iter()
                    .find(|r| r.handle == handle)
                    .map_or("?", |r| r.name.as_str())
            };
            let _ = lwrite!(
                text,
                "{} pays {}: {} gold\r\n",
                name(transfer.from),
                name(transfer.to),
                transfer.gold
            );
        }
    } else {
        text.push_str(tr!(
            "Split unavailable until every member has complete, priced measurements.\r\n"
        ));
    }
    text.push_str(tr!("Apply item price sets loot value; Party supply price sets consumption cost. Refresh after pricing.\r\n"));
    if let Some(feedback) = panel.party_feedback {
        text.push_str(feedback);
    }
    if let Some(feedback) = panel.feedback {
        text.push_str("\r\n");
        text.push_str(feedback);
    }
    text
}
