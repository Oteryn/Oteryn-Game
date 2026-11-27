//! Encode the command result without changing dispositions or payload bytes.
//! PARITY-REFUSAL-DETAIL-1: under capability 66 a refused step, use or item move appends its
//! refusal detail (field 15); without it every payload keeps its bytes.

use super::*;
use oteryn_protocol_oteryn::refusal_detail::{RefusalDetail, append_refusal_detail};

/// `payload` with the refusal detail the session selected capability 66 for.
fn detailed(session: &ServeState, mut payload: Vec<u8>, detail: Option<RefusalDetail>) -> Vec<u8> {
    append_refusal_detail(
        &mut payload,
        session.selected_capabilities.as_slice(),
        detail,
    );
    payload
}

/// An unavailable owner has no typed payload or effects. Channel and House
/// command receipts use the same registered operation-terminal result.
pub(super) fn encode_refusal(
    generation: u64,
    sequence: u64,
    command_id: u64,
) -> Result<Vec<u8>, ServeEnd> {
    encode_command_error_result(
        generation,
        sequence,
        command_id,
        FoundationProtocolError::CommandRefused,
    )
    .map_err(|_| ServeEnd::server("frame_encode_failed"))
}

pub(super) fn encode(
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    dispatch: &mut Dispatch,
) -> Result<Vec<u8>, ServeEnd> {
    let sequence = next_sequence(session)?;
    if matches!(dispatch, Dispatch::Unavailable) {
        return encode_refusal(session.generation, sequence, command.command_id);
    }
    session.integrity.observe(dispatch);
    inventory_capacity::reread_after_command(session, command.command_type, dispatch);
    let (status, result_payload) = match dispatch {
        Dispatch::ServerModal(
            crate::gameplay_transport::server_modal::ServerModalOutcome::Reply(payload)
            | crate::gameplay_transport::server_modal::ServerModalOutcome::LoggedOut(payload),
        ) => (CommandStatus::Accepted, payload.clone()),
        Dispatch::ServerModal(
            crate::gameplay_transport::server_modal::ServerModalOutcome::Unavailable,
        ) => {
            return Err(ServeEnd::server("result_unavailable"));
        }
        Dispatch::Vip(Some(payload)) => (CommandStatus::Accepted, payload.clone()),
        Dispatch::Vip(None) => {
            return Err(ServeEnd::server("result_unavailable"));
        }
        Dispatch::House(Some(payload)) => (CommandStatus::Accepted, payload.clone()),
        Dispatch::House(None) => {
            return Err(ServeEnd::server("result_unavailable"));
        }
        Dispatch::Training(result) => {
            let Ok(payload) =
                oteryn_protocol_oteryn::loop_training::encode_training_result(*result)
            else {
                return Err(ServeEnd::server("frame_encode_failed"));
            };
            (
                if result.disposition == 4 {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                payload,
            )
        }
        Dispatch::Appearance(Some(result)) => match appearance_change::result(*result) {
            Some(result) => result,
            None => return Err(ServeEnd::server("result_unavailable")),
        },
        Dispatch::Appearance(None) => {
            return Err(ServeEnd::server("result_unavailable"));
        }
        Dispatch::Step(outcome, step_wait_ms) => (
            if matches!(
                outcome.disposition,
                StepDisposition::Rejected | StepDisposition::TooEarly
            ) {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            detailed(
                session,
                encode_step_outcome(
                    session.selected_capabilities.as_slice(),
                    outcome.disposition,
                    if session.step_wait_receipts {
                        *step_wait_ms
                    } else {
                        0
                    },
                ),
                RefusalDetail::of_step(outcome.disposition),
            ),
        ),
        Dispatch::Use(outcome) => {
            let (disposition, gate_message_id) = negotiated_use_disposition(
                UseCapabilities {
                    item_use: session
                        .selected_capabilities
                        .contains(CAPABILITY_ITEM_USE_V1),
                    world_interaction: session
                        .selected_capabilities
                        .contains(CAPABILITY_WORLD_INTERACTION_V1),
                    rune_use: session
                        .selected_capabilities
                        .contains(CAPABILITY_RUNE_USE_V1),
                },
                outcome.disposition,
                outcome.gate_message_id,
            );
            use oteryn_protocol_oteryn::world_object::{
                CAPABILITY_GAMEPLAY_OCCURRENCES_V1, CAPABILITY_POTION_UTTERANCE_V1,
                COMMAND_TYPE_USE_INTENT, encode_use_result_with_utterance,
                encode_use_reward_result,
            };
            let payload = if command.command_type == COMMAND_TYPE_USE_INTENT
                && session
                    .selected_capabilities
                    .contains(CAPABILITY_GAMEPLAY_OCCURRENCES_V1)
                && disposition == UseDisposition::Committed
                && let Some(occurrence) = outcome.reward_granted
            {
                encode_use_reward_result(occurrence)
            } else {
                encode_use_result_with_utterance(
                    disposition,
                    gate_message_id,
                    outcome.potion_utterance.filter(|_| {
                        session
                            .selected_capabilities
                            .contains(CAPABILITY_POTION_UTTERANCE_V1)
                    }),
                )
            };
            (
                if disposition == UseDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                detailed(
                    session,
                    payload.unwrap_or_else(|_| {
                        encode_use_result(UseDisposition::Rejected).unwrap_or_default()
                    }),
                    outcome
                        .refusal_detail
                        .or(RefusalDetail::of_use(outcome.disposition)),
                ),
            )
        }
        Dispatch::ParameterSpell(outcome, payload) => (
            if outcome.disposition == SpellCastDisposition::Rejected {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            std::mem::take(payload),
        ),
        Dispatch::Spell(outcome) => (
            if outcome.disposition == SpellCastDisposition::Rejected {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            encode_spell_cast_result(outcome.disposition),
        ),
        Dispatch::UseItem(decision, _) => {
            let disposition = match decision {
                OpenDecision::Open => UseDisposition::Committed,
                OpenDecision::TooFar => UseDisposition::TooFar,
                OpenDecision::StaleState => UseDisposition::StaleState,
                OpenDecision::NothingToUse => UseDisposition::NothingToUse,
            };
            (
                CommandStatus::Accepted,
                detailed(
                    session,
                    encode_use_result(disposition).unwrap_or_default(),
                    RefusalDetail::of_use(disposition),
                ),
            )
        }
        Dispatch::CarriedBatch(reply) => (CommandStatus::Accepted, reply.encode().to_vec()),
        Dispatch::ContainerSort(_) => (CommandStatus::Accepted, vec![1]),
        Dispatch::ContainerPage(page, _) => (CommandStatus::Accepted, page.encode().to_vec()),
        Dispatch::ContainerView(outcome, _) => (
            CommandStatus::Accepted,
            encode_container_view_result(*outcome),
        ),
        Dispatch::Stash(reply) => {
            let Ok(payload) = oteryn_protocol_oteryn::stash::encode_stash_reply(reply) else {
                return Err(ServeEnd::server("frame_encode_failed"));
            };
            // Validate the complete native result at its production wire boundary.
            let Ok(decoded) = oteryn_protocol_oteryn::stash::decode_stash_reply(&payload) else {
                return Err(ServeEnd::server("frame_encode_failed"));
            };
            if &decoded != reply {
                return Err(ServeEnd::server("result_unavailable"));
            }
            (CommandStatus::Accepted, payload)
        }
        Dispatch::Depot(reply) => {
            let Ok(payload) = oteryn_protocol_oteryn::depot::encode_depot_reply(reply) else {
                return Err(ServeEnd::server("frame_encode_failed"));
            };
            (CommandStatus::Accepted, payload)
        }
        Dispatch::ItemMove(outcome, _, detail) => {
            let Ok(payload) = encode_item_move_result_for(
                *outcome,
                ItemMoveSelection {
                    equip_drop: session
                        .selected_capabilities
                        .contains(CAPABILITY_ITEM_EQUIP_DROP_V1),
                    world_interaction: session
                        .selected_capabilities
                        .contains(CAPABILITY_WORLD_INTERACTION_V1),
                    ..ItemMoveSelection::default()
                },
            ) else {
                return Err(ServeEnd::server("result_unavailable"));
            };
            (
                CommandStatus::Accepted,
                detailed(
                    session,
                    payload,
                    detail.or(RefusalDetail::of_item_move(*outcome)),
                ),
            )
        }
        Dispatch::Look(result, selection) => {
            let Ok(payload) = encode_item_look_result_for(result, *selection) else {
                return Err(ServeEnd::server("frame_encode_failed"));
            };
            (CommandStatus::Accepted, payload)
        }
        Dispatch::Reply(status, payload)
            if command.command_type == oteryn_protocol_oteryn::market::COMMAND_TYPE_MARKET
                || command.command_type
                    == oteryn_protocol_oteryn::depot_search::COMMAND_TYPE_DEPOT_SEARCH
                || routing::store::is_committed_claim(command.command_type, *status, payload) =>
        {
            (*status, payload.clone())
        }
        Dispatch::Reply(status, payload) => (*status, std::mem::take(payload)),
        Dispatch::NpcShop(status, payload, _, _) => (*status, std::mem::take(payload)),
        Dispatch::QuestLog(_) => (CommandStatus::Accepted, Vec::new()),
        Dispatch::Chat(result, _) => {
            let Ok(payload) = encode_chat_intent_result(result) else {
                return Err(ServeEnd::server("frame_encode_failed"));
            };
            (
                if result.disposition == oteryn_protocol_oteryn::chat::ChatDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                payload,
            )
        }
        Dispatch::Npc(disposition, _, _) | Dispatch::NpcTrade(disposition, _, _) => (
            if *disposition == NpcIntentDisposition::Rejected {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            encode_npc_intent_result(*disposition),
        ),
        Dispatch::Attack(disposition) => (
            if *disposition == AttackIntentDisposition::Rejected {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            encode_attack_intent_result(*disposition),
        ),
        Dispatch::Logout(result) => {
            let Ok(payload) = encode_logout_result(*result) else {
                return Err(ServeEnd::server("frame_encode_failed"));
            };
            (
                if result.is_rejected() {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                payload,
            )
        }
        Dispatch::NoLogout => (
            CommandStatus::Rejected,
            encode_logout_result(LogoutResult::Busy)
                .map_err(|_| ServeEnd::server("frame_encode_failed"))?,
        ),
        Dispatch::Achievements(AccountAchievementsReply::Page(payload)) => {
            // The page is written once; move it out instead of copying up to 32 KiB.
            (CommandStatus::Accepted, std::mem::take(payload))
        }
        Dispatch::CharacterInfo(
            crate::gameplay_transport::character_info::CharacterInfoReply::Payload(payload),
        ) => (CommandStatus::Accepted, std::mem::take(payload)),
        Dispatch::Timed(payload, _) => (
            if crate::gameplay_transport::timed_item_wire::accepted(payload) {
                CommandStatus::Accepted
            } else {
                CommandStatus::Rejected
            },
            std::mem::take(payload),
        ),
        Dispatch::Guild(payload) | Dispatch::Highscores(HighscoresReply::Result(payload)) => {
            (CommandStatus::Accepted, std::mem::take(payload))
        }
        // An unavailable input permit can refuse a core STEP before it reaches movement.
        Dispatch::Unregistered if command.command_type == COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT => (
            CommandStatus::Rejected,
            encode_step_outcome(
                session.selected_capabilities.as_slice(),
                StepDisposition::Rejected,
                0,
            ),
        ),
        // USE can be refused before it reaches its owner when input is unavailable.
        Dispatch::Unregistered
            if command.command_type
                == oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT =>
        {
            let disposition = UseDisposition::Rejected;
            let payload = encode_use_result(disposition)
                .map_err(|_| ServeEnd::server("frame_encode_failed"))?;
            (
                CommandStatus::Rejected,
                detailed(session, payload, RefusalDetail::of_use(disposition)),
            )
        }
        Dispatch::Unregistered
            if command.command_type
                == oteryn_protocol_oteryn::appearance::COMMAND_TYPE_APPEARANCE_CHANGE
                && session
                    .selected_capabilities
                    .contains(oteryn_protocol_oteryn::appearance::CAPABILITY_APPEARANCE_V1) =>
        {
            match appearance_change::result(
                oteryn_protocol_oteryn::appearance::ChangeResult::Rejected,
            ) {
                Some(result) => result,
                None => return Err(ServeEnd::server("result_unavailable")),
            }
        }
        Dispatch::Achievements(
            AccountAchievementsReply::Terminal(_) | AccountAchievementsReply::Rejected,
        )
        | Dispatch::CharacterInfo(
            crate::gameplay_transport::character_info::CharacterInfoReply::Rejected
            | crate::gameplay_transport::character_info::CharacterInfoReply::Terminal(_),
        )
        | Dispatch::Highscores(HighscoresReply::Terminal(_) | HighscoresReply::Rejected)
        | Dispatch::Unavailable
        | Dispatch::Unregistered => (CommandStatus::Rejected, Vec::new()),
    };
    let result = if let Dispatch::Achievements(AccountAchievementsReply::Terminal(error))
    | Dispatch::Highscores(HighscoresReply::Terminal(error))
    | Dispatch::CharacterInfo(
        crate::gameplay_transport::character_info::CharacterInfoReply::Terminal(error),
    ) = &dispatch
    {
        // Display contract §3.3, §4.3 and HIGHSCORES0 §6: fail closed with the registered
        // operation-terminal error and nothing else.
        encode_command_error_result(session.generation, sequence, command.command_id, *error)
    } else {
        encode_command_result(
            session.generation,
            sequence,
            command.command_id,
            status,
            &result_payload,
        )
    };
    let Ok(result) = result else {
        return Err(ServeEnd::server("result_unavailable"));
    };
    Ok(result)
}

pub(super) fn next_sequence(session: &ServeState) -> Result<u64, ServeEnd> {
    session
        .sequence
        .checked_add(1)
        .ok_or_else(|| ServeEnd::server("serve_state_unavailable"))
}
