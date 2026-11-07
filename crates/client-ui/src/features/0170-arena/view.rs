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
        let phase = match reply.phase {
            1 => "Preparation",
            2 => "Fighting",
            _ => "Idle",
        };
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
    if panel.hazards.selected != [0, 0] {
        text.push_str("Selected modifiers: ");
        for (index, id) in panel.hazards.selected.iter().enumerate() {
            if index != 0 {
                text.push_str(", ");
            }
            text.push_str(match id {
                1 => "Somersault",
                2 => "Going Down With Me",
                3 => "Exploding Corpses",
                4 => "That Escalated Quickly",
                5 => "The Floor is Lava",
                6 => "Beam me Up!",
                7 => "Tanked Up",
                8 => "Sown Sorrow",
                9 => "Bad Roots",
                _ => "Unknown modifier",
            });
        }
        text.push_str("\r\n");
    }
    for hazard in &panel.hazards.entries {
        let label = match hazard.kind {
            oteryn_client_runtime::arena::DromeHazardKind::Lava => {
                "The Floor is Lava: red tile warning"
            }
            oteryn_client_runtime::arena::DromeHazardKind::Beam => "Beam me Up: blue tile warning",
            oteryn_client_runtime::arena::DromeHazardKind::Tanked => {
                "Tanked Up: purple tile warning"
            }
            oteryn_client_runtime::arena::DromeHazardKind::Sorrow => "Sown Sorrow: green seed",
            oteryn_client_runtime::arena::DromeHazardKind::Roots => "Bad Roots: brown seed",
        };
        text.push_str(label);
        text.push_str("\r\n");
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
