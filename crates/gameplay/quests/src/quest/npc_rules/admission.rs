use super::{NpcRuleCatalog, NpcRuleError, NpcRuleProgram};

pub(super) fn binding_key(program: &NpcRuleProgram) -> String {
    let digest = program
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("oteryn:quest-npc-pin/{digest}")
}

pub(super) fn for_npc(
    source: &NpcRuleCatalog,
    npc: &str,
    native: &super::super::QuestStateCatalogue,
) -> Result<Option<NpcRuleCatalog>, NpcRuleError> {
    let mut programs = Vec::new();
    for program in &source.programs {
        if !program.document.rules.iter().any(|rule| rule.npc == npc) {
            continue;
        }
        // A source document being shipped does not admit it into a catalogue
        // owned by another executor. The existing digest marker is that binding.
        let Some(binding) = native.transition(&binding_key(program)) else {
            continue;
        };
        if binding.quest != program.document.quest {
            return Err(NpcRuleError::Invalid("NPC programme binding quest".into()));
        }
        programs.push(program.clone());
    }
    if programs.is_empty() {
        return Ok(None);
    }
    NpcRuleCatalog { programs }
        .with_native_bounds(native)
        .map(Some)
}
