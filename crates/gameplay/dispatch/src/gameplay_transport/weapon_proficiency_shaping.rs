//! Production [`ProficiencyShapingSource`] over the committed Global shaping values
//! (PROFICIENCY-1C). A definition's shaping key resolves to its vocation's 64-perk pool. Only the
//! evidenced cells are exposed: rank 0 and rank 10 values, the slot, reshape and clear costs and
//! the orb count. Rank 1-9 values are unpublished, so `RANK_UP` answers `NOT_ADMITTED` (1C §3).

use super::ProductionProficiencyCatalogue;
use crate::durability::character_proficiency_modification::ProficiencyShapingSource;
use oteryn_gameplay_content::content::{
    ImportedProficiencyShaping, import_committed_proficiency_shaping, proficiency_perk_identity,
};
use oteryn_gameplay_domain::domain::weapon_proficiency::{
    PROFICIENCY_MODIFICATION_RANK_MAX, PROFICIENCY_MODIFICATION_RANKS, ProficiencyShapingEntry,
    ProficiencyShapingRevision,
};
use std::sync::{Arc, OnceLock};

/// Immutable shaping content bound to the catalogue whose perks it modifies.
#[derive(Debug)]
pub struct ProductionProficiencyShaping {
    shaping: Arc<ImportedProficiencyShaping>,
    catalogue: Arc<ProductionProficiencyCatalogue>,
}

impl ProductionProficiencyShaping {
    /// The committed shaping values, imported once per process.
    pub fn embedded(catalogue: Arc<ProductionProficiencyCatalogue>) -> Option<Arc<Self>> {
        static SHAPING: OnceLock<Option<Arc<ImportedProficiencyShaping>>> = OnceLock::new();
        Some(Arc::new(Self {
            shaping: Arc::clone(
                SHAPING
                    .get_or_init(|| import_committed_proficiency_shaping().ok().map(Arc::new))
                    .as_ref()?,
            ),
            catalogue,
        }))
    }
}

impl ProficiencyShapingSource for ProductionProficiencyShaping {
    fn active(&self, shaping_key: &str) -> Option<ProficiencyShapingRevision> {
        let definition = shaping_key
            .strip_prefix("oteryn:proficiency-shaping.")
            .map(|rest| format!("oteryn:proficiency.{rest}"))?;
        let pool = self
            .shaping
            .pools
            .get(self.shaping.definition_vocations.get(&definition)?)?;
        let mut rank_values = [false; PROFICIENCY_MODIFICATION_RANKS];
        rank_values[0] = true;
        rank_values[usize::from(PROFICIENCY_MODIFICATION_RANK_MAX)] = true;
        let [first, second] = self.shaping.modify_dust;
        let revision = ProficiencyShapingRevision {
            shaping_key: shaping_key.to_owned(),
            revision: self.shaping.revision.clone(),
            pool: Some(
                pool.iter()
                    .map(|entry| ProficiencyShapingEntry {
                        perk_identity: entry.identity.clone(),
                        weight: Some(self.shaping.uniform_weight),
                        rank_values,
                    })
                    .collect(),
            ),
            modify_cost: [Some(first), Some(second)],
            rank_step_cost: [None; PROFICIENCY_MODIFICATION_RANK_MAX as usize],
            reshape_offer_cost: Some(self.shaping.reshape_offer_dust),
            clear_cost: Some(self.shaping.clear_dust),
            orb_count: Some(self.shaping.orb_count),
        };
        revision.is_well_formed().then_some(revision)
    }

    fn perk_identity(
        &self,
        definition_key: &str,
        definition_revision: &str,
        level: u8,
        perk_index: u8,
    ) -> Option<String> {
        let definition = self.catalogue.data.definitions().get(definition_key)?;
        if definition.identity.revision != definition_revision {
            return None;
        }
        definition
            .levels
            .get(usize::from(level))?
            .perks
            .get(usize::from(perk_index))
            .and_then(proficiency_perk_identity)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use oteryn_gameplay_domain::domain::weapon_proficiency::ProficiencyModificationCommandKind;

    fn source() -> Arc<ProductionProficiencyShaping> {
        let catalogue = ProductionProficiencyCatalogue::embedded().expect("catalogue");
        ProductionProficiencyShaping::embedded(catalogue).expect("shaping")
    }

    #[test]
    fn a_knight_sword_resolves_the_knight_pool_and_global_costs() {
        let revision = source()
            .active("oteryn:proficiency-shaping.tibia.p6")
            .expect("Sanguine 1H Sword has shaping");
        let pool = revision.pool.as_ref().expect("pool");
        assert_eq!(pool.len(), 64);
        assert!(pool.iter().all(|entry| entry.weight == Some(1)));
        assert!(
            pool.iter()
                .any(|entry| entry.perk_identity == "spell_augment:critical_extra_damage:105")
        );
        assert_eq!(revision.modify_cost, [Some(250), Some(1000)]);
        assert_eq!(
            (
                revision.reshape_offer_cost,
                revision.clear_cost,
                revision.orb_count
            ),
            (Some(250), Some(0), Some(1))
        );
        // MODIFY lands on the evidenced rank 0; ranks 1-9 are unpublished, so RANK_UP is not.
        assert!(
            revision
                .admitted(
                    ProficiencyModificationCommandKind::Modify { level: 0 },
                    1,
                    None
                )
                .is_some()
        );
        assert!(revision.rank_step_cost.iter().all(Option::is_none));
        assert!(
            revision
                .admitted(ProficiencyModificationCommandKind::Clear, 1, None)
                .is_some()
        );
    }

    #[test]
    fn migration_0575_activates_exactly_the_shaped_definitions() {
        const MIGRATION: &str = include_str!(
            "../../../../../apps/game-server/migrations/0575_proficiency_perk_shaping.sql"
        );
        let shaping = import_committed_proficiency_shaping().expect("shaping");
        let list = &MIGRATION[MIGRATION.find("unnest(ARRAY[").expect("activation list")..];
        let list = &list[..list.find("]::TEXT[]").expect("list end")];
        let activated: std::collections::BTreeSet<_> =
            list.split('\'').skip(1).step_by(2).collect();
        let shaped: std::collections::BTreeSet<_> = shaping
            .definition_vocations
            .keys()
            .filter_map(|key| key.strip_prefix("oteryn:proficiency."))
            .map(|rest| format!("oteryn:proficiency-shaping.{rest}"))
            .collect();
        assert_eq!(
            activated,
            shaped.iter().map(String::as_str).collect(),
            "a shaping.json definition change needs a forward activation migration"
        );
        assert!(MIGRATION.contains(&format!("'{}', NULL)", shaping.revision)));
    }

    #[test]
    fn dual_vocation_casters_and_unknown_keys_have_no_shaping() {
        let source = source();
        assert!(
            source
                .active("oteryn:proficiency-shaping.tibia.p217")
                .is_none()
        );
        assert!(source.active("oteryn:proficiency.tibia.p6").is_none());
    }

    #[test]
    fn a_selected_perk_identity_matches_its_pool_entry() {
        let source = source();
        // p6 level 3 perk 1 is Fierce Berserk's critical extra damage augment.
        assert_eq!(
            source
                .perk_identity("oteryn:proficiency.tibia.p6", "definition-r1", 2, 1)
                .as_deref(),
            Some("spell_augment:critical_extra_damage:105")
        );
        assert!(
            source
                .perk_identity("oteryn:proficiency.tibia.p6", "definition-r2", 2, 1)
                .is_none()
        );
    }
}
