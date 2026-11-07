//! Exact bounded W11 source closure, shared by Content templates and native consumers.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceWitness {
    schema: String,
    mechanics_source_revision: String,
    source_definitions_sha256: String,
    source_bindings: Vec<crate::content::ProjectV2SourceIdentityBinding>,
    callbacks: std::collections::BTreeMap<String, String>,
    records: Vec<crate::content::native_gameplay::CreatureProfileRecord>,
}

fn source_witness() -> Option<&'static SourceWitness> {
    static WITNESS: std::sync::OnceLock<Option<SourceWitness>> = std::sync::OnceLock::new();
    WITNESS
        .get_or_init(|| {
            serde_json::from_slice(include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/srx3-rotten-blood-witness.json"
            ))
            .ok()
        })
        .as_ref()
}

pub fn qualifies_document(
    document: &crate::content::native_gameplay::CreatureProfilesDocument,
) -> bool {
    use crate::content::ProjectV2SourceIdentityDisposition;
    let Some(witness) = source_witness() else {
        return false;
    };
    if witness.schema != "OTERYN_SRX3_ROTTEN_BLOOD_CALLBACK_WITNESS/v1"
        || witness.mechanics_source_revision != "201189f871d23597b82caf88fd611454a2ddefb8"
        || document.source_definitions_sha256.as_deref()
            != Some(witness.source_definitions_sha256.as_str())
        || witness.source_bindings.len() != 27
        || witness.records.len() != 27
        || witness.callbacks.len() != 34
    {
        return false;
    }
    // The bounded witness is generated from the real canonical native inputs,
    // bound to the loader's full canonical source hash. Every consumed record
    // includes its exact Creature, Behavior and complete attack closure; this is
    // implementation source data, never a manufactured WorldProject or grant.
    witness.records.iter().all(|expected| {
        let mut matches = document
            .records
            .iter()
            .filter(|record| record.profile.target == expected.profile.target);
        matches.next() == Some(expected) && matches.next().is_none()
    }) && witness.source_bindings.iter().all(|binding| {
        binding.target.revision == "definition-r1"
            && ((binding.target.key == "oteryn:creature.pillar_of_dark_energy"
                && binding.source_key == "oteryn:source.crystalserver"
                && binding.source_revision
                    == "crystalserver-creature-1530:00ce02a57ca5a12e48f32a3476e37471167e4c3f"
                && binding.identity_namespace == "crystalserver/monster-file"
                && binding.external_id
                    == "data-global/monster/quests/rotten_blood_quest/pillar of dark energy.lua")
                || (binding.target.key != "oteryn:creature.pillar_of_dark_energy"
                    && binding.source_key == "oteryn:source.canary"
                    && binding.source_revision == "47dfd51f45280a59a1d3e50ba7edd573d7234446"
                    && binding.identity_namespace == "canary/monster-file"))
            && binding.disposition == ProjectV2SourceIdentityDisposition::Exact
            && witness
                .records
                .iter()
                .any(|record| record.profile.target == binding.target)
    })
}

impl crate::content::native_gameplay::NativeGameplayState {
    pub fn qualifies_rotten_blood_callbacks(&self, active: [u8; 32]) -> bool {
        self.source_digest() == active && qualifies_document(self.creature_profiles())
    }
}

/// Pinned source body DATA, not current actor or mutation authority.
pub fn area_icons_source_body_digest() -> Option<[u8; 32]> {
    let hex = source_witness()?
        .callbacks
        .get("data-global/scripts/quests/rotten_blood_quest/monster_onthink_area_icons.lua")?;
    if hex.len() != 64 {
        return None;
    }
    let mut digest = [0; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(digest)
}
