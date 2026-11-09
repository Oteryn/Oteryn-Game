use crate::play::PlayLink;
use oteryn_client_ui::boss_encounter::Intent;
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Boss encounter / Bosstiary",
            labels: &[
                "Boss key",
                "Slot (1-2)",
                "Hazard level (1-12)",
                "Boss search",
            ],
            buttons: &[
                "Refresh",
                "Next page",
                "Enter",
                "Return after defeat",
                "Return after timeout",
                "Hazard query",
                "Set hazard",
                "Slots query",
                "Equip slot",
                "Remove slot",
                "Rewards",
                "Next rewards",
                "Lower hazard",
                "Raise hazard",
                "Previous boss",
                "Next boss",
                "Boss category",
                "Soul War boss",
                "Rotten Blood boss",
                "Apply search",
                "Next unlocked slot",
            ],
            snapshot: |mailbox| {
                let panel = &mailbox.boss_encounter;
                (
                    panel.open,
                    panel.pending,
                    [panel.fields.to_vec(), vec![panel.browser_search.clone()]].concat(),
                    oteryn_client_ui::boss_encounter::panel_text(panel)
                        .replace("\r\n", "\n")
                        .replace('\n', "\r\n"),
                )
            },
            set_open: |mailbox, value| mailbox.boss_encounter.open = value,
            submit: |mailbox, commands, action, values| {
                if mailbox.boss_encounter.pending {
                    return;
                }
                let search = values.get(3).cloned().unwrap_or_default();
                for (index, value) in values.into_iter().take(3).enumerate() {
                    super::super::input::edit(&mut mailbox.boss_encounter, index, value);
                }
                if let Some(chain) = match action {
                    17 => Some(oteryn_client_ui::boss_encounter::EndgameChain::SoulWar),
                    18 => Some(oteryn_client_ui::boss_encounter::EndgameChain::RottenBlood),
                    _ => None,
                } {
                    oteryn_client_ui::boss_encounter::select_next_boss(
                        &mut mailbox.boss_encounter,
                        chain,
                    );
                    return;
                }
                if action >= 14 {
                    if matches!(action, 14..=16 | 19) {
                        let category = mailbox.boss_encounter.browser_category;
                        let category = if action == 16 {
                            category.next()
                        } else {
                            category
                        };
                        super::super::input::browse(
                            &mut mailbox.boss_encounter,
                            &search,
                            category,
                            action == 14,
                        );
                    }
                    if action == 20 {
                        mailbox.boss_encounter.select_slot();
                    }
                    return;
                }
                let intent = match action {
                    0 => Intent::Refresh,
                    1 => Intent::Next,
                    2 => Intent::Enter,
                    3 => Intent::Defeated,
                    4 => Intent::Timeout,
                    5 => Intent::HazardQuery,
                    6 => Intent::HazardSelect,
                    7 => Intent::SlotsQuery,
                    8 => Intent::SlotEquip,
                    9 => Intent::SlotRemove,
                    10 => Intent::Rewards,
                    11 => Intent::RewardsNext,
                    12 => Intent::HazardLower,
                    13 => Intent::HazardRaise,
                    _ => return,
                };
                super::super::input::enqueue(mailbox, commands, intent);
            },
        },
    );
}

/// Independent window; closing the main Bosstiary panel leaves this tracker open.
pub(in crate::play) fn cooldown_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Boss cooldown tracker",
            labels: &[],
            buttons: &["Refresh", "Next page"],
            snapshot: |mailbox| {
                let p = &mailbox.boss_encounter;
                let elapsed = p.cooldown_received_at.map_or(0, |at| {
                    u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX)
                });
                (
                    p.cooldowns_open,
                    p.pending,
                    vec![],
                    oteryn_client_ui::boss_encounter::cooldown_text(p, elapsed),
                )
            },
            set_open: |mailbox, value| mailbox.boss_encounter.cooldowns_open = value,
            submit: |mailbox, commands, action, _| {
                let intent = match action {
                    0 => Intent::Refresh,
                    1 => Intent::Next,
                    _ => return,
                };
                super::super::input::enqueue(mailbox, commands, intent);
            },
        },
    );
}
