//! Typed immutable field recipes from the pinned XML and actual source parser.
//! Missing item metadata is Unknown; a qualified non-damaging barrier is None.
use super::condition::DotElement;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FieldSource {
    pub(crate) server: String,
    pub(crate) revision: String,
    pub(crate) items_xml_sha256: String,
    pub(crate) parser_sha256: String,
    pub(crate) condition_sha256: String,
    pub(crate) creature_sha256: String,
    pub(crate) server_item_id: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FieldElement {
    Fire,
    Energy,
    Poison,
    Drown,
    Physical,
}
impl FieldElement {
    pub(crate) const fn condition_element(self) -> DotElement {
        match self {
            Self::Fire => DotElement::Fire,
            Self::Energy => DotElement::Energy,
            Self::Poison => DotElement::Poison,
            Self::Drown => DotElement::Drown,
            Self::Physical => DotElement::Bleeding,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FieldDamageEntry {
    pub(crate) amount: u32,
    pub(crate) interval_ms: u32,
    pub(crate) repetitions: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum FieldConditionKind {
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
pub(crate) struct QualifiedFieldCondition {
    pub(crate) source: FieldSource,
    pub(crate) condition: FieldConditionKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldError {
    Unqualified,
    Unsupported,
    Bounds,
}
impl QualifiedFieldCondition {
    pub(crate) fn condition_values(
        &self,
    ) -> Result<Option<super::condition::ConditionValues>, FieldError> {
        use super::condition::{ConditionValues, DotSequenceStep, MAX_DOT_SEQUENCE_STEPS};
        self.validate()?;
        match &self.condition {
            FieldConditionKind::None => Ok(None),
            FieldConditionKind::Unsupported { .. } => Err(FieldError::Unsupported),
            FieldConditionKind::Damage { element, entries } => {
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
                    element: element.condition_element(),
                    steps,
                    len: u8::try_from(entries.len()).map_err(|_| FieldError::Bounds)?,
                    delayed: false,
                }))
            }
        }
    }
    pub(crate) fn validate(&self) -> Result<(), FieldError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Snapshot {
            schema: String,
            profiles: Vec<QualifiedFieldCondition>,
        }
        static SNAPSHOT: OnceLock<Option<Vec<QualifiedFieldCondition>>> = OnceLock::new();
        let profiles=SNAPSHOT.get_or_init(|| {
            let catalog:Snapshot=serde_json::from_str(include_str!(
                "../../../../tools/content-schema/spell-authoring/samples/native-field-profiles.json"
            )).ok()?;
            (catalog.schema=="OTERYN_NATIVE_FIELD_PROFILES/v1" && catalog.profiles.len()==90)
                .then_some(catalog.profiles)
        }).as_ref().ok_or(FieldError::Unqualified)?;
        if !profiles.contains(self) {
            return Err(FieldError::Unqualified);
        }
        Ok(())
    }
    pub(crate) fn validate_for(&self, server: &str, item: u32) -> Result<(), FieldError> {
        self.validate()?;
        if self.source.server != server || self.source.server_item_id != item {
            return Err(FieldError::Unqualified);
        }
        Ok(())
    }
    pub(crate) fn element(&self) -> Result<Option<DotElement>, FieldError> {
        self.validate()?;
        match &self.condition {
            FieldConditionKind::None => Ok(None),
            FieldConditionKind::Damage { element, .. } => Ok(Some(element.condition_element())),
            FieldConditionKind::Unsupported { .. } => Err(FieldError::Unsupported),
        }
    }
    pub(crate) fn total_damage(&self) -> Result<u32, FieldError> {
        self.validate()?;
        match &self.condition {
            FieldConditionKind::None => Ok(0),
            FieldConditionKind::Unsupported { .. } => Err(FieldError::Unsupported),
            FieldConditionKind::Damage { entries, .. } => {
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
            "../../../../tools/content-schema/spell-authoring/samples/native-field-profiles.json"
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
        assert_eq!(poison.total_damage().unwrap(), 100);
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
        assert_eq!(profile(2118).total_damage().unwrap(), 140);
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
