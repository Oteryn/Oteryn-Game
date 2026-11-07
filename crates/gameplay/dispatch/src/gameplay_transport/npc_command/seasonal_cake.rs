//! Brutus' native dialogue over the existing NPC command and complete reward-container owner.
use super::super::item_ref_admission::{NativeItemWeights, generation_item_facts, pinned_index};
use super::super::seasonal_cake::now_seconds;
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{RewardClaimMintOutcome, RewardClaimMintRequest};
use crate::durability::seasonal_cake::character::{
    CakeCharacterEvent, CakeCharacterProgress, reward_claim_key, reward_source,
};
use crate::foundation::{CommandId, CommandRef};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
enum Action {
    Help,
    Cupcake,
    Reward(u16),
    Reply(&'static str),
}
pub(super) struct Step {
    context: (u64, u64, String),
    action: Action,
}

fn word(text: &str, keyword: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|part| part.eq_ignore_ascii_case(keyword))
}
pub(super) fn prepare(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
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
    let text = &intent.text;
    let action = if word(text, "help") {
        Action::Help
    } else if word(text, "reward") || word(text, "yes") {
        Action::Reply(
            "If you have fed all five extraction machines with cake mass from the isle, I can hand you a reward. Ask for {one}.",
        )
    } else if word(text, "one") {
        Action::Reply(
            "You may choose a cake {cabinet}, cake {backpack} or cake {tapestry}. You can also receive a {cupcake}.",
        )
    } else if word(text, "cabinet") {
        Action::Reward(14756)
    } else if word(text, "backpack") {
        Action::Reward(20347)
    } else if word(text, "tapestry") {
        Action::Reward(20350)
    } else if word(text, "cupcake") {
        Action::Cupcake
    } else if word(text, "blueberry") {
        Action::Reward(28484)
    } else if word(text, "strawberry") {
        Action::Reward(28485)
    } else if word(text, "lemon") {
        Action::Reward(28486)
    } else {
        return None;
    };
    Some(Step { context, action })
}
fn response(
    intent: &NpcTalkIntent,
    text: impl Into<String>,
    inventory_changed: bool,
) -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: false,
        inventory_changed,
        disposition: NpcIntentDisposition::Ok,
        conversation: Some(NpcConversation {
            open: Some((intent.npc_actor, NpcConversationState::Open)),
            lines: vec![text.into()],
        }),
        window: None,
    }
}
fn progress_reply(progress: &CakeCharacterProgress) -> String {
    if progress.completed() {
        "You've already visited all five extraction machines - well done! Say {reward} to hear about your prize.".into()
    } else {
        format!(
            "You've visited {} of the five extraction machines. Step on the Cake Base isle between visits to Carlin, Thais, Edron, Darashia and Liberty Bay.",
            progress.visited_count()
        )
    }
}

pub(super) async fn apply(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let Some(_permit) = admission.spell_lane_permit().await else {
        return rejected();
    };
    let (Some(fence), Ok(id)) = (command.item_fence, CommandId::new(command.command_id)) else {
        return rejected();
    };
    let _slot = admission
        .revision_sequencer
        .acquire(fence.character_id)
        .await;
    if !admission
        .current_quest_fence(command.game_session_id)
        .await
        .ok()
        .flatten()
        .is_some_and(|current| current.item_fence() == fence)
    {
        return rejected();
    }
    let Some(active) = admission.active_generation else {
        return rejected();
    };
    let Some(native) = active.native_gameplay() else {
        return rejected();
    };
    {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || pinned_index(&runtime, admission.active_generation).is_none()
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(step.context)
        {
            return rejected();
        }
    }
    if let Action::Reply(text) = step.action {
        return response(&intent, text, false);
    }
    let reference = CommandRef::new(command.game_session_id, id);
    let occurrence = Sha256::new()
        .chain_update(b"oteryn:cake-npc:v1")
        .chain_update(reference.game_session_id().as_bytes())
        .chain_update(reference.command_id().get().to_be_bytes())
        .finalize()
        .into();
    let event = if matches!(step.action, Action::Help) {
        CakeCharacterEvent::Help
    } else {
        CakeCharacterEvent::Inspect
    };
    let now = now_seconds();
    let progress = match admission
        .root
        .commit_cake_character_event(
            admission.character,
            admission.holder,
            fence,
            occurrence,
            event,
            now,
        )
        .await
    {
        Ok(Some(progress)) => progress,
        Ok(None) => return response(&intent, "The cake event is not ready for rewards.", false),
        Err(_) => return rejected(),
    };
    match step.action {
        Action::Help => return response(&intent, progress_reply(&progress), false),
        Action::Cupcake => {
            return response(
                &intent,
                if !progress.completed() {
                    "I am sorry, but you did not earn your reward yet."
                } else if progress.cupcake_reward_given {
                    "Sorry, but you already got a cupcake this year."
                } else {
                    "You may choose a blueberry cupcake, a strawberry cupcake or a lemon cupcake. Which one do you want?"
                },
                false,
            );
        }
        _ => {}
    }
    let Action::Reward(source_id) = step.action else {
        return rejected();
    };
    let cupcake = (28484..=28486).contains(&source_id);
    if !progress.completed() {
        return response(
            &intent,
            "I am sorry, but you did not earn your reward yet.",
            false,
        );
    }
    if (cupcake && progress.cupcake_reward_given) || (!cupcake && progress.item_reward_given) {
        return response(
            &intent,
            "Sorry, but you already got this reward this year.",
            false,
        );
    }
    let Ok(Some(bag)) = admission
        .root
        .read_character_backpack(admission.character, fence.character_id)
        .await
    else {
        return rejected();
    };
    let Some(index) = native.item_index() else {
        return rejected();
    };
    let key = format!("oteryn:item.tibia.i{source_id}");
    let Some(revision) = index.revision(&key) else {
        return rejected();
    };
    let (Ok(item), Ok(backpack)) = (
        generation_item_facts(
            native,
            &TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.clone(),
                revision_ref: revision.into(),
            },
        ),
        generation_item_facts(native, &bag.backpack.definition),
    ) else {
        return rejected();
    };
    let request = RewardClaimMintRequest {
        command: reference,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: reward_claim_key(progress.event_start, cupcake),
            revision_ref: "cake-201189f8-r1".into(),
        },
        source_placement: reward_source(progress.event_start),
        item,
        quantity: 1,
        contents: Vec::new(),
        root_text: None,
        backpack,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    match admission
        .root
        .claim_cake_character_reward(
            admission.character,
            admission.holder,
            fence,
            request,
            now,
            &NativeItemWeights(native),
        )
        .await
    {
        Ok(Some(
            RewardClaimMintOutcome::Committed(_) | RewardClaimMintOutcome::AlreadyCommitted(_),
        )) => {
            admission.states.loop_ground.invalidate();
            response(&intent, "Here it is. Enjoy your reward!", true)
        }
        Ok(None) => response(
            &intent,
            "I am sorry, but you did not earn your reward yet.",
            false,
        ),
        Err(crate::durability::reward_claim_mint::RewardClaimMintError::Refused(_)) => response(
            &intent,
            "Please make room in your backpack and check your carrying capacity, then ask again.",
            false,
        ),
        Err(_) => rejected(),
    }
}
