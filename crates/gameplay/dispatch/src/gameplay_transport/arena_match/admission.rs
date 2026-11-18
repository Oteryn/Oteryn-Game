//! Crystal 201189f8 Halvar payment, storage door and action 25100 entry.
use super::*;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::MovementPositionSnapshot;
use crate::gameplay_transport::{connection::UseCommand, npc_command};
use crate::map::{
    boot::BundleMap,
    overlay::TilePos,
    view::{MapFacts, placement_key, placement_of},
};
use oteryn_gameplay_npc::travel_prompt::{TravelConfirmation, TravelPrompt};
use oteryn_protocol_oteryn::{
    npc_service::{NpcIntentDisposition, NpcTalkIntent},
    world_object::WorldObjectTarget,
    world_spatial_entities::EntityRef,
};

const HALVAR: &str = "oteryn:npc.halvar";
const TIERS: [&str; 3] = ["Greenhorn", "Scrapper", "Warlord"];
type Context = (EntityRef, u64, u64);
pub(in crate::gameplay_transport) type HalvarPrompt = TravelPrompt<u8, Context>;

pub(super) struct HalvarProof {
    intent: NpcTalkIntent,
    context: Context,
}

impl HalvarProof {
    pub(super) fn is_current(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        halvar_context(owner, runtime, actor, session, &self.intent) == Some(self.context)
    }
}

fn halvar_context(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    intent: &NpcTalkIntent,
) -> Option<Context> {
    let (catalogue, conversation, npc) = owner.states.npc_conversations.current_npc(
        &owner.states.chat,
        runtime,
        actor,
        session,
        intent,
    )?;
    (npc == HALVAR).then_some((intent.npc_actor, catalogue, conversation))
}

fn entry_binding(map: &BundleMap, position: MovementLocalPosition) -> Option<()> {
    if position.floor != 7
        || !matches!(
            (position.x, position.y),
            (32238 | 32241, 31101) | (32246, 31097)
        )
    {
        return None;
    }
    let at = TilePos {
        x: position.x.try_into().ok()?,
        y: position.y.try_into().ok()?,
        floor: -7,
    };
    let tile = map.overlay.base().tile(at.x, at.y, at.floor)?;
    let key = placement_key(at, 1)?;
    let (id, _) = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(1)?;
    (!map.facts.base_hidden(key)
        && (map.facts.quest_callback_native_object(key, *id, 1949)
            || map
                .facts
                .quest_callback_matches(key, None, Some(25100), Some(1949))))
    .then_some(())
}

