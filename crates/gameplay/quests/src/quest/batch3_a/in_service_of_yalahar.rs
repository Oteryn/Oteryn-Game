//! in_service_of_yalahar donor-source regression tests; no runtime adapter.
use super::input;
use crate::quest::npc_rules::{NpcInventorySnapshot, NpcRuleCatalog, NpcRuleError};
use std::collections::BTreeMap;

const YALAHAR: &str = include_str!(
    "../../../../../../content/dialogues/quest-rules/quest-batch3-a-in_service_of_yalahar.json"
);

#[test]
fn palimuth_job_donor_bug_fix_preserves_both_source_branches() -> Result<(), NpcRuleError> {
    // donor bug fix: palimuth.lua:65 intended ~=54, confirmed by the adjacent
    // ==54 branch at line 68. Coordinator chose B in this ANSWER:
    // https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589900671
    // QUESTION: https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589698604
    let source = NpcRuleCatalog::from_json_documents(&[YALAHAR])?;
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    for (stage, topic, reply) in [
        (
            53,
            0,
            "I'm an Augur of the city of Yalahar. My special duty consists of coordinating the efforts to keep the city and its services running.",
        ),
        (54, 6, "Did you bring me the vampiric crest?"),
    ] {
        let tracks = BTreeMap::from([(
            "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline".into(),
            stage,
        )]);
        let mut request = input("oteryn:npc.palimuth", &tracks, &inventory, &outfits);
        request.npc = "oteryn:npc.palimuth";
        request.message = "job";
        request.topic = 0;
        let plan = source
            .plan(&request)?
            .ok_or_else(|| NpcRuleError::Invalid("Palimuth job reply".into()))?;
        assert_eq!(plan.reply, [reply]);
        assert_eq!(plan.next_topic, topic);
        assert!(plan.track_changes.is_empty());
    }
    Ok(())
}
