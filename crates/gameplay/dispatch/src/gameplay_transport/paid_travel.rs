//! Ship/carpet confirmations feed the same travel hold and relocation used by Newhaven.
use super::*;
use crate::content::npc_catalogue::travel::TravelRoute;
use crate::durability::item_fee_burn::{FeeBurnCause, FeeBurnRequest, FeeChangeFacts};
use crate::durability::npc_travel::{PaidTravelRequest, TravelRefusal};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::npc_service::{NpcConversation, NpcConversationState};
use oteryn_simulation_determinism::SemanticTimeMicros;

#[path = "paid_travel/quest_confirmation.rs"]
mod quest_confirmation;

#[derive(Clone)]
pub(super) struct Prompt {
    npc: EntityRef,
    context: (u64, u64, String),
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    opened: u64,
    route: TravelRoute,
    fare: u64,
    storm_roll: Option<bool>,
}

fn contains_whole_keyword(text: &str, keyword: &str) -> bool {
    text.match_indices(keyword).any(|(start, matched)| {
        !text[..start].ends_with(char::is_alphanumeric)
            && !text[start + matched.len()..].starts_with(char::is_alphanumeric)
    })
}

fn open_conversation(npc: EntityRef) -> NpcConversation {
    NpcConversation {
        open: Some((npc, NpcConversationState::Open)),
        lines: Vec::new(),
    }
}

