//! News 8675 automatic spell acquisition through the native dispatch owner.

use std::collections::BTreeSet;

use crate::domain::CharacterId;
use crate::spell::{SpellBook, Vocation};

use super::ComposedFreshAdmission;

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
    pub(super) async fn reconcile_automatic_spells(
        &self,
        character: CharacterId,
        level: u32,
        vocation: Vocation,
    ) -> bool {
        let eligible = eligible_spell_keys(self.spells, level, vocation);
        self.root
            .acquire_automatic_spells(character, &eligible)
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
