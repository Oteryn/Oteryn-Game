//! NPC departures share one travel hold and arrival owner, including free Newhaven routes.
use super::npc_travel::{FrozenTravel, TravelCapture, TravelHoldSlot};
use super::*;
use crate::content::newhaven_travel::NewhavenRoute;
use crate::durability::character_last_position::FinalLastPosition;
use crate::durability::npc_travel::{PendingArrival, TravelOccurrence};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::collections::HashMap;

#[derive(Clone)]
struct Prompt {
    npc: EntityRef,
    conversation: u64,
    catalogue: u64,
    expires: u64,
    confirmations: u8,
}

#[derive(Clone)]
pub(super) enum Departure {
    Tutorial(NewhavenRoute),
    Paid(Box<crate::durability::npc_travel::PaidTravelRequest>),
}

pub(super) struct Invocation {
    pub(super) actor: ExactActorRef,
    pub(super) npc: EntityRef,
    pub(super) capture: TravelCapture,
    pub(super) frozen: FrozenTravel,
    pub(super) route: Departure,
    pub(super) refusal: Option<crate::durability::npc_travel::TravelRefusal>,
    pub(super) slot: TravelHoldSlot,
    pub(super) committed: bool,
    pub(super) placed: Option<FinalLastPosition>,
}

#[derive(Default)]
pub(super) struct NewhavenTravel {
    prompts: std::sync::Mutex<HashMap<GameSessionId, Prompt>>,
    paid_prompts: std::sync::Mutex<HashMap<GameSessionId, paid::Prompt>>,
    pub(super) holds: std::sync::Mutex<HashMap<GameSessionId, Invocation>>,
}

#[path = "paid_travel.rs"]
mod paid;

