use crate::play::feature_0115_mail::protocol::accepted;
use crate::play::feature_0115_mail::protocol::{handle, uuid};
use oteryn_client_runtime::{
    raw_feature::{FeatureCommand, GatedCommandResult},
    timed_item as wire,
};
use oteryn_client_ui::timed_item::{Intent, Panel};
pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let action = match intent {
        Intent::Inspect => wire::TimedAction::Inspect,
        Intent::Use => wire::TimedAction::Use,
        Intent::Repair => wire::TimedAction::Repair,
    };
    let npc = if intent == Intent::Repair {
        Some(oteryn_session::EntityRef {
            identity: uuid(&panel.fields[2])?,
            generation: panel.fields[3]
                .trim()
                .parse()
                .map_err(|_| tr!("Enter Aldo's current generation"))?,
        })
    } else {
        None
    };
    let payload = wire::encode_timed_item_intent(wire::TimedItemIntent {
        action,
        target: handle(&panel.fields[0])?,
        expected_revision: panel.fields[1]
            .trim()
            .parse()
            .map_err(|_| tr!("Enter the expected item revision"))?,
        npc,
    })
    .map_err(|_| tr!("Invalid timed item request"))?;
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_TIMED_ITEM_V1,
        kind: wire::COMMAND_TYPE_TIMED_ITEM_INTENT,
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

    // Timed refusals carry a typed result even when the envelope status is Rejected.
    let state = match wire::decode_timed_item_result(&result.payload) {
        Ok(state) => state,
        Err(_) => {
            panel.feedback = if accepted(result) {
                tr!("Invalid timed item reply")
            } else {
                tr!("The server refused the request")
            }
            .into();
            return;
        }
    };
    use wire::TimedDisposition as Disposition;
    panel.feedback = match state.disposition {
        Disposition::Rejected => tr!("The server refused the timed item request"),
        Disposition::Stale => tr!("The item revision is stale; inspect the item again"),
        Disposition::InsufficientFunds => tr!("Insufficient funds for repair"),
        Disposition::Ok if !accepted(result) => tr!("The server refused the request"),
        Disposition::Ok => {
            panel.summary = lfmt!("Revision {} | Charges {:?} | Remaining ms {:?}\r\nRetired: {} | Repair fee: {} gold", state.revision, state.charges, state.remaining_ms, state.retired, state.fee_gold);
            tr!("Timed item updated")
        }
    }.into();
}
