//! A live Albinius offering after the quest definition gains another transition.
//! Crystal 201189f8, data-global/npc/albinius.lua:172-192 and 194-214.
use super::{ComposedFreshAdmission, TestResult, fixture::debug};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, definition_predecessors,
};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.forgotten_knowledge_quest";
pub(super) const CASE: &str = "questpc2-albinius-portals-predecessor";

pub(super) fn install_albinius_successor(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    install_successor(
        owner,
        QuestTransition {
            quest: QUEST.into(),
            key: "oteryn:quest-transition/fixture/albinius-ice-offering".into(),
            effects: vec![QuestEffect {
                track: "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/access_ice".into(),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
            world_change: None,
        },
    )
}

pub(super) fn install_successor(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    transition: QuestTransition,
) -> TestResult {
    let original = owner
        .quest_catalogue
        .as_deref()
        .ok_or("original catalogue")?;
    let quest = transition.quest.clone();
    let hash = original
        .definition_hash(&quest)
        .ok_or("original quest hash")?;
    let hash: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();
    let mut successor = original
        .with_fixture_transition(transition)
        .map_err(debug)?;
    let binding = serde_json::json!({
        "schema": 1,
        "predecessors": [{"quest": quest, "definition_sha256": hash}]
    })
    .to_string();
    definition_predecessors::register(
        &mut successor,
        &[("predecessor-fixture.json", binding.as_str())],
    )?;
    owner.quest_catalogue = Some(Arc::new(successor));
    Ok(())
}