impl NewhavenTravel {
    pub(super) fn held(&self, session: GameSessionId) -> bool {
        self.holds.lock().map_or(true, |holds| {
            holds.get(&session).is_some_and(|held| held.slot.is_held())
        })
    }
    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut prompts) = self.paid_prompts.lock() {
            prompts.remove(&session);
        }
        if let Ok(mut prompts) = self.prompts.lock() {
            prompts.remove(&session);
        }
        if let Ok(mut holds) = self.holds.lock() {
            holds.remove(&session);
        }
    }
    pub(super) fn death(&self, session: GameSessionId) {
        if let Ok(mut holds) = self.holds.lock()
            && let Some(held) = holds.get_mut(&session)
        {
            held.slot.cancel_for_death();
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn newhaven_talk(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: NpcTalkIntent,
    ) -> npc_command::NpcTalkOutcome {
        use npc_command::NpcConversations;
        let session = command.game_session_id;
        if self.states.newhaven_travel.held(session) {
            let paid = self
                .states
                .newhaven_travel
                .holds
                .lock()
                .ok()
                .and_then(|holds| holds.get(&session).map(|held| held.frozen.fare > 0))
                .unwrap_or(false);
            let known = self.reconcile_newhaven_travel(actor, session).await;
            return npc_command::NpcTalkOutcome {
                unknown: known.is_none(),
                inventory_changed: paid && known == Some(true),
                ..npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            };
        }
        let ordinary = npc_command::npc_bank_talk(self, actor, command, intent.clone()).await;
        if ordinary.disposition != NpcIntentDisposition::Ok {
            return ordinary;
        }
        let ordinary = self
            .paid_travel_talk(actor, command, &intent, ordinary)
            .await;
        if ordinary.inventory_changed || ordinary.unknown || !self.tutorial_enabled().await {
            return ordinary;
        }
        let (route, conversation, catalogue, capture, work, npc_key) = {
            let mut runtime = self.runtime.lock().await;
            if self.states.spell_states.lock().await.is_dead(actor) {
                return ordinary;
            }
            let Some(npc) = self.states.chat.engaged(session) else {
                return ordinary;
            };
            if npc.placement_identity() != intent.npc_actor.identity {
                return ordinary;
            }
            let Some(key) = runtime.npc_key(npc).map(str::to_owned) else {
                return ordinary;
            };
            let Some(route) = NewhavenRoute::for_npc(&key) else {
                return ordinary;
            };
            let Some(conversation) = self.states.chat.npc_conversation_generation(session) else {
                return ordinary;
            };
            let Some(book) = self.states.chat.npc_talk() else {
                return ordinary;
            };
            let Ok(position) = runtime.read_actor_position(actor) else {
                return ordinary;
            };
            if !tutorial_session::tutorial_cell(position.position()) {
                return ordinary;
            }
            let Some(fence) = command.item_fence else {
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
            (
                route,
                conversation,
                book.catalogue_revision(),
                capture,
                work,
                key,
            )
        };
        let now = self.owner_now().get() / 1000;
        let text = intent.text.trim().to_ascii_lowercase();
        let prompt = {
            let Ok(mut prompts) = self.states.newhaven_travel.prompts.lock() else {
                return ordinary;
            };
            if text == "no" || text == "bye" {
                prompts.remove(&session);
                return ordinary;
            }
            if (route == NewhavenRoute::Anna && text == "passage")
                || (route == NewhavenRoute::Seagull && text == "rookgaard")
            {
                prompts.insert(
                    session,
                    Prompt {
                        npc: intent.npc_actor,
                        conversation,
                        catalogue,
                        expires: now.saturating_add(60_000),
                        confirmations: 1,
                    },
                );
                return NpcConversations::bank_reply(
                    &intent,
                    format!(
                        "Do you want a free passage to {}?",
                        route.key().replace('_', " ")
                    ),
                );
            }
            let Some(prompt) = prompts.get_mut(&session) else {
                return ordinary;
            };
            if prompt.npc != intent.npc_actor
                || prompt.conversation != conversation
                || prompt.catalogue != catalogue
                || now > prompt.expires
            {
                prompts.remove(&session);
                return ordinary;
            }
            if text != "yes" && text != "sure" {
                return ordinary;
            }
            if route == NewhavenRoute::Seagull && prompt.confirmations == 1 {
                prompt.confirmations = 2;
                return NpcConversations::bank_reply(
                    &intent,
                    "This passage is one way. Are you sure?".into(),
                );
            }
            prompts.remove(&session)
        };
        if prompt.is_none() {
            return ordinary;
        }
        let Ok(command_id) = crate::foundation::CommandId::new(command.command_id) else {
            return ordinary;
        };
        let Ok(occurrence) = TravelOccurrence::from_bytes(npc_trade_command::derived(
            b"oteryn:newhaven-travel:v1",
            session,
            command.command_id,
        )) else {
            return ordinary;
        };
        let destination = route.destination();
        let frozen = FrozenTravel {
            command: crate::foundation::CommandRef::new(session, command_id),
            npc: npc_key,
            route: route.key().into(),
            fare: 0,
            arrival: PendingArrival {
                occurrence,
                world_id: self.world_id,
                destination: crate::durability::character_last_position::LastPositionCell {
                    x: destination.0,
                    y: destination.1,
                    floor: destination.2,
                },
            },
        };
        let mut slot = TravelHoldSlot::default();
        {
            let runtime = self.runtime.lock().await;
            let Ok(owner) = runtime.owner_fence() else {
                return ordinary;
            };
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
                    route: Departure::Tutorial(route),
                    refusal: None,
                    slot,
                    committed: false,
                    placed: None,
                },
            );
        }
        let known = self.reconcile_newhaven_travel(actor, session).await;
        match known {
            Some(true) => {
                let mut reply = NpcConversations::bank_reply(&intent, "Safe travels.".into());
                if let Some(conversation) = reply.conversation.as_mut() {
                    conversation.open = None;
                }
                reply
            }
            Some(false) => {
                NpcConversations::bank_reply(&intent, "You cannot take this passage yet.".into())
            }
            None => npc_command::NpcTalkOutcome {
                unknown: true,
                ..npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            },
        }
    }
}
