//! Bounded source NPC teleport planning; destination admission remains with the map owner.
use super::{
    NpcRuleCatalog, NpcRuleEffect, NpcRuleError, NpcRuleInput, NpcRulePlan, NpcRulePosition,
};

/// A complete source branch plus its single destination and projected effects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcTeleportPlan {
    pub source: NpcRulePlan,
    pub destination: NpcRulePosition,
    pub magic: Vec<String>,
}

impl NpcRuleCatalog {
    /// Preview one teleport branch without permitting inventory or other value owners.
    pub fn plan_storage_teleport(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcTeleportPlan>, NpcRuleError> {
        let source = super::evaluate::plan_teleport(self, input)?;
        assemble(source, false)
    }

    /// Carried prices are fixed by the selected source and verified under the same
    /// Character root lock as the Arrival. Crystal 201189f8 npc/buddel.lua:59-67.
    pub fn plan_carried_teleport(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcTeleportPlan>, NpcRuleError> {
        assemble(self.plan_item_exchange(input)?, true)
    }
}

fn assemble(
    source: Option<NpcRulePlan>,
    carried: bool,
) -> Result<Option<NpcTeleportPlan>, NpcRuleError> {
    let Some(source) = source else {
        return Ok(None);
    };
    if !source
        .effects
        .iter()
        .any(|effect| matches!(effect, NpcRuleEffect::Teleport { .. }))
    {
        return Ok(None);
    }
    let mut destination = None;
    let mut magic = Vec::new();
    let mut prices = 0;
    for effect in &source.effects {
        match effect {
            NpcRuleEffect::Teleport { position } => {
                if destination.replace(*position).is_some() {
                    return Err(NpcRuleError::Invalid(
                        "multiple NPC teleport destinations".into(),
                    ));
                }
            }
            NpcRuleEffect::MagicEffect { effect } if effect == "CONST_ME_TELEPORT" => {
                magic.push(effect.clone());
                if magic.len() > 8 {
                    return Err(NpcRuleError::LimitExceeded);
                }
            }
            NpcRuleEffect::Set { .. }
            | NpcRuleEffect::Add { .. }
            | NpcRuleEffect::SetNowPlus { .. }
            | NpcRuleEffect::SetTopic { .. }
            | NpcRuleEffect::EndConversation { .. } => {}
            NpcRuleEffect::TakeItem {
                count: 1,
                subtype: None,
                optional: false,
                ..
            } if carried => {
                prices += 1;
                if prices > 1 || source.track_changes.is_empty() {
                    return Err(NpcRuleError::Invalid("carried NPC teleport price".into()));
                }
            }
            _ => {
                return Err(NpcRuleError::MissingNativeResolution(
                    "NPC teleport effect".into(),
                ));
            }
        }
    }
    Ok(destination.map(|destination| NpcTeleportPlan {
        source,
        destination,
        magic,
    }))
}

#[cfg(test)]
mod tests;
