use super::CyclopediaCharacterPanel;
use oteryn_client_runtime::{
    cyclopedia_character::protection::{Forecast, SELECTOR},
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Protection {
    pub open: bool,
    pub awaiting: bool,
    pub forecast: Option<Forecast>,
}
pub(super) fn request(p: &mut CyclopediaCharacterPanel) -> Option<FeatureCommand> {
    if p.target.is_some() {
        p.feedback = Some("Death loss previews are available for your own character");
        return None;
    }
    p.pending = true;
    p.command_kind = 388;
    p.feedback = None;
    p.protection.awaiting = true;
    Some(FeatureCommand {
        capability: 199,
        kind: 388,
        payload: vec![SELECTOR],
    })
}
pub(super) fn apply(p: &mut CyclopediaCharacterPanel, result: &GatedCommandResult) -> bool {
    p.protection.awaiting = false;
    if !p.pending {
        return false;
    }
    p.pending = false;
    let value = matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then(|| Forecast::decode(&result.payload))
    .flatten();
    let Some(value) = value else {
        p.feedback = Some("The server refused or returned an invalid death loss preview");
        return false;
    };
    p.protection.forecast = Some(value);
    p.feedback = None;
    true
}
