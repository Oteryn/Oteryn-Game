//! Encode the command result without changing dispositions or payload bytes.

use super::*;

pub(super) fn encode(
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    dispatch: &mut Dispatch,
) -> Result<Vec<u8>, ServeEnd> {
    let (status, result_payload) = match dispatch {
        Dispatch::Vip(Some(payload)) => (CommandStatus::Accepted, payload.clone()),
        Dispatch::Vip(None) => {
            return Err(ServeEnd::Disconnected);
        }
        Dispatch::House(Some(payload)) => (CommandStatus::Accepted, payload.clone()),
        Dispatch::House(None) => {
            return Err(ServeEnd::Disconnected);
        }
        Dispatch::Training(result) => {
            let Ok(payload) =
                oteryn_protocol_oteryn::loop_training::encode_training_result(*result)
            else {
                return Err(ServeEnd::Disconnected);
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
            None => return Err(ServeEnd::Disconnected),
        },
        Dispatch::Appearance(None) => {
            return Err(ServeEnd::Disconnected);
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
            encode_step_outcome(
                session.selected_capabilities.as_slice(),
                outcome.disposition,
                if session.step_wait_receipts {
                    *step_wait_ms
                } else {
                    0
                },
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
            (
                if disposition == UseDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                encode_use_result_with_gate(disposition, gate_message_id).unwrap_or_else(|_| {
                    encode_use_result(UseDisposition::Rejected).unwrap_or_default()
                }),
            )
        }
        Dispatch::Spell(outcome) => (
            if outcome.disposition == SpellCastDisposition::Rejected {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            encode_spell_cast_result(outcome.disposition),
        ),
        Dispatch::UseItem(decision, _) => (
            CommandStatus::Accepted,
            encode_use_result(match decision {
                OpenDecision::Open => UseDisposition::Committed,
                OpenDecision::TooFar => UseDisposition::TooFar,
                OpenDecision::StaleState => UseDisposition::StaleState,
                OpenDecision::NothingToUse => UseDisposition::NothingToUse,
            })
            .unwrap_or_default(),
        ),
        Dispatch::ContainerView(outcome, _) => (
            CommandStatus::Accepted,
            encode_container_view_result(*outcome),
        ),
        Dispatch::Depot(reply) => {
            let Ok(payload) = oteryn_protocol_oteryn::depot::encode_depot_reply(reply) else {
                return Err(ServeEnd::Disconnected);
            };
            (CommandStatus::Accepted, payload)
        }
        Dispatch::ItemMove(outcome, _) => {
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
                return Err(ServeEnd::Disconnected);
            };
            (CommandStatus::Accepted, payload)
        }
        Dispatch::Look(result, selection) => {
            let Ok(payload) = encode_item_look_result_for(result, *selection) else {
                return Err(ServeEnd::Disconnected);
            };
            (CommandStatus::Accepted, payload)
        }
        Dispatch::Reply(status, payload) => (*status, std::mem::take(payload)),
        Dispatch::QuestLog(_) => (CommandStatus::Accepted, Vec::new()),
        Dispatch::Chat(result) => {
            let Ok(payload) = encode_chat_intent_result(result) else {
                return Err(ServeEnd::Disconnected);
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
                return Err(ServeEnd::Disconnected);
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
        Dispatch::Achievements(AccountAchievementsReply::Page(payload)) => {
            // The page is written once; move it out instead of copying up to 32 KiB.
            (CommandStatus::Accepted, std::mem::take(payload))
        }
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
                None => return Err(ServeEnd::Disconnected),
            }
        }
        Dispatch::Achievements(
            AccountAchievementsReply::Terminal(_) | AccountAchievementsReply::Rejected,
        )
        | Dispatch::Highscores(HighscoresReply::Terminal(_) | HighscoresReply::Rejected)
        | Dispatch::Unregistered => (CommandStatus::Rejected, Vec::new()),
    };
    let result = if let Dispatch::Achievements(AccountAchievementsReply::Terminal(error))
    | Dispatch::Highscores(HighscoresReply::Terminal(error)) = &dispatch
    {
        // Display contract §3.3, §4.3 and HIGHSCORES0 §6: fail closed with the registered
        // operation-terminal error and nothing else.
        encode_command_error_result(
            session.generation,
            session.sequence,
            command.command_id,
            *error,
        )
    } else {
        encode_command_result(
            session.generation,
            session.sequence,
            command.command_id,
            status,
            &result_payload,
        )
    };
    let Ok(result) = result else {
        return Err(ServeEnd::Disconnected);
    };
    Ok(result)
}
