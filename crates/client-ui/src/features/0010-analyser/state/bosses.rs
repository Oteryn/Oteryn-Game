//! Analyser projection over existing boss owners; no invented kills or reward grants.
use super::Panel;
use crate::{boss_encounter::BossEncounterPanel, boss_rewards::BossRewardsPanel};
use oteryn_client_runtime::boss_rewards::{
    BossRewardsReply, BossRewardsResponse, CommandStatus, boss_rewards,
};
use std::{fmt::Write, time::Instant};
pub const MAX_DROPS: usize = 256;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drop {
    pub death: [u8; 16],
    pub ordinal: u16,
    pub definition: u32,
    pub quantity: u32,
    pub boss: Option<String>,
}
pub fn observe(panel: &mut Panel, bosses: &BossEncounterPanel, response: &BossRewardsResponse) {
    if !panel.selected
        || !matches!(
            response.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        )
    {
        return;
    }
    let Some(BossRewardsReply::Browse(reply)) = &response.reply else {
        return;
    };
    if reply.status != boss_rewards::BossRewardStatus::Ok {
        return;
    }
    let boss = bosses
        .rewards
        .as_ref()
        .and_then(|chest| {
            chest
                .entries
                .iter()
                .find(|entry| entry.death == reply.death)
        })
        .map(|entry| entry.boss_key.clone());
    for row in &reply.rows {
        if let Some(previous) = panel
            .boss_drops
            .iter_mut()
            .find(|entry| entry.death == reply.death && entry.ordinal == row.ordinal)
        {
            if previous.definition == row.definition_reference {
                previous.quantity = previous.quantity.max(row.quantity);
                if boss.is_some() {
                    previous.boss.clone_from(&boss);
                }
            }
            continue;
        }
        if panel.boss_drops.len() == MAX_DROPS {
            panel.boss_drops.pop_front();
            panel.boss_drops_truncated = true;
        }
        panel.boss_drops.push_back(Drop {
            death: reply.death,
            ordinal: row.ordinal,
            definition: row.definition_reference,
            quantity: row.quantity,
            boss: boss.clone(),
        });
    }
}
pub fn text(panel: &Panel, bosses: &BossEncounterPanel, rewards: &BossRewardsPanel) -> String {
    text_at(panel, bosses, rewards, Instant::now())
}
pub fn text_at(
    panel: &Panel,
    bosses: &BossEncounterPanel,
    rewards: &BossRewardsPanel,
    now: Instant,
) -> String {
    let elapsed = bosses.cooldown_received_at.map_or(0, |at| {
        u64::try_from(now.saturating_duration_since(at).as_millis()).unwrap_or(u64::MAX)
    });
    let mut text = lfmt!(
        "Boss Tracker\r\n{}\r\nPersonal drop chests (refresh confirms live contents)\r\n",
        crate::boss_encounter::cooldown_text(bosses, elapsed)
    );
    if let Some(feedback) = panel.feedback {
        let _ = write!(text, "{feedback}\r\n");
    }
    if let Some(chest) = &bosses.rewards {
        for entry in &chest.entries {
            let death: String = entry
                .death
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            let _ = lwrite!(
                text,
                "{} | {} items | death {} | expires {}\r\n",
                entry.boss_key,
                entry.item_count,
                death,
                entry.expires_at_unix_ms
            );
        }
        if chest.entries.len() == oteryn_client_runtime::boss_encounter::REWARD_CHEST_PAGE_SIZE {
            text.push_str(tr!("This page is full; check the next drop page\r\n"));
        }
    } else {
        text.push_str(tr!("Refresh drop chests to load server records\r\n"));
    }
    if rewards.pending {
        text.push_str(tr!("Reading selected drop contents\r\n"));
    } else if rewards
        .browse
        .as_ref()
        .is_some_and(|page| page.status == boss_rewards::BossRewardStatus::Ok)
        && rewards.feedback.is_none()
    {
        text.push_str(&crate::boss_rewards::panel_text(rewards));
    } else if let Some(feedback) = rewards.feedback {
        let _ = write!(text, "{feedback}\r\n");
    }
    text.push_str(tr!(
        "\r\nObserved reward history · largest quantity seen, may already be collected\r\n"
    ));
    for entry in &panel.boss_drops {
        let _ = lwrite!(
            text,
            "{} | item {} × {} | reward #{}\r\n",
            entry.boss.as_deref().unwrap_or(tr!("Boss key not loaded")),
            entry.definition,
            entry.quantity,
            entry.ordinal
        );
    }
    if panel.boss_drops_truncated {
        text.push_str(tr!(
            "Older observations were discarded at the 256-row limit\r\n"
        ));
    }
    text
}
