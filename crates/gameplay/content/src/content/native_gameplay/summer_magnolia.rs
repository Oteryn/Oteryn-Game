//! Accepted ENCOUNTER-RT0 §17.2: one public Magnolia identity, two lives.
use super::*;
use sha2::{Digest, Sha256};

/// Qualified from the active exact source profile, not an arbitrary HP override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedMagnoliaSecondLife {
    profile: [u8; 32],
    artifact: [u8; 32],
    policy: crate::content::creature_policy::CompiledCreaturePolicy,
}
impl QualifiedMagnoliaSecondLife {
    pub(in crate::content) fn bind_outer_artifact(&mut self, digest: [u8; 32]) {
        self.artifact = digest;
    }
    pub const fn artifact_digest(&self) -> [u8; 32] {
        self.artifact
    }
    pub fn policy(&self) -> &crate::content::creature_policy::CompiledCreaturePolicy {
        &self.policy
    }
    pub const fn profile_digest(&self) -> [u8; 32] {
        self.profile
    }
    pub const fn creature_key(&self) -> &'static str {
        "oteryn:creature.the_moonsnow_magnolia"
    }
    pub const fn first_health(&self) -> i64 {
        52_000
    }
    pub const fn second_health(&self) -> i64 {
        60_000
    }
}
impl NativeGameplayState {
    pub fn qualify_magnolia_second_life(&self) -> Option<QualifiedMagnoliaSecondLife> {
        let record = self.creature_profiles().records.iter().find(|record| {
            record.profile.target.key == "oteryn:creature.the_moonsnow_magnolia"
                && record.profile.target.revision == "definition-r1"
        })?;
        qualify_source(record, self.policies())
    }
}

/// Component tests use the same native policy decoder and exact sealed source
/// qualifier. No synthetic policy fields enter this test-support API.
#[cfg(any(test, feature = "test-support"))]
pub fn qualify_magnolia_second_life_test_source(
    creature_bytes: &[u8],
    presentation_bytes: &[u8],
    digest: [u8; 32],
) -> Option<QualifiedMagnoliaSecondLife> {
    let records: CreatureProfilesDocument = serde_json::from_slice(creature_bytes).ok()?;
    let policies =
        callback_source_native_test_policies(creature_bytes, presentation_bytes, digest).ok()?;
    let record = records.records.iter().find(|record| {
        record.profile.target.key == "oteryn:creature.the_moonsnow_magnolia"
            && record.profile.target.revision == "definition-r1"
    })?;
    qualify_source(record, &policies)
}

fn qualify_source(
    profile: &CreatureProfileRecord,
    policies: &CompiledCreaturePolicies,
) -> Option<QualifiedMagnoliaSecondLife> {
    let raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../tools/content-schema/native-gameplay/magnolia-source-native-record.json"
    ))
    .ok()?;
    let encoded = serde_json::to_vec(&raw).ok()?;
    let digest: [u8; 32] = Sha256::digest(encoded).into();
    let expected = [
        0x4d, 0xce, 0x44, 0x03, 0x91, 0x98, 0xd5, 0x85, 0x31, 0xd9, 0xf5, 0x98, 0x68, 0xe4, 0xd3,
        0x5e, 0xa5, 0x4d, 0xfd, 0x3c, 0x7a, 0xed, 0x33, 0xc2, 0x04, 0x43, 0x8b, 0x0a, 0x71, 0x6a,
        0x15, 0xca,
    ];
    let expected_record: CreatureProfileRecord = serde_json::from_value(raw).ok()?;
    if digest != expected || profile != &expected_record {
        return None;
    }
    let policy = policies
        .policies()
        .iter()
        .find(|policy| {
            policy.definition_key == "oteryn:creature.the_moonsnow_magnolia"
                && policy.definition_revision == "definition-r1"
                && policy.maximum_health == 52_000
        })?
        .as_ref()
        .clone();
    Some(QualifiedMagnoliaSecondLife {
        profile: digest,
        artifact: policies.source_digest(),
        policy,
    })
}

pub(super) fn source_qualification(
    records: &CreatureProfilesDocument,
    policies: &CompiledCreaturePolicies,
) -> Option<QualifiedMagnoliaSecondLife> {
    let record = records.records.iter().find(|record| {
        record.profile.target.key == "oteryn:creature.the_moonsnow_magnolia"
            && record.profile.target.revision == "definition-r1"
    })?;
    qualify_source(record, policies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_generated_magnolia_record_retains_native_phase_qualification()
    -> Result<(), Box<dyn std::error::Error>> {
        let creatures = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/creatures/definitions/spell-native-profiles.json"
        ))?;
        let presentations = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/presentations/definitions/spell-native-profiles.json"
        ))?;
        // Read the actual generated owner output, not a deepcopy of the seal.
        let policies = callback_source_native_test_policies(&creatures, &presentations, [1; 32])
            .map_err(|error| format!("{error:?}"))?;
        let proof = policies
            .magnolia_second_life()
            .ok_or("actual generated Magnolia silently lost its native phase qualification")?;
        let independently_qualified =
            qualify_magnolia_second_life_test_source(&creatures, &presentations, [1; 32])
                .ok_or("actual generated Magnolia source record is not source-qualified")?;
        assert_eq!(proof, &independently_qualified);
        assert_eq!(proof.policy().maximum_health, 52_000);
        assert_eq!(proof.artifact_digest(), [1; 32]);
        Ok(())
    }
}
