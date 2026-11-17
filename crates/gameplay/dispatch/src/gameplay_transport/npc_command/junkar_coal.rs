//! An explicit donation from a focused Junkar conversation uses the carried-item owner.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::durability::item_craft::{CoalDelivery, ItemCraftRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_gameplay_world_change::owner::COAL_ITEM;
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) async fn donate(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    context: (u64, u64, String),
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    let (Some(fence), Ok(id), Some(native), Ok(encoded)) = (
        command.item_fence,
        CommandId::new(command.command_id),
        admission
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay),
        oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(intent),
    ) else {
        return rejected();
    };
    let Some(permit) = admission.spell_lane_permit().await else {
        return unknown();
    };
    let Ok(inventory) = admission
        .root
        .read_item_use_inventory(admission.character, admission.holder, fence)
        .await
    else {
        return rejected();
    };
    let source = inventory
        .backpack
        .iter()
        .flat_map(|bag| &bag.entries)
        .find(|entry| entry.item.definition.production_key == COAL_ITEM)
        .map(|entry| entry.item.item_instance_id);
    {
        let runtime = admission.runtime.lock().await;
        if !runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
            || admission.states.spell_states.lock().await.is_dead(actor)
            || admission
                .states
                .npc_conversations
                .current_npc(
                    &admission.states.chat,
                    &runtime,
                    actor,
                    command.game_session_id,
                    intent,
                )
                .as_ref()
                != Some(&context)
            || !admission
                .states
                .player_trades
                .physical(command.game_session_id, source)
        {
            return rejected();
        }
    }
    let request = ItemCraftRequest {
        command: CommandRef::new(command.game_session_id, id),
        // Allow exact replay even after consuming the last coal.
        source: source.unwrap_or([0; 16]),
        tool: None,
        ingredient: None,
        station: None,
        intent: [b"oteryn:steamship-coal\0".as_slice(), &encoded].concat(),
        gathering: None,
        enchanting: None,
    };
    match admission
        .root
        .deliver_world_change_coal(
            &permit,
            admission.character,
            admission.holder,
            fence,
            request,
            native,
            &NativeItemWeights(native).with_wheel(admission.premium_coordinator),
            admission.reset_epoch,
            CoalDelivery::Steamship,
        )
        .await
    {
        Ok(Some(UseDisposition::Committed)) => {
            let mut reply = NpcConversations::bank_reply(intent, "Thank you for the coal.".into());
            reply.inventory_changed = true;
            reply
        }
        Ok(_) => rejected(),
        Err(_) => unknown(),
    }
}
