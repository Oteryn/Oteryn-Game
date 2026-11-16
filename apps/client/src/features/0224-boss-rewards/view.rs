use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::boss_rewards::BossRewardsPanel;
impl PlayLink {
    #[must_use]
    pub fn boss_rewards_panel(&self) -> BossRewardsPanel {
        lock(&self.mailbox).boss_rewards.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

/// A death id typed as 32 hexadecimal digits.
pub(super) fn parse_death_id(text: &str) -> Option<[u8; 16]> {
    let text = text.trim();
    if text.len() != 32 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0; 16];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Personal boss rewards for the shared frontend.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use oteryn_client_ui::boss_rewards::{Intent, panel_text};
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Boss rewards"),
        labels: &["Death id (32 hex digits)", "Reward ordinal"],
        buttons: &["Browse", "Take item", "Collect group", "Collect all"],
        snapshot: |mailbox| {
            let panel = &mailbox.boss_rewards;
            (
                panel.open && !mailbox.boss_rewards_shutdown,
                panel.pending,
                vec![
                    panel.death.iter().map(|byte| format!("{byte:02x}")).collect(),
                    panel.selected_ordinal.to_string(),
                ],
                panel_text(panel),
            )
        },
        set_open: |mailbox, value| mailbox.boss_rewards.open = value,
        submit: |mailbox, commands, action, values| {
            use crate::play::feature_0115_mail::view::form::field;
            let intent = match action {
                0 => Intent::Browse,
                1 => Intent::Take,
                2 => Intent::CollectGroup,
                3 => Intent::CollectAll,
                _ => return,
            };
            if intent != Intent::CollectAll {
                mailbox.boss_rewards.death = parse_death_id(field(&values, 0)).unwrap_or([0; 16]);
            }
            mailbox.boss_rewards.selected_ordinal = field(&values, 1).parse().unwrap_or(0);
            super::input::enqueue(mailbox, commands, intent);
        },
    }
}
