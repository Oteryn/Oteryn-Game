//! Fixed source item exchange dialogue over the existing NPC item transaction.
//! Al Dee: npc-br revid420190; Uzgod and Seymour: saved quest Wiki transcripts.
use super::super::{
    ComposedFreshAdmission, connection::UseCommand, item_ref_admission::NativeItemWeights,
    npc_trade_command,
};
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::npc_trade::{
    NpcTradeCause, NpcTradeCoins, NpcTradeError, NpcTradeRequest, NpcTradeSide,
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;

#[path = "oriental_trader.rs"]
mod oriental_trader;

struct Recipe {
    npc: &'static str,
    keyword: &'static str,
    output: &'static str,
    prompt: &'static str,
    success: &'static str,
    cancel: Option<&'static str>,
}
const PICK_RECIPE: Recipe = Recipe {
    npc: "oteryn:npc.al_dee",
    keyword: "pick",
    output: "oteryn:item.tibia.i3456",
    prompt: "Picks are hard to come by. I trade them only in exchange for high quality small axes. Would you like to make that deal?",
    success: "Splendid! Here, take your pick.",
    cancel: Some("Well, then don't."),
};
const KNIFE_RECIPE: Recipe = Recipe {
    npc: "oteryn:npc.uzgod",
    keyword: "draconian steel",
    output: "oteryn:item.tibia.i5908",
    prompt: "You bringing me draconian steel and obsidian lance in exchange for obsidian knife?",
    success: "Here you have it.",
    cancel: None,
};
const PRESENT_RECIPE: Recipe = Recipe {
    npc: "oteryn:npc.seymour",
    keyword: "box",
    output: "oteryn:item.tibia.i3374",
    prompt: "Do you have a suitable present box for me?",
    success: "THANK YOU! Here is a helmet that will serve you well.",
    cancel: None,
};
fn recipe(npc: &str) -> Option<&'static Recipe> {
    match npc {
        "oteryn:npc.al_dee" => Some(&PICK_RECIPE),
        "oteryn:npc.seymour" => Some(&PRESENT_RECIPE),
        "oteryn:npc.uzgod" => Some(&KNIFE_RECIPE),
        _ => None,
    }
}

