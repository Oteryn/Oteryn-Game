use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::boss_encounter::BossEncounterPanel;
impl PlayLink {
    #[must_use]
    pub fn boss_encounter_panel(&self) -> BossEncounterPanel {
        lock(&self.mailbox).boss_encounter.clone()
    }
}

#[cfg(test)]
#[path = "view/portraits.rs"]
mod portraits;

pub(in crate::play) fn cooldown_form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::play::feature_0115_mail::view::form::Form;
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
                0 => oteryn_client_ui::boss_encounter::Intent::Refresh,
                1 => oteryn_client_ui::boss_encounter::Intent::Next,
                _ => return,
            };
            super::input::enqueue(mailbox, commands, intent);
        },
    }
}
