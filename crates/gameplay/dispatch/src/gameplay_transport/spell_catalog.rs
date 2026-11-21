//! S-SPELLBOOK: read-only projection of the exact cast book and Character knowledge.
use super::ComposedFreshAdmission;
use crate::domain::CharacterId;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::spell::cast::PlayerSpellState;
use crate::spell::{Carrier, ManaCost, SpellBook};
use oteryn_protocol_oteryn::actor_spell_catalog::{self as wire, SpellCatalogEntry};

// Keep source identities intact while displaying the agreed conjuration label.
// A conflicting source alias is conservative even when that alias is inactive.
fn display_name<'a>(book: &'a SpellBook, spell: &'a crate::spell::SpellDefinition) -> &'a str {
    let Carrier::Rune { item, .. } = spell.carrier else {
        return &spell.name;
    };
    let mut label = None;
    for creator in &book.spells {
        if !matches!(creator.carrier, Carrier::Instant { .. })
            || !matches!(creator.execution, crate::spell::Execution::Conjure { result, .. } if result == item)
        {
            continue;
        }
        match label {
            Some(previous) if previous != creator.name => return &spell.name,
            None => label = Some(creator.name.as_str()),
            _ => {}
        }
    }
    label.unwrap_or(&spell.name)
}

pub(super) fn catalog(
    book: &SpellBook,
    state: &PlayerSpellState,
    now_micros: u64,
) -> Option<Vec<SpellCatalogEntry>> {
    catalog_with_visibility(book, state, now_micros, false)
}

pub(super) fn catalog_with_visibility(
    book: &SpellBook,
    state: &PlayerSpellState,
    now_micros: u64,
    include_other_vocations: bool,
) -> Option<Vec<SpellCatalogEntry>> {
    let groups: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tools/content-schema/spell-authoring/cooldown-groups.json"
    ))
    .ok()?;
    let groups = groups.get("groups")?.as_array()?;
    let facts = state.character_facts();
    let mut entries = Vec::new();
    for (index, spell) in book.spells.iter().enumerate() {
        let learned = state.learned_spells().contains(&spell.key);
        let vocation = spell.vocations.contains(&facts.vocation);
        if book.inactive_aliases.contains(&index)
            || (!include_other_vocations && !vocation && !learned)
        {
            continue;
        }
        let (words, rune_item, magic_level) = match &spell.carrier {
            Carrier::Instant { words, .. } => (words.clone(), None, 0),
            Carrier::Rune {
                item, magic_level, ..
            } => (String::new(), Some(*item), *magic_level),
        };
        let (mana, mana_percent) = match spell.mana {
            ManaCost::Fixed(mana) => (mana, false),
            ManaCost::PercentOfMaximum(mana) => (mana, true),
        };
        let mut cooldown_groups = Vec::new();
        for group in &spell.groups {
            let ordinal = groups.iter().position(|known| {
                known.get("key").and_then(|key| key.as_str()) == Some(&group.key)
            })?;
            cooldown_groups.push(u32::try_from(ordinal + 1).ok()?);
        }
        cooldown_groups.sort_unstable();
        cooldown_groups.dedup();
        entries.push(SpellCatalogEntry {
            cast_index: u32::try_from(index.checked_add(1)?).ok()?,
            name: display_name(book, spell).to_owned(),
            words,
            level: spell.level,
            mana,
            mana_percent,
            soul: spell.soul,
            learned,
            available: (if spell.learning_required {
                learned
            } else {
                vocation
            }) && facts.level >= spell.level
                && facts.magic_level >= magic_level
                && (!spell.premium || state.hunting_premium_current(now_micros)),
            cooldown_groups,
            rune_item,
        });
    }
    Some(entries)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn actor_spell_catalog(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<Vec<SpellCatalogEntry>> {
        self.actor_spell_catalog_with_visibility(actor, session, false)
            .await
    }

    pub(super) async fn actor_spell_catalog_with_visibility(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        include_other_vocations: bool,
    ) -> Option<Vec<SpellCatalogEntry>> {
        // This is a required join domain. Share one bounded retry budget across
        // its durable reads; only a busy holder is transient, never a bad fence.
        let retry = super::AdmissionRetryBudget::for_command(self.root);
        let refusal = |error| match error {
            crate::durability::DurabilityError::RootUnavailable => {
                super::connection::AdmissionRefusal::Unavailable
            }
            _ => super::connection::AdmissionRefusal::Rejected,
        };
        // A House resident uses its named custody fence rather than a Channel combat lease.
        let (character, house) = {
            let runtime = self.runtime.lock().await;
            if let Ok(lease) = runtime.bound_attacker_lease(actor, session) {
                (
                    CharacterId::from_bytes(*lease.character_id().as_bytes()).ok()?,
                    false,
                )
            } else {
                let house = runtime.house_actor_state(actor, session).ok()?;
                (
                    CharacterId::from_bytes(*house.lease.character_id().as_bytes()).ok()?,
                    true,
                )
            }
        };
        let house_fence = if house {
            let (current, _) = retry
                .retry_unavailable(async || {
                    super::FreshAdmissionStore::from_root(self.root.clone())
                        .current_session_at(session)
                        .await
                        .map_err(refusal)
                })
                .await
                .ok()?;
            Some(super::item_fence_of(current)?)
        } else {
            None
        };
        let learned = retry
            .retry_unavailable(async || {
                self.root
                    .read_character_known_spells(self.character, character)
                    .await
                    .map_err(refusal)
            })
            .await
            .ok()?;
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let state = if let Some(fence) = house_fence {
            if fence.character_id != character {
                return None;
            }
            states.house_state_mut(&runtime, actor, session, fence)?
        } else {
            let lease = runtime.bound_attacker_lease(actor, session).ok()?;
            if lease.character_id().as_bytes() != character.as_bytes() {
                return None;
            }
            states.get_mut(&runtime, actor, session)?
        };
        state.install_learned_spells(learned);
        if include_other_vocations {
            catalog_with_visibility(self.spells, state, self.owner_now().get(), true)
        } else {
            catalog(self.spells, state, self.owner_now().get())
        }
    }
}

