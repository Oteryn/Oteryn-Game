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
    pub gate: Option<NpcTeleportGate>,
}

/// A native condition checked by dispatch on the current actor before a held departure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcTeleportCondition {
    Poison,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct NpcTeleportGate {
    pub condition: NpcTeleportCondition,
    pub refusal: String,
    pub success: String,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub origin_effect: bool,
}

impl NpcRuleCatalog {
    /// Preview one teleport branch without permitting inventory or other value owners.
    pub fn plan_storage_teleport(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcTeleportPlan>, NpcRuleError> {
        let Some(source) = super::evaluate::plan_teleport(self, input)? else {
            return Ok(None);
        };
        let mut destination = None;
        let mut magic = Vec::new();
        let mut gate = None;
        for effect in &source.effects {
            match effect {
                NpcRuleEffect::Teleport {
                    position,
                    gate: required,
                } => {
                    gate = required.clone();
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
            gate,
        }))
    }
}

#[cfg(test)]
mod tests;
