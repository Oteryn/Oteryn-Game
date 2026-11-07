use super::ArenaPanel;
use oteryn_client_runtime::arena::ArenaDisposition;

pub fn panel_text(panel: &ArenaPanel) -> String {
    let mut text = String::from("Svargrond Arena\r\n");
    for challenge in &panel.challenges {
        text.push_str(&format!(
            "{:?}: {} gold, {} pits, {} seconds per pit\r\n",
            challenge.rank, challenge.fee, challenge.rooms, challenge.room_seconds
        ));
    }
    if let Some(reply) = panel.result {
        let s = reply.state;
        let disposition = match reply.disposition {
            ArenaDisposition::Applied => "Applied",
            ArenaDisposition::WrongState => "Action unavailable in the current state",
            ArenaDisposition::InsufficientGold => "Insufficient gold",
            ArenaDisposition::AlreadyClaimed => "Reward already claimed",
            ArenaDisposition::InventoryFull => "Inventory full",
            ArenaDisposition::TimedOut => "Time expired",
            ArenaDisposition::Busy => "Arena busy; retry",
            ArenaDisposition::InvalidReward => "Invalid reward",
            ArenaDisposition::AllChallengesCompleted => "All challenges completed",
            ArenaDisposition::ContentUnavailable => "Arena content unavailable",
        };
        let phase = match s.phase {
            1 => "Fighting",
            2 => "Next pit ready",
            3 => "Reward room",
            _ => "Idle",
        };
        text.push_str(&format!("{disposition}\r\n{phase}: {}, pit {}. Completed: {}\r\nServer time remaining: {} seconds (Refresh for current time)\r\n",
            rank_name(s.active_rank), s.pit, rank_name(s.completed_rank), s.remaining_seconds));
        for (index, mask) in s.claimed_masks.iter().enumerate() {
            text.push_str(&format!("{} rewards:", rank_name(index as u8 + 1)));
            for (bit, label) in ["Present", "Supplies", "Weapon", "Trophy"]
                .iter()
                .enumerate()
            {
                let state = if mask & (1 << bit) != 0 {
                    "claimed"
                } else {
                    "unclaimed"
                };
                text.push_str(&format!(" {label}: {state};"));
            }
            text.push_str("\r\n");
        }
    }
    text.push_str("Tibiadrome\r\n");
    if let Some(reply) = panel.drome {
        let disposition = match reply.disposition {
            1 => "Applied",
            2 => "Action unavailable in the current state",
            3 => "Premium and level 50 required",
            4 => "Re-entry cooldown active",
            5 => "Arena busy; retry",
            6 => "Arena content unavailable",
            7 => "No earned potion or insufficient drome points",
            8 => "Inventory full",
            _ => "Unknown result",
        };
        let phase = if reply.phase == 0 { "Idle" } else { "Active" };
        let rank = if reply.rank == 0 {
            "Unranked".to_owned()
        } else {
            reply.rank.to_string()
        };
        text.push_str(&format!(
            "{disposition}\r\n{phase}: wave {}. Rotation level: {}. Starting level: {}\r\nDrome points: {}. Rank: {rank}. Unclaimed potions: {}\r\nServer time remaining: {} seconds. Re-entry cooldown: {} seconds\r\n",
            reply.wave, reply.level, reply.start, reply.points, reply.potions,
            reply.remaining, reply.cooldown,
        ));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        let remaining = u64::from(reply.rotation_ends).saturating_sub(now);
        text.push_str(&format!(
            "Next rotation in approximately {}d {}h {}m (local clock estimate)\r\nRefresh Drome status for current server values\r\n",
            remaining / 86400, remaining / 3600 % 24, remaining / 60 % 60,
        ));
    } else {
        text.push_str("Refresh Drome status to load progress\r\n");
    }
    if panel.pending.is_some() {
        text.push_str("Waiting for the server\r\n");
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}

fn rank_name(rank: u8) -> &'static str {
    match rank {
        1 => "Greenhorn",
        2 => "Scrapper",
        3 => "Warlord",
        _ => "None",
    }
}
