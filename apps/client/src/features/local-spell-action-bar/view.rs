#[path = "hotkeys.rs"]
mod hotkeys;
#[path = "item_actions.rs"]
mod item_actions;
pub use hotkeys::ActionKeys;
#[cfg(test)]
pub(in crate::play) use hotkeys::bind as bind_custom_key;
#[cfg(test)]
pub(in crate::play) use hotkeys::native_key;
pub use item_actions::ItemUseMode;
#[cfg(test)]
pub(in crate::play) use item_actions::editor_items;
pub(in crate::play) use item_actions::{item_source, item_target, visible_items};
#[path = "amount.rs"]
mod amount;
#[path = "layout.rs"]
mod layout;
pub use layout::{BarDock, BarGesture, BarLayout};
#[path = "assignments.rs"]
mod assignments;
#[path = "item_cooldowns.rs"]
mod item_cooldowns;
use crate::play::PlayView;
use crate::spell::SPELL_SLOTS;
pub(crate) use assignments::BarActionsState;
pub(in crate::play) use assignments::ItemCooldownRead;
#[cfg(test)]
pub(in crate::play) use assignments::activate_shared as activate_extra_shared;
pub(in crate::play) use assignments::label as action_label;
#[cfg(test)]
pub(in crate::play) use assignments::set_action as set_extra;
pub use assignments::{BarAction, BarActions, PassiveAbility, SpellAim};
pub(in crate::play) use assignments::{activate as activate_extra, clear_extra};

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
            self.resource_status(),
            self.battle_instinct
                .map_or_else(String::new, |status| lfmt!(
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
            Some(spell) => lfmt!("Spell {spell}: Click a tile; Escape cancels", spell = spell),
            None => lfmt!(
                "spells 1-{SPELL_SLOTS} {line}",
                SPELL_SLOTS = SPELL_SLOTS,
                line = line
            )
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
        let mut labels = oteryn_client_ui::spell_action_bar::slot_labels(
            &mailbox.spell_bar,
            &mailbox.spell_catalog.catalog,
        );
        for (slot, action) in &mailbox.bar_actions.saved.0 {
            if let Some(label) = labels.get_mut(*slot) {
                *label = assignments::slot_label(&mailbox, *slot, action);
            }
        }
        if !mailbox.local_preferences.action_bar.tooltips {
            labels.iter_mut().for_each(String::clear);
        }
        labels
    }
    pub fn action_bar_grid(&self) -> (usize, usize) {
        crate::play::mailbox::lock(&self.mailbox)
            .local_preferences
            .action_bar
            .grid()
    }
    pub fn spell_slot_labels(&self) -> Vec<String> {
        let mut mailbox = crate::play::mailbox::lock(&self.mailbox);
        crate::play::feature_0287_actor_spell_catalog::view::resolve_archive_bindings(&mut mailbox);
        if mailbox.local_preferences.action_bar.auto_insert {
            let learned: Vec<_> = mailbox
                .spell_catalog
                .catalog
                .entries()
                .iter()
                .filter(|entry| entry.learned)
                .map(|entry| entry.cast_index)
                .collect();
            for id in learned {
                if !mailbox.spell_bar.observed_spells.insert(id) {
                    continue;
                }
                let fields = mailbox.spell_bar.bar.fields();
                if fields
                    .iter()
                    .any(|field| field.parse::<u32>().ok() == Some(id))
                {
                    continue;
                }
                if let Some(slot) = fields.iter().enumerate().position(|(slot, field)| {
                    field.is_empty() && !mailbox.bar_actions.saved.0.contains_key(&slot)
                }) {
                    crate::play::feature_0287_actor_spell_catalog::view::assign_cast_index(
                        &mut mailbox,
                        slot,
                        Some(id),
                    );
                }
            }
        }
        let mut labels = oteryn_client_ui::spell_action_bar::slot_labels(
            &mailbox.spell_bar,
            &mailbox.spell_catalog.catalog,
        );
        for (slot, label) in labels.iter_mut().enumerate() {
            if !mailbox.local_preferences.action_bar.amounts {
                continue;
            }
            let Some(spell) = u32::try_from(slot + 1)
                .ok()
                .and_then(std::num::NonZeroU32::new)
                .and_then(|slot| mailbox.spell_bar.bar.spell(slot))
            else {
                continue;
            };
            if let Some(definition) = mailbox
                .spell_catalog
                .catalog
                .entries()
                .iter()
                .find(|entry| entry.cast_index == spell.get())
                .and_then(|entry| entry.rune_item)
            {
                label.push_str(&format!(
                    "\nVisible: {}",
                    amount::visible_count(&mailbox, definition)
                        .map_or_else(|| "?".into(), |count| count.to_string())
                ));
            }
        }
        for (slot, action) in &mailbox.bar_actions.saved.0 {
            if let Some(label) = labels.get_mut(*slot) {
                *label = format!(
                    "{}: {}",
                    slot + 1,
                    assignments::slot_label(&mailbox, *slot, action)
                );
                if !mailbox.local_preferences.action_bar.amounts {
                    *label = label
                        .lines()
                        .filter(|line| !line.starts_with("Visible:"))
                        .collect::<Vec<_>>()
                        .join("\n");
                }
            }
        }
        for (slot, key) in &mailbox.bar_actions.keys.0 {
            if let Some(label) = labels.get_mut(*slot) {
                label.push_str(&format!("\n{key}"));
            }
        }
        mailbox.local_preferences.action_bar.project(&mut labels);
        labels
    }
    pub fn cast_spell_slot(&self, slot: std::num::NonZeroU32) {
        super::input::send_cast(self, slot);
    }
}

