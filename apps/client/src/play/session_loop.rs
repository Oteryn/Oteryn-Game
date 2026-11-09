//! Session task phases: receive one shell command, service the session, and publish its result.
use super::{
    Outbox, feature_0043_vip::protocol::send_vip, public_class, session_stepper::publish,
    step_direction,
};
use crate::combat_input::PlayCommand;
use oteryn_session::LogoutResult;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

/// How long the session task serves liveness probes while no step is pending; well inside the
/// server's probe cadence.
const IDLE_SLICE: Duration = Duration::from_millis(20);

pub use super::stepper::Stepper;
/// Initializes the live map, sends commands and services session liveness.
pub async fn run_session<T: Stepper>(
    mut session: T,
    mut commands: UnboundedReceiver<PlayCommand>,
    outbox: Outbox,
) {
    if !session_initialize::initialize(&session, &outbox) {
        return;
    }
    let mut modal =
        super::feature_0450_server_modal::protocol::Driver::new(session.feature_selected(450));
    let mut held = Vec::new();
    loop {
        if let Err(error) = modal.poll(&mut session).await {
            outbox.end(public_class(&error));
            return;
        }
        let ended = super::feature_0014_container_tree::protocol::dispatch! {
            outbox.death_command(&mut commands), session, outbox, held, publish;
            Ok(PlayCommand::HudStamina) => match session.feature(&super::resource_hud::stamina_command()).await {
                Ok(result) => {
                    outbox.hud_stamina_result(&result);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Feature(command)) => match session.feature(&command).await {
                Ok(result) => {
                    match command.capability {
                        7 => outbox.console_result(&result),
                        10 => outbox.analyser_result(&command, &result),
                        72 => outbox.task_board_full_result(&result),
                        14 => outbox.player_trade_container_result(&result),
                        25 | 67 | 87 => outbox.properties_result(&command, &result),
                        53 => outbox.world_event_result(&result),
                        54 | 55 | 115 | 150 | 207 => outbox.properties_result(&command, &result),
                        39 | 57 | 134 => outbox.properties_result(&command, &result),
                        97 | 149 => outbox.properties_result(&command, &result),
                        59 | 86 | 99 => outbox.properties_result(&command, &result),
                        76 | 103 | 146 | 152 | 447 => outbox.properties_result(&command, &result),
                        117 => outbox.boss_encounter_result(&result),
                        151 => outbox.daily_boost_result(&result),
                        182 => outbox.daily_reward_details_result(&result),
                        170 => outbox.arena_result(&result),
                        195 | 448 => outbox.weapon_proficiency_result(&result),
                        197 => outbox.cyclopedia_tracker_result(&result),
                        199 => outbox.cyclopedia_character_result(&result),
                        225 => outbox.boss_difficulty_result(&result),
                        250 => outbox.hireling_management_result(&result),
                        276 => outbox.condition_status_result(&result),
                        384 => outbox.mechanics_status_result(&result),
                        277 => outbox.rest_status_result(&result),
                        278 => outbox.battle_instinct_status_result(&result),
                        279 | 370 => outbox.skill_progress_result(&result),
                        280 => outbox.party_effects_result(&result),
                        381 => outbox.potion_durations_result(&result),
                        382 => outbox.death_protection_result(&result),
                        449 => outbox.spellbook_details_result(&result),
                        _ => {}
                    }
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::BrowseFieldMove(intent)) => {
                match session.move_item(oteryn_client_runtime::inventory::MoveRequest {source:intent.source,destination:intent.destination,count:None}).await {
                    Ok(response) => {
                        outbox.browse_field_moved(response);
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::ServerMechanic(request)) => {
                match session.server_mechanic(&request).await {
                    Ok(response) => {
                        match request.capability {
                            290 if request.command_type==505 => outbox.cast_chat_result(&response),
                            290 => outbox.cast_control_result(&response),
                            291 => outbox.cast_chat_result(&response),
                            292 => outbox.cast_consent_result(&response),
                            64 => outbox.target_move_result(&request, &response),
                            82 => outbox.creature_push_result(&response),
                            98 | 132 | 200 | 232 | 285 => outbox.discovery_result(request.capability, &response),
                            9 => outbox.highscores_result(&response),
                            17 if request.command_type == oteryn_session::battle_metadata::COMMAND_TYPE_BATTLE_METADATA_QUERY => outbox.battle_metadata_result(&response),
                            14 => outbox.browse_field_result(&response),
                            152 => outbox.item_forge_result(&response),
                            153 => outbox.depot_search_result(&response),
                            154 => outbox.store_result(&response),
                            179 => outbox.hotkey_equip_result(&response),
                            230 => outbox.browse_field_result(&response),
                            _ => {}
                        }
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::Animus(payload)) => match session.animus(&payload).await {
                Ok(result) => {
                    outbox.animus_result(&result);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Wheel(request)) => match session.wheel(&request).await {
                Ok(response) => {
                    outbox.wheel_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Exiva(payload)) => match outbox.exiva_run(&mut session, &payload).await
            {
                Ok(()) => {
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::QuestLog(payload)) => match session.quest_log(&payload).await {
                Ok(result) => {
                    outbox.quest_result(&result);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::LoyaltySkills(payload)) => match session.loyalty_skills(&payload).await
            {
                Ok(result) => {
                    outbox.loyalty_skills_result(&result);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Stash(command)) => match session.stash(&command).await {
                Ok(response) => {
                    outbox.stash_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::TargetWalk(position)) => {
                match outbox.target_walk_run(&mut session, position).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::Step(direction)) => {
                match session.step(step_direction(direction)).await {
                    Ok(outcome) => {
                        outbox.action_feedback(session.refusal_detail().map(|detail| detail.reference_message()));
                        outbox.stepped(outcome);
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(command @ (PlayCommand::Combat(_) | PlayCommand::Battle(_))) => {
                match outbox.battle_run(&mut session, command).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::Logout) => match session.logout().await {
                Ok(Some(LogoutResult::Accepted)) => {
                    // The server ended the session and closes the transport.
                    outbox.logged_out();
                    return;
                }
                Ok(refused) => {
                    outbox.logout_refused(refused);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::TaskBoard(command)) => match session.task_board(&command).await {
                Ok(response) => {
                    outbox.task_board_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::NpcTalk(intent)) => {
                match outbox.dialogue_run(&mut session, &intent).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::NpcTrade(intent)) => {
                match outbox.npc_trade_run(&mut session, &intent).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::NpcShop(command)) => {
                match outbox.npc_shop_run(&mut session, &command).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::BossRewards(command)) => match session.boss_rewards(&command).await {
                Ok(response) => {
                    outbox.boss_rewards_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Prey(command)) => match session.prey(&command).await {
                Ok(response) => {
                    outbox.prey_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::DailyReward(command)) => match session.daily_reward(&command).await {
                Ok(response) => {
                    outbox.daily_reward_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Imbue(command)) => match session.imbue(&command).await {
                Ok(response) => {
                    outbox.imbue_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::PlayerTrade(request)) => match session.player_trade(&request).await {
                Ok(response) => {
                    publish(&mut session, &outbox, &mut held);
                    outbox.player_trade_result(&response);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Vip(request)) => match send_vip(&mut session, &request).await {
                Ok(response) => {
                    outbox.vip_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Guild(command)) => match outbox.guild_run(&mut session, &command).await
            {
                Ok(()) => {
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Party(request)) => match outbox.party_run(&mut session, &request).await
            {
                Ok(()) => {
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Hireling(request)) => match session.hireling(&request).await {
                Ok(response) => {
                    outbox.hireling_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                match session.serve(IDLE_SLICE).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => return,
            Ok(PlayCommand::Market(request)) => {
                match outbox.market_execute(&mut session, &request).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::MovementSpeech(intent)) => match session.chat_public(&intent).await {
                Ok(outcome) => {
                    outbox.chat_public_result(&outcome);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::ChatPublic(intent)) => match session.chat_public(&intent).await {
                Ok(outcome) => {
                    outbox.chat_result(&intent, &outcome);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::PlayerTyping(command)) => match session.player_typing(&command).await {
                Ok(response) => {
                    outbox.player_typing_result(command, response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
        };
        if matches!(ended, oteryn_session::SessionError::ActivityScopeChanged) {
            // A shared activity moves every participant, including one awaiting a source
            // command. Never replay queued source actions against the fresh destination.
            publish(&mut session, &outbox, &mut held);
            continue;
        }
        outbox.end(public_class(&ended));
        return;
    }
}

#[path = "session_initialize.rs"]
mod session_initialize;
