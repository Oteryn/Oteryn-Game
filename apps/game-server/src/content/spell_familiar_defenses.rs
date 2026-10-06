//! Explicit source-pinned familiar defenses, selected by full active Creature reference.
//! The two Canary datapacks have conflicting heals; neither a name nor missing data selects one.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use crate::foundation::{CompiledCreaturePolicy, FamiliarSelfHealDefense};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcePin {
    head: String,
    path: String,
    blob: String,
    sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DefenseRecord {
    creature_key: String,
    creature_revision: String,
    source: SourcePin,
    definition_source: SourcePin,
    maximum_health: i64,
    base_speed: i32,
    interval_ms: u32,
    chance_percent: u8,
    heal_min: i64,
    heal_max: i64,
    effect: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    records: Vec<DefenseRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledFamiliarDefenses {
    source_digest: [u8; 32],
    records: Vec<FamiliarSelfHealDefense>,
}
impl CompiledFamiliarDefenses {
    pub(crate) fn from_active_artifact(
        bytes: &[u8],
        digest: [u8; 32],
        creatures: &[CompiledCreaturePolicy],
    ) -> Result<Self, &'static str> {
        if digest == [0; 32] || bytes.len() > 32 * 1024 {
            return Err("unqualified familiar defense artifact");
        }
        let document: Document =
            serde_json::from_slice(bytes).map_err(|_| "familiar defense document")?;
        let canonical: Document = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/native-gameplay/familiar-defenses-source-vectors.json"
        ))
        .map_err(|_| "canonical familiar source vectors")?;
        if document.schema != canonical.schema || document.records.len() > 9 {
            return Err("familiar defense schema");
        }
        let mut records = Vec::new();
        for record in document.records {
            // Entire source HEAD/blob/hash, target binding and interval/chance/magnitude/cue
            // must match an independently captured pinned source vector, including datapack.
            if !canonical.records.contains(&record)
                || records.iter().any(|r: &FamiliarSelfHealDefense| {
                    r.creature_key == record.creature_key
                        && r.creature_revision == record.creature_revision
                })
            {
                return Err("familiar defense source conflict");
            }
            let creature = creatures
                .iter()
                .find(|r| {
                    r.definition_key == record.creature_key
                        && r.definition_revision == record.creature_revision
                })
                .ok_or("familiar defense missing actual Creature")?;
            if !creature.is_familiar
                || creature.maximum_health != record.maximum_health
                || creature.base_speed != record.base_speed
                || record.interval_ms == 0
                || record.chance_percent > 100
                || record.heal_min <= 0
                || record.heal_min > record.heal_max
            {
                return Err("familiar defense actual Creature mismatch");
            }
            let profile_digest =
                Sha256::digest(serde_json::to_vec(&record).map_err(|_| "defense source encoding")?)
                    .into();
            records.push(FamiliarSelfHealDefense {
                source_digest: digest,
                creature_key: record.creature_key,
                creature_revision: record.creature_revision,
                profile_digest,
                interval_ms: record.interval_ms,
                chance_percent: record.chance_percent,
                heal_min: record.heal_min,
                heal_max: record.heal_max,
                effect: record.effect,
            });
        }
        Ok(Self {
            source_digest: digest,
            records,
        })
    }
    pub(crate) fn defense(&self, key: &str, revision: &str) -> Option<&FamiliarSelfHealDefense> {
        self.records
            .iter()
            .find(|r| r.creature_key == key && r.creature_revision == revision)
    }
    pub(crate) fn source_digest(&self) -> [u8; 32] {
        self.source_digest
    }
    /// Data-only outer envelope binding; does not mint runtime authority.
    pub(crate) fn bind_qualified_outer_artifact(
        &mut self,
        digest: [u8; 32],
    ) -> Result<(), &'static str> {
        if digest == [0; 32] {
            return Err("unqualified familiar outer artifact");
        }
        self.source_digest = digest;
        for record in &mut self.records {
            record.source_digest = digest;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    fn document() -> Document {
        serde_json::from_str(include_str!(
            "../../../../tools/content-schema/native-gameplay/familiar-defenses.json"
        ))
        .unwrap()
    }
    fn policy(record: &DefenseRecord) -> CompiledCreaturePolicy {
        CompiledCreaturePolicy {
            definition_key: record.creature_key.clone(),
            definition_revision: record.creature_revision.clone(),
            display_name: "Source-qualified test familiar".into(),
            maximum_health: record.maximum_health,
            base_speed: record.base_speed,
            outfit_look_type: 991,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: true,
            condition_immunities: Vec::new(),
            armor: None,
            mitigation: None,
            resistances: Vec::new(),
            damage_immunities: Vec::new(),
            healing_from_damage: vec![],
            flags: crate::foundation::CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
            preferred_distance: None,
            reward_boss: Some(false),
        }
    }
    #[test]
    fn source_defenses_require_full_pins_and_actual_creature_and_keep_datapack_conflict_explicit() {
        let mut doc = document();
        let creatures: Vec<_> = doc.records.iter().map(policy).collect();
        let bytes = serde_json::to_vec(&doc).unwrap();
        let mut compiled =
            CompiledFamiliarDefenses::from_active_artifact(&bytes, [1; 32], &creatures).unwrap();
        let knight = compiled
            .defense("canary:creature/knight_familiar", "canary-47dfd51f")
            .unwrap();
        assert_eq!(
            (
                knight.interval_ms,
                knight.chance_percent,
                knight.heal_min,
                knight.heal_max
            ),
            (2000, 75, 300, 300)
        );
        assert!(
            compiled
                .defense("canary:creature/knight_familiar", "unqualified-revision")
                .is_none()
        );
        let profile = knight.profile_digest;
        compiled.bind_qualified_outer_artifact([2; 32]).unwrap();
        let rebound = compiled
            .defense("canary:creature/knight_familiar", "canary-47dfd51f")
            .unwrap();
        assert_eq!(rebound.source_digest, [2; 32]);
        assert_eq!(rebound.profile_digest, profile);
        assert!(
            CompiledFamiliarDefenses::from_active_artifact(&bytes, [0; 32], &creatures).is_err()
        );
        assert!(CompiledFamiliarDefenses::from_active_artifact(&bytes, [1; 32], &[]).is_err());
        let original = doc.records[0].clone();
        doc.records[0].source.head = "unqualified".into();
        assert!(
            CompiledFamiliarDefenses::from_active_artifact(
                &serde_json::to_vec(&doc).unwrap(),
                [1; 32],
                &creatures
            )
            .is_err()
        );
        doc.records[0] = original.clone();
        doc.records[0].heal_min += 1;
        assert!(
            CompiledFamiliarDefenses::from_active_artifact(
                &serde_json::to_vec(&doc).unwrap(),
                [1; 32],
                &creatures
            )
            .is_err()
        );
        doc.records[0] = original;
        let all: Document = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/native-gameplay/familiar-defenses-source-vectors.json"
        ))
        .unwrap();
        doc.records.push(
            all.records
                .iter()
                .find(|r| {
                    r.creature_key == "canary:creature/knight_familiar"
                        && r.source.path.starts_with("data-canary/")
                })
                .unwrap()
                .clone(),
        );
        assert!(
            CompiledFamiliarDefenses::from_active_artifact(
                &serde_json::to_vec(&doc).unwrap(),
                [1; 32],
                &creatures
            )
            .is_err()
        );
        let mut changed = creatures;
        changed[0].maximum_health += 1;
        assert!(CompiledFamiliarDefenses::from_active_artifact(&bytes, [1; 32], &changed).is_err());
    }
}
