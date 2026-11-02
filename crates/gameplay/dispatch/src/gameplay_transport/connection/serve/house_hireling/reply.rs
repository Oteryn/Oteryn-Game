//! Existing bounded native reply payloads; House custody changes no wire format.
use super::*;
use oteryn_protocol_oteryn::refusal_detail::{RefusalDetail, append_refusal_detail};

pub(in super::super) fn result(
    selected: SelectedCapabilities,
    dispatch: &Dispatch,
) -> Result<(CommandStatus, Vec<u8>), ServeEnd> {
    let detail = |mut payload: Vec<u8>, detail| {
        append_refusal_detail(&mut payload, selected.as_slice(), detail);
        payload
    };
    Ok(match dispatch {
        Dispatch::Logout(result) => (
            if result.is_rejected() {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            encode_logout_result(*result).map_err(|_| ServeEnd::Disconnected)?,
        ),
        Dispatch::NoLogout => (
            CommandStatus::Rejected,
            encode_logout_result(LogoutResult::Busy).map_err(|_| ServeEnd::Disconnected)?,
        ),
        Dispatch::Step(outcome, wait) => (
            if matches!(
                outcome.disposition,
                StepDisposition::Rejected | StepDisposition::TooEarly
            ) {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            detail(
                encode_step_outcome(selected.as_slice(), outcome.disposition, *wait),
                RefusalDetail::of_step(outcome.disposition),
            ),
        ),
        Dispatch::Reply(status, payload) | Dispatch::NpcShop(status, payload, _, _) => {
            (*status, payload.clone())
        }
        Dispatch::Use(outcome) => {
            let (disposition, gate) = negotiated_use_disposition(
                UseCapabilities {
                    item_use: selected.contains(CAPABILITY_ITEM_USE_V1),
                    world_interaction: selected.contains(CAPABILITY_WORLD_INTERACTION_V1),
                    rune_use: selected.contains(CAPABILITY_RUNE_USE_V1),
                },
                outcome.disposition,
                outcome.gate_message_id,
            );
            (
                if disposition == UseDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                detail(
                    encode_use_result_with_gate(disposition, gate)
                        .map_err(|_| ServeEnd::Disconnected)?,
                    outcome
                        .refusal_detail
                        .or(RefusalDetail::of_use(outcome.disposition)),
                ),
            )
        }
        Dispatch::UseItem(decision, _) => {
            let disposition = match decision {
                OpenDecision::Open => UseDisposition::Committed,
                OpenDecision::TooFar => UseDisposition::TooFar,
                OpenDecision::StaleState => UseDisposition::StaleState,
                OpenDecision::NothingToUse => UseDisposition::NothingToUse,
            };
            (
                CommandStatus::Accepted,
                detail(
                    encode_use_result(disposition).map_err(|_| ServeEnd::Disconnected)?,
                    RefusalDetail::of_use(disposition),
                ),
            )
        }
        Dispatch::ContainerView(outcome, _) => (
            CommandStatus::Accepted,
            encode_container_view_result(*outcome),
        ),
        Dispatch::Npc(disposition, _, _) | Dispatch::NpcTrade(disposition, _, _) => (
            if *disposition == NpcIntentDisposition::Rejected {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            encode_npc_intent_result(*disposition),
        ),
        Dispatch::Stash(reply) => {
            let payload = oteryn_protocol_oteryn::stash::encode_stash_reply(reply)
                .map_err(|_| ServeEnd::Disconnected)?;
            let decoded = oteryn_protocol_oteryn::stash::decode_stash_reply(&payload)
                .map_err(|_| ServeEnd::Disconnected)?;
            if &decoded != reply {
                return Err(ServeEnd::Disconnected);
            }
            (CommandStatus::Accepted, payload)
        }
        Dispatch::Unregistered => (CommandStatus::Rejected, Vec::new()),
        _ => return Err(ServeEnd::Disconnected),
    })
}
