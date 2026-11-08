use super::*;

#[test]
fn m09_tower_reward_plan_admits_all_four_literal_arrow_outputs() -> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::load_embedded()?;
    let tracks = BTreeMap::from([(
        "oteryn:quest-progress/quest/u8_1/tower_defence_quest/questline".into(),
        4,
    )]);
    let outfits = BTreeMap::new();
    let inventory = NpcInventorySnapshot::default();
    let plan = catalogue
        .plan_item_exchange(&NpcRuleInput {
            npc: "oteryn:npc.bunny_bonecrusher",
            trigger: NpcRuleTrigger::Say,
            message: "password",
            topic: 2,
            tracks: &tracks,
            inventory: &inventory,
            now_epoch_seconds: 1_700_000_000,
            premium_current: false,
            calendar_month: 11,
            level: 50,
            vocation: "oteryn:vocation.none",
            sex: 0,
            outfits: &outfits,
        })?
        .ok_or_else(|| NpcRuleError::Invalid("missing Tower reward".into()))?;
    let outputs = plan
        .effects
        .iter()
        .filter_map(|effect| match effect {
            NpcRuleEffect::GiveItem {
                item,
                count,
                subtype: None,
            } => Some((item.as_str(), *count)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        outputs,
        vec![
            ("oteryn:item.tibia.i762", 50),
            ("oteryn:item.tibia.i774", 50),
            ("oteryn:item.tibia.i763", 50),
            ("oteryn:item.tibia.i761", 50)
        ]
    );
    assert!(plan.track_changes.iter().any(|state| state.after == 5));
    Ok(())
}
