//! Typed immutable field recipes from the pinned XML and actual source parser.
//! Missing item metadata is Unknown; a qualified non-damaging barrier is None.
use oteryn_gameplay_domain::domain::condition::DotElement;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldSource {
    pub server: String,
    pub revision: String,
    pub items_xml_sha256: String,
    pub parser_sha256: String,
    pub condition_sha256: String,
    pub creature_sha256: String,
    pub server_item_id: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldElement {
    Fire,
    Energy,
    Poison,
    Drown,
    Physical,
    /// Crystal `agony` (`item_parse.cpp` `parseFieldConditions`: `CONDITION_AGONY`), the
    /// agony field 43297 (Crystal 201189f8 `data/items/items.xml:75219`).
    Agony,
}
impl FieldElement {
    /// The damage-over-time element; `None` for `agony`, which is the §3.5 `agony` condition
    /// rather than an element of damage over time.
    pub const fn condition_element(self) -> Option<DotElement> {
        match self {
            Self::Fire => Some(DotElement::Fire),
            Self::Energy => Some(DotElement::Energy),
            Self::Poison => Some(DotElement::Poison),
            Self::Drown => Some(DotElement::Drown),
            Self::Physical => Some(DotElement::Bleeding),
            Self::Agony => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDamageEntry {
    pub amount: u32,
    pub interval_ms: u32,
    pub repetitions: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldConditionKind {
    None,
    Damage {
        element: FieldElement,
        entries: Vec<FieldDamageEntry>,
    },
    Unsupported {
        source_element: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualifiedFieldCondition {
    pub source: FieldSource,
    pub condition: FieldConditionKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldError {
    Unqualified,
    Unsupported,
    Bounds,
}
impl QualifiedFieldCondition {
    /// C07-2, owner decision 2026-10-30. Keep the pinned donor record intact;
    /// select Global damage only after that complete record has been validated.
    /// Rune/bomb/wall: TibiaQA 24717; Fire Field Rune 1189770 (9 s),
    /// Electrified 1040087 (11 s), Poisoned 937846 (4 s). The owner's poison
    /// measurement below overrides Poison Gas rev 1119032.
    /// Map fire, owner Global PKG1R4: 2123 has a separate 19 entry hit,
    /// then seven Burning ticks of 9 every 9 s; 2124 has six Burning ticks
    /// of 9 every 9 s, first on step-in, with no separate entry hit.
    /// Campfire, owner Global PKG1R6 (2026-10-30, 0% resistance): 1998 big
    /// flames shares 2123 (17:43:03 entry through 17:44:06); 1999 medium
    /// (17:46:45..17:47:30) and 2000 embers (17:36:18..17:37:03) share 2124.
    /// PKG1R7 owner-approved donor chains: 2131 shares 2118's record;
    /// Zaoan fire 10069..10071 shares 2119's record, so both use FIRE.
    /// Final fire 2120/2125 stays harmless (Fire 1011382). Safe rune products
    /// 2132 (21465's decay), 2134 and 2135 share their recipe; player immunity remains with the contact owner.
    /// Yellow flame 21465 is explicitly excluded, even when rune-created.
    fn global_entries(&self) -> Option<&'static [FieldDamageEntry]> {
        const fn entry(amount: u32, interval_ms: u32, repetitions: u32) -> FieldDamageEntry {
            FieldDamageEntry {
                amount,
                interval_ms,
                repetitions,
            }
        }
        const FIRE: &[FieldDamageEntry] = &[entry(20, 9_000, 1), entry(10, 9_000, 7)];
        /// TibiaWiki Energy Field (Field) rev 1011133 and Energy Field Rune
        /// rev 1189766: 30 on entry, then 25 after leaving the field.
        /// Electrified rev 1040087 gives the 11 s interval.
        const ENERGY: &[FieldDamageEntry] = &[entry(30, 11_000, 1), entry(25, 11_000, 1)];
        // Owner Global measurement 2026-10-30, map field, no resistances:
        // entry 4, then 4x4, 3x5, 2x7, 1x10 every 4 s (59 total).
        // Overrides TibiaWiki Poison Gas rev 1119032's 105-damage recipe.
        const POISON: &[FieldDamageEntry] = &[
            entry(4, 4_000, 5),
            entry(3, 4_000, 5),
            entry(2, 4_000, 7),
            entry(1, 4_000, 10),
        ];
        const BURNING_TICK_DAMAGE: u32 = 9;
        const BURNING_TICK_INTERVAL_MS: u32 = 9_000;
        const MAP_FIRE: &[FieldDamageEntry] = &[
            entry(19, BURNING_TICK_INTERVAL_MS, 1),
            entry(BURNING_TICK_DAMAGE, BURNING_TICK_INTERVAL_MS, 7),
        ];
        const MAP_FIRE_MEDIUM: &[FieldDamageEntry] =
            &[entry(BURNING_TICK_DAMAGE, BURNING_TICK_INTERVAL_MS, 6)];
        if self.source.server != "canary" {
            return None;
        }
        match self.source.server_item_id {
            2118 | 2119 | 2131 | 2132 | 10069..=10071 => Some(FIRE),
            2122 | 2135 => Some(ENERGY),
            105 | 2121 | 2134 => Some(POISON),
            1998 | 2123 => Some(MAP_FIRE),
            1999 | 2000 | 2124 => Some(MAP_FIRE_MEDIUM),
            _ => None,
        }
    }
    #[allow(
        dead_code,
        reason = "read only by the actual spell field owner, not mounted in every crate"
    )]
    pub fn condition_values(
        &self,
    ) -> Result<Option<oteryn_gameplay_domain::domain::condition::ConditionValues>, FieldError>
    {
        use oteryn_gameplay_domain::domain::condition::{
            ConditionValues, DotSequenceStep, MAX_DOT_SEQUENCE_STEPS,
        };
        self.validate()?;
        match &self.condition {
            FieldConditionKind::None => Ok(None),
            FieldConditionKind::Unsupported { .. } => Err(FieldError::Unsupported),
            FieldConditionKind::Damage { element, entries } => {
                // Crystal's agony field ramp has 260 runs (260 down to 1), above
                // MAX_DOT_SEQUENCE_STEPS, and no condition value carries an agony sequence, so
                // the recipe is recorded but not lowered.
                let element = element.condition_element().ok_or(FieldError::Unsupported)?;
                let entries = self.global_entries().unwrap_or(entries);
                if entries.is_empty() || self.total_damage()? == 0 {
                    return Ok(None);
                }
                if entries.len() > MAX_DOT_SEQUENCE_STEPS {
                    return Err(FieldError::Bounds);
                }
                let mut steps = [DotSequenceStep::default(); MAX_DOT_SEQUENCE_STEPS];
                for (step, entry) in steps.iter_mut().zip(entries) {
                    *step = DotSequenceStep {
                        amount: entry.amount,
                        interval_ms: entry.interval_ms,
                        repetitions: entry.repetitions,
                    };
                }
                Ok(Some(ConditionValues::DamageSequence {
                    element,
                    steps,
                    len: u8::try_from(entries.len()).map_err(|_| FieldError::Bounds)?,
                    delayed: false,
                }))
            }
        }
    }
    pub fn validate(&self) -> Result<(), FieldError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Snapshot {
            schema: String,
            profiles: Vec<QualifiedFieldCondition>,
        }
        static SNAPSHOT: OnceLock<Option<Vec<QualifiedFieldCondition>>> = OnceLock::new();
        let profiles=SNAPSHOT.get_or_init(|| {
            let catalog:Snapshot=serde_json::from_str(include_str!(
                "../../../../../tools/content-schema/spell-authoring/samples/native-field-profiles.json"
            )).ok()?;
            (catalog.schema=="OTERYN_NATIVE_FIELD_PROFILES/v1" && catalog.profiles.len()==90)
                .then_some(catalog.profiles)
        }).as_ref().ok_or(FieldError::Unqualified)?;
        if !profiles.contains(self) {
            return Err(FieldError::Unqualified);
        }
        Ok(())
    }
    pub fn validate_for(&self, server: &str, item: u32) -> Result<(), FieldError> {
        self.validate()?;
        if self.source.server != server || self.source.server_item_id != item {
            return Err(FieldError::Unqualified);
        }
        Ok(())
    }
    pub fn element(&self) -> Result<Option<DotElement>, FieldError> {
        self.validate()?;
        match &self.condition {
            FieldConditionKind::None => Ok(None),
            FieldConditionKind::Damage { element, .. } => element
                .condition_element()
                .map(Some)
                .ok_or(FieldError::Unsupported),
            FieldConditionKind::Unsupported { .. } => Err(FieldError::Unsupported),
        }
    }
    pub fn total_damage(&self) -> Result<u32, FieldError> {
        self.validate()?;
        match &self.condition {
            FieldConditionKind::None => Ok(0),
            FieldConditionKind::Unsupported { .. } => Err(FieldError::Unsupported),
            FieldConditionKind::Damage { entries, .. } => {
                let entries = self.global_entries().unwrap_or(entries);
                entries.iter().try_fold(0u32, |sum, entry| {
                    sum.checked_add(
                        entry
                            .amount
                            .checked_mul(entry.repetitions)
                            .ok_or(FieldError::Bounds)?,
                    )
                    .ok_or(FieldError::Bounds)
                })
            }
        }
    }
}
#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    fn profile(item: u32) -> QualifiedFieldCondition {
        let catalog: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tools/content-schema/spell-authoring/samples/native-field-profiles.json"
        ))
        .unwrap();
        serde_json::from_value(
            catalog["profiles"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| {
                    r["source"]["server"] == "canary" && r["source"]["server_item_id"] == item
                })
                .unwrap()
                .clone(),
        )
        .unwrap()
    }
    #[test]
    fn source_poison_preserves_ramp_fire_count_and_barrier_known_none() {
        let poison = profile(105);
        assert_eq!(poison.element().unwrap(), Some(DotElement::Poison));
        assert_eq!(poison.total_damage().unwrap(), 59);
        let FieldConditionKind::Damage { entries, .. } = poison.condition else {
            panic!("damage")
        };
        assert_eq!(
            entries
                .iter()
                .map(|e| (e.amount, e.repetitions))
                .collect::<Vec<_>>(),
            vec![(5, 4), (4, 5), (3, 7), (2, 9), (1, 21)]
        );
        assert_eq!(profile(2118).total_damage().unwrap(), 90);
        assert_eq!(profile(2128).element().unwrap(), None);
        assert_eq!(profile(2130).element().unwrap(), None);
    }
    #[test]
    fn invented_element_damage_id_pin_and_unknown_fields_refuse() {
        let valid = profile(2118);
        assert!(valid.validate_for("canary", 2118).is_ok());
        assert!(valid.validate_for("canary", 2128).is_err());
        let mut wrong = valid.clone();
        wrong.source.revision = "HEAD".into();
        assert!(wrong.validate().is_err());
        let mut wrong = valid.clone();
        wrong.condition = FieldConditionKind::None;
        assert!(wrong.validate().is_err());
        let mut value = serde_json::to_value(valid).unwrap();
        value["invented"] = serde_json::json!(true);
        assert!(serde_json::from_value::<QualifiedFieldCondition>(value).is_err());
    }
}
#[cfg(test)]
#[path = "field_condition_crystal_parity_tests.rs"]
mod crystal_parity_tests;
