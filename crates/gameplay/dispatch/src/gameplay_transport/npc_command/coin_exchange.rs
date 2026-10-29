//! Banker exchange dialogue over the existing NPC_TALK command and inventory projection.
use super::*;
use crate::durability::coin_exchange::CoinExchangeRequest;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_economy::coin_exchange::CoinExchange;

const CANCEL: &str = "Well, can I help you with something else?";
#[derive(Debug, Clone, Copy)]
enum Topic {
    PlatinumDestination,
    Amount(CoinExchange),
    Confirm(CoinExchange, u64),
}
pub(super) struct Pending {
    topic: Topic,
    context: (u64, u64, String),
    npc: EntityRef,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    node: ([u8; 16], u64),
    opened: u64,
}
pub(super) enum Step {
    Reply(String),
    Exchange(CoinExchangeRequest),
}
// Canary MsgContains uses a non-alphanumeric boundary on each side of the keyword.
fn contains_keyword(text: &str, keyword: &str) -> bool {
    text.match_indices(keyword).any(|(start, matched)| {
        start
            .checked_sub(1)
            .and_then(|n| text.as_bytes().get(n))
            .is_none_or(|b| !b.is_ascii_alphanumeric())
            && text
                .as_bytes()
                .get(start + matched.len())
                .is_none_or(|b| !b.is_ascii_alphanumeric())
    })
}
fn amount_prompt(exchange: CoinExchange) -> &'static str {
    match exchange {
        CoinExchange::GoldToPlatinum => "How many platinum coins would you like to get?",
        CoinExchange::PlatinumToGold => {
            "How many platinum coins would you like to change into {gold}?"
        }
        CoinExchange::PlatinumToCrystal => "How many crystal coins would you like to get?",
        CoinExchange::CrystalToPlatinum => {
            "How many crystal coins would you like to change into platinum?"
        }
    }
}
fn parse_amount(text: &str) -> Option<u64> {
    let digits: String = text
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    digits
        .parse::<u64>()
        .ok()
        .filter(|n| (1..=u64::from(u32::MAX)).contains(n))
}

/// Bank-account commands precede the money-changing topics in the reference NPC handler.
/// In particular, an account command must cancel an unfinished exchange instead of being
/// mistaken for the exchange amount (or its confirmation response).
fn bank_command_precedes_exchange(has_pending_exchange: bool, text: &str) -> bool {
    has_pending_exchange && super::super::bank_npc::parse(text).is_ok()
}
fn quote(exchange: CoinExchange, amount: u64) -> String {
    // Preserve the two reversed-quantity confirmation strings in pinned Canary. The actual
    // conversion uses the correct 100:1 amounts, independently of this dialogue text.
    match exchange {
        CoinExchange::GoldToPlatinum => format!(
            "So you would like me to change {} of your gold coins into {amount} platinum coins?",
            amount * 100
        ),
        CoinExchange::PlatinumToGold => format!(
            "So you would like me to change {} of your platinum coins into {amount} gold coins for you?",
            amount * 100
        ),
        CoinExchange::PlatinumToCrystal => format!(
            "So you would like me to change {} of your platinum coins into {amount} crystal coins for you?",
            amount * 100
        ),
        CoinExchange::CrystalToPlatinum => format!(
            "So you would like me to change {} of your crystal coins into {amount} platinum coins for you?",
            amount * 100
        ),
    }
}

pub(super) fn prepare(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_bank(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    );
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.get_mut(&command.game_session_id)?;
    let pending = state.exchange.take();
    let context = context?;
    let fence = command.item_fence?;
    let node = (
        *admission.holder.fact().node_id().as_bytes(),
        admission.holder.fact().registration_revision(),
    );
    let now = admission.owner_now().get();
    let text = intent.text.to_ascii_lowercase();
    let pending = pending.filter(|p| {
        p.context == context
            && p.npc == intent.npc_actor
            && p.fence == fence
            && p.node == node
            && now >= p.opened
            && now - p.opened < 30_000_000
    });
    if bank_command_precedes_exchange(pending.is_some(), &intent.text) {
        return None;
    }
    // Preserve the pinned handler order: change gold precedes every exchange topic;
    // change platinum precedes platinum/crystal topics; change crystal precedes crystal topics.
    let gold_topic = pending.as_ref().is_some_and(|p| {
        matches!(
            p.topic,
            Topic::Amount(CoinExchange::GoldToPlatinum)
                | Topic::Confirm(CoinExchange::GoldToPlatinum, _)
        )
    });
    let crystal_or_idle = pending.as_ref().is_none_or(|p| {
        matches!(
            p.topic,
            Topic::Amount(CoinExchange::CrystalToPlatinum)
                | Topic::Confirm(CoinExchange::CrystalToPlatinum, _)
        )
    });
    let selected = if contains_keyword(&text, "change gold") {
        Some(Topic::Amount(CoinExchange::GoldToPlatinum))
    } else if !gold_topic && contains_keyword(&text, "change platinum") {
        Some(Topic::PlatinumDestination)
    } else if crystal_or_idle && contains_keyword(&text, "change crystal") {
        Some(Topic::Amount(CoinExchange::CrystalToPlatinum))
    } else {
        None
    };
    let next = if let Some(selected) = selected {
        selected
    } else {
        let pending = pending?;
        match pending.topic {
            Topic::PlatinumDestination => {
                if contains_keyword(&text, "gold") {
                    Topic::Amount(CoinExchange::PlatinumToGold)
                } else if contains_keyword(&text, "crystal") {
                    Topic::Amount(CoinExchange::PlatinumToCrystal)
                } else {
                    return Some(Step::Reply(CANCEL.into()));
                }
            }
            Topic::Amount(exchange) => {
                let Some(amount) = parse_amount(&text) else {
                    return Some(Step::Reply(exchange.insufficient_coins().into()));
                };
                Topic::Confirm(exchange, amount)
            }
            Topic::Confirm(exchange, amount) => {
                return Some(if contains_keyword(&text, "yes") {
                    Step::Exchange(CoinExchangeRequest {
                        exchange,
                        amount,
                        npc_identity: intent.npc_actor.identity,
                        npc_generation: intent.npc_actor.generation,
                        npc_key: context.2,
                        catalogue_revision: context.0,
                        conversation_revision: context.1,
                    })
                } else {
                    Step::Reply(CANCEL.into())
                });
            }
        }
    };
    let line = match next {
        Topic::PlatinumDestination => {
            "Would you like to change your platinum coins into {gold} or {crystal}?".into()
        }
        Topic::Amount(exchange) => amount_prompt(exchange).into(),
        Topic::Confirm(exchange, amount) => quote(exchange, amount),
    };
    state.bank = None;
    state.premium = None;
    state.exchange = Some(Pending {
        topic: next,
        context,
        npc: intent.npc_actor,
        fence,
        node,
        opened: now,
    });
    Some(Step::Reply(line))
}

