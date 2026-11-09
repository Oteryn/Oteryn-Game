//! Crystal201189f brutus_the_baker.lua: the annual errand and its two choices.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintOutcome, RewardClaimMintRequest, RewardClaimRefusal,
};
use crate::foundation::{CommandId, CommandRef};

#[derive(Clone)]
pub(super) struct Pending {
    character: crate::domain::CharacterId,
    request: RewardClaimMintRequest,
    intent: NpcTalkIntent,
    context: (u64, u64, String),
    cupcake: bool,
}
#[derive(Clone, Copy)]
enum Topic {
    Help,
    Reward,
    One,
    Cupcake,
    Item(u16),
}
pub(super) struct Step {
    context: (u64, u64, String),
    topic: Topic,
}
pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    if context.2 != "oteryn:npc.brutus_the_baker" {
        return None;
    }
    let has = |word| super::cake::keyword(&intent.text, word);
    // Source order matters: "blueberry cupcake" opens the menu, not a MINT.
    let topic = if has("help") {
        Topic::Help
    } else if has("reward") || has("yes") {
        Topic::Reward
    } else if has("one") {
        Topic::One
    } else if has("cabinet") {
        Topic::Item(14756)
    } else if has("backpack") {
        Topic::Item(20347)
    } else if has("tapestry") {
        Topic::Item(20350)
    } else if has("cupcake") {
        Topic::Cupcake
    } else if has("blueberry") {
        Topic::Item(28484)
    } else if has("strawberry") {
        Topic::Item(28485)
    } else if has("lemon") {
        Topic::Item(28486)
    } else {
        return None;
    };
    Some(Step { context, topic })
}
fn clear(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.cake_brutus = None;
    }
}
async fn current(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    context: &(u64, u64, String),
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
) -> bool {
    let runtime = admission.runtime.lock().await;
    let states = admission.states.spell_states.lock().await;
    !states.is_dead(actor)
        && states.spell_idle(&runtime, actor, command.game_session_id)
        && admission
            .states
            .player_trades
            .physical_service(command.game_session_id)
        && admission.states.npc_conversations.current_npc(
            &admission.states.chat,
            &runtime,
            actor,
            command.game_session_id,
            intent,
        ) == Some(context.clone())
        && runtime
            .bound_attacker_lease(actor, command.game_session_id)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
}
fn unknown() -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: true,
        ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
    }
}
const NOT_EARNED: &str = "I am sorry, but you did not earn your reward yet.";
fn already(cupcake: bool) -> &'static str {
    if cupcake {
        "Sorry, but you already got a cupcake this year."
    } else {
        "Sorry, but you already got a reward this year."
    }
}
pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let (Some(fence), Ok(id), Some(native)) = (
        command.item_fence,
        CommandId::new(command.command_id),
        admission
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay),
    ) else {
        return rejected();
    };
    let Some(permit) = admission.spell_lane_permit().await else {
        return rejected();
    };
    let _revision = admission
        .revision_sequencer
        .acquire(fence.character_id)
        .await;
    let Ok(Some(proof)) = admission.current_quest_fence(command.game_session_id).await else {
        return rejected();
    };
    if proof.item_fence() != fence
        || !current(admission, actor, command, &intent, &step.context, fence).await
    {
        return rejected();
    }
    let status = match admission
        .root
        .cake_brutus_status(
            &permit,
            admission.character,
            admission.holder,
            fence,
            native.source_digest(),
            matches!(step.topic, Topic::Help),
        )
        .await
    {
        Ok(Some(status)) => status,
        Ok(None) => return rejected(),
        Err(_) => return unknown(),
    };
    let reply = match step.topic {
        Topic::Help if status.cities==31 => "You've already visited all five extraction machines - well done! Say {reward} to hear about your prize.".into(),
        Topic::Help if status.started => format!("You're already helping! You've visited {} of the 5 extraction machines so far. Remember: step on the Cake Base between each visit!",status.cities.count_ones()),
        Topic::Help => "Yes, yes! Just step somewhere on the cake base on the isle. Then travel to one of our ingredient extractors at Thais, Carlin, Darashia, Liberty Bay and Edron. They will extract the cake mass that is stuck on you. So if you want to feed another extractor, you'll have to visit the isle again of course. If you manage to feed all five extractors before the time runs out, I will give you a special reward.".into(),
        Topic::Reward => "Well, not all our experiments with Kullu's recipes went well. But even the failures are good for something. We used them to create some beautiful as well as practical pieces of furniture, storage and decoration. If you are interested - and have fed all five extractors with cake mass that is stuck on you -, I can hand you one.".into(),
        Topic::One => "You have a choice of three rewards. If you didn't already claim your reward, you can tell me which one you want: a cake cabinet, a cake backpack or a cake tapestry. I will also give you a very sweet and tasty cupcake, if you want one. What are you interested in?".into(),
        Topic::Cupcake if status.cities!=31 => NOT_EARNED.into(),
        Topic::Cupcake if status.cupcake_claimed => already(true).into(),
        Topic::Cupcake => "You can have a blueberry cupcake, a strawberry cupcake or a lemon cupcake. Blueberries will regenerate your mana, strawberries are able to heal your wounds and lemons can steady your hand and sharpen your eyes. So, which one do you want?".into(),
        Topic::Item(_) if status.cities!=31 => NOT_EARNED.into(),
        Topic::Item(source) => {
            let definition = {
                let runtime = admission.runtime.lock().await;
                let Some(index) = super::super::item_ref_admission::pinned_index(&runtime,admission.active_generation) else { return rejected(); };
                let key = format!("oteryn:item.tibia.i{source}");
                let Some(revision) = index.revision(&key) else { return rejected(); };
                TypedDefinitionRef { family:"Item".into(),production_key:key,revision_ref:revision.into() }
            };
            let Ok(Some(backpack)) = admission.root.read_character_backpack(admission.character,fence.character_id).await else { return rejected(); };
            let facts = |definition: &TypedDefinitionRef| super::super::item_ref_admission::generation_item_facts(native,definition);
            let (Ok(item),Ok(backpack)) = (facts(&definition),facts(&backpack.backpack.definition)) else { return rejected(); };
            let cupcake = source>=28484;
            let category = if cupcake { "cupcake" } else { "item" };
            let request = RewardClaimMintRequest {
                command:CommandRef::new(command.game_session_id,id),
                claim:TypedDefinitionRef { family:"RewardClaim".into(),production_key:format!("oteryn:reward-claim.cake-brutus-{category}.{}",status.year),revision_ref:"oteryn:cake-brutus.v1".into() },
                source_placement:"oteryn:npc.brutus_the_baker".into(),item,quantity:1,
                contents:Vec::new(),additional_items:Vec::new(),root_text:None,backpack,
                content_revision:format!("sha256:{}",native.source_digest().iter().map(|b|format!("{b:02x}")).collect::<String>()),
                ruleset_revision:"oteryn:cake-brutus.v1".into(),sim_revision:"oteryn:cake-brutus.v1".into(),
                achievement:None,quest_transition:None,key_number:None,
            };
            if !current(admission,actor,command,&intent,&step.context,fence).await { return rejected(); }
            let pending = Pending { character:fence.character_id,request,intent:intent.clone(),context:step.context,cupcake };
            {
                let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else { return rejected(); };
                let state=sessions.entry(command.game_session_id).or_default();
                if state.cake.is_some() || state.seasonal.is_some() || state.cake_brutus.is_some() { return unknown(); }
                state.cake_brutus=Some(pending.clone());
            }
            return execute(admission,actor,command,fence,pending).await;
        }
    };
    NpcConversations::bank_reply(&intent, reply)
}
async fn execute(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    pending: Pending,
) -> NpcTalkOutcome {
    let Some(native) = admission
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
    else {
        return unknown();
    };
    let weights = super::super::item_ref_admission::NativeItemWeights(native)
        .with_wheel(admission.premium_coordinator);
    match admission
        .root
        .claim_cake_brutus_reward(
            admission.character,
            admission.holder,
            fence,
            pending.request.clone(),
            &weights,
        )
        .await
    {
        Ok(RewardClaimMintOutcome::Committed(_) | RewardClaimMintOutcome::AlreadyCommitted(_)) => {
            #[cfg(test)]
            if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
                && let Some(state) = sessions.get_mut(&command.game_session_id)
                && state.cake_brutus_interrupt_after_commit == Some(command.command_id)
            {
                state.cake_brutus_interrupt_after_commit = None;
                return unknown();
            }
            committed_reply(admission, actor, command, fence, pending).await
        }
        Err(RewardClaimMintError::Unavailable(_)) => unknown(),
        Err(error) => {
            clear(admission, command.game_session_id);
            let reply = match error {
                RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed) => {
                    already(pending.cupcake)
                }
                RewardClaimMintError::Refused(RewardClaimRefusal::CarryingCapacity) => {
                    "You cannot carry this reward."
                }
                RewardClaimMintError::Refused(RewardClaimRefusal::MainBackpackFull) => {
                    "Make room in your backpack for your reward."
                }
                _ => return NpcTalkOutcome::only(NpcIntentDisposition::Rejected),
            };
            NpcConversations::bank_reply(&pending.intent, reply.into())
        }
    }
}
async fn committed_reply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    pending: Pending,
) -> NpcTalkOutcome {
    clear(admission, command.game_session_id);
    let mut outcome = NpcConversations::bank_reply(
        &pending.intent,
        if pending.cupcake {
            "Here it is. Enjoy your meal!"
        } else {
            "Here it is. Handle it with care."
        }
        .into(),
    );
    outcome.inventory_changed = true;
    if !current(
        admission,
        actor,
        command,
        &pending.intent,
        &pending.context,
        fence,
    )
    .await
        && let Some(conversation) = outcome.conversation.as_mut()
    {
        conversation.open = None;
    }
    outcome
}
pub(in crate::gameplay_transport) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
) -> Option<NpcTalkOutcome> {
    let pending = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .cake_brutus
        .clone()?;
    if pending.request.command.command_id().get() != command.command_id {
        return None;
    }
    let Some(_permit) = admission.spell_lane_permit().await else {
        return Some(unknown());
    };
    let _revision = admission
        .revision_sequencer
        .acquire(pending.character)
        .await;
    let Ok(Some(proof)) = admission.current_quest_fence(command.game_session_id).await else {
        return Some(unknown());
    };
    let fence = proof.item_fence();
    if fence.character_id != pending.character {
        return Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected));
    }
    match admission
        .root
        .reconcile_cake_brutus_reward(
            admission.character,
            admission.holder,
            fence,
            pending.request.clone(),
        )
        .await
    {
        Ok(Some(_)) => Some(committed_reply(admission, actor, command, fence, pending).await),
        Ok(None) => {
            clear(admission, command.game_session_id);
            Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
        }
        Err(_) => Some(unknown()),
    }
}

#[cfg(test)]
impl NpcConversations {
    pub(in crate::gameplay_transport) fn fixture_interrupt_cake_brutus_commit(
        &self,
        session: GameSessionId,
        command: u64,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.lock().map_err(|_| "sessions")?;
        let state = sessions.get_mut(&session).ok_or("session")?;
        if command == 0 || state.cake_brutus_interrupt_after_commit.is_some() {
            return Err("fault already armed or invalid command".into());
        }
        state.cake_brutus_interrupt_after_commit = Some(command);
        Ok(())
    }
}
