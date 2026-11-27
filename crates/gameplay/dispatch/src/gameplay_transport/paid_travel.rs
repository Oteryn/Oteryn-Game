//! Ship/carpet confirmations feed the same travel hold and relocation used by Newhaven.
use super::*;
use crate::content::npc_catalogue::travel::TravelRoute;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::item_fee_burn::{FeeBurnCause, FeeBurnRequest, FeeChangeFacts};
use crate::durability::npc_travel::{PaidTravelRequest, TravelRefusal};
use crate::map::view::MapFacts;
use npc_command::{
    NpcReadRefusal, npc_read_attempt, progression_read_error, read_npc_backpack,
    read_npc_quest_fence,
};
use oteryn_protocol_oteryn::{
    chat::{ChatLine, ChatSpeaker, ChatSpeechMode},
    npc_service::{NpcConversation, NpcConversationState},
    world_spatial::ActorPosition,
};
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::num::NonZeroU64;

#[path = "paid_travel/quest_confirmation.rs"]
mod quest_confirmation;

#[derive(Clone)]
pub(in crate::gameplay_transport) struct PaidDeparture {
    pub(in crate::gameplay_transport) request: PaidTravelRequest,
    pub(in crate::gameplay_transport) speaker: ExactActorRef,
    speaker_ref: ChatSpeaker,
    speaker_name: String,
    speaker_position: ActorPosition,
}

impl PaidDeparture {
    /// The source actor was authenticated before the voyage. The traveller may be
    /// out of range or resuming a committed voyage when these lines are sent.
    fn success_reply(
        &self,
        chat: &chat_intent::ChatRuntime,
        session: GameSessionId,
    ) -> npc_command::NpcTalkOutcome {
        let farewell = "Set the sails!";
        chat.end_engagement(session, self.speaker);
        chat.queue_npc_reply(
            session,
            ChatLine::Local {
                speaker: self.speaker_ref,
                speaker_name: self.speaker_name.clone(),
                mode: ChatSpeechMode::Say,
                text: farewell.into(),
                position: self.speaker_position,
            },
        );
        // Domain 7 uses its conversation handle to attribute lines. The ordinary
        // conversation follow publishes the close after this final attributed line.
        let npc = EntityRef {
            identity: self.speaker_ref.identity,
            generation: self.speaker_ref.generation.get(),
        };
        npc_command::NpcTalkOutcome {
            conversation: Some(NpcConversation {
                lines: vec![format!("{}: {farewell}", self.speaker_name)],
                ..open_conversation(npc)
            }),
            inventory_changed: self.request.fee.fee_gold_units > 0,
            ..npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Ok)
        }
    }
}

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

enum TravelPreparationError {
    Read(NpcReadRefusal),
    Route(TravelRefusal),
}

impl From<NpcReadRefusal> for TravelPreparationError {
    fn from(error: NpcReadRefusal) -> Self {
        Self::Read(error)
    }
}

