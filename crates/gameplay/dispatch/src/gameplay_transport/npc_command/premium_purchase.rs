//! NPC Premium service composition. The Character slot covers live checks, gate and COMMIT.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::content::npc_catalogue::premium_services::PremiumNpcService;
use crate::durability::item_fee_burn::FeeChangeFacts;
use crate::durability::npc_purchase::{
    NpcPurchaseRequest, PurchaseOutcome, PurchaseReceipt, PurchaseRefusal, PurchaseService,
};
use crate::foundation::{CommandId, CommandRef};

pub(super) struct PendingPurchase {
    service: PremiumNpcService,
    context: (u64, u64, String),
    npc: EntityRef,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    opened: u64,
    node: ([u8; 16], u64),
}

pub(super) enum Step {
    Reply(String),
    Buy(Box<PendingPurchase>),
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
    let book = admission.states.chat.npc_talk()?;
    let entry = book.get(&context.2)?;
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.get_mut(&command.game_session_id)?;
    let pending = state.premium.take();
    let now = admission.owner_now().get();
    if intent.text.trim_matches(' ').eq_ignore_ascii_case("yes") {
        if let Some(pending) = pending {
            let valid = pending.context == context
                && pending.npc == intent.npc_actor
                && Some(pending.fence) == command.item_fence
                && pending.node
                    == (
                        *admission.holder.fact().node_id().as_bytes(),
                        admission.holder.fact().registration_revision(),
                    )
                && now >= pending.opened
                && now - pending.opened < 30_000_000;
            return Some(if valid {
                Step::Buy(Box::new(pending))
            } else {
                Step::Reply("The offer has expired.".into())
            });
        }
        return None;
    }
    let service =
        oteryn_gameplay_npc::npc_promotion::select(&intent.text, &entry.premium_services)?;
    let fence = command.item_fence?;
    let line = oteryn_gameplay_npc::npc_promotion::prompt(service);
    state.bank = None;
    state.premium = Some(PendingPurchase {
        service: service.clone(),
        context,
        npc: intent.npc_actor,
        fence,
        opened: now,
        node: (
            *admission.holder.fact().node_id().as_bytes(),
            admission.holder.fact().registration_revision(),
        ),
    });
    Some(Step::Reply(line))
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let pending = match step {
        Step::Reply(line) => return NpcConversations::bank_reply(&intent, line),
        Step::Buy(pending) => *pending,
    };
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    let Ok(id) = CommandId::new(command.command_id) else {
        return rejected();
    };
    let reference = CommandRef::new(command.game_session_id, id);
    let mut slot = admission
        .revision_sequencer
        .acquire(pending.fence.character_id)
        .await;
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if fence.item_fence() != pending.fence {
        return rejected();
    }
    let runtime = admission.runtime.lock().await;
    if admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        &runtime,
        actor,
        command.game_session_id,
        &intent,
    ) != Some(pending.context.clone())
    {
        return rejected();
    }
    let Some((gold, platinum, crystal)) = bank_coin_definitions(admission, &runtime) else {
        return rejected();
    };
    let Some(index) =
        super::super::item_ref_admission::pinned_index(&runtime, admission.active_generation)
    else {
        return rejected();
    };
    let Some(default_backpack_revision) = index.revision("oteryn:item.tibia.i2854") else {
        return rejected();
    };
    let default_backpack = crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i2854".into(),
        revision_ref: default_backpack_revision.into(),
    };
    drop(runtime);
    let current = match admission
        .root
        .read_character_backpack(admission.character, fence.character_id)
        .await
    {
        Ok(current) => current,
        Err(_) => return unknown(),
    };
    let Some(native) = admission
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
    else {
        return rejected();
    };
    let definition = current
        .as_ref()
        .map(|current| &current.backpack.definition)
        .unwrap_or(&default_backpack);
    let Ok(backpack) = super::super::item_ref_admission::generation_item_facts(native, definition)
    else {
        return rejected();
    };
    let service = match &pending.service {
        PremiumNpcService::Promotion { service } => PurchaseService::Promotion(service.clone()),
        PremiumNpcService::Blessing(blessing) => {
            PurchaseService::Blessing(blessing.service.clone())
        }
    };
    let derive = |slot: u8| {
        let tag = format!("oteryn:npc-premium-purchase:v1:{}:{slot}", service.key());
        super::super::npc_trade_command::derived(
            tag.as_bytes(),
            command.game_session_id,
            command.command_id,
        )
    };
    let Some(account) = admission
        .states
        .premium_sessions
        .lock()
        .ok()
        .and_then(|sessions| sessions.account_of(command.game_session_id))
    else {
        return rejected();
    };
    let request = NpcPurchaseRequest {
        command: reference,
        occurrence: derive(0),
        transaction_id: derive(1),
        event_id: derive(2),
        account,
        npc_key: pending.context.2.clone(),
        npc_identity: intent.npc_actor.identity,
        npc_generation: intent.npc_actor.generation,
        service: service.clone(),
        coins: FeeChangeFacts {
            item_instance_ids: [derive(3), derive(4)],
            gold,
            platinum,
            crystal,
            backpack,
        },
        expected_backpack: current
            .as_ref()
            .map(|current| current.backpack.item_instance_id),
        occurred_at_unix_ms: super::super::npc_trade_command::unix_ms_now(),
    };
    let Some(gate) = admission.premium_coordinator else {
        return rejected();
    };
    match slot
        .commit_npc_purchase(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            request,
            gate,
        )
        .await
    {
        Ok(PurchaseOutcome::Committed(receipt)) => completion(&intent, &receipt),
        Ok(PurchaseOutcome::Refused(reason)) => NpcConversations::bank_reply(
            &intent,
            match reason {
                PurchaseRefusal::NotEligible => "You cannot be promoted yet.",
                PurchaseRefusal::AlreadyHeld => "You already have this blessing.",
                PurchaseRefusal::PremiumRequired => "You need current Premium for this service.",
                PurchaseRefusal::InsufficientFunds => {
                    "You do not have enough gold for this service."
                }
            }
            .into(),
        ),
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => unknown(),
        Err(error) => {
            super::super::operator_event(&format!("npc_premium_purchase_refused reason={error}"));
            rejected()
        }
    }
}

