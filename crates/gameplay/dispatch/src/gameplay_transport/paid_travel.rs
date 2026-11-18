//! Ship/carpet confirmations feed the same travel hold and relocation used by Newhaven.
use super::*;
use crate::content::npc_catalogue::travel::TravelRoute;
use crate::durability::item_fee_burn::{FeeBurnCause, FeeBurnRequest, FeeChangeFacts};
use crate::durability::npc_travel::{PaidTravelRequest, TravelRefusal};
use crate::map::view::MapFacts;

#[path = "paid_travel/quest_confirmation.rs"]
mod quest_confirmation;
#[path = "paid_travel/svargrond.rs"]
mod svargrond;

use crate::durability::npc_travel::svargrond::{
    BUDDEL, ISKAN, Payment, RING, pays_ring, route as source_route,
};

#[derive(Clone)]
pub(super) struct Prompt {
    npc: EntityRef,
    context: (u64, u64, String),
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    opened: u64,
    route: TravelRoute,
    fare: u64,
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
    pub(in crate::gameplay_transport) async fn resume_paid_travel(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Option<npc_command::NpcTalkOutcome> {
        let (inventory_changed, is_iskan, npc) = {
            let holds = self.states.newhaven_travel.holds.lock().ok()?;
            let held = holds.get(&command.game_session_id)?;
            if held.frozen.command.command_id().get() != command.command_id
                || !matches!(held.route, Departure::Paid(_))
            {
                return None;
            }
            (held.inventory_changed(), held.frozen.npc == ISKAN, held.npc)
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
                let mut outcome = npc_command::NpcConversations::bank_reply(
                    &intent,
                    if is_iskan {
                        String::new()
                    } else {
                        "Set the sails!".into()
                    },
                );
                outcome.inventory_changed = inventory_changed;
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
    pub(super) async fn paid_travel_talk(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: &NpcTalkIntent,
        ordinary: npc_command::NpcTalkOutcome,
    ) -> npc_command::NpcTalkOutcome {
        let session = command.game_session_id;
        let text = intent.text.trim().to_ascii_lowercase();
        let (context, routes) = {
            let runtime = self.runtime.lock().await;
            if self.states.spell_states.lock().await.is_dead(actor) {
                return ordinary;
            }
            let Some(context) = self.states.npc_conversations.current_npc(
                &self.states.chat,
                &runtime,
                actor,
                session,
                intent,
            ) else {
                if let Ok(mut prompts) = self.states.newhaven_travel.paid_prompts.lock() {
                    prompts.remove(&session);
                }
                return ordinary;
            };
            let Some(book) = self.states.chat.npc_talk() else {
                return ordinary;
            };
            let Some(entry) = book.get(&context.2) else {
                return ordinary;
            };
            if entry.travel_routes.is_empty() && !matches!(context.2.as_str(), BUDDEL | ISKAN) {
                return ordinary;
            }
            (context, entry.travel_routes.clone())
        };
        let Some(fence) = command.item_fence else {
            return ordinary;
        };
        let now = self.owner_now().get();
        let previous = match self.states.newhaven_travel.paid_prompts.lock() {
            Ok(mut prompts) => prompts.remove(&session),
            Err(_) => return ordinary,
        };
        let mut prompt = if text == "yes" {
            let Some(prompt) = previous else {
                return ordinary;
            };
            if !prompt.matches_confirmation(intent.npc_actor, &context, fence, now) {
                return refusal(intent, TravelRefusal::StaleQuote);
            }
            prompt
        } else {
            let npc = context.2.as_str();
            let keyword = if npc == ISKAN
                && oteryn_gameplay_quests::quest::dark_trails::message_contains(&text, "passage")
            {
                "passage"
            } else {
                text.as_str()
            };
            let declined_ring = previous.as_ref().filter(|prompt| {
                text == "no"
                    && pays_ring(&prompt.route)
                    && prompt.matches_confirmation(intent.npc_actor, &context, fence, now)
            });
            if text == "no" && npc == BUDDEL && declined_ring.is_none() && previous.is_some() {
                return npc_command::NpcConversations::bank_reply(
                    intent,
                    "You will miss the trip.".into(),
                );
            }
            let native = svargrond::select(npc, keyword, declined_ring);
            // Match the complete authored destination, including spaces and apostrophes.
            let (mut route, payment) = if let Some((route, payment)) = native {
                (route, Some(payment))
            } else {
                let Some(route) = routes.into_iter().find(|route| {
                    text == route.keyword || text == route.route.key.replace('_', " ")
                }) else {
                    return ordinary;
                };
                (route, None)
            };
            let Ok(Some(current)) = self.current_quest_fence(session).await else {
                return ordinary;
            };
            if current.item_fence() != fence {
                return ordinary;
            }
            let quote = |route: TravelRoute| {
                self.root.quote_npc_travel(
                    self.character,
                    self.holder,
                    current,
                    route,
                    &self.premium_coordinator,
                )
            };
            // Iskan first asks even when the mission is unfinished. The durable
            // confirmation will check the unlocked passage and outstanding fine.
            let fare = if npc == ISKAN {
                let barbarian = self
                    .root
                    .read_character_quest_state(self.character, current.character_id)
                    .await;
                if !barbarian.is_ok_and(|state| {
                    state
                        .tracks()
                        .get(oteryn_gameplay_quests::quest::iskan::BARBARIAN)
                        .is_some_and(|stage| *stage >= 8)
                }) {
                    return npc_command::NpcConversations::bank_reply(
                        intent,
                        "Sorry, not possible.".into(),
                    );
                }
                0
            } else {
                let mut result = quote(route.clone()).await;
                if payment == Some(Payment::Ring)
                    && matches!(result, Ok(Err(TravelRefusal::UnavailableRoute)))
                    && let Some(paid) = source_route(npc, &route.keyword, Payment::Fare)
                {
                    route = paid;
                    result = quote(route.clone()).await;
                }
                match result {
                    Ok(Ok(fare)) => fare,
                    Ok(Err(TravelRefusal::UnavailableRoute)) if npc == BUDDEL => {
                        return npc_command::NpcConversations::bank_reply(
                            intent,
                            "Sorry, not possible.".into(),
                        );
                    }
                    Ok(Err(cause)) => return refusal(intent, cause),
                    Err(_) => {
                        return npc_command::NpcTalkOutcome {
                            unknown: true,
                            ..npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
                        };
                    }
                }
            };
            let line = if payment.is_some() {
                svargrond::offer(npc, &route, fare)
            } else {
                format!(
                    "Do you seek a passage to {} for {fare} gold?",
                    route.keyword
                )
            };
            if let Ok(mut prompts) = self.states.newhaven_travel.paid_prompts.lock() {
                prompts.insert(
                    session,
                    Prompt {
                        npc: intent.npc_actor,
                        context,
                        fence,
                        opened: now,
                        route,
                        fare,
                    },
                );
            }
            return npc_command::NpcConversations::bank_reply(intent, line);
        };
        if context.2 == BUDDEL && svargrond::select_buddel_arrival(&mut prompt.route).is_none() {
            if let Ok(mut prompts) = self.states.newhaven_travel.paid_prompts.lock() {
                prompts.insert(session, prompt);
            }
            return npc_command::NpcConversations::bank_reply(
                intent,
                "Sorry, not possible.".into(),
            );
        }
        let Some(request) = self
            .paid_travel_request(actor, command, intent, &prompt)
            .await
        else {
            return refusal(intent, TravelRefusal::UnavailableRoute);
        };
        let inventory_changed = request.ring.is_some() || prompt.fare > 0;
        let frozen = FrozenTravel {
            command: request.command,
            npc: context.2,
            route: request.route.route.key.clone(),
            fare: prompt.fare,
            arrival: request.arrival,
        };
        {
            let mut runtime = self.runtime.lock().await;
            if self
                .states
                .npc_conversations
                .current_npc(&self.states.chat, &runtime, actor, session, intent)
                .as_ref()
                != Some(&prompt.context)
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
                let mut result = npc_command::NpcConversations::bank_reply(
                    intent,
                    if prompt.context.2 == ISKAN {
                        String::new()
                    } else {
                        "Set the sails!".into()
                    },
                );
                result.inventory_changed = inventory_changed;
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
                if cause == TravelRefusal::UnavailableRoute && prompt.context.2 == ISKAN {
                    npc_command::NpcConversations::bank_reply(
                        intent,
                        "I need you to finish my mission before I can take you there.".into(),
                    )
                } else {
                    refusal(intent, cause)
                }
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
        let ring_key = pays_ring(&prompt.route).then_some(RING);
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
        let reference = crate::foundation::CommandRef::new(
            command.game_session_id,
            crate::foundation::CommandId::new(command.command_id).ok()?,
        );
        let ring = if let Some(key) = ring_key {
            use crate::durability::npc_trade::{
                NpcTradeCause, NpcTradeCoins, NpcTradeRequest, NpcTradeSide,
            };
            let item = super::super::item_ref_admission::generation_item_facts(
                native,
                &crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.into(),
                    revision_ref: native.item_index()?.revision(key)?.into(),
                },
            )
            .ok()?;
            Some(Box::new(NpcTradeRequest {
                fluid_subtype: None,
                full_charges: None,
                cause: NpcTradeCause {
                    npc_key: BUDDEL.into(),
                    catalogue_revision: prompt.context.0,
                    offer_index: 1,
                    side: NpcTradeSide::Sell,
                    occurrence: occurrence.as_bytes(),
                },
                command: reference,
                quantity: 1,
                unit_price: 0,
                offer_count: 1,
                item,
                transaction_id: derive(1),
                event_id: derive(2),
                occurred_at_unix_ms: super::super::npc_trade_command::unix_ms_now(),
                output_item_instance_ids: [derive(3), derive(4), derive(5)],
                coins: NpcTradeCoins {
                    gold: gold.clone(),
                    platinum: platinum.clone(),
                    crystal: crystal.clone(),
                },
                backpack: backpack.clone(),
            }))
        } else {
            None
        };
        let destination = &prompt.route.route.destination;
        Some(PaidTravelRequest {
            command: reference,
            ring,
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
