//! Session task phases: receive one shell command, service the session, and publish its result.
use super::feature_0450_server_modal::protocol::Completion as ModalCompletion;
use super::{
    Outbox, feature_0043_vip::protocol::send_vip, public_class, session_stepper::publish,
    step_direction,
};
use crate::combat_input::PlayCommand;
use oteryn_session::LogoutResult;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;
/// How long the idle session task serves liveness probes; well inside the server's probe cadence.
const IDLE_SLICE: Duration = Duration::from_millis(20);

pub use super::stepper::Stepper;

mod scope_transition;
use scope_transition::{initialize, publish_scope_transition, query_initial_returner, reset_modal};

pub(in crate::play) async fn run_session_returning<T: Stepper>(
    session: T,
    commands: UnboundedReceiver<PlayCommand>,
    outbox: Outbox,
) -> T {
    let modal = super::feature_0450_server_modal::protocol::Driver::new(
        session.feature_selected(450),
    )
    .with_defaults(session.feature_selected(505))
    .with_input(session.feature_selected(
        oteryn_session::property_services::server_modal::input::CAPABILITY_SERVER_MODAL_INPUT_V1,
    ));
    run_session_returning_with_modal(session, commands, outbox, modal).await
}

#[cfg(test)]
pub(in crate::play) async fn run_session_with_modal<T: Stepper>(
    session: T,
    commands: UnboundedReceiver<PlayCommand>,
    outbox: Outbox,
    modal: super::feature_0450_server_modal::protocol::Driver,
) {
    run_session_returning_with_modal(session, commands, outbox, modal).await;
}

