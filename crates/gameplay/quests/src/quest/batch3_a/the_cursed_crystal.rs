//! the_cursed_crystal donor-source regression tests; no runtime adapter.
use super::input;
use crate::quest::npc_rules::{NpcInventorySnapshot, NpcRuleCatalog, NpcRuleError};
use std::collections::BTreeMap;

const CURSED: &str = include_str!(
    "../../../../../../content/dialogues/quest-rules/quest-batch3-a-the_cursed_crystal.json"
);

#[test]
fn cursed_crystal_expiry_resets_before_the_same_message_selects_a_branch()
-> Result<(), NpcRuleError> {
    let source = NpcRuleCatalog::from_json_documents(&[CURSED])?;
    let prefix = "oteryn:quest-progress/quest/u10_70/the_cursed_crystal/";
    let tracks = BTreeMap::from([
        (format!("{prefix}time"), 100),
        (format!("{prefix}oneeyedjoe"), 4),
        (format!("{prefix}questline"), 4),
        (format!("{prefix}sheet_of_paper"), 1),
        (format!("{prefix}small_crystal_bell"), 1),
    ]);
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let mut request = input("oteryn:npc.one_eyed_joe", &tracks, &inventory, &outfits);
    request.npc = "oteryn:npc.one_eyed_joe";
    request.message = "mission";
    request.topic = 0;
    request.now_epoch_seconds = 100;
    let plan = source
        .plan(&request)?
        .ok_or_else(|| NpcRuleError::Invalid("Cursed repeat".into()))?;
    assert_eq!(plan.next_topic, 1);
    assert_eq!(plan.reply.len(), 2);
    assert!(
        plan.track_changes
            .iter()
            .any(|change| change.key.ends_with("/oneeyedjoe") && change.after == -1)
    );
    assert!(
        plan.track_changes
            .iter()
            .any(|change| change.key.ends_with("/small_crystal_bell") && change.after == -1)
    );
    assert!(
        !plan
            .track_changes
            .iter()
            .any(|change| change.key.ends_with("/time"))
    );
    let no_expiry = BTreeMap::from([
        (format!("{prefix}time"), 0),
        (format!("{prefix}oneeyedjoe"), 4),
    ]);
    request.tracks = &no_expiry;
    assert!(source.plan(&request)?.is_none());
    Ok(())
}

#[test]
fn cursed_crystal_held_shared_acceptance_cannot_publish_success_or_local_progress()
-> Result<(), NpcRuleError> {
    let source = NpcRuleCatalog::from_json_documents(&[CURSED])?;
    let tracks = BTreeMap::new();
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let mut request = input("oteryn:npc.one_eyed_joe", &tracks, &inventory, &outfits);
    request.npc = "oteryn:npc.one_eyed_joe";
    request.message = "yes";
    request.topic = 2;
    let plan = source
        .plan(&request)?
        .ok_or_else(|| NpcRuleError::Invalid("Cursed held acceptance".into()))?;
    assert!(plan.track_changes.is_empty());
    assert!(plan.effects.is_empty());
    assert!(plan.reply.is_empty());
    assert_eq!(plan.next_topic, 2);
    Ok(())
}
