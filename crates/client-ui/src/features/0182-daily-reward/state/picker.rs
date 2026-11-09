use super::*;
use daily_reward::picker::{CLAIM, Claim, Offer, Pick, QUERY};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
pub fn request_picker(
    panel: &mut DailyRewardPanel,
    selection: Option<&str>,
) -> Option<FeatureCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let (kind, payload) = if let Some(text) = selection {
        let parsed = (|| {
            let offer = panel.picker?;
            if text.len() > 256
                || panel
                    .wall
                    .is_none_or(|w| !w.claim_enabled || w.claimed || w.lane_day != offer.day)
            {
                return None;
            }
            let mut picks = text
                .split(',')
                .map(|entry| {
                    let (id, count) = entry.trim().split_once(':')?;
                    Some(Pick {
                        item: id.trim().parse().ok()?,
                        count: count.trim().parse().ok()?,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            picks.sort_by_key(|p| p.item);
            if picks.iter().any(|p| !offer.items.contains(&p.item))
                || picks.iter().map(|p| u16::from(p.count)).sum::<u16>() > u16::from(offer.maximum)
            {
                return None;
            }
            Claim {
                epoch: offer.epoch,
                day: offer.day,
                picks,
            }
            .encode()
            .ok()
        })();
        let Some(bytes) = parsed else {
            panel.feedback = Some(
                "Choose listed item IDs and positive counts within the allowance (id:count, id:count)",
            );
            return None;
        };
        (CLAIM, bytes)
    } else {
        (QUERY, vec![])
    };
    panel.pending = true;
    panel.picker_pending = Some(kind);
    panel.feedback = None;
    Some(FeatureCommand {
        capability: 182,
        kind,
        payload,
    })
}
pub fn apply_picker(panel: &mut DailyRewardPanel, kind: u32, result: &GatedCommandResult) -> bool {
    if panel.picker_pending != Some(kind) {
        return false;
    }
    panel.pending = false;
    panel.picker_pending = None;
    if kind == QUERY {
        if matches!(
            result.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) && let Ok(offer) = Offer::decode(&result.payload)
        {
            panel.picker = Some(offer);
            panel.feedback = None;
            return true;
        }
        panel.picker = None;
        panel.feedback = Some("The server refused the item catalogue");
        return false;
    }
    let reply = decode_reply(DailyRewardCommand::Claim, &result.payload).ok();
    super::apply_result(
        panel,
        &DailyRewardResponse {
            status: result.status,
            reply,
        },
    )
}
pub fn picker_text(panel: &DailyRewardPanel) -> String {
    let mut text = String::from(
        "Daily reward item selection\nEnter item:count pairs, for example 266:3,268:2.\n",
    );
    if let Some(o) = panel.picker {
        text.push_str(&format!(
            "Day {} — {} — allowance {} total items\n",
            o.day,
            if o.premium { "Premium" } else { "Free" },
            o.maximum
        ));
        if (o.day - 1) % 7 + 1 == 6 {
            text.push_str("Each exercise weapon has 50 charges.\n");
        }
        for id in o.items.into_iter().filter(|id| *id != 0) {
            text.push_str(&format!("{id}: {}\n", super::picker_names::name(id)));
        }
        if o.maximum == 0 {
            text.push_str("No item selection available for this day or already claimed.\n");
        }
    } else {
        text.push_str("Refresh to load the server catalogue.\n");
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}
