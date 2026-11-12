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
        "oteryn:item.tibia.i7439" => tr!("Berserk Potion"),
        "oteryn:item.tibia.i7440" => tr!("Mastermind Potion"),
        "oteryn:item.tibia.i7443" => tr!("Bullseye Potion"),
        "oteryn:item.tibia.i49271" => tr!("Transcendence Potion"),
        "oteryn:item.tibia.i35563" => tr!("Magic Shield Potion"),
        "wealth_duplex" => tr!("Wealth Duplex"),
        _ => key,
    }
}
pub fn panel_text(panel: &MechanicsStatusPanel) -> String {
    if !panel.selected {
        return tr!("Character effects unavailable in this session").into();
    }
    let mut text = panel.error.map_or_else(String::new, |error| {
        lfmt!(
            "{error}\r\nShowing last confirmed observation.\r\n",
            error = error
        )
    });
    let Some(status) = &panel.snapshot else {
        text.push_str(tr!("Loading character effects…"));
        return text;
    };
    text.push_str(&lfmt!(
        "Confirmed server observation\r\nDust: {} / {}\r\n\r\nBlessing consumption (latest deaths)\r\n",
        status.dust, status.dust_limit
    ));
    if status.deaths.is_empty() {
        text.push_str(tr!("No recorded deaths\r\n"));
    }
    for (index, death) in status.deaths.iter().enumerate() {
        text.push_str(&lfmt!(
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
                text.push_str(&lfmt!("Consumed: {}\r\n", blessing_name(key)));
                any_regular = true;
            }
        }
        if !any_regular {
            text.push_str(tr!("No regular blessings consumed\r\n"));
        }
        text.push_str(if twist {
            tr!("Twist of Fate: consumed\r\n")
        } else {
            tr!("Twist of Fate: not consumed\r\n")
        });
    }
    text.push_str(tr!("\r\nActive potion and concoction timers\r\n"));
    if status.timers.is_empty() {
        text.push_str(tr!("No active timed effects\r\n"));
    }
    for timer in &status.timers {
        text.push_str(&lfmt!(
            "{}: {}s{}\r\n",
            timer_name(&timer.key),
            timer.remaining_ms.div_ceil(1000),
            if timer.online_only {
                tr!(" (online time)")
            } else {
                ""
            }
        ));
    }
    text.push_str(tr!("Kooldown Aid and Antidote are instant; no active-effect timer.\r\n\r\nParty member effects\r\n"));
    if status.members.is_empty() {
        text.push_str(tr!("No party\r\n"));
    }
    for member in &status.members {
        text.push_str(&format!("{}\r\n", member.name));
        if let Some(conditions) = &member.conditions {
            text.push_str(&crate::condition_status::status_text(conditions));
            text.push_str("\r\n");
            for (label, value) in [
                tr!("Damage reduction"),
                tr!("Auto attack"),
                tr!("Spell damage"),
                tr!("Spell healing"),
            ]
            .into_iter()
            .zip(member.bonuses[..4].iter())
            {
                text.push_str(&format!("{label}: {value}%\r\n"));
            }
            text.push_str(&lfmt!("Mantra: +{}\r\n", member.bonuses[4]));
            for (label, remaining) in [
                tr!("Train Party"),
                tr!("Protect Party"),
                tr!("Enchant Party"),
                tr!("Heal Party"),
                tr!("Enlighten Party"),
            ]
            .into_iter()
            .zip(member.party_spell_remaining_ms)
            {
                if remaining > 0 {
                    text.push_str(&lfmt!(
                        "{label}: {}s\r\n",
                        remaining.div_ceil(1000),
                        label = label
                    ));
                }
            }
        } else {
            text.push_str(tr!("Effects not observed in this Channel\r\n"));
        }
    }
    text
}
