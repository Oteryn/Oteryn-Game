//! Production [`ProficiencyShapingSource`] over the committed Global shaping values
//! (PROFICIENCY-1C). A definition's shaping key resolves to its vocation's 64-perk pool. Only the
//! donor-evidenced cells are exposed: rank 0 through rank 10 values, the slot, reshape and clear
//! costs and the orb count.

use super::ProductionProficiencyCatalogue;
use crate::durability::character_proficiency_modification::ProficiencyShapingSource;
use oteryn_gameplay_content::content::{
    ImportedProficiencyShaping, ImportedShapingUnit, ProjectV2ExactRatio,
    import_committed_proficiency_shaping, proficiency_perk_identity,
};
use oteryn_gameplay_domain::domain::weapon_proficiency::{
    ProficiencyModifiedPerk, ProficiencyShapingEntry, ProficiencyShapingRevision,
};
use std::sync::{Arc, OnceLock};

/// Immutable shaping content bound to the catalogue whose perks it modifies.
#[derive(Debug)]
pub struct ProductionProficiencyShaping {
    shaping: Arc<ImportedProficiencyShaping>,
    catalogue: Arc<ProductionProficiencyCatalogue>,
}

/// The committed shaping values, imported once per process.
fn committed() -> Option<&'static Arc<ImportedProficiencyShaping>> {
    static SHAPING: OnceLock<Option<Arc<ImportedProficiencyShaping>>> = OnceLock::new();
    SHAPING
        .get_or_init(|| import_committed_proficiency_shaping().ok().map(Arc::new))
        .as_ref()
}

#[cfg(test)]
fn retired_r1() -> Option<&'static Arc<ImportedProficiencyShaping>> {
    static SHAPING: OnceLock<Option<Arc<ImportedProficiencyShaping>>> = OnceLock::new();
    SHAPING
        .get_or_init(|| {
            oteryn_gameplay_content::content::import_retired_proficiency_shaping_r1()
                .ok()
                .map(Arc::new)
        })
        .as_ref()
}

fn revision(
    shaping: &ImportedProficiencyShaping,
    shaping_key: &str,
) -> Option<ProficiencyShapingRevision> {
    let definition = shaping_key
        .strip_prefix("oteryn:proficiency-shaping.")
        .map(|rest| format!("oteryn:proficiency.{rest}"))?;
    let pool = shaping
        .pools
        .get(shaping.definition_vocations.get(&definition)?)?;
    let [first, second] = shaping.modify_dust;
    let revision = ProficiencyShapingRevision {
        shaping_key: shaping_key.to_owned(),
        revision: shaping.revision.clone(),
        pool: Some(
            pool.iter()
                .map(|entry| ProficiencyShapingEntry {
                    perk_identity: entry.identity.clone(),
                    weight: Some(shaping.uniform_weight),
                    rank_values: std::array::from_fn(|rank| {
                        shaping.revision != "shaping-r1" || rank == 0 || rank == 10
                    }),
                })
                .collect(),
        ),
        modify_cost: [Some(first), Some(second)],
        rank_step_cost: if shaping.revision == "shaping-r1" {
            [None; 10]
        } else {
            shaping.rank_step_dust.map(Some)
        },
        reshape_offer_cost: Some(shaping.reshape_offer_dust),
        clear_cost: Some(shaping.clear_dust),
        orb_count: Some(shaping.orb_count),
    };
    revision.is_well_formed().then_some(revision)
}

impl ProductionProficiencyShaping {
    pub fn embedded(catalogue: Arc<ProductionProficiencyCatalogue>) -> Option<Arc<Self>> {
        Some(Arc::new(Self {
            shaping: Arc::clone(committed()?),
            catalogue,
        }))
    }
}

/// PROF-EFFECT-0 value of a stored modification row. The row must name `definition_key`'s shaping
/// key and the committed revision; any other row has no known value.
pub(super) fn modified_value(
    row: &ProficiencyModifiedPerk,
    definition_key: &str,
) -> Option<(&'static str, ImportedShapingUnit, ProjectV2ExactRatio)> {
    let shaping = committed()?;
    let rest = definition_key.strip_prefix("oteryn:proficiency.")?;
    if row.shaping_key != format!("oteryn:proficiency-shaping.{rest}")
        || row.shaping_revision != shaping.revision
    {
        return None;
    }
    let entry = shaping
        .pools
        .get(shaping.definition_vocations.get(definition_key)?)?
        .get(usize::from(row.entry_index))?;
    let value = *entry.rank_values.get(usize::from(row.rank))?;
    Some((entry.identity.as_str(), entry.unit, value))
}

impl ProficiencyShapingSource for ProductionProficiencyShaping {
    fn active(&self, shaping_key: &str) -> Option<ProficiencyShapingRevision> {
        revision(&self.shaping, shaping_key)
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
        // MODIFY lands on rank 0 and every donor-evidenced rank step is admitted.
        assert!(
            revision
                .admitted(
                    ProficiencyModificationCommandKind::Modify { level: 0 },
                    1,
                    None
                )
                .is_some()
        );
        assert_eq!(
            revision.rank_step_cost,
            [
                Some(125),
                Some(200),
                Some(275),
                Some(350),
                Some(425),
                Some(500),
                Some(575),
                Some(650),
                Some(725),
                Some(800),
            ]
        );
        assert!(
            revision
                .admitted(ProficiencyModificationCommandKind::Clear, 1, None)
                .is_some()
        );
    }

    #[test]
    fn shaping_r2_admits_rank_up_while_retained_r1_rejects_it() {
        let row = ProficiencyModifiedPerk {
            level: 0,
            shaping_key: "oteryn:proficiency-shaping.tibia.p6".into(),
            shaping_revision: "shaping-r2".into(),
            entry_index: 0,
            rank: 0,
            pending_offer: None,
        };
        let current = source().active(&row.shaping_key).expect("current revision");
        assert_eq!(current.revision, "shaping-r2");
        assert!(
            current
                .admitted(ProficiencyModificationCommandKind::RankUp, 1, Some(&row))
                .is_some()
        );

        let old = revision(retired_r1().expect("retained r1"), &row.shaping_key)
            .expect("retained revision");
        assert_eq!(old.revision, "shaping-r1");
        assert!(
            old.admitted(ProficiencyModificationCommandKind::RankUp, 1, Some(&row))
                .is_none()
        );
    }

    #[test]
    fn migration_0915_activates_exactly_the_shaped_definitions() {
        const MIGRATION: &str = include_str!(
            "../../../../../apps/game-server/migrations/0915_proficiency_shaping_r2.sql"
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
        assert!(MIGRATION.contains(&format!("'{}', ARRAY['shaping-r1']", shaping.revision)));
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
