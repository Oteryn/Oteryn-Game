//! Native Yeo barter ingress. Text/placement are native adaptations of retained wiki ratio.
use super::super::item_ref_admission::{generation_item_facts, pinned_index};
use super::super::npc_trade_command::{derived, occurrence};
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::npc_trade::yeo::{GLOO, NPC, POSITION};
use crate::durability::npc_trade::{
    self, NpcTradeCause, NpcTradeCoins, NpcTradeOutcome, NpcTradeRequest, NpcTradeSide,
};
use crate::foundation::{CommandId, CommandRef};
#[derive(Clone, PartialEq, Eq)]
struct Source {
    context: (u64, u64, String),
    npc: EntityRef,
    epoch: u64,
    digest: [u8; 32],
}
#[derive(Clone)]
pub(super) struct Pending {
    source: Source,
    fence: CurrentCharacterItemFence,
    opened: u64,
    request: Option<NpcTradeRequest>,
}
pub(super) enum Step {
    Reply,
    Commit(Box<Pending>),
}
fn source(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
) -> Option<Source> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    let npc = service_context::live_npc(runtime, actor, command.game_session_id, intent.npc_actor)?;
    if context.2 != NPC
        || (npc.position.x, npc.position.y, npc.position.floor)
            != (
                i32::from(POSITION.0),
                i32::from(POSITION.1),
                i16::from(POSITION.2),
            )
        || !runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|f| f.control_loss.is_none())
    {
        return None;
    }
    Some(Source {
        context,
        npc: intent.npc_actor,
        epoch: admission.reset_epoch,
        digest: runtime.content_pin().server_artifact_digest(),
    })
}
fn clear(admission: &super::super::ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.yeo = None;
    }
}
fn success(intent: &NpcTalkIntent) -> NpcTalkOutcome {
    let mut reply =
        NpcConversations::bank_reply(intent, "Here is your bottle of gloo for ten shrimp.".into());
    reply.inventory_changed = true;
    reply
}
pub(super) async fn resume(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
) -> Option<NpcTalkOutcome> {
    let pending = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .yeo
        .clone()?;
    let request = pending.request.as_ref()?;
    {
        let runtime = admission.runtime.lock().await;
        if service_context::live_npc(&runtime, actor, command.game_session_id, intent.npc_actor)?
            .key
            != NPC
        {
            return None;
        }
    }
    match admission
        .root
        .read_npc_trade(
            admission.character,
            pending.fence.character_id,
            request.command,
            request.cause.occurrence,
        )
        .await
    {
        Ok(Some(receipt))
            if receipt.npc_key == NPC
                && receipt.side == NpcTradeSide::Buy
                && receipt.quantity == 1
                && receipt.unit_price == 0
                && receipt.catalogue_revision == request.cause.catalogue_revision
                && receipt.offer_index == 0
                && !receipt.sell_all =>
        {
            clear(admission, command.game_session_id);
            Some(success(intent))
        }
        Ok(None) => {
            let current = {
                let runtime = admission.runtime.lock().await;
                source(admission, &runtime, actor, command, intent)
            };
            if current == Some(pending.source) && command.item_fence == Some(pending.fence) {
                return None;
            }
            clear(admission, command.game_session_id);
            Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
        }
        _ => Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }),
    }
}
pub(super) fn prepare(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let source = source(admission, runtime, actor, command, intent)?;
    let fence = command.item_fence?;
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.get_mut(&command.game_session_id)?;
    if let Some(pending) = &state.yeo
        && pending.request.is_some()
    {
        return Some(Step::Commit(Box::new(pending.clone())));
    }
    let text = intent.text.trim().to_ascii_lowercase();
    if matches!(
        text.as_str(),
        "gloo" | "shrimp" | "trade" | "offer" | "offers"
    ) {
        state.yeo = Some(Pending {
            source,
            fence,
            opened: admission.owner_now().get(),
            request: None,
        });
        return Some(Step::Reply);
    }
    if text != "yes" {
        return None;
    }
    let pending = state.yeo.take()?;
    if pending.source != source
        || pending.fence != fence
        || admission.owner_now().get().saturating_sub(pending.opened) >= 30_000_000
    {
        return None;
    }
    state.yeo = Some(pending.clone());
    Some(Step::Commit(Box::new(pending)))
}
pub(super) async fn apply(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let Step::Commit(mut pending) = step else {
        return NpcConversations::bank_reply(&intent,"I can barter one bottle of gloo for ten shrimp. Place the shrimp in your main backpack. Do you agree?".into());
    };
    let Some(_permit) = admission.spell_lane_permit().await else {
        return rejected();
    };
    if !admission
        .states
        .player_trades
        .physical_service(command.game_session_id)
    {
        return rejected();
    }
    let Some(native) = admission
        .active_generation
        .and_then(|g| g.native_gameplay())
    else {
        return rejected();
    };
    {
        let runtime = admission.runtime.lock().await;
        let Ok(states) = admission.states.spell_states.try_lock() else {
            return rejected();
        };
        if source(admission, &runtime, actor, command, &intent) != Some(pending.source.clone())
            || command.item_fence != Some(pending.fence)
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == pending.fence.character_id.as_bytes()
                        && lease.generation() == pending.fence.character_lease_generation
                })
        {
            return rejected();
        }
    }
    let request = if let Some(request) = &pending.request {
        request.clone()
    } else {
        let (item, coins) = {
            let runtime = admission.runtime.lock().await;
            if source(admission, &runtime, actor, command, &intent) != Some(pending.source.clone())
            {
                return rejected();
            }
            let Ok(states) = admission.states.spell_states.try_lock() else {
                return rejected();
            };
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return rejected();
            }
            let Some(index) = pinned_index(&runtime, admission.active_generation) else {
                return rejected();
            };
            let definition = |key: &str| {
                index.revision(key).map(|revision| TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.into(),
                    revision_ref: revision.into(),
                })
            };
            let (Some(gold), Some(platinum), Some(crystal), Some(gloo)) = (
                definition("oteryn:item.tibia.i3031"),
                definition("oteryn:item.tibia.i3035"),
                definition("oteryn:item.tibia.i3043"),
                definition(GLOO),
            ) else {
                return rejected();
            };
            let Ok(item) = generation_item_facts(native, &gloo) else {
                return rejected();
            };
            (
                item,
                NpcTradeCoins {
                    gold,
                    platinum,
                    crystal,
                },
            )
        };
        let Ok(Some(backpack)) = admission
            .root
            .read_character_backpack(admission.character, pending.fence.character_id)
            .await
        else {
            return rejected();
        };
        let Ok(backpack) = generation_item_facts(native, &backpack.backpack.definition) else {
            return rejected();
        };
        let Ok(id) = CommandId::new(command.command_id) else {
            return rejected();
        };
        let id_for = |tag: &[u8]| derived(tag, command.game_session_id, command.command_id);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_millis()).ok())
            .unwrap_or(0);
        NpcTradeRequest {
            fluid_subtype: None,
            full_charges: None,
            cause: NpcTradeCause {
                npc_key: NPC.into(),
                catalogue_revision: pending.source.context.0,
                offer_index: 0,
                side: NpcTradeSide::Buy,
                occurrence: occurrence(
                    command.game_session_id,
                    command.command_id,
                    intent.npc_actor,
                ),
            },
            command: CommandRef::new(command.game_session_id, id),
            quantity: 1,
            unit_price: 0,
            offer_count: 1,
            item,
            coins,
            backpack,
            transaction_id: id_for(b"oteryn:yeo-barter:transaction:v1"),
            event_id: id_for(b"oteryn:yeo-barter:event:v1"),
            occurred_at_unix_ms: now,
            output_item_instance_ids: [
                id_for(b"oteryn:yeo-barter:output1:v1"),
                id_for(b"oteryn:yeo-barter:output2:v1"),
                id_for(b"oteryn:yeo-barter:output3:v1"),
            ],
        }
    };
    pending.request = Some(request.clone());
    {
        let mut sessions = match admission.states.npc_conversations.sessions.lock() {
            Ok(s) => s,
            Err(_) => return rejected(),
        };
        sessions.entry(command.game_session_id).or_default().yeo = Some(*pending.clone());
    }
    #[cfg(test)]
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&command.game_session_id)
    {
        state.yeo_last_request = Some(request.clone());
    }
    match admission
        .root
        .commit_yeo_barter(
            admission.character,
            admission.holder,
            pending.fence,
            request,
            POSITION,
        )
        .await
    {
        Ok(NpcTradeOutcome::Committed(_) | NpcTradeOutcome::AlreadyCommitted(_)) => {
            clear(admission, command.game_session_id);
            success(&intent)
        }
        Err(npc_trade::NpcTradeError::Unavailable(_)) => NpcTalkOutcome {
            unknown: true,
            ..rejected()
        },
        _ => {
            clear(admission, command.game_session_id);
            NpcConversations::bank_reply(
                &intent,
                "I need ten plain shrimp in your main backpack and room for one bottle of gloo."
                    .into(),
            )
        }
    }
}

#[cfg(test)]
impl NpcConversations {
    pub(in crate::gameplay_transport) fn yeo_request_for_test(
        &self,
        session: GameSessionId,
    ) -> Option<NpcTradeRequest> {
        self.sessions
            .lock()
            .ok()?
            .get(&session)?
            .yeo_last_request
            .clone()
    }
}
