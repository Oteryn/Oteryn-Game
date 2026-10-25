use super::AbilityError;
pub use crate::foundation::ability_data::{AbilityIntent, ProposalSource, TargetId};

pub struct ClientAbilityAdapter;

impl ClientAbilityAdapter {
    pub fn normalize(actor: &str, candidates: &[&str]) -> Result<AbilityIntent, AbilityError> {
        AbilityIntent::normalize(ProposalSource::Client, actor, candidates)
    }
}

pub struct AiAbilityAdapter;

impl AiAbilityAdapter {
    pub fn normalize(actor: &str, candidates: &[&str]) -> Result<AbilityIntent, AbilityError> {
        AbilityIntent::normalize(ProposalSource::Ai, actor, candidates)
    }
}

pub struct ScriptAbilityAdapter;

impl ScriptAbilityAdapter {
    pub fn normalize(actor: &str, candidates: &[&str]) -> Result<AbilityIntent, AbilityError> {
        AbilityIntent::normalize(ProposalSource::Script, actor, candidates)
    }
}
