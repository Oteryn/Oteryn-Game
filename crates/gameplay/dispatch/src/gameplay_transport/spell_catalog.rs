//! S-SPELLBOOK: read-only projection of the exact cast book and Character knowledge.
use super::ComposedFreshAdmission;
use crate::domain::CharacterId;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::spell::cast::PlayerSpellState;
use crate::spell::{Carrier, ManaCost, SpellBook};
use oteryn_protocol_oteryn::actor_spell_catalog::{self as wire, SpellCatalogEntry};

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
            name: spell.name.clone(),
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