async fn run_session_returning_with_modal<T: Stepper>(
    mut session: T,
    commands: UnboundedReceiver<PlayCommand>,
    outbox: Outbox,
    mut modal: super::feature_0450_server_modal::protocol::Driver,
) -> T {
    modal = modal.with_outbox(&outbox);
    outbox.performance_reset();
    let commands = std::sync::Arc::new(std::sync::Mutex::new(commands));
    outbox.scope_receiver(&commands);
    if outbox.session_cancelled() {
        return session;
    }
    if let Err(error) = initialize(&session, &outbox) {
        outbox.end(public_class(&error));
        return session;
    }
    let mut held = Vec::new();
    // Publish the admission queue before any query can append a new scope's
    // snapshot with the same initial sequence and revisions.
    publish(&mut session, &outbox, &mut held);
    let mut personal_marks_refresh = std::time::Instant::now();
    let mut read_initial_features = !outbox.scope_waiting();
    let mut query_returner = true;
    'session: loop {
        if outbox.session_cancelled() {
            return session;
        }
        if outbox.take_scope_modal_reset() {
            reset_modal(&session, &outbox, &mut modal);
        }
        if outbox.scope_needs_initialize() {
            if let Err(error) = initialize(&session, &outbox) {
                outbox.end(public_class(&error));
                return session;
            }
            read_initial_features = true;
        }
        if std::mem::take(&mut read_initial_features) {
            // An initial recovery receipt clears the admission mailbox. Wait for
            // the destination to be presented and initialized before querying.
            if query_initial_returner(
                &mut session,
                &outbox,
                &mut held,
                &mut modal,
                &mut query_returner,
            )
            .await
            {
                continue 'session;
            }
            for command in outbox.server_info_initial_commands() {
                let Some(query) = oteryn_session::server_info::Query::decode(&command.payload)
                else {
                    continue;
                };
                match session.feature(&command).await {
                    Ok(result) => outbox.server_info_result(&query, &result),
                    Err(error) => {
                        if publish_scope_transition(
                            &error,
                            &mut session,
                            &outbox,
                            &mut held,
                            &mut modal,
                        ) {
                            continue 'session;
                        }
                        outbox.end(public_class(&error));
                        return session;
                    }
                }
            }
        }
        let scope_waiting = outbox.scope_waiting();
        match if scope_waiting {
            Ok(None)
        } else {
            modal.poll(&mut session).await
        } {
            Ok(Some(ModalCompletion::Answered(answer))) => outbox.store_modal_answered(answer),
            Ok(Some(ModalCompletion::LoggedOut)) => {
                outbox.logged_out();
                return session;
            }
            Ok(None) => {}
            Err(error) => {
                if publish_scope_transition(&error, &mut session, &outbox, &mut held, &mut modal) {
                    continue;
                }
                outbox.end(public_class(&error));
                return session;
            }
        }
        if !scope_waiting
            && session.feature_selected(451)
            && std::time::Instant::now() >= personal_marks_refresh
        {
            let command = oteryn_client_ui::personal_map_marks::request();
            match session.feature(&command).await {
                Ok(result) => {
                    if let Err(error) = outbox.personal_map_marks_result(&result) {
                        outbox.end(public_class(&error));
                        return session;
                    }
                    publish(&mut session, &outbox, &mut held);
                }
                Err(error) => {
                    if publish_scope_transition(
                        &error,
                        &mut session,
                        &outbox,
                        &mut held,
                        &mut modal,
                    ) {
                        personal_marks_refresh = std::time::Instant::now() + Duration::from_secs(5);
                        continue;
                    }
                    outbox.end(public_class(&error));
                    return session;
                }
            }
            personal_marks_refresh = std::time::Instant::now() + Duration::from_secs(5);
        }

        if let Some(command) = outbox.server_info_quest_observation_command() {
            match session.feature(&command).await {
                Ok(result) => {
                    outbox.server_info_result(&oteryn_session::server_info::Query::Quests, &result)
                }
                Err(error) => {
                    if publish_scope_transition(
                        &error,
                        &mut session,
                        &outbox,
                        &mut held,
                        &mut modal,
                    ) {
                        continue;
                    }
                    outbox.end(public_class(&error));
                    return session;
                }
            }
            publish(&mut session, &outbox, &mut held);
        }
        if let Some(command) = outbox.highscores_continuation_command() {
            match session.server_mechanic(&command).await {
                Ok(result) => outbox.highscores_result(&result),
                Err(error) => {
                    if publish_scope_transition(
                        &error,
                        &mut session,
                        &outbox,
                        &mut held,
                        &mut modal,
                    ) {
                        continue;
                    }
                    outbox.end(public_class(&error));
                    return session;
                }
            }
            publish(&mut session, &outbox, &mut held);
        }
        let ended = super::feature_0014_container_tree::protocol::dispatch! {
            outbox.scope_command(&commands), session, outbox, held, publish;
            Ok(command @ (PlayCommand::SkillsCombat(_) | PlayCommand::HudStamina)) => match outbox.hud_character_query(&mut session, command).await {
                Ok(()) => {
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::SupportDelivery(command, path)) => {
                if !session.feature_selected(command.capability) {
                    continue;
                }
                match session.feature(&command).await {
                    Ok(result) => {
                        crate::support_delivery::acknowledge(&path, &command, &result);
                        continue;
                    }
                    Err(error) => error,
                }
            },
            Ok(PlayCommand::Feature(command)) => {
                let item_scope = outbox.bar_action_scope();
                let started = std::time::Instant::now();
                outbox.feature_sent(command.capability);
                match session.feature(&command).await {
                Ok(result) => {
                    outbox.performance_command_result(started.elapsed());
                    let mount_accepted = command.capability == 502 && result.status == oteryn_session::CommandStatus::Accepted;
                    match command.capability {
                        502 if !mount_accepted => outbox.mount_result(result.status),
                        7 => outbox.console_result(&result),
                        10 => outbox.analyser_result(&command, &result),
                        63 => outbox.market_details_result(&command, &result),
                        72 if command.kind==oteryn_client_runtime::task_board::task_board::soulpit::COMMAND_TYPE => outbox.soulseal_result(&command,&result),
                        72 | 474 => outbox.task_board_full_result(&result),
                        14 => outbox.player_trade_container_result(&result),
                        25 | 67 | 87 => outbox.properties_result(&command, &result),
                        53 => outbox.world_event_result(&result),
                        54 | 55 | 115 | 150 | 207 | 511 => outbox.properties_result(&command, &result),
                        39 | 49 | 57 | 134 => outbox.properties_result(&command, &result),
                        97 if command.kind == 113 && command.payload == [5] => outbox.premium_game_result(&command,&result),
                        97 | 149 => outbox.properties_result(&command, &result),
                        59 | 86 | 99 => outbox.properties_result(&command, &result),
                        76 | 103 | 146 | 152 | 447 => outbox.properties_result(&command, &result),
                        117 => outbox.boss_encounter_result(&result),
                        151 => outbox.daily_boost_result(&result),
                        182 => match command.kind {
                            738=>outbox.daily_reward_details_result(&result),
                            oteryn_session::daily_reward::returner::QUERY | oteryn_session::daily_reward::returner::CLAIM =>outbox.daily_reward_returner_result(command.kind,&result),
                            _ => {}
                        },
                        132 => outbox.trade_inspection_result(&command, &result),
                        170 => outbox.arena_result(&result),
                        195 | 448 => outbox.weapon_proficiency_result(&result),
                        148 if command.kind == 6121 => outbox.gem_presentation_result(&command, &result),
                        148 => outbox.wheel_presentation_result(&command, &result),
                        197 => outbox.cyclopedia_tracker_result(&result),
                        199 => outbox.cyclopedia_character_result(&result),
                        225 => outbox.boss_difficulty_result(&result),
                        497 => outbox.sanction_record_result(&result),
                        512 => outbox.channel_switch_result(&command, &result),
                        504 => outbox.tutorial_vocation_result(&result),
                        250 => outbox.hireling_management_result(&result),
                        272 => outbox.market_funds_result(&result),
                        276 => outbox.condition_status_result(&result),
                        384 => outbox.mechanics_status_result(&result),
                        277 => outbox.rest_status_result(&result),
                        278 => outbox.battle_instinct_status_result(&result),
                        279 | 370 => outbox.skill_progress_result(&result),
                        280 => outbox.party_effects_result(&result),
                        381 => outbox.potion_durations_result(&result),
                        382 => outbox.death_protection_result(&result),
                        449 => {
                            outbox.spellbook_details_result(&result);
                            for info_command in outbox.server_info_spell_commands() {
                                let query=oteryn_session::server_info::Query::decode(&info_command.payload);
                                match session.feature(&info_command).await {
                                    Ok(info_result)=>if let Some(query)=query {outbox.server_info_result(&query,&info_result);},
                                    Err(error)=>{if publish_scope_transition(&error,&mut session,&outbox,&mut held,&mut modal) {break;}outbox.end(public_class(&error));return session;}
                                }
                            }
                        },
                        500 => if let Some(query)=oteryn_session::server_info::Query::decode(&command.payload) {outbox.server_info_result(&query,&result);},
                        463 => outbox.bar_item_cooldowns_result(item_scope, &result),
                        _ => {}
                    }
                    publish(&mut session, &outbox, &mut held);
                    // The exchange can drain own-actor motion received before the reply.
                    if mount_accepted {
                        outbox.mount_result(result.status);
                    }
                    continue;
                }
                Err(error) => error,
            }},
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
                            63 if request.command_type==oteryn_client_runtime::coin_market::COMMAND => outbox.coin_market_result(&response),
                            64 => outbox.target_move_result(&request, &response),
                            82 => outbox.creature_push_result(&response),
                            98 | 132 | 200 | 232 | 285 => outbox.discovery_result(request.capability, &response),
                            9 | 507 => outbox.highscores_result(&response),
                            17 if request.command_type == oteryn_session::battle_metadata::COMMAND_TYPE_BATTLE_METADATA_QUERY => outbox.battle_metadata_result(&response),
                            14 => outbox.browse_field_result(&response),
                            152 => outbox.item_forge_result(&response),
                            153 => outbox.depot_search_result(&response),
                            154 if request.command_type==oteryn_client_runtime::coin_transfer::COMMAND => outbox.coin_gift_result(&response),
                            154 | 479 => outbox.store_result(&response),
                            15 if request.command_type == oteryn_client_runtime::world_interaction::world_object::COMMAND_TYPE_USE_INTENT =>
                                outbox.store_lamp_result(&response),
                            450 if request.command_type == oteryn_client_runtime::server_modal::COMMAND_TYPE_HIRELING_STORE_SELECTION => outbox.store_result(&response),
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
                    let query=oteryn_session::server_info::Query::Quests;
                    if let Some(info_command)=outbox.server_info_command(query.clone()) {
                        match session.feature(&info_command).await {
                            Ok(info_result)=>outbox.server_info_result(&query,&info_result),
                            Err(error)=>{if publish_scope_transition(&error,&mut session,&outbox,&mut held,&mut modal) {continue;}outbox.end(public_class(&error));return session;}
                        }
                    }
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
            Ok(PlayCommand::Stash(command)) => match outbox.stash_execute(&mut session, &command).await {
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
                let started = std::time::Instant::now();
                match session.step(step_direction(direction)).await {
                    Ok(outcome) => {
                        outbox.performance_command_result(started.elapsed());
                        outbox.action_feedback(session.refusal_detail().map(|detail| detail.reference_message()));
                        outbox.stepped(outcome);
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(command @ (PlayCommand::Combat(_) | PlayCommand::Battle(_) | PlayCommand::StopAttacking(_))) => {
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
                    return session;
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
            Ok(PlayCommand::Vip(request)) => match send_vip(&mut session, &outbox, &request).await {
                Ok(()) => {
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
            Ok(PlayCommand::Hireling(request)) => match session.hireling(&outbox.wire_hireling(request)).await {
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
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => return session,
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
                    outbox.chat_public_result(&intent, &outcome);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::ChatClearOldest) => {
                session.chat_clear_oldest();
                publish(&mut session, &outbox, &mut held);
                continue;
            }
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
        if publish_scope_transition(&ended, &mut session, &outbox, &mut held, &mut modal) {
            continue;
        }
        outbox.end(public_class(&ended));
        return session;
    }
}
