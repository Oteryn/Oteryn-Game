use super::*;

#[test]
fn m09_tower_reward_plan_admits_all_four_literal_arrow_outputs() -> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::load_embedded()?;
    // This is the production planner's admission gate. It formerly refused the
    // whole NPC because the source reward contains four independent outputs.
    assert!(
        catalogue
            .item_exchange_keys("oteryn:npc.bunny_bonecrusher")
            .is_some()
    );
    let reward = catalogue
        .programs()
        .iter()
        .flat_map(|program| &program.document.rules)
        .find(|rule| rule.key == "oteryn:quest-npc/tower-defence/reward")
        .ok_or_else(|| NpcRuleError::Invalid("missing Tower reward".into()))?;
    let outputs = reward
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
    Ok(())
}