pub(super) fn entrance_forcefield(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    position: MovementLocalPosition,
) -> bool {
    owner
        .bundle_map()
        .is_some_and(|map| entry_binding(map, position).is_some())
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn read_arena_payment(&self, session: GameSessionId) -> Option<MatchState> {
        let mut fence = self.current_quest_fence(session).await.ok()??;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        fence.expected_character_revision = slot.cursor(self.root, self.character).await.ok()?;
        self.root
            .read_arena_admission(self.character, self.holder, fence)
            .await
            .ok()
    }

    pub(in crate::gameplay_transport) async fn arena_door_permission(
        &self,
        session: GameSessionId,
        map: &BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<bool> {
        let key = u64::from_be_bytes(target.placement.get(32..40)?.try_into().ok()?);
        let (at, ordinal) = placement_of(key)?;
        if (at.x, at.floor, ordinal) != (32233, -7, 1)
            || !(31098..=31100).contains(&at.y)
            || !map
                .facts
                .quest_callback_matches(key, None, Some(41152), Some(5131))
        {
            return None;
        }
        Some(
            self.read_arena_payment(session)
                .await
                .is_some_and(|state| state.paid_rank > 0),
        )
    }

    pub(in crate::gameplay_transport) async fn step_arena_entrance(
        &self,
        _permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        if origin.position() == entered.position() || !entrance_forcefield(self, entered.position())
        {
            return entered;
        }
        let result = self.enter_arena_forcefield(actor, session, entered).await;
        let mut runtime = self.runtime.lock().await;
        let Ok(current) = runtime.read_actor_position(actor) else {
            return entered;
        };
        if current != entered {
            return current;
        }
        let feedback = match result {
            Some(D::WrongState) => "You cannot enter without Halvar's permission.",
            Some(D::Busy) => "Someone is currently in the next arena pit. Please wait.",
            _ => "Sorry, not possible.",
        };
        self.states.chat.push_quest_feedback(session, feedback);
        let _ = runtime.relocate_arena_player(actor, session, current, origin.position());
        runtime.read_actor_position(actor).unwrap_or(current)
    }

    async fn enter_arena_forcefield(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> Option<D> {
        let command = crate::gameplay_transport::quest_callbacks::step_command(
            actor,
            session,
            entered.revision(),
            self.reset_epoch,
        )?;
        let supplied = self.current_quest_fence(session).await.ok()??.item_fence();
        let caller = self.arena_caller(actor, supplied).await?;
        let cause = durable::MatchCause::Step {
            command,
            actor: actor.placement_identity(),
            revision: entered.revision(),
            epoch: self.reset_epoch,
        };
        let payload = wire::encode_match_request(A::Enter).ok()?;
        let reply = self
            .arena_match_native(
                caller,
                cause,
                A::Enter,
                &payload,
                ArenaIngress::Step(entered),
            )
            .await?;
        Some(wire::decode_match_result(&reply).ok()?.disposition)
    }

    async fn arena_caller(
        &self,
        actor: ExactActorRef,
        supplied: crate::durability::item_transfer::CurrentCharacterItemFence,
    ) -> Option<ArenaCaller> {
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        Some(ArenaCaller {
            actor,
            supplied,
            world: binding.world_id(),
            channel: binding.channel_id()?,
        })
    }

    pub(in crate::gameplay_transport) async fn halvar_talk(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &NpcTalkIntent,
        ordinary: npc_command::NpcTalkOutcome,
    ) -> npc_command::NpcTalkOutcome {
        if ordinary.disposition != NpcIntentDisposition::Ok
            || ordinary.inventory_changed
            || ordinary.unknown
        {
            return ordinary;
        }
        self.try_halvar_talk(actor, command, intent)
            .await
            .unwrap_or(ordinary)
    }

    async fn try_halvar_talk(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &NpcTalkIntent,
    ) -> Option<npc_command::NpcTalkOutcome> {
        let session = command.game_session_id;
        let context = {
            let runtime = self.runtime.lock().await;
            halvar_context(self, &runtime, actor, session, intent)?
        };
        let reply = |text: String| npc_command::NpcConversations::bank_reply(intent, text);
        let now = self.owner_now().get() / 1_000;
        let words = intent
            .text
            .split(|character: char| !character.is_ascii_alphabetic());
        let conversations = &self.states.npc_conversations;
        if words.clone().any(|word| {
            ["fight", "pit", "challenge", "arena"]
                .iter()
                .any(|key| word.eq_ignore_ascii_case(key))
        }) {
            let state = self.read_arena_payment(session).await?;
            if state.paid_rank > 0 {
                return Some(reply("You already paid the fee, go and fight!".into()));
            }
            let rank = state.completed.checked_add(1)?;
            if rank > 3 {
                return Some(reply(
                    "You've already completed the arena in all difficulty levels.".into(),
                ));
            }
            if state.rank != 0 && state.phase != 3 {
                return Some(reply("Sorry, not possible.".into()));
            }
            conversations
                .set_halvar_prompt(session, HalvarPrompt::new(rank, context, now, false))?;
            let index = usize::from(rank - 1);
            return Some(reply(format!(
                "The fee for one try in {} is {} gold pieces. Do you want to pay the fee?",
                TIERS[index],
                donor::FEES[index],
            )));
        }
        let mut prompt = conversations.take_halvar_prompt(session)?;
        let TravelConfirmation::Ready(rank) = prompt.confirm(&context, now) else {
            return None;
        };
        if !words
            .into_iter()
            .any(|word| word.eq_ignore_ascii_case("yes"))
        {
            return Some(reply("Come back when you are ready then.".into()));
        }
        let proof = HalvarProof {
            intent: intent.clone(),
            context,
        };
        let Some(supplied) = command.item_fence else {
            return Some(reply("Sorry, not possible.".into()));
        };
        let caller = self.arena_caller(actor, supplied).await?;
        let cause = durable::MatchCause::Halvar {
            command: CommandRef::new(session, CommandId::new(command.command_id).ok()?),
            rank,
        };
        let payload = wire::encode_match_request(A::Enter).ok()?;
        let bytes = self
            .arena_match_native(
                caller,
                cause,
                A::Enter,
                &payload,
                ArenaIngress::Halvar(&proof),
            )
            .await?;
        let disposition = wire::decode_match_result(&bytes).ok()?.disposition;
        let mut outcome = reply(
            match disposition {
                D::Applied => "You can pass the door now and enter the teleporter to the pits.",
                D::InsufficientGold => "You do not have enough money.",
                _ => "Sorry, not possible.",
            }
            .into(),
        );
        outcome.inventory_changed = disposition == D::Applied;
        Some(outcome)
    }
}