impl TravelPreparationError {
    fn reply(
        self,
        intent: &NpcTalkIntent,
        command: connection::UseCommand,
    ) -> npc_command::NpcTalkOutcome {
        match self {
            Self::Read(error) => error.reply(intent, command, "paid_travel_prepare"),
            Self::Route(cause) => refusal(intent, cause),
        }
    }
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
        let (departure, npc) = {
            let holds = self.states.newhaven_travel.holds.lock().ok()?;
            let held = holds.get(&command.game_session_id)?;
            if held.frozen.command.command_id().get() != command.command_id {
                return None;
            }
            let Departure::Paid(departure) = &held.route else {
                return None;
            };
            (departure.clone(), held.npc)
        };
        let result = self
            .reconcile_newhaven_travel(actor, command.game_session_id)
            .await;
        let intent = NpcTalkIntent {
            npc_actor: npc,
            text: String::new(),
        };
        Some(match result {
            Some(true) => departure.success_reply(&self.states.chat, command.game_session_id),
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
    fn paid_confirmation_refusal(
        &self,
        command: connection::UseCommand,
        intent: &NpcTalkIntent,
        prompt: Prompt,
        error: TravelPreparationError,
    ) -> npc_command::NpcTalkOutcome {
        let mut outcome = error.reply(intent, command);
        if outcome.disposition == NpcIntentDisposition::RateLimited
            && let Ok(mut prompts) = self.states.newhaven_travel.paid_prompts.lock()
        {
            prompts.insert(command.game_session_id, prompt);
            if let Some(conversation) = outcome.conversation.as_mut() {
                conversation.open = Some((intent.npc_actor, NpcConversationState::Confirming));
            }
        }
        outcome
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
        let (context, routes, previous) = {
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
            (context, entry.travel_routes.clone(), previous)
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
                text == route.keyword || text == trigger
            }
        });
        let confirming = text == "yes" || (route.is_none() && callback_confirmation);
        let Some(fence) = command.item_fence else {
            return if route.is_some() || (confirming && previous.is_some()) {
                NpcReadRefusal::Stale.reply(intent, command, "paid_travel_prepare")
            } else {
                ordinary
            };
        };
        let budget = AdmissionRetryBudget::for_command(self.root);
        let prompt = if confirming {
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
                    match self
                        .travel_fare(command, wreck_route.clone(), &budget)
                        .await
                    {
                        Ok(_) => prompt.route = wreck_route,
                        Err(TravelPreparationError::Route(
                            TravelRefusal::UnavailableRoute | TravelRefusal::PremiumRequired,
                        )) => {}
                        Err(error) => {
                            return self.paid_confirmation_refusal(command, intent, prompt, error);
                        }
                    }
                }
            }
            prompt
        } else {
            let Some(route) = route else {
                return ordinary;
            };
            let fare = match self.travel_fare(command, route.clone(), &budget).await {
                Ok(fare) => fare,
                Err(error) => return error.reply(intent, command),
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
                return NpcReadRefusal::Stale.reply(intent, command, "paid_travel_fare");
            }
            let Ok(mut prompts) = self.states.newhaven_travel.paid_prompts.lock() else {
                return NpcReadRefusal::Unavailable.reply(intent, command, "paid_travel_fare");
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
        let request = match self
            .paid_travel_request(actor, command, intent, &prompt, &budget)
            .await
        {
            Ok(request) => request,
            Err(error) => return self.paid_confirmation_refusal(command, intent, prompt, error),
        };
        let frozen = FrozenTravel {
            command: request.command,
            npc: context.2,
            route: request.route.route.key.clone(),
            fare: prompt.fare,
            arrival: request.arrival,
        };
        let departure = {
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
            let chat = &self.states.chat;
            let Some(speaker_name) = chat
                .npc_talk()
                .and_then(|book| book.display_name(&frozen.npc))
            else {
                return NpcReadRefusal::Unavailable.reply(intent, command, "paid_travel_prepare");
            };
            let Some(speaker) = runtime.visible_entities().npcs.into_iter().find(|npc| {
                npc.actor.placement_identity() == intent.npc_actor.identity
                    && npc.generation == intent.npc_actor.generation
            }) else {
                return NpcReadRefusal::Stale.reply(intent, command, "paid_travel_prepare");
            };
            let Some(generation) = NonZeroU64::new(speaker.generation) else {
                return NpcReadRefusal::Stale.reply(intent, command, "paid_travel_prepare");
            };
            let departure = Box::new(PaidDeparture {
                request,
                speaker: speaker.actor,
                speaker_ref: ChatSpeaker {
                    identity: speaker.actor.placement_identity(),
                    generation,
                },
                speaker_name: speaker_name.to_owned(),
                speaker_position: ActorPosition {
                    x: speaker.position.x,
                    y: speaker.position.y,
                    floor: speaker.position.floor,
                },
            });
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || states.has_pending_spell_commit(actor, session) {
                return NpcReadRefusal::Unavailable.reply(intent, command, "paid_travel_prepare");
            }
            let Ok(position) = runtime.read_actor_position(actor) else {
                return NpcReadRefusal::Unavailable.reply(intent, command, "paid_travel_prepare");
            };
            let capture = TravelCapture {
                session: fence,
                content: position.context(),
                position_revision: position.revision(),
            };
            let Ok(work) = runtime.issue_owner_work() else {
                return NpcReadRefusal::Unavailable.reply(intent, command, "paid_travel_prepare");
            };
            let Ok(owner) = runtime.owner_fence() else {
                return NpcReadRefusal::Unavailable.reply(intent, command, "paid_travel_prepare");
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
                return NpcReadRefusal::Unavailable.reply(intent, command, "paid_travel_prepare");
            }
            let Ok(mut holds) = self.states.newhaven_travel.holds.lock() else {
                return NpcReadRefusal::Unavailable.reply(intent, command, "paid_travel_prepare");
            };
            holds.insert(
                session,
                Invocation {
                    actor,
                    npc: intent.npc_actor,
                    capture,
                    frozen,
                    route: Departure::Paid(departure.clone()),
                    slot,
                    refusal: None,
                    committed: false,
                    placed: None,
                },
            );
            departure
        };
        match self.reconcile_newhaven_travel(actor, session).await {
            Some(true) => departure.success_reply(&self.states.chat, session),
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

    async fn travel_fare(
        &self,
        command: connection::UseCommand,
        route: TravelRoute,
        budget: &AdmissionRetryBudget<'_>,
    ) -> Result<u64, TravelPreparationError> {
        let current = read_npc_quest_fence(self, command, budget).await?;
        if Some(current.item_fence()) != command.item_fence {
            return Err(NpcReadRefusal::Stale.into());
        }
        budget
            .retry_command_guard(async || {
                npc_read_attempt(
                    self.root
                        .quote_npc_travel(
                            self.character,
                            self.holder,
                            current,
                            route.clone(),
                            &self.premium_coordinator,
                        )
                        .await,
                    progression_read_error,
                )
            })
            .await
            .map_err(NpcReadRefusal::Durability)?
            .map_err(|error| match error {
                CharacterProgressionError::AuthorityRejected
                | CharacterProgressionError::CharacterRevisionMismatch => NpcReadRefusal::Stale,
                CharacterProgressionError::Unavailable(error) => NpcReadRefusal::Durability(error),
                _ => NpcReadRefusal::Unavailable,
            })?
            .map_err(TravelPreparationError::Route)
    }

    async fn paid_travel_request(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: &NpcTalkIntent,
        prompt: &Prompt,
        budget: &AdmissionRetryBudget<'_>,
    ) -> Result<PaidTravelRequest, TravelPreparationError> {
        let current = read_npc_quest_fence(self, command, budget).await?;
        if current.item_fence() != prompt.fence {
            return Err(NpcReadRefusal::Stale.into());
        }
        let runtime = self.runtime.lock().await;
        self.states
            .npc_conversations
            .current_npc(
                &self.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                intent,
            )
            .ok_or(NpcReadRefusal::Stale)?;
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
            return Err(TravelPreparationError::Route(
                TravelRefusal::UnavailableRoute,
            ));
        }
        let (gold, platinum, crystal) = npc_command::bank_coin_definitions(self, &runtime)
            .ok_or(NpcReadRefusal::Unavailable)?;
        let index =
            super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                .ok_or(NpcReadRefusal::Unavailable)?;
        let backpack = crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: index
                .revision("oteryn:item.tibia.i2854")
                .ok_or(NpcReadRefusal::Unavailable)?
                .into(),
        };
        drop(runtime);
        let native = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or(NpcReadRefusal::Unavailable)?;
        let stored = read_npc_backpack(self, current.character_id, budget).await?;
        let backpack = super::super::item_ref_admission::generation_item_facts(
            native,
            stored
                .as_ref()
                .map(|stored| &stored.backpack.definition)
                .unwrap_or(&backpack),
        )
        .map_err(|_| NpcReadRefusal::Unavailable)?;
        let derive = |slot: u8| {
            super::super::npc_trade_command::derived(
                format!("oteryn:npc-travel:v1:{slot}").as_bytes(),
                command.game_session_id,
                command.command_id,
            )
        };
        let occurrence =
            TravelOccurrence::from_bytes(derive(0)).map_err(|_| NpcReadRefusal::Unavailable)?;
        let destination = &prompt.route.route.destination;
        Ok(PaidTravelRequest {
            command: crate::foundation::CommandRef::new(
                command.game_session_id,
                crate::foundation::CommandId::new(command.command_id)
                    .map_err(|_| NpcReadRefusal::Stale)?,
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
