use super::DailyBoostPanel;
pub fn panel_text(p: &DailyBoostPanel) -> String {
    if !p.selected {
        return tr!("Unavailable").into();
    }
    if let Some(f) = p.feedback {
        return f.into();
    }
    if p.pending {
        return tr!("Loading daily boost...").into();
    }
    p.boost.as_ref().map_or_else(
        || tr!("Ready").into(),
        |v| {
            let mut text = lfmt!("Daily spotlight\nReset epoch: {}\n", v.reset_epoch);
            for card in spotlight_cards(p) {
                text.push_str(&format!("{}: {}\n{}\n", card.kind, card.name, card.key));
            }
            text
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loading_hides_previous_daily_selection() {
        let panel = DailyBoostPanel {
            selected: true,
            pending: true,
            boost: Some(oteryn_client_runtime::cui3::daily_boost::DailyBoostView {
                reset_epoch: 9,
                creature_key: "rat".into(),
                boss_key: "boss".into(),
            }),
            ..Default::default()
        };
        assert_eq!(panel_text(&panel), "Loading daily boost...");
    }
}

#[cfg(test)]
mod spotlight_tests {
    use super::*;
    #[test]
    fn spotlight_projects_both_server_choices_and_reset_time() {
        let mut panel = DailyBoostPanel {
            selected: true,
            ..Default::default()
        };
        let mut payload = 12345u64.to_be_bytes().to_vec();
        for key in ["oteryn:creature.cave_rat", "oteryn:boss.dragon_lord"] {
            payload.push(key.len() as u8);
            payload.extend_from_slice(key.as_bytes());
        }
        crate::daily_boost::apply_result(
            &mut panel,
            &oteryn_client_runtime::raw_feature::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
                payload,
            },
        );
        let cards = spotlight_cards(&panel);
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].name, "Cave Rat");
        assert_eq!(cards[1].name, "Dragon Lord");
        assert_eq!(cards[1].reset_epoch, 12345);
        assert!(panel_text(&panel).contains("Daily spotlight"));
        panel.pending = true;
        assert!(spotlight_cards(&panel).is_empty());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotlightCard {
    pub kind: &'static str,
    pub key: String,
    pub name: String,
    pub reset_epoch: u64,
}
/// Both cards share the reset boundary from the same validated server observation.
pub fn spotlight_cards(panel: &DailyBoostPanel) -> Vec<SpotlightCard> {
    if !panel.selected || panel.pending || panel.feedback.is_some() {
        return Vec::new();
    }
    let Some(boost) = &panel.boost else {
        return Vec::new();
    };
    [
        (tr!("Boosted creature"), &boost.creature_key),
        (tr!("Boosted boss"), &boost.boss_key),
    ]
    .into_iter()
    .map(|(kind, key)| {
        let name = key
            .rsplit(['.', ':'])
            .next()
            .unwrap_or(key)
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().chain(chars).collect::<String>()
                })
            })
            .collect::<Vec<_>>()
            .join(" ");
        SpotlightCard {
            kind,
            key: key.clone(),
            name,
            reset_epoch: boost.reset_epoch,
        }
    })
    .collect()
}

/// Separate "Daily Reward XP boost" (source 4) and "Store XP boost" (source 3) lines, each with
/// its own remaining hours; a source without an active boost has no line. The potion durations
/// wire carries no paused state.
pub fn xp_boost_lines(
    active: &[oteryn_client_runtime::potion_durations::PotionDuration],
    elapsed_ms: u64,
) -> String {
    let mut text = String::new();
    for source in [4u8, 3u8] {
        let Some(entry) = active.iter().find(|e| e.source == source) else {
            continue;
        };
        let remaining = entry.remaining_ms.saturating_sub(elapsed_ms);
        if remaining == 0 {
            continue;
        }
        let name = if source == 4 {
            tr!("Daily Reward XP boost")
        } else {
            tr!("Store XP boost")
        };
        let state = lfmt!("{} h remaining", remaining.div_ceil(3_600_000));
        text.push_str(&format!("- {name}: +{}% ({state})\n", entry.percent));
    }
    text
}

#[cfg(test)]
mod xp_boost_tests {
    use super::*;
    use oteryn_client_runtime::potion_durations::PotionDuration;
    fn boost(source: u8, percent: u16, remaining_ms: u64) -> PotionDuration {
        PotionDuration {
            source,
            item_id: 0,
            percent,
            remaining_ms,
        }
    }
    #[test]
    fn xp_boosts_show_two_separate_lines_with_own_time_and_no_paused_state() {
        let active = [boost(3, 50, 7_200_000), boost(4, 25, 1_800_000)];
        let text = xp_boost_lines(&active, 0);
        assert_eq!(
            text,
            "- Daily Reward XP boost: +25% (1 h remaining)\n- Store XP boost: +50% (2 h remaining)\n"
        );
        assert!(!text.contains("paused"));
        assert_eq!(
            xp_boost_lines(&active[..1], 0),
            "- Store XP boost: +50% (2 h remaining)\n"
        );
        assert_eq!(
            xp_boost_lines(&active[1..], 1_000_000),
            "- Daily Reward XP boost: +25% (1 h remaining)\n"
        );
        assert_eq!(xp_boost_lines(&active, 7_200_000), "");
    }
}
