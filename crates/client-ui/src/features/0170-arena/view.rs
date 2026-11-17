use super::ArenaPanel;
use oteryn_client_runtime::arena::ArenaDisposition;

pub fn panel_text(panel: &ArenaPanel) -> String {
    let mut text = String::from(tr!("Svargrond Arena\r\n"));
    for challenge in &panel.challenges {
        text.push_str(&lfmt!(
            "{:?}: {} gold, {} pits, {} seconds per pit\r\n",
            challenge.rank,
            challenge.fee,
            challenge.rooms,
            challenge.room_seconds
        ));
    }
    if let Some(reply) = panel.result {
        let s = reply.state;
        let disposition = match reply.disposition {
            ArenaDisposition::Applied => tr!("Applied"),
            ArenaDisposition::WrongState => tr!("Action unavailable in the current state"),
            ArenaDisposition::InsufficientGold => tr!("Insufficient gold"),
            ArenaDisposition::AlreadyClaimed => tr!("Reward already claimed"),
            ArenaDisposition::InventoryFull => tr!("Inventory full"),
            ArenaDisposition::TimedOut => tr!("Time expired"),
            ArenaDisposition::Busy => tr!("Arena busy; retry"),
            ArenaDisposition::InvalidReward => tr!("Invalid reward"),
            ArenaDisposition::AllChallengesCompleted => tr!("All challenges completed"),
            ArenaDisposition::ContentUnavailable => tr!("Arena content unavailable"),
        };
        let phase = match s.phase {
            1 => tr!("Fighting"),
            2 => tr!("Next pit ready"),
            3 => tr!("Reward room"),
            _ => tr!("Idle"),
        };
        text.push_str(&lfmt!("{disposition}\r\n{phase}: {}, pit {}. Completed: {}\r\nServer time remaining: {} seconds (Refresh for current time)\r\n",
            rank_name(s.active_rank), s.pit, rank_name(s.completed_rank), s.remaining_seconds,disposition=disposition,phase=phase));
        for (index, mask) in s.claimed_masks.iter().enumerate() {
            text.push_str(&format!("{} rewards:", rank_name(index as u8 + 1)));
            for (bit, label) in [
                tr!("Present"),
                tr!("Supplies"),
                tr!("Weapon"),
                tr!("Trophy"),
            ]
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
    text.push_str(tr!("Tibiadrome\r\n"));
    if let Some(reply) = panel.drome {
        let disposition = match reply.disposition {
            1 => tr!("Applied"),
            2 => tr!("Action unavailable in the current state"),
            3 => tr!("Premium and level 50 required"),
            4 => tr!("Re-entry cooldown active"),
            5 => tr!("Arena busy; retry"),
            6 => tr!("Arena content unavailable"),
            7 => tr!("No earned potion or insufficient drome points"),
            8 => tr!("Inventory full"),
            _ => tr!("Unknown result"),
        };
        let phase = match reply.phase {
            1 => tr!("Preparation"),
            2 => tr!("Fighting"),
            _ => tr!("Idle"),
        };
        let rank = if reply.rank == 0 {
            tr!("Unranked").to_owned()
        } else {
            reply.rank.to_string()
        };
        text.push_str(&lfmt!(
            "{disposition}\r\n{phase}: wave {}. Rotation level: {}. Starting level: {}\r\nDrome points: {}. Rank: {rank}. Unclaimed potions: {}\r\nServer time remaining: {} seconds. Re-entry cooldown: {} seconds\r\n",
            reply.wave, reply.level, reply.start, reply.points, reply.potions,
            reply.remaining, reply.cooldown,
        disposition=disposition,phase=phase,rank=rank));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        let remaining = u64::from(reply.rotation_ends).saturating_sub(now);
        text.push_str(&lfmt!(
            "Next rotation in approximately {}d {}h {}m (local clock estimate)\r\nRefresh Drome status for current server values\r\n",
            remaining / 86400, remaining / 3600 % 24, remaining / 60 % 60,
        ));
    } else {
        text.push_str(tr!("Refresh Drome status to load progress\r\n"));
    }
    if panel.pending.is_some() {
        text.push_str(tr!("Waiting for the server\r\n"));
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
        1 => tr!("Greenhorn"),
        2 => tr!("Scrapper"),
        3 => tr!("Warlord"),
        _ => tr!("None"),
    }
}
