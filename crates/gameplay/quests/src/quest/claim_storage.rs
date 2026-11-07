//! Read projections of source storage whose write is an existing once-claim.
//! No second reward receipt or mutable storage writer is introduced.
use std::collections::{BTreeMap, BTreeSet};

pub struct ClaimStorageBinding {
    pub claim: &'static str,
    pub track: &'static str,
    pub quest: &'static str,
    pub value: i64,
}

// The four source choices share Reward10102. Their already-admitted canonical
// claim is in content/interactions/reward_claims/reward-claim-variants-00000-00099.json;
// the quest identity and claim reference are in quests-00000-00099.json.
const BINDINGS: &[ClaimStorageBinding] = &[ClaimStorageBinding {
    claim: "oteryn:reward-claim.quest.u7_24.the_annihilator.reward",
    track: "oteryn:quest-progress/quest/u7_24/the_annihilator/reward",
    quest: "oteryn:quest.the_annihilator_quest",
    value: 1,
}];

pub fn bindings() -> &'static [ClaimStorageBinding] {
    BINDINGS
}

pub fn affects_claim(claim: &str) -> bool {
    BINDINGS.iter().any(|binding| binding.claim == claim)
}

/// Only exact committed claim identities project a value into the read copy.
pub fn project(claims: &BTreeSet<String>, tracks: &mut BTreeMap<String, i64>) {
    for binding in BINDINGS {
        if claims.contains(binding.claim) {
            tracks.insert(binding.track.into(), binding.value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_claim_storage_is_exact_and_preserves_unrelated_progress() {
        let binding = &bindings()[0];
        let mut tracks = BTreeMap::from([("other/track".into(), 7)]);
        project(
            &BTreeSet::from([format!("{}.other", binding.claim)]),
            &mut tracks,
        );
        assert_eq!(tracks.len(), 1);
        assert!(!affects_claim(&format!("{}.other", binding.claim)));
        project(&BTreeSet::from([binding.claim.into()]), &mut tracks);
        assert_eq!(tracks.get(binding.track), Some(&1));
        assert_eq!(tracks.get("other/track"), Some(&7));
    }
}
