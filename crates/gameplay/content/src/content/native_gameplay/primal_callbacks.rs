//! Source-qualified W10 callback closure shared by physical and durable owners.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceWitness {
    schema: String,
    mechanics_source_revision: String,
    source_definitions_sha256: String,
    source_bindings: Vec<crate::content::ProjectV2SourceIdentityBinding>,
    callbacks: std::collections::BTreeMap<String, String>,
    records: Vec<super::CreatureProfileRecord>,
    encounter_profiles: Vec<crate::content::ProjectV2AuthoringProfile>,
}
fn witness() -> Option<&'static SourceWitness> {
    static WITNESS: std::sync::OnceLock<Option<SourceWitness>> = std::sync::OnceLock::new();
    WITNESS
        .get_or_init(|| {
            serde_json::from_slice(include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/srx3-primal-witness.json"
            ))
            .ok()
        })
        .as_ref()
}
fn qualifies_document(document: &super::CreatureProfilesDocument) -> bool {
    use crate::content::ProjectV2SourceIdentityDisposition;
    let Some(witness) = witness() else {
        return false;
    };
    witness.schema == "OTERYN_SRX3_PRIMAL_CALLBACK_WITNESS/v1"
        && witness.mechanics_source_revision == "201189f871d23597b82caf88fd611454a2ddefb8"
        && document.source_definitions_sha256.as_deref()
            == Some(witness.source_definitions_sha256.as_str())
        && witness.records.len() == 13
        && witness.source_bindings.len() == 13
        && witness.callbacks.len() == 3
        && witness.encounter_profiles.len() == 1
        && witness.encounter_profiles.iter().all(|profile| {
            profile.target.family == crate::content::ProjectV2Family::Encounter
                && profile.target.key == "oteryn:encounter.the_primal_menace"
                && profile.target.revision == "definition-r1"
                && matches!(&profile.data, crate::content::ProjectV2AuthoringProfileData::Encounter(data)
                    if data.scope == crate::content::ProjectV2EncounterScope::Instance
                        && data.encounter_type == crate::content::ProjectV2EncounterType::Boss)
        })
        && witness.records.iter().all(|expected| {
            let mut records = document
                .records
                .iter()
                .filter(|record| record.profile.target == expected.profile.target);
            records.next() == Some(expected) && records.next().is_none()
        })
        && witness.source_bindings.iter().all(|binding| {
            binding.target.revision == "definition-r1"
                && binding.source_key == "oteryn:source.canary"
                && binding.source_revision == "47dfd51f45280a59a1d3e50ba7edd573d7234446"
                && binding.identity_namespace == "canary/monster-file"
                && binding.disposition == ProjectV2SourceIdentityDisposition::Exact
                && witness
                    .records
                    .iter()
                    .any(|record| record.profile.target == binding.target)
        })
}

impl super::NativeGameplayState {
    /// Exact bounded Primal callback closure under this current loader-decoded pin.
    /// The witness carries no runtime grant, actor identity or placement authority.
    pub fn qualifies_primal_callbacks(&self, active_server_digest: [u8; 32]) -> bool {
        self.source_digest() == active_server_digest && qualifies_document(self.creature_profiles())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_primal_callback_witness_requires_actual_full_native_closure()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .name("srx3-primal-source".into())
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let world = oteryn_protocol_oteryn::WorldId::decode(&[
                    1, 144, 0, 0, 0, 8, 112, 0, 128, 0, 0, 0, 0, 0, 0, 8,
                ])
                .map_err(|error| format!("{error:?}"))?;
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/spells.manifest.json");
                let native = crate::content::native_gameplay::retained_callback_native_fixture_at(
                    world, &path,
                );
                assert!(native.qualifies_primal_callbacks(native.source_digest()));
                assert!(!native.qualifies_primal_callbacks([0; 32]));
                let mut changed = native.creature_profiles().clone();
                changed.source_definitions_sha256 = Some("0".repeat(64));
                assert!(!qualifies_document(&changed));
                let mut changed = native.creature_profiles().clone();
                changed
                    .records
                    .retain(|record| record.profile.target.key != "oteryn:creature.fungosaurus");
                assert!(!qualifies_document(&changed));
                let mut changed = native.creature_profiles().clone();
                let record = changed
                    .records
                    .iter_mut()
                    .find(|record| record.profile.target.key == "oteryn:creature.the_primal_menace")
                    .ok_or("actual Primal source missing")?;
                record.profile.target.revision.push_str("-substituted");
                assert!(!qualifies_document(&changed));
                Ok(())
            })?
            .join()
            .map_err(|_| "source witness test thread failed")??;
        Ok(())
    }
}