pub(super) async fn talk(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
) -> Option<NpcTalkOutcome> {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    let reference = CommandRef::new(
        command.game_session_id,
        CommandId::new(command.command_id).ok()?,
    );
    let occurrence = npc_trade_command::occurrence(
        command.game_session_id,
        command.command_id,
        intent.npc_actor,
    );
    let yes = intent.text.trim().eq_ignore_ascii_case("yes");
    // Exact durable replay precedes the live NPC, inventory and conversation. A removed
    // source or ended dialogue cannot make a committed exchange run a second time.
    if yes && let Some(fence) = command.item_fence {
        match admission
            .root
            .read_npc_trade(
                admission.character,
                fence.character_id,
                reference,
                occurrence,
            )
            .await
        {
            Ok(Some(receipt))
                if receipt.side == NpcTradeSide::Exchange && recipe(&receipt.npc_key).is_some() =>
            {
                return Some(NpcTalkOutcome {
                    inventory_changed: true,
                    ..NpcTalkOutcome::only(NpcIntentDisposition::Ok)
                });
            }
            Ok(None) => {}
            Err(NpcTradeError::Unavailable(_)) => return Some(unknown()),
            _ => return Some(rejected()),
        }
    }
    let runtime = admission.runtime.lock().await;
    let npc =
        service_context::live_npc(&runtime, actor, command.game_session_id, intent.npc_actor)?;
    let recipe = recipe(&npc.key);
    if recipe.is_none() && npc.key != "oteryn:npc.yasir" {
        return None;
    }
    // An NPC may also own unrelated quest dialogue. Select our exact recipe
    // before touching ordinary conversation state; unmatched text falls through.
    let mut confirmed = false;
    let step = if let Some(recipe) = recipe {
        Some(quest_dialogue::prepare(
            admission,
            actor,
            command,
            intent,
            &runtime,
            |context, topic, fresh, _| {
                if context.2 != recipe.npc {
                    return None;
                }
                let words = oteryn_gameplay_npc::greeting::words(&intent.text);
                let (line, next) = if oteryn_gameplay_npc::greeting::contains_words(
                    &words,
                    &oteryn_gameplay_npc::greeting::words(recipe.keyword),
                ) {
                    (Some(recipe.prompt), 1)
                } else if !fresh
                    && topic.number == 1
                    && topic.value.as_deref() == Some(recipe.output)
                    && yes
                {
                    confirmed = true;
                    (Some(recipe.success), 0)
                } else if !fresh
                    && topic.number == 1
                    && topic.value.as_deref() == Some(recipe.output)
                    && words.iter().any(|w| w == "no")
                {
                    (recipe.cancel, 0)
                } else {
                    return None;
                };
                Some(quest_dialogue::Choice {
                    transition: None,
                    next_topic: Some(quest_dialogue::Topic {
                        number: next,
                        value: (next == 1).then(|| recipe.output.to_owned()),
                    }),
                    reply: line.into_iter().map(str::to_owned).collect(),
                })
            },
        )?)
    } else {
        None
    };
    let ordinary = admission.states.npc_conversations.talk_in(
        &admission.states.chat,
        &runtime,
        actor,
        command.game_session_id,
        intent,
        admission.owner_now(),
        super::super::item_ref_admission::pinned_index(&runtime, admission.active_generation),
    );
    if ordinary.disposition != NpcIntentDisposition::Ok {
        return Some(ordinary);
    }
    let Some(recipe) = recipe else {
        return Some(oriental_trader::open_alias(
            admission, actor, command, intent, &runtime, ordinary,
        ));
    };
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        &runtime,
        actor,
        command.game_session_id,
        intent,
    );
    let coins = bank_coin_definitions(admission, &runtime);
    drop(runtime);
    let step = step?;
    if !confirmed {
        return Some(quest_dialogue::apply(admission, actor, command, intent.clone(), step).await);
    }
    let Some(fence) = command.item_fence else {
        return Some(rejected());
    };
    let Some(permit) = admission.spell_lane_permit().await else {
        return Some(rejected());
    };
    if !admission
        .states
        .player_trades
        .physical_service(command.game_session_id)
    {
        return Some(rejected());
    }
    let Some(native) = admission
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
    else {
        return Some(rejected());
    };
    let Some(index) = native.item_index() else {
        return Some(rejected());
    };
    let Some((gold, platinum, crystal)) = coins else {
        return Some(rejected());
    };
    let Some(revision) = index.revision(recipe.output) else {
        return Some(rejected());
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: recipe.output.into(),
        revision_ref: revision.into(),
    };
    let Ok(item) = super::super::item_ref_admission::generation_item_facts(native, &definition)
    else {
        return Some(rejected());
    };
    let backpack = match admission
        .root
        .read_character_backpack(admission.character, fence.character_id)
        .await
    {
        Ok(Some(current)) => match super::super::item_ref_admission::generation_item_facts(
            native,
            &current.backpack.definition,
        ) {
            Ok(facts) => facts,
            Err(_) => return Some(rejected()),
        },
        Ok(None) => return Some(rejected()),
        Err(_) => return Some(unknown()),
    };
    {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                intent,
            ) != context
        {
            return Some(rejected());
        }
    }
    if permit.check_scope(fence.runtime_scope).is_err() {
        return Some(rejected());
    }
    let Some(context) = context else {
        return Some(rejected());
    };
    let id =
        |tag: &[u8]| npc_trade_command::derived(tag, command.game_session_id, command.command_id);
    let request = NpcTradeRequest {
        fluid_subtype: None,
        full_charges: None,
        cause: NpcTradeCause {
            npc_key: recipe.npc.into(),
            catalogue_revision: context.0,
            offer_index: 0,
            side: NpcTradeSide::Exchange,
            occurrence,
        },
        command: reference,
        quantity: 1,
        unit_price: 0,
        offer_count: 1,
        item,
        transaction_id: id(b"oteryn:pick-exchange.transaction:v1"),
        event_id: id(b"oteryn:pick-exchange.event:v1"),
        occurred_at_unix_ms: npc_trade_command::unix_ms_now(),
        output_item_instance_ids: [
            id(b"oteryn:pick-exchange.output1:v1"),
            id(b"oteryn:pick-exchange.output2:v1"),
            id(b"oteryn:pick-exchange.output3:v1"),
        ],
        coins: NpcTradeCoins {
            gold,
            platinum,
            crystal,
        },
        backpack,
    };
    let result = if recipe.npc == "oteryn:npc.uzgod" {
        admission
            .root
            .exchange_obsidian_knife(
                admission.character,
                admission.holder,
                fence,
                request,
                &NativeItemWeights(native),
            )
            .await
    } else if recipe.npc == "oteryn:npc.seymour" {
        admission
            .root
            .exchange_present_for_helmet(
                admission.character,
                admission.holder,
                fence,
                request,
                &NativeItemWeights(native),
            )
            .await
    } else {
        admission
            .root
            .exchange_small_axe_for_pick(
                admission.character,
                admission.holder,
                fence,
                request,
                &NativeItemWeights(native),
            )
            .await
    };
    match result {
        Ok(_) => {
            let mut outcome =
                quest_dialogue::apply(admission, actor, command, intent.clone(), step).await;
            // The transaction committed even if the conversation ended before publication.
            if outcome.disposition != NpcIntentDisposition::Ok {
                outcome = NpcTalkOutcome::only(NpcIntentDisposition::Ok);
            }
            outcome.inventory_changed = true;
            Some(outcome)
        }
        Err(NpcTradeError::Unavailable(_)) => Some(unknown()),
        Err(_) => Some(rejected()),
    }
}
