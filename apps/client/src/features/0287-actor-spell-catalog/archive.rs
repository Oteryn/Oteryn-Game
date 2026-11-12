//! Archive bindings retain a display identity, never a cast index from an old session.
use crate::play::{
    PlayLink,
    mailbox::{SharedMailbox, lock},
};
use oteryn_client_runtime::spell_catalog::SpellCatalogEntry;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivedSpell {
    name: String,
    words: String,
    rune_item: Option<u32>,
}
impl ArchivedSpell {
    fn matches(&self, spell: &SpellCatalogEntry) -> bool {
        if self.words != spell.words || self.rune_item != spell.rune_item {
            return false;
        }
        if self.name == spell.name {
            return true;
        }
        let Some(item) = self.rune_item else {
            return false;
        };
        self.name.eq_ignore_ascii_case(&spell.name)
            || match item {
                3149 => legacy_rune_name(
                    &self.name,
                    &spell.name,
                    "energybomb rune",
                    "Energy Bomb Rune",
                ),
                3153 => {
                    legacy_rune_name(&self.name, &spell.name, "antidote rune", "Cure Poison Rune")
                }
                3192 => {
                    legacy_rune_name(&self.name, &spell.name, "firebomb rune", "Fire Bomb Rune")
                }
                3197 => legacy_rune_name(
                    &self.name,
                    &spell.name,
                    "desintegrate rune",
                    "Disintegrate Rune",
                ),
                _ => false,
            }
    }
    fn valid(&self) -> bool {
        !self.name.is_empty()
            && self.name.len() <= 128
            && self.words.len() <= 128
            && !self
                .name
                .chars()
                .chain(self.words.chars())
                .any(char::is_control)
            && self.rune_item.is_none_or(|item| item > 0 && item < 1 << 28)
    }
}
fn legacy_rune_name(saved: &str, current: &str, legacy: &str, canonical: &str) -> bool {
    (saved.eq_ignore_ascii_case(legacy) && current.eq_ignore_ascii_case(canonical))
        || (saved.eq_ignore_ascii_case(canonical) && current.eq_ignore_ascii_case(legacy))
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "spell", deny_unknown_fields)]
pub enum ArchiveSlot {
    /// Existing manual assignments remain available until this slot is managed by the archive.
    #[default]
    Unmanaged,
    Empty,
    Spell(ArchivedSpell),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveBindings {
    #[serde(deserialize_with = "read_slots", serialize_with = "write_slots")]
    pub slots: [ArchiveSlot; oteryn_client_runtime::spell::ACTION_BAR_SLOTS],
}
fn write_slots<S: serde::Serializer>(
    slots: &[ArchiveSlot; oteryn_client_runtime::spell::ACTION_BAR_SLOTS],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    slots.as_slice().serialize(serializer)
}
fn read_slots<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<[ArchiveSlot; oteryn_client_runtime::spell::ACTION_BAR_SLOTS], D::Error> {
    let slots = Vec::<ArchiveSlot>::deserialize(deserializer)?;
    if slots.len() > oteryn_client_runtime::spell::ACTION_BAR_SLOTS {
        return Err(serde::de::Error::custom("Too many action slots"));
    }
    let mut slots = slots.into_iter();
    Ok(std::array::from_fn(|_| slots.next().unwrap_or_default()))
}
impl Default for ArchiveBindings {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| ArchiveSlot::default()),
        }
    }
}
impl ArchiveBindings {
    pub fn valid(&self) -> bool {
        self.slots.iter().all(|slot| match slot {
            ArchiveSlot::Spell(spell) => spell.valid(),
            _ => true,
        })
    }
    pub fn labels(&self) -> Vec<String> {
        self.slots
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                format!(
                    "Slot {}: {}",
                    i + 1,
                    match slot {
                        ArchiveSlot::Spell(spell) => spell.name.as_str(),
                        ArchiveSlot::Empty => "Empty",
                        ArchiveSlot::Unmanaged => "Manual binding",
                    }
                )
            })
            .collect()
    }
}
pub(super) fn assign(mailbox: &SharedMailbox, row: usize, slot: usize) -> bool {
    let mut mailbox = lock(mailbox);
    let state = &mut mailbox.spellbook_details;
    if !state.selected
        || state.pending.is_some()
        || state.confirmed_filter.as_ref() != Some(&state.filter)
    {
        return false;
    }
    let Some(entry) = state.page.as_ref().and_then(|page| page.entries.get(row)) else {
        return false;
    };
    let Some(target) = state.archive.slots.get_mut(slot) else {
        return false;
    };
    *target = ArchiveSlot::Spell(ArchivedSpell {
        name: entry.spell.name.clone(),
        words: entry.spell.words.clone(),
        rune_item: entry.spell.rune_item,
    });
    state.archive_edit = crate::play::feature_0017_attack::view::next_edit();
    state.feedback = format!(
        "Assigned {} to slot {}. Unavailable spells stay inactive until the live catalog contains them.",
        entry.spell.name,
        slot + 1
    );
    crate::play::feature_local_spell_action_bar::view::clear_extra(&mut mailbox, slot);
    true
}
pub(super) fn clear(mailbox: &SharedMailbox, slot: usize) -> bool {
    let mut mailbox = lock(mailbox);
    let state = &mut mailbox.spellbook_details;
    let Some(target) = state.archive.slots.get_mut(slot) else {
        return false;
    };
    *target = ArchiveSlot::Empty;
    state.archive_edit = crate::play::feature_0017_attack::view::next_edit();
    state.feedback = format!("Cleared slot {}", slot + 1);
    crate::play::feature_local_spell_action_bar::view::clear_extra(&mut mailbox, slot);
    true
}
impl PlayLink {
    pub fn archive_assign(&self, row: usize, slot: usize) -> bool {
        assign(&self.mailbox, row, slot)
    }
    pub fn archive_clear(&self, slot: usize) -> bool {
        clear(&self.mailbox, slot)
    }
    pub fn sync_archive_bindings(&self, bindings: &mut ArchiveBindings, edit: u64) -> Option<u64> {
        let mut mailbox = lock(&self.mailbox);
        let state = &mut mailbox.spellbook_details;
        let result = if state.archive_edit > edit {
            *bindings = state.archive.clone();
            Some(state.archive_edit)
        } else {
            let previous = state.archive.clone();
            state.archive = bindings.clone();
            for (slot, (old, new)) in previous.slots.iter().zip(&bindings.slots).enumerate() {
                if !matches!(old, ArchiveSlot::Unmanaged) && matches!(new, ArchiveSlot::Unmanaged) {
                    mailbox
                        .spell_bar
                        .bar
                        .bind(slot, NonZeroU32::new(slot as u32 + 1));
                }
            }
            mailbox.spellbook_details.archive_edit = edit;
            None
        };
        resolve(&mut mailbox);
        result
    }
}