pub(super) async fn apply(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: NpcTalkIntent,
    step: Step,
    permit: Option<&SpellLanePermit>,
) -> NpcTalkOutcome {
    match step {
        Step::Reply(line) => NpcConversations::bank_reply(&intent, line),
        Step::Exchange(request) => {
            let Some(permit) = permit else {
                return held_bank_outcome();
            };
            execute(admission, actor, command, Some(request), permit)
                .await
                .unwrap_or_else(held_bank_outcome)
        }
    }
}

pub(super) async fn resume(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
) -> Option<NpcTalkOutcome> {
    let Some(permit) = admission.spell_lane_permit().await else {
        return Some(held_bank_outcome());
    };
    execute(admission, actor, command, None, &permit).await
}
async fn execute(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    request: Option<CoinExchangeRequest>,
    permit: &SpellLanePermit,
) -> Option<NpcTalkOutcome> {
    let fence = match command.item_fence {
        Some(fence) => fence,
        None => return Some(held_bank_outcome()),
    };
    let reference = CommandRef::new(
        command.game_session_id,
        CommandId::new(command.command_id).ok()?,
    );
    let native = admission
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay);
    let weights = native.map(super::super::item_ref_admission::NativeItemWeights);
    let runtime = admission.runtime.lock().await;
    let coins = bank_coin_definitions(admission, &runtime)
        .map(|(gold, platinum, crystal)| [gold, platinum, crystal]);
    // Reacquire the Channel lock and recheck the complete quoted conversation before acceptance.
    if let Some(request) = request.as_ref() {
        let intent = NpcTalkIntent {
            npc_actor: EntityRef {
                identity: request.npc_identity,
                generation: request.npc_generation,
            },
            text: String::new(),
        };
        if admission
            .states
            .npc_conversations
            .current_bank(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            )
            .is_none_or(|c| {
                c.0 != request.catalogue_revision
                    || c.1 != request.conversation_revision
                    || c.2 != request.npc_key
            })
        {
            return Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected));
        }
    }
    let result = admission
        .root
        .exchange_carried_coins(
            permit,
            admission.character,
            admission.holder,
            fence,
            reference,
            request,
            None,
            weights.as_ref(),
            coins,
            None,
        )
        .await;
    match result {
        Ok(None) => None,
        Err(_) => Some(held_bank_outcome()),
        Ok(Some(receipt)) => {
            let intent = NpcTalkIntent {
                npc_actor: EntityRef {
                    identity: receipt.npc_identity,
                    generation: receipt.npc_generation,
                },
                text: String::new(),
            };
            let current = admission
                .states
                .npc_conversations
                .current_bank(
                    &admission.states.chat,
                    &runtime,
                    actor,
                    command.game_session_id,
                    &intent,
                )
                .is_some();
            let mut outcome = NpcConversations::bank_reply(&intent, receipt.line);
            outcome.inventory_changed = receipt.changed;
            if !current && let Some(conversation) = outcome.conversation.as_mut() {
                conversation.open = None;
            }
            Some(outcome)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coin_exchange_canary_amount_parser_and_prompts() {
        assert!(contains_keyword("yes please", "yes"));
        assert!(!contains_keyword("yesterday", "yes"));
        assert!(!contains_keyword("exchange gold", "change gold"));
        assert!(contains_keyword("please change gold!", "change gold"));
        for (text, value) in [
            ("1 platinum", Some(1)),
            ("no 12 please", Some(12)),
            ("-7", Some(7)),
            ("0", None),
            ("4294967295", Some(4294967295)),
            ("4294967296", None),
            ("18446744073709551616", None),
            ("none", None),
        ] {
            assert_eq!(parse_amount(text), value);
        }
        assert_eq!(
            quote(CoinExchange::GoldToPlatinum, 2),
            "So you would like me to change 200 of your gold coins into 2 platinum coins?"
        );
        assert_eq!(
            amount_prompt(CoinExchange::CrystalToPlatinum),
            "How many crystal coins would you like to change into platinum?"
        );
        for command in [
            "balance",
            "deposit all",
            "deposit 7 gold",
            "withdraw 8 gold",
            "transfer 9 gold to Al Dric",
        ] {
            assert!(bank_command_precedes_exchange(true, command), "{command}");
            assert!(
                !bank_command_precedes_exchange(false, command),
                "an idle exchange handler must leave {command} to the bank handler"
            );
        }
        for response in ["1", "gold", "crystal", "yes", "no"] {
            assert!(
                !bank_command_precedes_exchange(true, response),
                "{response}"
            );
        }
    }
}
