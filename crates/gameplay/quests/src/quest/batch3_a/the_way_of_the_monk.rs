//! the_way_of_the_monk donor-source regression tests; no runtime adapter.
use super::input;
use crate::quest::npc_rules::{NpcInventorySnapshot, NpcRuleCatalog, NpcRuleError};
use std::collections::BTreeMap;

const MONK: &str = include_str!(
    "../../../../../../content/dialogues/quest-rules/quest-batch3-a-the_way_of_the_monk.json"
);

#[test]
fn manop_scope_fix_plans_two_aliases_and_keeps_vocation_transfer_held() -> Result<(), NpcRuleError>
{
    // donor bug fix: explicit MONK_QUEST and matching compatibility writes,
    // ambassador_manop.lua:69,167-170. This tests the source planner, not a new
    // durable NPC caller. QUESTION / approved general ANSWER point 3:
    // https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589102441
    let source = NpcRuleCatalog::from_json_documents(&[MONK])?;
    let tracks = BTreeMap::new();
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let mut request = input("oteryn:npc.ambassador_manop", &tracks, &inventory, &outfits);
    request.npc = "oteryn:npc.ambassador_manop";
    request.message = "yes";
    request.topic = 2;
    let plan = source
        .plan(&request)?
        .ok_or_else(|| NpcRuleError::Invalid("Manop invitation".into()))?;
    assert_eq!(plan.next_topic, 0);
    assert_eq!(plan.track_changes.len(), 2);
    for suffix in ["questline", "questlog"] {
        assert!(
            plan.track_changes
                .iter()
                .any(|change| change.key.ends_with(suffix)
                    && change.before == 0
                    && change.after == 1)
        );
    }
    request.topic = 4;
    request.level = 8;
    let held = source
        .plan(&request)?
        .ok_or_else(|| NpcRuleError::Invalid("Manop held transfer".into()))?;
    assert!(held.track_changes.is_empty());
    assert!(held.effects.is_empty());
    assert!(held.reply.is_empty());
    assert_eq!(held.next_topic, 4);
    request.level = 7;
    let too_low = source
        .plan(&request)?
        .ok_or_else(|| NpcRuleError::Invalid("Manop level refusal".into()))?;
    assert!(too_low.reply[0].contains("level 8"));
    assert!(too_low.track_changes.is_empty());
    assert_eq!(too_low.next_topic, 4);
    Ok(())
}
