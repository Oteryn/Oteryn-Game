use crate::play::feature_0115_mail::protocol::accepted;
use oteryn_client_runtime::{
    raid_schedule as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::raid_schedule::{Intent, Panel};
pub(in crate::play) fn command(_: &Panel, _: Intent) -> Result<FeatureCommand, String> {
    let payload = Vec::new();
    wire::decode_raid_schedule_query(&payload).map_err(|_| tr!("Invalid raid query"))?;
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_RAID_SCHEDULE_V1,
        kind: wire::COMMAND_TYPE_RAID_SCHEDULE_QUERY,
        payload,
    })
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) {
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;
    if !accepted(result) {
        panel.feedback = tr!("The server refused the request").into();
        return;
    }
    let Ok(state) = wire::decode_raid_schedule_state(&result.payload) else {
        panel.feedback = tr!("Invalid raid schedule reply").into();
        return;
    };
    panel.summary = if state.raid_key.is_empty() {
        tr!("No raid scheduled").into()
    } else {
        lfmt!(
            "{} | {}\r\nCommitted steps: {} | Active creatures: {}",
            state.raid_key,
            if state.running {
                tr!("Running")
            } else {
                tr!("Scheduled")
            },
            state.committed_steps,
            state.active_creatures
        )
    };
    panel.feedback = tr!("Raid schedule updated").into();
}
