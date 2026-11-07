use super::MechanicsStatusPanel;
fn blessing_name(key: &str) -> String {
    key.strip_prefix("oteryn:blessing.")
        .unwrap_or(key)
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
fn timer_name(key: &str) -> &str {
    match key {
        "oteryn:item.tibia.i7439" => "Berserk Potion",
        "oteryn:item.tibia.i7440" => "Mastermind Potion",
        "oteryn:item.tibia.i7443" => "Bullseye Potion",
        "oteryn:item.tibia.i49271" => "Transcendence Potion",
        "oteryn:item.tibia.i35563" => "Magic Shield Potion",
        "wealth_duplex" => "Wealth Duplex",
        _ => key,
    }
}
pub fn panel_text(panel: &MechanicsStatusPanel) -> String {
    if !panel.selected {
        return "Character effects unavailable in this session".into();
    }
    let mut text = panel.error.map_or_else(String::new, |error| {
        format!("{error}\r\nShowing last confirmed observation.\r\n")
    });
    let Some(status) = &panel.snapshot else {
        text.push_str("Loading character effects…");
        return text;
    };
    text.push_str(&format!(
        "Confirmed server observation\r\nDust: {} / {}\r\n\r\nBlessing consumption (latest deaths)\r\n",
        status.dust, status.dust_limit
    ));
    if status.deaths.is_empty() {
        text.push_str("No recorded deaths\r\n");
    }
    for (index, death) in status.deaths.iter().enumerate() {
        text.push_str(&format!(
            "Death {} ({}s before observation)\r\n",
            index + 1,
            (status.observed_unix_ms - death.at_unix_ms) / 1000
        ));
        let mut any_regular = false;
        let mut twist = false;
        for key in &death.consumed {
            if key == "oteryn:blessing.twist_of_fate" {
                twist = true;
            } else {
                text.push_str(&format!("Consumed: {}\r\n", blessing_name(key)));
                any_regular = true;
            }
        }
        if !any_regular {
            text.push_str("No regular blessings consumed\r\n");
        }
        text.push_str(if twist {
            "Twist of Fate: consumed\r\n"
        } else {
            "Twist of Fate: not consumed\r\n"
        });
    }
    text.push_str("\r\nActive potion and concoction timers\r\n");
    if status.timers.is_empty() {
        text.push_str("No active timed effects\r\n");
    }
    for timer in &status.timers {
        text.push_str(&format!(
            "{}: {}s{}\r\n",
            timer_name(&timer.key),
            timer.remaining_ms.div_ceil(1000),
            if timer.online_only {
                " (online time)"
            } else {
                ""
            }
        ));
    }
    text.push_str("Kooldown Aid and Antidote are instant; no active-effect timer.\r\n\r\nParty member effects\r\n");
    if status.members.is_empty() {
        text.push_str("No party\r\n");
    }
    for member in &status.members {
        text.push_str(&format!("{}\r\n", member.name));
        if let Some(conditions) = &member.conditions {
            text.push_str(&crate::condition_status::status_text(conditions));
            text.push_str("\r\n");
            for (label, value) in [
                "Damage reduction",
                "Auto attack",
                "Spell damage",
                "Spell healing",
            ]
            .into_iter()
            .zip(member.bonuses[..4].iter())
            {
                text.push_str(&format!("{label}: {value}%\r\n"));
            }
            text.push_str(&format!("Mantra: +{}\r\n", member.bonuses[4]));
            for (label, remaining) in [
                "Train Party",
                "Protect Party",
                "Enchant Party",
                "Heal Party",
                "Enlighten Party",
            ]
            .into_iter()
            .zip(member.party_spell_remaining_ms)
            {
                if remaining > 0 {
                    text.push_str(&format!("{label}: {}s\r\n", remaining.div_ceil(1000)));
                }
            }
        } else {
            text.push_str("Effects not observed in this Channel\r\n");
        }
    }
    text
}