fn completion(intent: &NpcTalkIntent, receipt: &PurchaseReceipt) -> NpcTalkOutcome {
    let line = match receipt.service {
        PurchaseService::Promotion(_) => "You are now promoted.",
        PurchaseService::Blessing(_) => "You have received the blessing.",
    };
    NpcTalkOutcome {
        inventory_changed: true,
        ..NpcConversations::bank_reply(intent, line.into())
    }
}

pub(in crate::gameplay_transport) async fn resume_premium_talk(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    _actor: ExactActorRef,
    command: UseCommand,
) -> Option<NpcTalkOutcome> {
    let (fence, reference) = match retained_bank_key(&command) {
        Ok(Some(key)) => key,
        Ok(None) => return None,
        Err(_) => return Some(held_bank_outcome()),
    };
    let _slot = admission
        .revision_sequencer
        .acquire(fence.character_id)
        .await;
    match admission
        .root
        .reconcile_premium_purchase(admission.character, admission.holder, fence, reference)
        .await
    {
        Ok(Some(receipt)) => Some(completion(
            &NpcTalkIntent {
                npc_actor: EntityRef {
                    identity: receipt.npc_identity,
                    generation: receipt.npc_generation,
                },
                text: String::new(),
            },
            &receipt,
        )),
        Ok(None) => None,
        Err(_) => Some(held_bank_outcome()),
    }
}

pub(in crate::gameplay_transport) async fn resume_npc_talk(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
) -> Option<NpcTalkOutcome> {
    if let Some(outcome) = resume_premium_talk(admission, actor, command).await {
        return Some(outcome);
    }
    super::resume_bank_talk(admission, actor, command).await
}
