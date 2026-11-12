use crate::play::feature_0115_mail::protocol::accepted;
use oteryn_client_runtime::{
    house_move_out as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::house_move_out::{Intent, Panel};

pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let action = match intent {
        Intent::Query => 0,
        Intent::Notice => 1,
        Intent::Cancel => 2,
    };
    let request = wire::HouseMoveOutRequest {
        house: panel.fields[0].trim().into(),
        action,
        due_at_unix_ms: if action == 1 {
            panel.fields[1]
                .trim()
                .parse()
                .map_err(|_| tr!("Enter a positive Unix time in milliseconds"))?
        } else {
            0
        },
    };
    let payload = wire::encode_house_move_out_request(&request)
        .map_err(|_| tr!("Enter a valid house key and notice time"))?;
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_HOUSE_MOVE_OUT_V1,
        kind: if action == 0 {
            wire::COMMAND_TYPE_HOUSE_MOVE_OUT_QUERY
        } else {
            wire::COMMAND_TYPE_HOUSE_MOVE_OUT
        },
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
    let Ok(reply) = wire::decode_house_move_out_reply(&result.payload) else {
        panel.feedback = tr!("Invalid move-out reply").into();
        return;
    };
    let state = match reply.state {
        0 => tr!("No notice"),
        1 => tr!("Notice"),
        2 => tr!("Frozen"),
        3 => tr!("Drained (owner retained)"),
        _ => tr!("Cancelled"),
    };
    panel.summary = lfmt!(
        "{}: due {} ms; delivered {}/{}",
        state,
        reply.due_at_unix_ms,
        reply.delivered,
        reply.census_count
    );
    panel.feedback = tr!("Move-out status updated").into();
}
