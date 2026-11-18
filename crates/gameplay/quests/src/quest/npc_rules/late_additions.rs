//! Additive programmes pin the complete published quest before changing its definition.
use super::{NpcRuleCatalog, NpcRuleDocument, NpcRuleProgram, admission, bounds};
use crate::quest::{
    NpcDefinitionPredecessor, QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue,
    QuestTransition,
};

pub(super) const COBRA_PROGRAM: &str = "oteryn:quest-npc-program.ancient-tombs-cobra-greeting";

pub(super) fn contains(document: &NpcRuleDocument) -> bool {
    super::questpc2_gerimor_source::reports(document)
        || (document.quest == "oteryn:quest.the_ancient_tombs_quest"
            && document.program_key.as_deref() == Some(COBRA_PROGRAM))
}

/// Crystal 201189f8 data-global/npc/cobra.lua:48-69 and data-global/npc/gerimor.lua:356-447.
pub(super) fn extend(
    active: &mut QuestStateCatalogue,
    source: &NpcRuleCatalog,
) -> Result<(), String> {
    for program in source.programs().iter().filter(|p| contains(&p.document)) {
        attach_program(active, program)?;
    }
    Ok(())
}

fn attach_program(
    active: &mut QuestStateCatalogue,
    program: &NpcRuleProgram,
) -> Result<(), String> {
    let quest = &program.document.quest;
    let hash = active
        .definition_hash(quest)
        .ok_or("missing NPC predecessor definition")?;
    let writes = bounds::literal_write_bounds(program);
    for track in &program.document.tracks {
        let target = active
            .tracks
            .get_mut(&track.key)
            .ok_or("unknown additive NPC track")?;
        if target.quest != *quest || target.initial != track.initial {
            return Err(format!("NPC track differs from predecessor: {}", track.key));
        }
        if let Some(&(min, max)) = writes.get(&track.key) {
            target.min = target.min.min(min);
            target.max = target.max.max(max);
        }
    }
    let first = program
        .document
        .tracks
        .first()
        .ok_or("empty additive NPC programme")?;
    let key = admission::binding_key(program);
    if active
        .transitions
        .insert(
            key.clone(),
            QuestTransition {
                key,
                quest: quest.clone(),
                effects: vec![QuestEffect {
                    track: first.key.clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Computed,
                }],
                completes: false,
                experience: None,
                world_change: None,
            },
        )
        .is_some()
    {
        return Err("duplicate additive NPC programme pin".into());
    }
    let next = crate::quest::definition_hash(quest, &active.tracks, &active.transitions);
    active.hashes.insert(quest.clone(), next);
    active
        .predecessors
        .entry(quest.clone())
        .or_default()
        .push(NpcDefinitionPredecessor {
            hash,
            defaults: Default::default(),
        });
    Ok(())
}
