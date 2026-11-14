//! Atomic presentation controls over the retained encounter and difficulty owners.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::{boss_difficulty, boss_encounter as boss};
#[derive(Clone, Debug, Default)]
pub(crate) struct BosstiarySnapshot {
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    pub panel: boss::BossEncounterPanel,
    pub difficulty: boss_difficulty::BossDifficultyPanel,
}
impl PartialEq for BosstiarySnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
            && self.panel == other.panel
            && self.difficulty == other.difficulty
    }
}
impl Eq for BosstiarySnapshot {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BosstiaryControl {
    Toggle(bool),
    Close(bool),
    Request(boss::Intent),
    Search(String),
    Category,
    Browse(bool),
    Select(String),
    Chain(boss::EndgameChain),
    Slot,
    SlotBoss(bool),
    Edit(usize, String),
    Difficulty(boss_difficulty::Intent, u16),
}
impl BosstiaryControl {
    pub(crate) fn needs_confirmation(&self) -> bool {
        matches!(
            self,
            Self::Request(
                boss::Intent::Enter
                    | boss::Intent::Defeated
                    | boss::Intent::Timeout
                    | boss::Intent::SlotEquipSelected
                    | boss::Intent::SlotRemoveSelected
            ) | Self::Difficulty(
                boss_difficulty::Intent::Start | boss_difficulty::Intent::Cancel,
                _
            )
        )
    }
}
impl PlayLink {
    pub(crate) fn bosstiary_snapshot(&self) -> BosstiarySnapshot {
        let mailbox = lock(&self.mailbox);
        BosstiarySnapshot {
            session: std::sync::Arc::downgrade(&self.mailbox),
            panel: mailbox.boss_encounter.clone(),
            difficulty: mailbox.boss_difficulty.clone(),
        }
    }
    /// Compare, validate, prepare and enqueue while holding the same owner lock.
    pub(crate) fn bosstiary_dispatch_bound(
        &self,
        expected: &BosstiarySnapshot,
        control: BosstiaryControl,
        confirmed: bool,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || m.boss_encounter_shutdown
            || m.server_modal.is_active()
            || m.world_input_blocked()
            || self.commands.is_closed()
            || m.boss_encounter != expected.panel
            || m.boss_difficulty != expected.difficulty
            || control.needs_confirmation() && !confirmed
        {
            return false;
        }
        if let BosstiaryControl::Toggle(cooldowns) | BosstiaryControl::Close(cooldowns) = control {
            let close = matches!(control, BosstiaryControl::Close(_));
            let open = if cooldowns {
                &mut m.boss_encounter.cooldowns_open
            } else {
                &mut m.boss_encounter.open
            };
            *open = !close && !*open;
            return true;
        }
        if !expected.panel.open && !expected.panel.cooldowns_open
            || !expected.panel.selected
            || expected.panel.pending
            || expected.difficulty.pending
        {
            return false;
        }
        let mut panel = expected.panel.clone();
        let mut difficulty = expected.difficulty.clone();
        let command = match control {
            BosstiaryControl::Request(intent) => {
                // Only the detailed, server-eligible slot controls are exposed here.
                if matches!(
                    intent,
                    boss::Intent::SlotsQuery | boss::Intent::SlotEquip | boss::Intent::SlotRemove
                ) {
                    return false;
                }
                let Some(command) = boss::request(&mut panel, intent) else {
                    return false;
                };
                Some(command)
            }
            BosstiaryControl::Search(search) => {
                if search.len() > 48 || search.contains('\0') {
                    return false;
                }
                panel.browser_search.clone_from(&search);
                panel.boss_search = search;
                let Some(command) = boss::request(&mut panel, boss::Intent::Catalogue) else {
                    return false;
                };
                Some(command)
            }
            BosstiaryControl::Category => {
                panel.browser_category = panel.browser_category.next();
                let Some(command) = boss::request(&mut panel, boss::Intent::Catalogue) else {
                    return false;
                };
                Some(command)
            }
            BosstiaryControl::Browse(previous) => {
                let search = panel.browser_search.clone();
                let category = panel.browser_category;
                super::super::input::browse(&mut panel, &search, category, previous);
                None
            }
            BosstiaryControl::Select(key) => {
                if !boss::boss_records(&panel).contains(&key.as_str()) {
                    return false;
                }
                panel.fields[0] = key;
                None
            }
            BosstiaryControl::Chain(chain) => {
                boss::select_next_boss(&mut panel, chain);
                None
            }
            BosstiaryControl::Slot => {
                panel.select_slot();
                None
            }
            BosstiaryControl::SlotBoss(previous) => {
                panel.select_slot_boss(previous);
                None
            }
            BosstiaryControl::Edit(index, value) => {
                if index > 2
                    || index == 0 && panel.catalogue.is_some()
                    || value.len() > 512
                    || value.contains('\0')
                {
                    return false;
                }
                super::super::input::edit(&mut panel, index, value);
                None
            }
            BosstiaryControl::Difficulty(intent, level) => {
                let Some(oteryn_client_runtime::boss_difficulty::Reply::Window {
                    boss_key,
                    boss_creature_key,
                    ..
                }) = &difficulty.reply
                else {
                    return false;
                };
                if panel.fields[0] != *boss_key && panel.fields[0] != *boss_creature_key {
                    return false;
                }
                difficulty.difficulty = level;
                let Some(command) = boss_difficulty::request(&mut difficulty, intent) else {
                    return false;
                };
                Some(command)
            }
            BosstiaryControl::Toggle(_) | BosstiaryControl::Close(_) => return false,
        };
        if let Some(command) = command
            && self
                .commands
                .send(crate::combat_input::PlayCommand::Feature(command))
                .is_err()
        {
            return false;
        }
        m.boss_encounter = panel;
        m.boss_difficulty = difficulty;
        true
    }
}

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bosstiary_test_initialize(&self, selected: bool) {
        self.boss_encounter_initialize(selected);
    }
    pub(crate) fn bosstiary_test_reply(
        &self,
        reply: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        self.boss_encounter_result(reply);
    }
    pub(crate) fn bosstiary_test_end(&self) {
        lock(&self.0).boss_encounter_end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{combat_input::PlayCommand, play::play_channel};
    #[test]
    fn bosstiary_owner_rejects_identical_data_from_another_session() {
        let (first, mut first_commands, first_events) = play_channel();
        let (second, mut second_commands, second_events) = play_channel();
        first_events.bosstiary_test_initialize(true);
        second_events.bosstiary_test_initialize(true);
        let expected = first.bosstiary_snapshot();
        assert_eq!(expected.panel, second.bosstiary_snapshot().panel);
        assert!(!second.bosstiary_dispatch_bound(
            &expected,
            BosstiaryControl::Toggle(false),
            false
        ));
        assert!(!second.bosstiary_snapshot().panel.open);
        assert!(first_commands.try_recv().is_err());
        assert!(second_commands.try_recv().is_err());
    }
    #[test]
    fn bosstiary_owner_routes_default_requests_without_changing_confirmed_data()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_client_runtime::boss_encounter::{
            RewardChest, Snapshot,
            bosstiary_slots::{BossSlotReply, SlotDetails},
            catalogue::{Entry, Page, Query},
        };
        for intent in [
            boss::Intent::Refresh,
            boss::Intent::Next,
            boss::Intent::Catalogue,
            boss::Intent::CatalogueNext,
            boss::Intent::Enter,
            boss::Intent::Defeated,
            boss::Intent::Timeout,
            boss::Intent::HazardQuery,
            boss::Intent::HazardSelect,
            boss::Intent::HazardLower,
            boss::Intent::HazardRaise,
            boss::Intent::SlotsDetailed,
            boss::Intent::SlotEquipSelected,
            boss::Intent::SlotRemoveSelected,
            boss::Intent::Rewards,
            boss::Intent::RewardsNext,
        ] {
            let (link, mut commands, events) = play_channel();
            events.bosstiary_test_initialize(true);
            {
                let mut m = lock(&link.mailbox);
                let p = &mut m.boss_encounter;
                p.open = true;
                p.fields = ["oteryn:creature.alpha".into(), "1".into(), "2".into()];
                p.hazard = Some((12, 2));
                p.snapshot = Some(Snapshot {
                    cooldown_next: Some("oteryn:creature.alpha".into()),
                    ..Default::default()
                });
                p.catalogue_query = Some(Query {
                    after: 0,
                    category: 0,
                    search: String::new(),
                });
                p.catalogue = Some(Page {
                    next: Some(64),
                    points: 77,
                    entries: vec![Entry {
                        key: p.fields[0].clone(),
                        name: "Server Alpha".into(),
                        category: 1,
                        kills: 25,
                        thresholds: [25, 100, 300],
                        points: [5, 20, 50],
                        look: 21,
                        object: false,
                    }],
                });
                p.slots = Some(BossSlotReply {
                    details: Some(SlotDetails {
                        base_bonus: 25,
                        bonuses: [25, 0],
                        boosted: String::new(),
                        now: 100,
                    }),
                    status: 0,
                    unlocked: 3,
                    bonus: 25,
                    points: 77,
                    remove_fee: 123,
                    ready_at_ms: 0,
                    bosses: [
                        if intent == boss::Intent::SlotRemoveSelected {
                            p.fields[0].clone()
                        } else {
                            String::new()
                        },
                        String::new(),
                    ],
                });
                p.rewards = Some(RewardChest {
                    now: 100,
                    entries: (1..=64)
                        .map(
                            |id| oteryn_client_runtime::boss_encounter::BossRewardChestEntry {
                                death: [id; 16],
                                boss_key: p.fields[0].clone(),
                                minted_at_unix_ms: 0,
                                expires_at_unix_ms: 1000,
                                item_count: 1,
                            },
                        )
                        .collect(),
                });
            }
            let expected = link.bosstiary_snapshot();
            let mut prepared = expected.panel.clone();
            let command = boss::request(&mut prepared, intent)
                .ok_or("existing owner rejected test intent")?;
            let control = BosstiaryControl::Request(intent);
            if control.needs_confirmation() {
                assert!(!link.bosstiary_dispatch_bound(&expected, control.clone(), false));
                assert_eq!(expected, link.bosstiary_snapshot());
            }
            assert!(
                link.bosstiary_dispatch_bound(&expected, control.clone(), true),
                "{intent:?}"
            );
            let PlayCommand::Feature(sent) = commands.try_recv()? else {
                return Err("feature command".into());
            };
            assert_eq!(
                (sent.capability, sent.kind, sent.payload),
                (command.capability, command.kind, command.payload)
            );
            assert_eq!(link.bosstiary_snapshot().panel, prepared);
            assert_eq!(link.bosstiary_snapshot().panel.slots, expected.panel.slots);
            assert!(!link.bosstiary_dispatch_bound(&expected, control, true));
            assert!(commands.try_recv().is_err());
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn bosstiary_difficulty_requests_use_selected_server_context()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{combat_input::PlayCommand, play::play_channel};
    for intent in [
        boss_difficulty::Intent::Preview,
        boss_difficulty::Intent::Start,
        boss_difficulty::Intent::Cancel,
    ] {
        let (link, mut commands, events) = play_channel();
        events.bosstiary_test_initialize(true);
        {
            let mut m = lock(&link.mailbox);
            m.boss_encounter.open = true;
            m.boss_encounter.fields[0] = "oteryn:creature.maior_domus".into();
            m.boss_difficulty = boss_difficulty::BossDifficultyPanel {
                selected: true,
                context: Some([7; 16]),
                boss_key: "oteryn:boss.maior_domus".into(),
                reply: Some(oteryn_client_runtime::boss_difficulty::Reply::Window {
                    context: [7; 16],
                    boss_key: "oteryn:boss.maior_domus".into(),
                    boss_creature_key: "oteryn:creature.maior_domus".into(),
                    content_revision: "server-revision".into(),
                    selected_difficulty: 1,
                    personal_highest: 2,
                    unlocked_max: 3,
                    maximum_difficulty: 15,
                }),
                ..Default::default()
            };
        }
        let expected = link.bosstiary_snapshot();
        let mut prepared = expected.difficulty.clone();
        prepared.difficulty = 2;
        let command =
            boss_difficulty::request(&mut prepared, intent).ok_or("difficulty request")?;
        let control = BosstiaryControl::Difficulty(intent, 2);
        if control.needs_confirmation() {
            assert!(!link.bosstiary_dispatch_bound(&expected, control.clone(), false));
            assert_eq!(link.bosstiary_snapshot(), expected);
        }
        assert!(link.bosstiary_dispatch_bound(&expected, control, true));
        let PlayCommand::Feature(sent) = commands.try_recv()? else {
            return Err("feature command".into());
        };
        assert_eq!(
            (sent.capability, sent.kind, sent.payload),
            (command.capability, command.kind, command.payload)
        );
        assert_eq!(link.bosstiary_snapshot().difficulty, prepared);
        assert_eq!(link.bosstiary_snapshot().panel, expected.panel);
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