impl crate::play::PlayLink {
    pub fn cooldown_bar_icons(&self) -> Vec<oteryn_client_ui::spell_action_bar::CooldownIcon> {
        let mailbox = crate::play::mailbox::lock(&self.mailbox);
        cooldown_bar_icons(&mailbox)
    }
}
pub(in crate::play) fn cooldown_bar_icons(
    mailbox: &crate::play::mailbox::Mailbox,
) -> Vec<oteryn_client_ui::spell_action_bar::CooldownIcon> {
    if !mailbox.local_preferences.action_bar.cooldown_bar_enabled {
        return Vec::new();
    }
    let mut icons = oteryn_client_ui::spell_action_bar::cooldown_icons(
        &mailbox.spell_bar,
        &mailbox.spell_catalog.catalog,
    );
    for icon in &mut icons {
        if !icon.group {
            if let Some(info) = mailbox.spell_catalog.info.get(&icon.id) {
                icon.tooltip = oteryn_client_ui::server_info::spell_text(info);
            }
        } else {
            let mut durations: Vec<_> = mailbox
                .spell_catalog
                .info
                .values()
                .flat_map(|s| s.group_cooldowns_ms.iter())
                .filter(|(id, _)| *id == icon.id)
                .map(|(_, ms)| *ms)
                .collect();
            durations.sort_unstable();
            durations.dedup();
            if !durations.is_empty() {
                icon.tooltip = format!("Base durations (ms): {durations:?}");
            }
        }
    }
    icons
}

impl PlayView {
    /// Refresh before dispatching hotkeys, using the same target and generation as a world click.
    pub fn observe_bar_pointer(&self, link: &crate::play::PlayLink) {
        use oteryn_client_runtime::world_interaction::{Selection, Target};
        let cursor = self
            .creature_drag
            .pointer
            .and_then(|(x, y)| self.click_tile(x, y))
            .map(|tile| {
                let position = oteryn_session::ActorPosition {
                    x: tile.x,
                    y: tile.y,
                    floor: self.floor,
                };
                let target = self
                    .world_context_target(tile)
                    .unwrap_or(Target::Position(position));
                let binding = match self.scene.target_at(tile).and_then(|target| target.item) {
                    Some(crate::play::GroundItem::Map { header, .. }) => Some((
                        header.content_generation,
                        header.bundle_digest,
                        header.reset_epoch,
                    )),
                    _ => None,
                };
                (target, Selection { position, binding })
            });
        crate::play::mailbox::lock(&link.mailbox).bar_actions.cursor = cursor;
    }
}


impl PlayView {
    pub fn clear_bar_pointer(&mut self, link: &crate::play::PlayLink) {
        self.creature_drag.pointer = None;
        crate::play::mailbox::lock(&link.mailbox).bar_actions.cursor = None;
    }
}

// Kept outside Win32 so the editor transactions can be exercised on every test host.
#[cfg(test)]
pub(in crate::play) fn choose_editor_assignment(
    target: &mut Option<BarAction>,
    selected: Option<BarAction>,
) {
    if let Some(action) = selected {
        *target = Some(action);
    }
}

#[cfg(test)]
pub(in crate::play) fn editor_active(mailbox: &crate::play::mailbox::Mailbox, scope: u64) -> bool {
    mailbox.bar_actions.scope == scope
        && !mailbox.world_input_blocked()
        && !mailbox.custom_hotkeys_ended
}
