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
    outbox.discovery_initialize(&session);
    outbox.properties_initialize(&session);
    outbox.analyser_initialize(session.feature_selected(10));
    if let Err(error) = outbox
        .spell_catalog_initialize(session.feature_selected(287), session.feature_snapshot(100))
    {
        outbox.end(public_class(&error));
        return;
    }
    outbox.spell_bar_initialize(session.feature_snapshot(20));
    outbox.appearance_initialize(&session);
    outbox.equipment_initialize(session.equipment_selected());
    outbox.world_context_initialize(session.feature_selected(28), session.feature_selected(11));
    outbox.targeting_initialize(session.feature_selected(15));
    // Publish sources only after their equipment, movement and targeting routes are ready.
    outbox.inventory_initialize(session.inventory(), session.item_count_selected());
    outbox.container_initialize_with_house_capacity(
        session.container_views(),
        session.feature_selected(446),
    );
    outbox.target_walk_initialize(session.feature_selected(64));
    outbox.vitals(session.actor_vitals());
    outbox.channel_list_initialize(session.feature_snapshot(48));
    outbox.world_light_initialize(session.feature_snapshot(18));
    outbox.world_weather_initialize(session.feature_snapshot(22));
    outbox.map_initialize(session.feature_snapshot(17));
    outbox.animus_initialize(session.animus_selected());
    outbox.wheel_initialize(session.wheel_selected());
    outbox.exiva_initialize(session.exiva_selected());
    outbox.loyalty_skills_initialize(session.loyalty_skills_selected());
    outbox.mechanics_status_initialize(session.feature_selected(384));
    outbox.condition_status_initialize(session.feature_selected(276));
    outbox.rest_status_initialize(session.feature_selected(277));
    outbox.battle_instinct_status_initialize(session.feature_selected(278));
    outbox.skill_progress_initialize(session.feature_selected(279));
    outbox.skill_display_initialize(session.feature_selected(370));
    outbox.party_effects_initialize(session.feature_selected(280));
    outbox.potion_durations_initialize(session.feature_selected(381));
    outbox.death_protection_initialize(session.feature_selected(382));
    outbox.quest_initialize(session.quest_state());
    outbox.task_board_initialize(session.task_board_selected());
    outbox.stash_initialize(session.stash_selected());
    outbox.dialogue_initialize(&session);
    outbox.npc_shop_initialize(&session);
    outbox.prey_initialize(session.prey_selected());
    outbox.daily_reward_initialize(session.daily_reward_selected());
    outbox.boss_rewards_initialize(session.boss_rewards_selected());
    outbox.imbue_initialize(session.imbue_selected());
    outbox.player_trade_initialize(
        session.player_trade_selected(),
        session.player_trade_state(),
    );
    outbox.vip_initialize(session.vip_selected());
    outbox.guild_initialize(session.guild_selected());
    outbox.party_initialize(
        session.party_selected(),
        session.feature_selected(oteryn_session::CAPABILITY_CHAT_V1),
    );
    outbox.hireling_initialize(session.hireling_selected());
    outbox.hireling_management_initialize(session.feature_selected(250));
    outbox.market_initialize(session.market_selected());
    outbox.chat_with_movement_initialize(&session);
    outbox.chat_private_publish(&session);
    outbox.player_typing_initialize(session.player_typing_selected());
    outbox.arena_initialize(session.feature_selected(170));
    outbox.boss_encounter_initialize(session.feature_selected(117));
    outbox.boss_difficulty_initialize(session.feature_selected(225));
    outbox.cast_control_initialize(session.server_mechanic_selected(290));
    outbox.cast_consent_initialize(session.server_mechanic_selected(292));
    outbox.cast_chat_initialize(
        session.server_mechanic_selected(290) && session.server_mechanic_selected(291),
    );
    outbox.browse_field_initialize(session.server_mechanic_selected(230));
    if let Some(payload) = session.feature_snapshot(14) {
        outbox.browse_field_views(&payload);
    }
    if let Some(inventory) = session.inventory() {
        outbox.browse_field_inventory(inventory);
    }
    outbox.guild_chat_publish(&session);
    outbox.chat_initialize(session.feature_selected(oteryn_session::CAPABILITY_CHAT_V1));
    outbox.chat_party_room_initialize(session.feature_selected(240));
    outbox.cyclopedia_character_initialize(session.feature_selected(199));
    outbox.cyclopedia_tracker_initialize(session.feature_selected(197));
    outbox.world_event_initialize(session.feature_selected(53));
    outbox.daily_boost_initialize(session.feature_selected(151));
    outbox.highscores_initialize(session.server_mechanic_selected(9));
    outbox.depot_search_initialize(session.server_mechanic_selected(153));
    outbox.target_move_initialize(session.feature_selected(64));
    outbox.creature_push_initialize(session.feature_selected(82));
    outbox.hotkey_equip_initialize(session.server_mechanic_selected(179));
    outbox.hotkey_equip_items(session.hotkey_equip_items());
    outbox.item_forge_initialize(session.server_mechanic_selected(152));
    outbox.store_initialize(session.server_mechanic_selected(154));
    outbox.vip_presence_initialize(session.feature_selected(258));
    outbox.weapon_proficiency_initialize(session.feature_selected(195));
    outbox.weapon_proficiency_state_initialize(session.feature_selected(448));
    outbox.depot_initialize(session.feature_selected(37));
    outbox.achievements_initialize();
    outbox.player_trade_items(session.player_trade_items());
    let mut held = Vec::new();
    loop {
        let ended = super::feature_0014_container_tree::protocol::dispatch! {
            outbox.death_command(&mut commands), session, outbox, held, publish;
            Ok(PlayCommand::Feature(command)) => match session.feature(&command).await {
                Ok(result) => {
                    match command.capability {
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
