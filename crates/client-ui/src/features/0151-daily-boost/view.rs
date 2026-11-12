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
