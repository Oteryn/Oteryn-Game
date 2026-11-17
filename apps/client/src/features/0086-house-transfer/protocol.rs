use crate::play::feature_0115_mail::protocol::accepted;
use oteryn_client_runtime::{
    house_transfer as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::house_transfer::{Intent, Panel};

pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let request = wire::HouseTransferIntent {
        house: panel.fields[0].trim().into(),
        exit: matches!(intent, Intent::Exit),
    };
    let payload =
        wire::encode_house_transfer_intent(&request).map_err(|_| tr!("Enter a valid house key"))?;
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_HOUSE_TRANSFER_V1,
        kind: wire::COMMAND_TYPE_HOUSE_TRANSFER,
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
    let (Ok(reply), Ok(request)) = (
        wire::decode_house_transfer_ack(&result.payload),
        wire::decode_house_transfer_intent(&command.payload),
    ) else {
        panel.feedback = tr!("Invalid transfer reply").into();
        return;
    };
    if reply.house.as_deref()
        != if request.exit {
            None
        } else {
            Some(request.house.as_str())
        }
    {
        panel.feedback = tr!("Invalid transfer destination").into();
        return;
    }
    panel.summary = lfmt!(
        "{} at {},{},{}; scope {}",
        reply.house.as_deref().unwrap_or(tr!("Channel")),
        reply.position[0],
        reply.position[1],
        reply.position[2],
        reply.scope_generation
    );
    panel.feedback = tr!("Transfer committed").into();
}
