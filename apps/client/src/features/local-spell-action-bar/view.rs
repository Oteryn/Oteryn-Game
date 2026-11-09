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
                "Slot number (1–12)",
                "Server cast index (see live catalogue)",
            ],
            buttons: &["Assign spell", "Clear slot", "Cast slot"],
            snapshot: |mailbox| {
                let panel = &mailbox.spell_bar;
                (
                    panel.open,
                    false,
                    vec!["1".into(), String::new()],
                    format!(
                        "{}\r\n{}\r\n{}",
                        oteryn_client_ui::spell_action_bar::panel_text(
                            panel,
                            &mailbox.spell_catalog.catalog
                        ),
                        oteryn_client_ui::spell_action_bar::vitals_text(mailbox.vitals.as_ref()),
                        oteryn_client_ui::spell_catalog::panel_text(&mailbox.spell_catalog)
                    ),
                )
            },
            set_open: |mailbox, open| mailbox.spell_bar.open = open,
            submit: |mailbox, commands, action, fields| {
                super::input::edit_slot(mailbox, commands, action, &fields)
            },
        },
    );
}

impl crate::play::PlayLink {
    pub fn assign_spell_slot(&self, slot: usize, cast_index: Option<u32>) -> bool {
        super::super::feature_0287_actor_spell_catalog::view::assign_cast_index(
            &mut crate::play::mailbox::lock(&self.mailbox),
            slot,
            cast_index,
        )
    }
    pub fn spell_slot_tooltips(&self) -> Vec<String> {
        let mailbox = crate::play::mailbox::lock(&self.mailbox);
        let mut labels = oteryn_client_ui::spell_action_bar::slot_labels(&mailbox.spell_bar, &mailbox.spell_catalog.catalog);
        if !mailbox.local_preferences.action_bar.tooltips { labels.iter_mut().for_each(String::clear); }
        labels
    }
    pub fn action_bar_grid(&self) -> (usize, usize) {
        crate::play::mailbox::lock(&self.mailbox).local_preferences.action_bar.grid()
    }
    pub fn spell_slot_labels(&self) -> Vec<String> {
        let mut mailbox = crate::play::mailbox::lock(&self.mailbox);
        crate::play::feature_0287_actor_spell_catalog::view::resolve_archive_bindings(&mut mailbox);
        if mailbox.local_preferences.action_bar.auto_insert {
            let learned: Vec<_> = mailbox.spell_catalog.catalog.entries().iter().filter(|entry| entry.learned).map(|entry| entry.cast_index).collect();
            for id in learned {
                if !mailbox.spell_bar.observed_spells.insert(id) { continue; }
                let fields=mailbox.spell_bar.bar.fields();
                if fields.iter().any(|field| field.parse::<u32>().ok()==Some(id)) { continue; }
                if let Some(slot)=fields.iter().position(String::is_empty) {
                    mailbox.spell_bar.bar.bind(slot,std::num::NonZeroU32::new(id));
                }
            }
        }
        let mut labels = oteryn_client_ui::spell_action_bar::slot_labels(
            &mailbox.spell_bar, &mailbox.spell_catalog.catalog,
        );
        mailbox.local_preferences.action_bar.project(&mut labels);
        if mailbox.local_preferences.action_bar.amounts {
            let items=mailbox.inventory.rows();
            for (field, label) in mailbox.spell_bar.bar.fields().iter().zip(&mut labels) {
                if let Some(item)=mailbox.spell_catalog.catalog.entries().iter().find(|entry| field.parse::<u32>().ok()==Some(entry.cast_index)).and_then(|entry| entry.rune_item) {
                    let count:u64=items.iter().filter(|entry| entry.item_definition_ref.get()==item).map(|entry| u64::from(entry.count.get())).sum();
                    if mailbox.inventory.inventory.is_some() { label.push_str(&format!(" ×{count}")); }
                }
            }
        }
        labels
    }
    pub fn cast_spell_slot(&self, slot: std::num::NonZeroU32) {
        super::input::send_cast(self, slot);
    }
}
