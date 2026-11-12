use super::SanctionsPanel;
use oteryn_client_runtime::{
    CommandStatus,
    raw_feature::{FeatureCommand, GatedCommandResult},
    sanction_record::{
        CAPABILITY_SANCTION_RECORD_V1, COMMAND_TYPE_SANCTION_RECORD_QUERY, decode_reply,
        encode_query,
    },
};

/// The query of the own record, sent when the window opens and on Refresh. A session without
/// capability 497 sends nothing.
pub fn request(panel: &mut SanctionsPanel) -> Option<FeatureCommand> {
    if !panel.selected {
        panel.feedback = Some(tr!("The sanction record is unavailable in this session"));
        return None;
    }
    if panel.pending {
        panel.feedback = Some(tr!("Waiting for the server"));
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(FeatureCommand {
        capability: CAPABILITY_SANCTION_RECORD_V1,
        kind: COMMAND_TYPE_SANCTION_RECORD_QUERY,
        payload: encode_query(),
    })
}

/// Takes the server's record; a refusal or an unreadable reply keeps the record shown before.
pub fn apply_result(panel: &mut SanctionsPanel, result: &GatedCommandResult) {
    panel.pending = false;
    if result.status != CommandStatus::Accepted {
        panel.feedback = Some(tr!("The server refused the sanction record"));
        return;
    }
    match decode_reply(&result.payload) {
        Some(record) => {
            panel.record = Some(record);
            panel.feedback = None;
        }
        None => panel.feedback = Some(tr!("The server sent an unreadable sanction record")),
    }
}