// The transport only calls this for a selected domain, preserving older clients.
pub(super) async fn observe<A: super::connection::FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Option<Vec<u8>> {
    wire::encode_catalog(&authority.observe_spell_catalog(actor, session).await?).ok()
}

#[cfg(test)]
mod display_name_tests {
    use super::*;
    use crate::spell::{Execution, SpellDefinition};

    fn rune_and_creator(
        item: u32,
        legacy: &str,
        current: &str,
    ) -> Result<(SpellDefinition, SpellDefinition), String> {
        let template = crate::spell::cast::v1_spell_book()?
            .spells
            .into_iter()
            .next()
            .ok_or("missing template")?;
        let mut rune = template.clone();
        rune.key = format!("test:rune/{item}");
        rune.name = legacy.into();
        rune.carrier = Carrier::Rune {
            item,
            charges: 1,
            magic_level: 0,
        };
        let mut creator = template;
        creator.key = format!("test:conjure/{item}");
        creator.name = current.into();
        creator.carrier = Carrier::Instant {
            words: format!("test conjure {item}"),
            takes_parameter: false,
        };
        creator.execution = Execution::Conjure {
            reagent: None,
            result: item,
            count: 1,
        };
        Ok((rune, creator))
    }

    fn project(book: &SpellBook) -> Result<Vec<SpellCatalogEntry>, String> {
        let state =
            PlayerSpellState::new(crate::gameplay_transport::actor_spell::tests::FACTS, 0, 0)
                .ok_or("state")?;
        catalog_with_visibility(book, &state, 0, true).ok_or("catalog".into())
    }

    #[test]
    fn rune_display_names_use_agreed_conjuration_without_changing_cast_identity()
    -> Result<(), String> {
        for (item, legacy, current) in [
            (3153, "antidote rune", "Cure Poison Rune"),
            (3197, "desintegrate rune", "Disintegrate Rune"),
            (3149, "energybomb rune", "Energy Bomb Rune"),
            (3192, "firebomb rune", "Fire Bomb Rune"),
            (3203, "animate dead rune", "Animate Dead Rune"),
        ] {
            let (rune, creator) = rune_and_creator(item, legacy, current)?;
            let mut second = creator.clone();
            second.key.push_str("/second");
            second.carrier = Carrier::Instant {
                words: "test second creator".into(),
                takes_parameter: false,
            };
            let mut book =
                SpellBook::new(vec![rune, creator, second]).map_err(|e| e.to_string())?;
            let before = book.spells.clone();
            let entries = project(&book)?;
            let rune_entry = entries
                .iter()
                .find(|entry| entry.rune_item == Some(item))
                .ok_or("rune entry")?;
            assert_eq!(rune_entry.name, current);
            for entry in &entries {
                let source = &book.spells[entry.cast_index as usize - 1];
                assert_eq!(
                    entry.mana,
                    match source.mana {
                        ManaCost::Fixed(mana) => mana,
                        ManaCost::PercentOfMaximum(mana) => mana,
                    }
                );
                if entry.rune_item.is_none() {
                    assert_eq!(entry.name, source.name);
                }
            }
            assert_eq!(book.spells, before);
            assert_eq!(
                book.rune(item).map(|spell| &spell.key),
                Some(&before[rune_entry.cast_index as usize - 1].key)
            );
            let inactive = book
                .spells
                .iter()
                .position(|spell| matches!(spell.carrier, Carrier::Instant { .. }))
                .ok_or("creator")?;
            book.spells[inactive].name = "inactive conflicting creator".into();
            book.inactive_aliases.insert(inactive);
            assert_eq!(
                project(&book)?
                    .iter()
                    .find(|entry| entry.rune_item == Some(item))
                    .ok_or("rune entry")?
                    .name,
                legacy
            );
        }
        Ok(())
    }

    #[test]
    fn rune_display_names_keep_source_when_no_creator_or_creators_conflict() -> Result<(), String> {
        let (rune, creator) = rune_and_creator(3153, "antidote rune", "Cure Poison Rune")?;
        let book = SpellBook::new(vec![rune.clone()]).map_err(|e| e.to_string())?;
        assert_eq!(project(&book)?[0].name, "antidote rune");
        let mut conflicting = creator.clone();
        conflicting.key.push_str("/conflict");
        conflicting.name = "Different creator".into();
        conflicting.carrier = Carrier::Instant {
            words: "test conflicting creator".into(),
            takes_parameter: false,
        };
        let book = SpellBook::new(vec![rune, creator, conflicting]).map_err(|e| e.to_string())?;
        assert_eq!(
            project(&book)?
                .iter()
                .find(|entry| entry.rune_item == Some(3153))
                .ok_or("rune entry")?
                .name,
            "antidote rune"
        );
        Ok(())
    }
}
