//! Authored conditional drops borrow the captured Quest objective and existing corpse MINT.
use super::kill_reward::PendingKillSettlement;
use crate::combat::{LootDefinitionRef, LootTableDefinition};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    objective: String,
    item_source_id: u32,
    sources: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    rules: Vec<Rule>,
}

fn rules() -> Result<&'static [Rule], ()> {
    static RULES: OnceLock<Result<Vec<Rule>, ()>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            let document: Document = serde_json::from_str(include_str!(
                "../../../../../content/quests/authored-death-loot.json"
            ))
            .map_err(|_| ())?;
            let objectives =
                oteryn_gameplay_quests::quest::death_objectives::load().map_err(|_| ())?;
            let mut seen = BTreeSet::new();
            if document.schema != 1
                || document.rules.len() > 16
                || document.rules.iter().any(|rule| {
                    !seen.insert(&rule.objective)
                        || !objectives
                            .iter()
                            .any(|objective| objective.key == rule.objective)
                        || rule.item_source_id == 0
                        || rule.item_source_id > u32::from(u16::MAX)
                        || rule.sources.is_empty()
                        || rule.sources.len() > 8
                        || rule
                            .sources
                            .iter()
                            .any(|source| source.is_empty() || source.len() > 1024)
                })
            {
                return Err(());
            }
            Ok(document.rules)
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(|_| ())
}

/// The same retained capture is reused after duplicate lethal notifications and requeues.
/// No quest snapshot or roster is reselected while settling an accepted death.
pub(super) fn capture(entry: &mut PendingKillSettlement) {
    let items = selected(entry);
    entry.callback_loot.extend(items.into_iter().map(|id| {
        LootDefinitionRef::new("Item", format!("oteryn:item.tibia.i{id}"), "definition-r1")
    }));
}

fn selected(entry: &PendingKillSettlement) -> Vec<u32> {
    let (Some(captured), Ok(rules)) = (&entry.grizzly, rules()) else {
        return Vec::new();
    };
    let keys = captured.loot_objectives(entry.facts.principal.character.as_bytes());
    let mut seen = BTreeSet::new();
    rules
        .iter()
        .filter_map(|rule| {
            (keys.contains(&rule.objective.as_str()) && seen.insert(rule.item_source_id))
                .then_some(rule.item_source_id)
        })
        .collect()
}

/// The tail appended by capture has explicit provenance. An unrelated callback that
/// selected the very same Item keeps its original chance and source ordinal.
pub(super) fn append(table: &mut LootTableDefinition, entry: &PendingKillSettlement) {
    append_with_guaranteed_tail(table, &entry.callback_loot, selected(entry).len());
}

fn append_with_guaranteed_tail(
    table: &mut LootTableDefinition,
    items: &[LootDefinitionRef],
    guaranteed: usize,
) {
    let first = table.entries.len();
    super::echo_warden::drops::append_callback_loot(table, items);
    if guaranteed > items.len() {
        return;
    }
    for entry in &mut table.entries[first + items.len() - guaranteed..] {
        // The ordinary planner applies a95..105%factor even at nominal100%.
        // Reuse the tutorial guarantee's admitted2M rated probability.
        entry.details.rated_probability_ppm = Some(2_000_000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{LootSelectionAlgorithm, LootTableEntry};
    use oteryn_gameplay_items::loot_plan::{LootPlanDeathKey, plan_creature_loot};

    #[test]
    fn conditional_guarantee_keeps_ordinary_rolls_and_survives_low_chance_factor()
    -> Result<(), String> {
        let rag = LootDefinitionRef::new("Item", "oteryn:item.tibia.i24415", "definition-r1");
        let coin = LootDefinitionRef::new("Item", "oteryn:item.tibia.i3031", "definition-r1");
        let original = LootTableEntry {
            item: rag.clone(),
            min_count: 1,
            max_count: 1,
            probability_ppm: Some(1_000_000),
            details: Default::default(),
        };
        let mut table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![original.clone()],
        };
        append_with_guaranteed_tail(&mut table, &[coin, rag.clone(), rag], 1);
        assert_eq!(
            table.entries[0], original,
            "ordinary rag entry is unchanged"
        );
        assert_eq!(
            table.entries[1].details.rated_probability_ppm, None,
            "unrelated callback stays unchanged"
        );
        assert_eq!(
            table.entries[2].details.rated_probability_ppm, None,
            "same-Item unrelated callback stays unchanged"
        );
        let reference =
            LootDefinitionRef::new("Loot", "oteryn:loot.creature.wolf", "definition-r1");
        let mut low_factor_witness = false;
        for generation in 1..=4096 {
            let plan = plan_creature_loot(
                LootPlanDeathKey::new([1; 16], [2; 16], 1, 2, generation),
                &reference,
                &table,
            )
            .map_err(|e| format!("{e:?}"))?;
            assert!(
                plan.entries.iter().any(|entry| entry.draw_ordinal == 3),
                "conditional captured drop is always present"
            );
            low_factor_witness |= !plan.entries.iter().any(|entry| entry.draw_ordinal == 0);
        }
        assert!(
            low_factor_witness,
            "exercise a death whose ordinary nominal100% roll fails"
        );
        Ok(())
    }
}
