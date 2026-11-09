#[path = "protocol/reply.rs"]
pub mod reply;
use super::ItemForgePanel;
use oteryn_client_runtime::cui3::item_forge::{COMMAND_TYPE_ITEM_FORGE, Request};
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
use std::num::NonZeroU64;
pub fn command(payload: Vec<u8>) -> ServerMechanicCommand {
    ServerMechanicCommand {
        capability: 152,
        command_type: COMMAND_TYPE_ITEM_FORGE,
        payload,
    }
}
pub fn control_request(p: &mut ItemForgePanel) -> Option<ServerMechanicCommand> {
    if p.query_current_dust {
        return request(
            p,
            ServerMechanicCommand {
                capability: 152,
                command_type:
                    oteryn_client_runtime::cui3::item_forge::COMMAND_TYPE_FORGE_DUST_QUERY,
                payload: Vec::new(),
            },
        );
    }
    let pair = p.operation.requires_pair();
    let r = Request {
        operation: p.operation,
        occurrence: p.occurrence,
        first_item: pair.then(|| NonZeroU64::new(p.first_item)).flatten(),
        second_item: pair.then(|| NonZeroU64::new(p.second_item)).flatten(),
        boost: p.boost,
        protect: p.protect,
        history_limit: if p.operation
            == oteryn_client_runtime::cui3::item_forge::Operation::QueryHistory
        {
            p.history_limit
        } else {
            0
        },
    };
    let Some(bytes) = r.encode() else {
        p.feedback = Some(tr!("Select valid items and forge options").into());
        return None;
    };
    request(p, command(bytes.to_vec()))
}
pub fn request(p: &mut ItemForgePanel, c: ServerMechanicCommand) -> Option<ServerMechanicCommand> {
    if !p.selected || p.pending {
        return None;
    }
    let submitted = (c.command_type == COMMAND_TYPE_ITEM_FORGE)
        .then(|| Request::decode(&c.payload))
        .flatten();
    if let Some(submitted) = submitted.filter(|request| {
        request.operation != oteryn_client_runtime::cui3::item_forge::Operation::QueryHistory
    }) {
        if p.retry.is_some_and(|retry| retry != submitted) {
            p.feedback = Some(
                "Resolve or retry the same existing attempt before another paid action".into(),
            );
            return None;
        }
        p.retry = Some(submitted);
    }
    p.submitted = submitted;
    p.pending = true;
    p.feedback = None;
    Some(c)
}
pub fn apply_result(p: &mut ItemForgePanel, r: &ServerMechanicResponse) {
    let was_pending = p.pending;
    p.pending = false;
    let submitted = p.submitted.take();
    if matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.response.clone_from(&r.payload);
        if let Some(reply) = super::reply::decode(&r.payload) {
            if let Some(attempt) = submitted.filter(|request| {
                request.operation
                    != oteryn_client_runtime::cui3::item_forge::Operation::QueryHistory
            }) {
                if !matches!(&reply, super::reply::Reply::Result(receipt) if receipt.operation == attempt.operation)
                {
                    p.feedback =
                        Some("Outcome did not match the attempt; retry the same occurrence".into());
                    if oteryn_client_runtime::forge_preview::resource_operation(attempt.operation) {
                        p.resource_dialog.message("The reply did not match this operation. Check Forge history before retrying the same unresolved attempt.");
                    }
                    return;
                }
                if let super::reply::Reply::Result(receipt) = &reply
                    && attempt.operation.requires_pair()
                {
                    super::exaltation::show_result(p, attempt, receipt.clone());
                }
                p.retry = None;
                p.preview.invalidate();
            }
            if was_pending
                && (submitted.is_some_and(|r| {
                    r.operation == oteryn_client_runtime::cui3::item_forge::Operation::QueryHistory
                        || oteryn_client_runtime::forge_preview::resource_operation(r.operation)
                }) || matches!(reply, super::reply::Reply::CurrentDust { .. }))
            {
                p.resource_dialog.result(reply.clone());
            }
            p.decoded = Some(reply);
            p.feedback = None;
        } else {
            p.feedback = Some(tr!("Invalid forge reply; refresh to retry").into());
            if was_pending
                && submitted.is_some_and(|r| {
                    oteryn_client_runtime::forge_preview::resource_operation(r.operation)
                })
            {
                p.resource_dialog.message("The operation result could not be confirmed. Check Forge history; no new operation was submitted.");
            }
        }
    } else {
        if r.status == CommandStatus::Rejected
            && submitted.is_some_and(|request| {
                request.operation
                    != oteryn_client_runtime::cui3::item_forge::Operation::QueryHistory
            })
        {
            p.retry = None;
            p.preview.invalidate();
        }
        p.feedback = Some(lfmt!("Server refused forge action: {:?}", r.status));
        if was_pending
            && submitted.is_some_and(|r| {
                oteryn_client_runtime::forge_preview::resource_operation(r.operation)
            })
        {
            p.resource_dialog.message("The server refused the resource operation. Refresh your resources before another attempt.");
        }
    }
}
/// Read-only projection through the existing Forge capability.
pub fn preview_request(
    panel: &mut ItemForgePanel,
    selection: oteryn_client_runtime::forge_preview::Selection,
) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
    super::preview::request(panel, selection)
}

#[cfg(test)]
// Direct fixture construction keeps the wire assertions readable.
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn controls_encode_reply_and_refusal() {
        let mut p = ItemForgePanel {
            selected: true,
            operation: oteryn_client_runtime::cui3::item_forge::Operation::Fusion,
            occurrence: [1; 16],
            first_item: 2,
            second_item: 3,
            ..Default::default()
        };
        let c = control_request(&mut p).unwrap();
        assert_eq!(c.payload.len(), 51);
        assert_eq!(
            Request::decode(&c.payload)
                .unwrap()
                .first_item
                .unwrap()
                .get(),
            2
        );
        apply_result(
            &mut p,
            &ServerMechanicResponse {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload: vec![7],
            },
        );
        assert_eq!(p.response, [7]);
        apply_result(
            &mut p,
            &ServerMechanicResponse {
                command_id: 2,
                status: CommandStatus::Rejected,
                server_sequence: 2,
                payload: vec![],
            },
        );
        assert!(p.feedback.unwrap().contains("refused"));
    }
}
