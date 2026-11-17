//! News 8675 automatic spell acquisition through the native dispatch owner.

use std::collections::BTreeSet;

use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::GameSessionId;
use crate::spell::{SpellBook, Vocation};

use super::ComposedFreshAdmission;
use super::connection::{AdmissionRefusal, AdmittedSession, FirstEntryOutcome};
use super::spell_character_facts::{CastFactsLoad, load_character_cast_facts};

/// The exact catalogue slice acquired at this progression point. `learning_required` preserves
/// explicitly excluded spells rather than turning this compatibility projection into a grant.
fn eligible_spell_keys(book: &SpellBook, level: u32, vocation: Vocation) -> BTreeSet<String> {
    book.spells
        .iter()
        .filter(|spell| {
            !spell.learning_required && spell.level <= level && spell.vocations.contains(&vocation)
        })
        .map(|spell| spell.key.clone())
        .collect()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Fresh and resumed connections share the same idempotent, exact-fence repair.
    pub(super) async fn repair_pending_spell_entry(&self, admitted: AdmittedSession) -> bool {
        let (FirstEntryOutcome::AwaitingSpellRepair, Some(expected), Some(actor)) = (
            admitted.first_entry,
            admitted.item_fence,
            admitted.runtime_actor,
        ) else {
            return false;
        };
        if expected.game_session_id != admitted.game_session_id
            || expected.connection_generation.get() != admitted.continuity.connection_generation
        {
            return false;
        }
        let Ok(retry) =
            super::AdmissionRetryBudget::for_foreground_root(&admitted.game_session_id, self.root)
        else {
            return false;
        };
        retry
            .retry_unavailable(async || {
                let loaded =
                    load_character_cast_facts(self.root, self.character, expected.character_id)
                        .await;
                let (facts, revision, needs_grant) = match loaded {
                    CastFactsLoad::Ready {
                        facts,
                        character_revision,
                    } => (facts, character_revision, true),
                    CastFactsLoad::NoVocation {
                        facts,
                        character_revision,
                    } => (facts, character_revision, false),
                    CastFactsLoad::Unavailable => return Err(AdmissionRefusal::Unavailable),
                };
                let fence = self
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(|()| AdmissionRefusal::Unavailable)?
                    .ok_or(AdmissionRefusal::Rejected)?;
                if fence.item_fence() != expected {
                    return Err(AdmissionRefusal::Rejected);
                }
                if fence.expected_character_revision != revision
                    || (needs_grant
                        && !self
                            .reconcile_automatic_spells_fenced(fence, facts.level, facts.vocation)
                            .await)
                {
                    return Err(AdmissionRefusal::Unavailable);
                }
                let runtime = self.runtime.lock().await;
                if !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .or_else(|_| {
                        runtime.house_player_control_facts(actor, admitted.game_session_id)
                    })
                    .is_ok_and(|facts| facts.control_loss.is_none())
                {
                    return Err(AdmissionRefusal::Rejected);
                }
                Ok(())
            })
            .await
            .is_ok()
    }

    pub(super) async fn reconcile_automatic_spells(
        &self,
        character: CharacterId,
        session: GameSessionId,
        revision: CharacterRevision,
        level: u32,
        vocation: Vocation,
    ) -> bool {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        if fence.character_id != character || fence.expected_character_revision != revision {
            return false;
        }
        self.reconcile_automatic_spells_fenced(fence, level, vocation)
            .await
    }

    async fn reconcile_automatic_spells_fenced(
        &self,
        fence: CurrentCharacterGameplayFence,
        level: u32,
        vocation: Vocation,
    ) -> bool {
        let eligible = eligible_spell_keys(self.spells, level, vocation);
        self.root
            .acquire_automatic_spells(self.character, self.holder, fence, &eligible)
            .await
            .is_ok()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures")]
mod tests {
    use super::*;

    #[test]
    fn dispatch_planner_gates_level_vocation_and_explicit_exclusions() {
        let mut book = crate::spell::cast::v1_spell_book().expect("real native spell book");
        let candidate = book
            .spells
            .iter()
            .find(|spell| !spell.vocations.is_empty())
            .cloned()
            .expect("vocation spell");
        let vocation = *candidate.vocations.iter().next().expect("vocation");
        let mut excluded = candidate.clone();
        excluded.key.push_str(".explicitly_excluded");
        excluded.learning_required = true;
        book.spells.push(excluded.clone());

        assert!(
            !eligible_spell_keys(&book, candidate.level.saturating_sub(1), vocation)
                .contains(&candidate.key)
        );
        let learned = eligible_spell_keys(&book, candidate.level, vocation);
        assert!(learned.contains(&candidate.key));
        assert!(!learned.contains(&excluded.key));
    }

    #[test]
    fn level_up_dispatch_adds_the_newly_eligible_spell() {
        let book = crate::spell::cast::v1_spell_book().expect("real native spell book");
        let spell = book
            .spells
            .iter()
            .filter(|spell| spell.level > 0 && !spell.learning_required)
            .find(|spell| !spell.vocations.is_empty())
            .expect("level-gated vocation spell");
        let vocation = *spell.vocations.iter().next().expect("vocation");

        let before = eligible_spell_keys(&book, spell.level - 1, vocation);
        let after = eligible_spell_keys(&book, spell.level, vocation);
        assert!(!before.contains(&spell.key));
        assert!(after.contains(&spell.key));
        assert!(after.is_superset(&before));
    }

    #[test]
    fn relog_dispatch_reconciles_the_same_catalogue_without_removing_knowledge() {
        let book = crate::spell::cast::v1_spell_book().expect("real native spell book");
        let spell = book
            .spells
            .iter()
            .find(|spell| !spell.learning_required && !spell.vocations.is_empty())
            .expect("vocation spell");
        let vocation = *spell.vocations.iter().next().expect("vocation");

        let first_login = eligible_spell_keys(&book, spell.level, vocation);
        let relog = eligible_spell_keys(&book, spell.level, vocation);
        assert_eq!(relog, first_login);
        assert!(relog.contains(&spell.key));
    }
}