pub(in crate::play) fn resolve(mailbox: &mut crate::play::mailbox::Mailbox) {
    for (slot, binding) in mailbox.spellbook_details.archive.slots.iter().enumerate() {
        let spell = match binding {
            ArchiveSlot::Unmanaged => continue,
            ArchiveSlot::Empty => None,
            ArchiveSlot::Spell(identity) => {
                let mut matching = mailbox
                    .spell_catalog
                    .catalog
                    .entries()
                    .iter()
                    .filter(|spell| identity.matches(spell));
                let found = matching.next();
                if matching.next().is_some() {
                    None
                } else {
                    found.and_then(|spell| NonZeroU32::new(spell.cast_index))
                }
            }
        };
        mailbox.spell_bar.bar.bind(slot, spell);
    }
}

pub(in crate::play) fn manual(mailbox: &mut crate::play::mailbox::Mailbox) {
    if mailbox.spellbook_details.archive != ArchiveBindings::default() {
        mailbox.spellbook_details.archive = ArchiveBindings::default();
        mailbox.spellbook_details.archive_edit =
            crate::play::feature_0017_attack::view::next_edit();
    }
}

/// Store display identity from the current session catalog, never a stale numeric index.
pub(in crate::play) fn assign_cast_index(
    mailbox: &mut crate::play::mailbox::Mailbox,
    slot: usize,
    index: Option<u32>,
) -> bool {
    let binding = match index {
        None => ArchiveSlot::Empty,
        Some(index) => {
            let Some(spell) = mailbox
                .spell_catalog
                .catalog
                .entries()
                .iter()
                .find(|entry| entry.cast_index == index)
            else {
                return false;
            };
            ArchiveSlot::Spell(ArchivedSpell {
                name: spell.name.clone(),
                words: spell.words.clone(),
                rune_item: spell.rune_item,
            })
        }
    };
    let Some(target) = mailbox.spellbook_details.archive.slots.get_mut(slot) else {
        return false;
    };
    *target = binding;
    mailbox.spellbook_details.archive_edit = crate::play::feature_0017_attack::view::next_edit();
    crate::play::feature_local_spell_action_bar::view::clear_extra(mailbox, slot);
    resolve(mailbox);
    true
}
