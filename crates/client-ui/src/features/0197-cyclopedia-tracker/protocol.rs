use super::CyclopediaTrackerPanel;
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_tracker::{self, TrackerAction, TrackerRequest},
    raw_feature::{FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Read,
    Add,
    Remove,
}
pub fn request(panel: &mut CyclopediaTrackerPanel, intent: Intent) -> Option<FeatureCommand> {
    if !panel.selected {
        panel.feedback = Some(tr!("Feature unavailable in this session"));
        return None;
    }
    if panel.pending {
        panel.feedback = Some(tr!("Waiting for the server"));
        return None;
    }
    let typed = match intent {
        Intent::Read => TrackerRequest::Read,
        Intent::Add => TrackerRequest::Change {
            action: TrackerAction::Add,
            creature_key: panel.creature_key.clone(),
        },
        Intent::Remove => TrackerRequest::Change {
            action: TrackerAction::Remove,
            creature_key: panel.creature_key.clone(),
        },
    };
    let Some(payload) = cyclopedia_tracker::encode_request(&typed) else {
        panel.feedback = Some(tr!("Enter a valid creature id"));
        return None;
    };
    panel.pending = true;
    panel.feedback = None;
    Some(FeatureCommand {
        capability: 197,
        kind: if panel.boss { 384 } else { 383 },
        payload,
    })
}
pub fn apply_result(panel: &mut CyclopediaTrackerPanel, result: &GatedCommandResult) -> bool {
    panel.pending = false;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some(tr!("The server refused the tracker action"));
        return false;
    }
    apply_snapshot(panel, &result.payload)
}
pub fn apply_snapshot(panel: &mut CyclopediaTrackerPanel, payload: &[u8]) -> bool {
    let snapshot =
        if let Some((bestiary, bosstiary)) = cyclopedia_tracker::decode_domain_snapshot(payload) {
            panel.bestiary_snapshot = Some(bestiary.clone());
            panel.bosstiary_snapshot = Some(bosstiary.clone());
            Some(if panel.boss { bosstiary } else { bestiary })
        } else {
            cyclopedia_tracker::decode_snapshot(payload)
        };
    match snapshot {
        Some(value) => {
            if value.boss {
                panel.bosstiary_snapshot = Some(value.clone());
            } else {
                panel.bestiary_snapshot = Some(value.clone());
            }
            panel.snapshot = Some(value);
            panel.feedback = None;
            true
        }
        None => {
            panel.feedback = Some(tr!("The server returned an invalid tracker snapshot"));
            false
        }
    }
}