fn refusal(intent: &NpcTalkIntent, cause: TravelRefusal) -> npc_command::NpcTalkOutcome {
    let line = match cause {
        TravelRefusal::UnavailableRoute => "I'm sorry but I don't sail there.",
        TravelRefusal::PremiumRequired => {
            "I'm sorry, but you need a premium account in order to travel."
        }
        TravelRefusal::CombatBlocked => {
            "First get rid of those blood stains! You are not going to ruin my vehicle!"
        }
        TravelRefusal::Exhausted => {
            "Sorry, but you need to wait three seconds before travel again."
        }
        TravelRefusal::ArrivalPending => "You cannot travel while another arrival is pending.",
        TravelRefusal::StaleQuote => {
            "The offer has expired. Please ask for your destination again."
        }
        TravelRefusal::InsufficientFunds => "You don't have enough money.",
    };
    npc_command::NpcConversations::bank_reply(intent, line.into())
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn follow_npc_session(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        shown: Option<EntityRef>,
    ) -> Option<npc_command::NpcUpdate> {
        let runtime = self.runtime.lock().await;
        let conversations = &self.states.npc_conversations;
        let chat = &self.states.chat;
        let mut update = conversations.follow(chat, &runtime, session, shown);
        if let Ok(mut prompts) = self.states.newhaven_travel.paid_prompts.lock()
            && let Some(prompt) = prompts.get(&session)
        {
            let npc = prompt.npc;
            let intent = NpcTalkIntent {
                npc_actor: npc,
                text: String::new(),
            };
            if prompt.is_live(self.owner_now().get())
                && conversations
                    .current_npc(chat, &runtime, actor, session, &intent)
                    .as_ref()
                    == Some(&prompt.context)
            {
                if let Some((shown_npc, state)) = update
                    .as_mut()
                    .and_then(|update| update.conversation.as_mut())
                    .and_then(|conversation| conversation.open.as_mut())
                    && *shown_npc == npc
                {
                    *state = NpcConversationState::Confirming;
                }
            } else {
                prompts.remove(&session);
                // Ordinary follow already reports a closed or changed NPC. The
                // same focused NPC needs an explicit correction out of Confirming.
                if shown == Some(npc) {
                    update
                        .get_or_insert_default()
                        .conversation
                        .get_or_insert_with(|| open_conversation(npc));
                }
            }
        }
        update
    }

    pub(in crate::gameplay_transport) async fn resume_paid_travel(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Option<npc_command::NpcTalkOutcome> {
        let (fare, npc) = {
            let holds = self.states.newhaven_travel.holds.lock().ok()?;
            let held = holds.get(&command.game_session_id)?;
            if held.frozen.command.command_id().get() != command.command_id
                || !matches!(held.route, Departure::Paid(_))
            {
                return None;
            }
            (held.frozen.fare, held.npc)
        };
        let result = self
            .reconcile_newhaven_travel(actor, command.game_session_id)
            .await;
        let intent = NpcTalkIntent {
            npc_actor: npc,
            text: String::new(),
        };
        Some(match result {
            Some(true) => {
                let mut outcome =
                    npc_command::NpcConversations::bank_reply(&intent, "Set the sails!".into());
                outcome.inventory_changed = fare > 0;
                if let Some(conversation) = outcome.conversation.as_mut() {
                    conversation.open = None;
                }
                outcome
            }
            Some(false) => {
                let cause = self
                    .states
                    .newhaven_travel
                    .holds
                    .lock()
                    .ok()
                    .and_then(|mut holds| holds.remove(&command.game_session_id))
                    .and_then(|held| held.refusal)
                    .unwrap_or(TravelRefusal::UnavailableRoute);
                refusal(&intent, cause)
            }
            None => npc_command::NpcTalkOutcome {
                unknown: true,
                ..npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            },
        })
    }
    pub(in crate::gameplay_transport) fn has_pending_paid_decline(
        &self,
        command: connection::UseCommand,
        intent: &NpcTalkIntent,
        context: &(u64, u64, String),
        received_at: SemanticTimeMicros,
    ) -> bool {
        let text = intent.text.to_ascii_lowercase();
        let Some(fence) = command.item_fence else {
            return false;
        };
        let Some(entry) = self
            .states
            .chat
            .npc_talk()
            .and_then(|book| book.get(&context.2))
        else {
            return false;
        };
        self.states
            .newhaven_travel
            .paid_prompts
            .lock()
            .is_ok_and(|prompts| {
                prompts.get(&command.game_session_id).is_some_and(|prompt| {
                    prompt.matches_confirmation(intent.npc_actor, context, fence, received_at.get())
                        && entry
                            .travel_decline(&prompt.route.route.key, &text)
                            .is_some()
                })
            })
    }

    pub(super) async fn paid_travel_talk(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: &NpcTalkIntent,
        mut ordinary: npc_command::NpcTalkOutcome,
        received_at: SemanticTimeMicros,
    ) -> npc_command::NpcTalkOutcome {
        let session = command.game_session_id;
        let text = intent.text.trim().to_ascii_lowercase();
        let (context, routes, fence, previous, source_confirmation) = {
            let runtime = self.runtime.lock().await;
            let previous = match self.states.newhaven_travel.paid_prompts.lock() {
                Ok(mut prompts) => prompts.remove(&session),
                Err(_) => return ordinary,
            };
            let conversations = &self.states.npc_conversations;
            let chat = &self.states.chat;
            if ordinary.conversation.is_none()
                && let Some(prompt) = &previous
            {
                let update = conversations
                    .follow(chat, &runtime, session, Some(prompt.npc))
                    .unwrap_or_default();
                ordinary.conversation = Some(
                    update
                        .conversation
                        .unwrap_or_else(|| open_conversation(prompt.npc)),
                );
                ordinary.window = ordinary.window.or(update.window);
            }
            if ordinary.disposition != NpcIntentDisposition::Ok
                || ordinary.unknown
                || self.states.spell_states.lock().await.is_dead(actor)
            {
                return ordinary;
            }
            let Some(context) = conversations.current_npc(chat, &runtime, actor, session, intent)
            else {
                return ordinary;
            };
            let Some(book) = chat.npc_talk() else {
                return ordinary;
            };
            let Some(entry) = book.get(&context.2) else {
                return ordinary;
            };
            if entry.travel_routes.is_empty() {
                return ordinary;
            }
            let Some(fence) = command.item_fence else {
                return ordinary;
            };
            let source_confirmation = previous
                .as_ref()
                .is_some_and(|prompt| prompt.matches_source_confirmation(&text));
            if let Some(prompt) = previous.as_ref()
                && !source_confirmation
                && prompt.matches_confirmation(intent.npc_actor, &context, fence, received_at.get())
                && let Some(decline) = entry.travel_decline(&prompt.route.route.key, &text)
            {
                let ungreet = decline.ungreet;
                if ungreet
                    && !self.states.chat.engaged(session).is_some_and(|npc| {
                        npc.placement_identity() == intent.npc_actor.identity
                            && npc.actor_local_generation() == intent.npc_actor.generation
                            && self.states.chat.end_engagement(session, npc)
                    })
                {
                    return ordinary;
                }
                if decline.reset_topic {
                    npc_command::catalogue::reset_travel_topic(
                        &self.states.npc_conversations,
                        session,
                        &context,
                    );
                }
                let name = self.states.chat.speaker_name(session).unwrap_or_default();
                let window = if ungreet {
                    self.states.npc_conversations.close_window(session, None)
                } else {
                    ordinary.window
                };
                return npc_command::NpcTalkOutcome {
                    conversation: Some(NpcConversation {
                        open: (!ungreet).then_some((intent.npc_actor, NpcConversationState::Open)),
                        lines: decline
                            .reply
                            .iter()
                            .map(|line| {
                                oteryn_gameplay_npc::quest_greetings::render_player_name(
                                    line, &name,
                                )
                            })
                            .collect(),
                    }),
                    window,
                    ..npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Ok)
                };
            }
            (
                context,
                entry.travel_routes.clone(),
                fence,
                previous,
                source_confirmation,
            )
        };
        if previous.as_ref().is_some_and(|prompt| {
            prompt.route.crystal_callback
                && prompt.npc == intent.npc_actor
                && prompt.context == context
        }) && (contains_whole_keyword(&text, "no") || contains_whole_keyword(&text, "cancel"))
        {
            return ordinary;
        }
        let callback_confirmation = previous
            .as_ref()
            .is_some_and(|prompt| prompt.route.crystal_callback)
            && contains_whole_keyword(&text, "yes");
        let route = routes.into_iter().find(|route| {
            let trigger = route.route.key.replace('_', " ");
            if route.crystal_callback {
                contains_whole_keyword(&text, &trigger)
            } else {
                route.matches_keyword(&text)
            }
        });
        let prompt = if text == "yes"
            || source_confirmation
            || (route.is_none() && callback_confirmation)
        {
            let Some(mut prompt) = previous else {
                return ordinary;
            };
            if !prompt.matches_confirmation(intent.npc_actor, &context, fence, received_at.get()) {
                return refusal(intent, TravelRefusal::StaleQuote);
            }
            if prompt.route.service == "oteryn:service.travel.jack_fate"
                && prompt.route.route.key == "thais"
            {
                // Crystal 201189f8, npc/jack_fate.lua:86–99. The warning and the
                // second confirmation are part of the same fenced conversation.
                let Some(storm) = prompt.storm_roll else {
                    let Some(roll) = self.states.newhaven_travel.roll_jack_fate_storm() else {
                        return refusal(intent, TravelRefusal::UnavailableRoute);
                    };
                    prompt.storm_roll = Some(roll);
                    let Ok(mut prompts) = self.states.newhaven_travel.paid_prompts.lock() else {
                        return ordinary;
                    };
                    prompts.insert(session, prompt);
                    return npc_command::NpcConversations::reply_with_state(
                        intent,
                        "A tropical storm may wreck the ship. Do you still want to sail?".into(),
                        NpcConversationState::Confirming,
                    );
                };
                // The pending travel request seals the selected arrival for reconciliation.
                if storm && self.session_premium_gate(session).await.premium_current() {
                    // Tibia's Goroma passage starts at level 20. Apply the
                    // quest's Premium gate only to the shipwreck branch.
                    let mut wreck_route = prompt.route.clone();
                    wreck_route.route.min_level = Some(20);
                    wreck_route.route.premium = true;
                    let destination = &mut wreck_route.route.destination;
                    (destination.x, destination.y, destination.floor) = (32161, 32558, 6);
                    let Ok(Some(current)) = self.current_quest_fence(session).await else {
                        return refusal(intent, TravelRefusal::StaleQuote);
                    };
                    if current.item_fence() != fence {
                        return refusal(intent, TravelRefusal::StaleQuote);
                    }
                    match self
                        .root
                        .quote_npc_travel(
                            self.character,
                            self.holder,
                            current,
                            wreck_route.clone(),
                            &self.premium_coordinator,
                        )
                        .await
                    {
                        Ok(Ok(_)) => prompt.route = wreck_route,
                        Ok(Err(
                            TravelRefusal::UnavailableRoute | TravelRefusal::PremiumRequired,
                        )) => {}
                        Ok(Err(cause)) => return refusal(intent, cause),
                        Err(_) => {
                            if let Ok(mut prompts) = self.states.newhaven_travel.paid_prompts.lock()
                            {
                                prompts.insert(session, prompt);
                            }
                            return npc_command::NpcTalkOutcome {
                                unknown: true,
                                ..npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
                            };
                        }
                    }
                }
            }
            prompt
        } else {
            let Some(route) = route else {
                return ordinary;
            };
            let Ok(Some(current)) = self.current_quest_fence(session).await else {
                return ordinary;
            };
            if current.item_fence() != fence {
                return ordinary;
            }
            let fare = match self
                .root
                .quote_npc_travel(
                    self.character,
                    self.holder,
                    current,
                    route.clone(),
                    &self.premium_coordinator,
                )
                .await
            {
                Ok(Ok(fare)) => fare,
                Ok(Err(cause)) => return refusal(intent, cause),
                Err(_) => {
                    return npc_command::NpcTalkOutcome {
                        unknown: true,
                        ..npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
                    };
                }
            };
            let line = format!(
                "Do you seek a passage to {} for {fare} gold?",
                route.keyword
            );
            let runtime = self.runtime.lock().await;
            if self
                .states
                .npc_conversations
                .current_npc(&self.states.chat, &runtime, actor, session, intent)
                .as_ref()
                != Some(&context)
            {
                return ordinary;
            }
            let Ok(mut prompts) = self.states.newhaven_travel.paid_prompts.lock() else {
                return ordinary;
            };
            prompts.insert(
                session,
                Prompt {
                    npc: intent.npc_actor,
                    context,
                    fence,
                    opened: self.owner_now().get(),
                    route,
                    fare,
                    storm_roll: None,
                },
            );
            return npc_command::NpcConversations::reply_with_state(
                intent,
                line,
                NpcConversationState::Confirming,
            );
        };
        let Some(request) = self
            .paid_travel_request(actor, command, intent, &prompt)
            .await
        else {
            return refusal(intent, TravelRefusal::UnavailableRoute);
        };
        let frozen = FrozenTravel {
            command: request.command,
            npc: context.2,
            route: request.route.route.key.clone(),
            fare: prompt.fare,
            arrival: request.arrival,
        };
        {
            let mut runtime = self.runtime.lock().await;
            if self.states.npc_conversations.current_npc(
                &self.states.chat,
                &runtime,
                actor,
                session,
                intent,
            ) != Some(prompt.context)
            {
                return refusal(intent, TravelRefusal::StaleQuote);
            }
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || states.has_pending_spell_commit(actor, session) {
                return ordinary;
            }
            let Ok(position) = runtime.read_actor_position(actor) else {
                return ordinary;
            };
            let capture = TravelCapture {
                session: fence,
                content: position.context(),
                position_revision: position.revision(),
            };
            let Ok(work) = runtime.issue_owner_work() else {
                return ordinary;
            };
            let Ok(owner) = runtime.owner_fence() else {
                return ordinary;
            };
            let mut slot = TravelHoldSlot::default();
            if slot
                .start(
                    frozen.clone(),
                    capture.clone(),
                    work,
                    owner,
                    std::time::Instant::now(),
                )
                .is_err()
            {
                return ordinary;
            }
            let Ok(mut holds) = self.states.newhaven_travel.holds.lock() else {
                return ordinary;
            };
            holds.insert(
                session,
                Invocation {
                    actor,
                    npc: intent.npc_actor,
                    capture,
                    frozen,
                    route: Departure::Paid(Box::new(request)),
                    slot,
                    refusal: None,
                    committed: false,
                    placed: None,
                },
            );
        }
        match self.reconcile_newhaven_travel(actor, session).await {
            Some(true) => {
                let mut result =
                    npc_command::NpcConversations::bank_reply(intent, "Set the sails!".into());
                result.inventory_changed = prompt.fare > 0;
                if let Some(conversation) = result.conversation.as_mut() {
                    conversation.open = None;
                }
                result
            }
            Some(false) => {
                let cause = self
                    .states
                    .newhaven_travel
                    .holds
                    .lock()
                    .ok()
                    .and_then(|mut holds| holds.remove(&session))
                    .and_then(|held| held.refusal)
                    .unwrap_or(TravelRefusal::UnavailableRoute);
                refusal(intent, cause)
            }
            None => npc_command::NpcTalkOutcome {
                unknown: true,
                ..npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            },
        }
    }

    async fn paid_travel_request(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: &NpcTalkIntent,
        prompt: &Prompt,
    ) -> Option<PaidTravelRequest> {
        let current = self
            .current_quest_fence(command.game_session_id)
            .await
            .ok()??;
        if current.item_fence() != prompt.fence {
            return None;
        }
        let runtime = self.runtime.lock().await;
        self.states.npc_conversations.current_npc(
            &self.states.chat,
            &runtime,
            actor,
            command.game_session_id,
            intent,
        )?;
        let destination = &prompt.route.route.destination;
        let cell = crate::foundation::MovementLocalPosition {
            x: destination.x,
            y: destination.y,
            floor: destination.floor,
        };
        if !super::super::respawn_cell_admissible(
            self.movement_cells().index(),
            self.movement_cells().scope(),
            self.world_id,
            runtime.content_pin().server_artifact_digest(),
            &std::collections::BTreeSet::new(),
            cell,
        ) || self.bundle_map().is_some_and(|map| {
            crate::content::bundle_spell_tiles::tile_pos(crate::content::LogicalCell {
                x: cell.x,
                y: cell.y,
                z: i32::from(cell.floor),
            })
            .is_none_or(|tile| map.facts.house_tile(tile) || map.facts.doors().blocks(tile))
        }) {
            return None;
        }
        let (gold, platinum, crystal) = npc_command::bank_coin_definitions(self, &runtime)?;
        let index =
            super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)?;
        let backpack = crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: index.revision("oteryn:item.tibia.i2854")?.into(),
        };
        drop(runtime);
        let native = self.active_generation?.native_gameplay()?;
        let stored = self
            .root
            .read_character_backpack(self.character, current.character_id)
            .await
            .ok()?;
        let backpack = super::super::item_ref_admission::generation_item_facts(
            native,
            stored
                .as_ref()
                .map(|stored| &stored.backpack.definition)
                .unwrap_or(&backpack),
        )
        .ok()?;
        let derive = |slot: u8| {
            super::super::npc_trade_command::derived(
                format!("oteryn:npc-travel:v1:{slot}").as_bytes(),
                command.game_session_id,
                command.command_id,
            )
        };
        let occurrence = TravelOccurrence::from_bytes(derive(0)).ok()?;
        let destination = &prompt.route.route.destination;
        Some(PaidTravelRequest {
            command: crate::foundation::CommandRef::new(
                command.game_session_id,
                crate::foundation::CommandId::new(command.command_id).ok()?,
            ),
            arrival: PendingArrival {
                occurrence,
                world_id: self.world_id,
                destination: crate::durability::character_last_position::LastPositionCell {
                    x: destination.x,
                    y: destination.y,
                    floor: destination.floor,
                },
            },
            route: prompt.route.clone(),
            postman_catalogue: self.quest_catalogue.clone(),
            fee: FeeBurnRequest {
                cause: FeeBurnCause::NpcTravel {
                    service: prompt.route.service.clone(),
                    occurrence: occurrence.as_bytes(),
                },
                fee_gold_units: prompt.fare,
                transaction_id: derive(1),
                event_id: derive(2),
                occurred_at_unix_ms: super::super::npc_trade_command::unix_ms_now(),
                server_build_id: "oteryn-game-server".into(),
                change: FeeChangeFacts {
                    item_instance_ids: [derive(3), derive(4)],
                    gold,
                    platinum,
                    crystal,
                    backpack,
                },
            },
        })
    }
}
