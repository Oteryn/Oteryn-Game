//! CL6 observations from existing server owners.
pub use oteryn_client_runtime::mechanics_status::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::MechanicsStatusPanel;
pub use view::panel_text;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cl6_all_owner_projections_reach_native_text_and_refusals_retain_confirmation()
    -> Result<(), &'static str> {
        let mut panel = MechanicsStatusPanel {
            selected: true,
            ..Default::default()
        };
        let mut value = MechanicsStatus {
            observed_unix_ms: 100,
            dust: 73,
            dust_limit: 100,
            deaths: vec![DeathConsumption {
                at_unix_ms: 50,
                consumed: vec![
                    "oteryn:blessing.spiritual_shielding".into(),
                    "oteryn:blessing.twist_of_fate".into(),
                ],
            }],
            timers: vec![
                EffectTimer {
                    key: "oteryn:item.tibia.i7440".into(),
                    remaining_ms: 60_001,
                    online_only: false,
                },
                EffectTimer {
                    key: "wealth_duplex".into(),
                    remaining_ms: 1234,
                    online_only: true,
                },
            ],
            members: vec![
                MemberEffects {
                    bonuses: [0; 5],
                    party_spell_remaining_ms: [120001; 5],
                    character: [1; 16],
                    name: "Friend".into(),
                    conditions: Some(oteryn_client_runtime::condition_status::ConditionStatus {
                        conditions: vec![
                            oteryn_client_runtime::condition_status::ActiveCondition {
                                icon: 13,
                                sub_id: 0,
                                remaining_ms: 4001,
                            },
                        ],
                    }),
                },
                MemberEffects {
                    bonuses: [0; 5],
                    party_spell_remaining_ms: [0; 5],
                    character: [2; 16],
                    name: "Away".into(),
                    conditions: None,
                },
            ],
        };
        value.members[0].bonuses = [6, 12, 6, 12, 50];
        assert_eq!(request(&mut panel), Some(vec![]));
        assert!(panel.finish(true, &value.encode().ok_or("encode")?));
        let text = panel_text(&panel);
        for expected in [
            "Dust: 73 / 100",
            "Spiritual Shielding",
            "Twist of Fate: consumed",
            "Mastermind Potion: 61s",
            "Wealth Duplex: 2s (online time)",
            "Friend",
            "Haste: 5s",
            "Damage reduction: 6%",
            "Auto attack: 12%",
            "Spell damage: 6%",
            "Spell healing: 12%",
            "Mantra: +50",
            "Away",
            "not observed in this Channel",
            "Train Party: 121s",
            "Protect Party: 121s",
            "Enchant Party: 121s",
            "Heal Party: 121s",
            "Enlighten Party: 121s",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(false, &[]));
        assert_eq!(panel.snapshot, Some(value.clone()));
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(true, &[255]));
        assert_eq!(panel.snapshot, Some(value));
        Ok(())
    }
}
