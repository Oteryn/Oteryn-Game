use super::*;
use crate::durability::guild_bank::{GuildBankTransfer, GuildBankTransferResult};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::CommandRef;
use oteryn_protocol_oteryn::npc_service::TRAVEL_CONFIRMATION_TIMEOUT_SECONDS;

pub(super) struct Conversation<'a> {
    pub intent: &'a NpcTalkIntent,
    pub command: CommandRef,
    pub fence: CurrentCharacterItemFence,
    pub catalogue_revision: u64,
    pub conversation_generation: u64,
    pub npc_key: &'a str,
}

#[derive(PartialEq, Eq)]
struct Context {
    fence: CurrentCharacterItemFence,
    npc: EntityRef,
    catalogue_revision: u64,
    conversation_generation: u64,
}

pub(super) struct Pending {
    context: Context,
    expires_at_ms: u64,
    step: Step,
}

enum Step {
    Amount,
    Recipient(u64),
    Confirm(u64, String),
}

pub(super) async fn talk(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    at: Conversation<'_>,
) -> Option<NpcTalkOutcome> {
    let state = &admission.states.npc_conversations;
    let session = at.fence.game_session_id;
    let now = admission.owner_now().get() / 1_000;
    let context = Context {
        fence: at.fence,
        npc: at.intent.npc_actor,
        catalogue_revision: at.catalogue_revision,
        conversation_generation: at.conversation_generation,
    };
    let pending = {
        let mut sessions = state.sessions.lock().ok()?;
        sessions.get_mut(&session)?.guild_bank.take()
    }
    .filter(|pending| pending.context == context && now < pending.expires_at_ms);
    let text = at.intent.text.trim();
    let step = if let Some(args) = transfer_arguments(text) {
        parse_start(args)?
    } else {
        match pending?.step {
            Step::Amount => Step::Recipient(amount(text)?),
            Step::Recipient(amount) => Step::Confirm(amount, recipient(text)?.to_owned()),
            Step::Confirm(amount, recipient) if text.eq_ignore_ascii_case("yes") => {
                return Some(commit(admission, at, amount, recipient).await);
            }
            Step::Confirm(_, _) => return None,
        }
    };
    let line = match &step {
        Step::Amount => "How much guild gold do you want to send?".into(),
        Step::Recipient(amount) => {
            format!("Which guild should receive the {amount} gold?")
        }
        Step::Confirm(amount, name) => {
            format!("Send {amount} gold from your guild bank to {name}?")
        }
    };
    let mut sessions = state.sessions.lock().ok()?;
    let entry = sessions.get_mut(&session)?;
    entry.bank = None;
    entry.guild_bank = Some(Pending {
        context,
        expires_at_ms: now + u64::from(TRAVEL_CONFIRMATION_TIMEOUT_SECONDS) * 1_000,
        step,
    });
    Some(NpcConversations::bank_reply(at.intent, line))
}

fn transfer_arguments(text: &str) -> Option<&str> {
    let (guild, rest) = text.split_once(char::is_whitespace)?;
    if !guild.eq_ignore_ascii_case("guild") {
        return None;
    }
    let rest = rest.trim_start();
    let (verb, args) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    verb.eq_ignore_ascii_case("transfer").then_some(args.trim())
}

fn parse_start(args: &str) -> Option<Step> {
    if args.is_empty() {
        return Some(Step::Amount);
    }
    let mut words = args.splitn(2, char::is_whitespace);
    let amount = amount(words.next()?)?;
    let Some(destination) = words.next() else {
        return Some(Step::Recipient(amount));
    };
    let mut destination = destination.trim_start().splitn(2, char::is_whitespace);
    if !destination.next()?.eq_ignore_ascii_case("to") {
        return None;
    }
    Some(Step::Confirm(
        amount,
        recipient(destination.next()?)?.to_owned(),
    ))
}

fn amount(text: &str) -> Option<u64> {
    let value = text.parse().ok()?;
    (1..=oteryn_protocol_oteryn::guild_bank::BALANCE_MAX)
        .contains(&value)
        .then_some(value)
}

fn recipient(text: &str) -> Option<&str> {
    let text = text.trim();
    (!text.is_empty() && text.chars().count() <= 29 && !text.chars().any(char::is_control))
        .then_some(text)
}

async fn commit(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    at: Conversation<'_>,
    amount: u64,
    recipient: String,
) -> NpcTalkOutcome {
    let occurred_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok());
    let Some(occurred_at) = occurred_at else {
        return held_bank_outcome();
    };
    match admission
        .root
        .transfer_guild_bank(
            admission.character,
            admission.holder,
            at.fence,
            at.command,
            Some(GuildBankTransfer {
                recipient,
                amount,
                npc: at.intent.npc_actor,
                npc_key: at.npc_key.into(),
                occurred_at,
            }),
        )
        .await
    {
        Ok(Some(GuildBankTransferResult::Completed { code, .. })) => {
            result(at.intent.npc_actor, code)
        }
        Ok(Some(GuildBankTransferResult::Suspended)) => {
            NpcConversations::bank_reply(at.intent, bank_robbery::clue(at.npc_key).into())
        }
        Err(crate::durability::guild::GuildError::Unavailable(_)) => held_bank_outcome(),
        Ok(None) | Err(_) => NpcTalkOutcome::only(NpcIntentDisposition::Rejected),
    }
}

pub(super) async fn resume(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
) -> Option<NpcTalkOutcome> {
    match admission
        .root
        .transfer_guild_bank(admission.character, admission.holder, fence, command, None)
        .await
    {
        Ok(Some(GuildBankTransferResult::Completed { npc, code })) => {
            let mut reply = result(npc, code);
            if let Some(conversation) = &mut reply.conversation
                && !admission
                    .states
                    .chat
                    .engaged(fence.game_session_id)
                    .is_some_and(|actor| {
                        actor.placement_identity() == npc.identity
                            && actor.actor_local_generation() == npc.generation
                    })
            {
                conversation.open = None;
            }
            Some(reply)
        }
        Ok(_) => None,
        Err(_) => Some(held_bank_outcome()),
    }
}

fn result(npc_actor: EntityRef, code: u8) -> NpcTalkOutcome {
    let line = match code {
        1 => "The guild bank transfer is complete.",
        4 => "Your guild bank has insufficient gold for this transfer.",
        _ => "The guild bank transfer was declined.",
    };
    NpcConversations::bank_reply(
        &NpcTalkIntent {
            npc_actor,
            text: String::new(),
        },
        format!("{line} Check your Inbox for the receipt."),
    )
}
