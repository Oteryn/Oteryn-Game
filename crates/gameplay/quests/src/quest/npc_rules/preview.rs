//! Preserve the generic QuestState preview budget independently of source NPC executors.
use super::*;

impl NpcRuleCatalog {
    /// Pure generic preview under the QuestState write budget. Durable NPC execution
    /// uses the stricter storage/item planners, including the pinned Father exception.
    pub fn plan(&self, input: &NpcRuleInput<'_>) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        plan_with_native(self, input, &BTreeMap::new())
    }
}

pub(super) fn plan_with_native(
    catalogue: &NpcRuleCatalog,
    input: &NpcRuleInput<'_>,
    resolutions: &BTreeMap<String, Vec<NpcRuleEffect>>,
) -> Result<Option<NpcRulePlan>, NpcRuleError> {
    evaluate::plan(
        catalogue,
        input,
        resolutions,
        false,
        false,
        super::super::QUESTSTATE0_RL_02,
    )
}
