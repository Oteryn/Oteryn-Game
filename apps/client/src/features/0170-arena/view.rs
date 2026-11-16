use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::arena::ArenaPanel;
impl PlayLink {
    pub fn arena_panel(&self) -> ArenaPanel {
        lock(&self.mailbox).arena.clone()
    }
}

/// Arena and Drome controls for the shared frontend; the native lists are number fields here.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use oteryn_client_runtime::arena::ArenaAction;
    use oteryn_client_ui::arena::{DROME_REWARDS, Intent, drome_intent, panel_text, request};
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Arenas — Ctrl+Shift+Z"),
        labels: &[
            "Reward rank: 1 Greenhorn, 2 Scrapper, 3 Warlord",
            "Drome reward number (1-18)",
        ],
        buttons: &[
            "Catalogue",
            "Refresh status",
            "Enter",
            "Next pit",
            "Leave",
            "Present",
            "Supplies",
            "Club",
            "Axe",
            "Sword",
            "Trophy",
            "Drome status",
            "Drome enter",
            "Drome leave",
            "Lower start -5",
            "Claim potion",
            "Buy Drome reward",
        ],
        snapshot: |mailbox| {
            let mut text = panel_text(&mailbox.arena);
            text.push_str("\nDrome rewards:\n");
            for (index, reward) in DROME_REWARDS.iter().enumerate() {
                text.push_str(&format!("{}. {reward}\n", index + 1));
            }
            (
                mailbox.arena.open,
                mailbox.arena.pending.is_some(),
                vec![String::new(); 2],
                text,
            )
        },
        set_open: |mailbox, value| mailbox.arena.open = value,
        submit: |mailbox, commands, action, values| {
            use crate::play::feature_0115_mail::view::form::field;
            let number = |index: usize| field(&values, index).parse::<isize>().ok();
            let intent = match action {
                0 => Some(Intent::Catalogue),
                1 => Some(Intent::Action(ArenaAction::Status)),
                2 => Some(Intent::Action(ArenaAction::Enter)),
                3 => Some(Intent::Action(ArenaAction::Advance)),
                4 => Some(Intent::Action(ArenaAction::Leave)),
                5..=10 => {
                    let (category, choice) = match action {
                        5 => (1, 1),
                        6 => (2, 1),
                        7 => (3, 1),
                        8 => (3, 2),
                        9 => (3, 3),
                        _ => (4, 1),
                    };
                    let intent = number(0)
                        .filter(|rank| (1..=3).contains(rank))
                        .and_then(|rank| u8::try_from(rank).ok())
                        .map(|rank| {
                            Intent::Action(ArenaAction::ClaimReward {
                                rank,
                                category,
                                choice,
                            })
                        });
                    if intent.is_none() {
                        mailbox.arena.feedback = Some(tr!("Enter a reward rank from 1 to 3"));
                    }
                    intent
                }
                11..=15 => drome_intent(action + 189, 0),
                16 => {
                    let intent = number(1)
                        .and_then(|reward| reward.checked_sub(1))
                        .and_then(|reward| drome_intent(205, reward));
                    if intent.is_none() {
                        mailbox.arena.feedback =
                            Some(tr!("Enter a Drome reward number from 1 to 18"));
                    }
                    intent
                }
                _ => None,
            };
            if let Some(intent) = intent
                && let Some(command) = request(&mut mailbox.arena, intent)
                && commands
                    .send(crate::combat_input::PlayCommand::Feature(command))
                    .is_err()
            {
                mailbox.arena.pending = None;
                mailbox.arena.feedback = Some(tr!("The session has ended"));
            }
        },
    }
}
