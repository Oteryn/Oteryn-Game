use crate::play::PlayView;
use crate::spell::SPELL_SLOTS;

impl PlayView {
    #[must_use]
    pub const fn actor_vitals(&self) -> Option<&oteryn_session::ActorVitals> {
        self.vitals.as_ref()
    }

    #[must_use]
    pub fn hud_status(&self) -> String {
        if self.preferences().compact_status {
            return oteryn_client_ui::spell_action_bar::vitals_text(self.actor_vitals());
        }
        format!(
            "{}   {}   {}   {}   {}   {}   {}",
            self.served_channel_text().unwrap_or_default(),
            self.world_clock_text().unwrap_or_default(),
            oteryn_client_ui::spell_action_bar::vitals_text(self.actor_vitals()),
            self.battle_instinct
                .map_or_else(String::new, |status| format!(
                    "Battle Instinct: {} (+{} melee, +{} shielding)",
                    if !status.active {
                        "inactive"
                    } else if status.melee == 0 && status.shielding == 0 {
                        "eligible"
                    } else {
                        "active"
                    },
                    status.melee,
                    status.shielding,
                )),
            self.combat_status(),
            self.target_walk_feedback().unwrap_or(""),
            self.world_context_feedback.as_deref().unwrap_or("")
        )
    }

    /// The native status projection: spell hotkeys, combat/logout feedback and server notices.
    #[must_use]
    pub fn combat_status(&self) -> String {
        let line = self
            .logout_line
            .as_deref()
            .or(self.action_line)
            .or(self.combat_line)
            .unwrap_or("");
        let mut status = match self.cursor_spell {
            Some(spell) => format!("Spell {spell}: Click a tile; Escape cancels"),
            None => format!("spells 1-{SPELL_SLOTS} {line}")
                .trim_end()
                .to_owned(),
        };
        if let Some(notice) = self.death_notice() {
            status = format!("{notice} — {status}");
        }
        match self.server_message_status() {
            Some(message) => format!("{status} — {message}"),
            None => status,
        }
    }
}

#[cfg(windows)]
pub(in crate::play) fn open_window(link: &crate::play::PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Spell action bar",
            labels: &[
                "Slot 1 cast index",
                "Slot 2 cast index",
                "Slot 3 cast index",
                "Slot 4 cast index",
            ],
            buttons: &["Apply", "Cast 1", "Cast 2", "Cast 3", "Cast 4"],
            snapshot: |mailbox| {
                let panel = &mailbox.spell_bar;
                (
                    panel.open,
                    false,
                    panel.bar.fields(),
                    format!(
                        "{}\r\n{}",
                        oteryn_client_ui::spell_action_bar::panel_text(
                            panel,
                            &mailbox.spell_catalog.catalog
                        ),
                        oteryn_client_ui::spell_action_bar::vitals_text(mailbox.vitals.as_ref())
                    ),
                )
            },
            set_open: |mailbox, open| mailbox.spell_bar.open = open,
            submit: |mailbox, commands, action, fields| {
                super::input::submit(mailbox, commands, action, &fields)
            },
        },
    );
}
