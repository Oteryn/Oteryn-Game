//! Source Baker speech and carried turn-ins use the existing consume/transform owner.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::durability::consume_transform::{CakeBakerContext, ConsumeTransformRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[derive(Clone, Copy)]
enum Topic {
    Mass,
    Base,
    Deliver,
}
pub(super) struct Step {
    context: (u64, u64, String),
    topic: Topic,
}

// Crystal data/npclib/npc.lua:13-19 has a left boundary and no right boundary.
fn keyword(text: &str, needle: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains(needle)
        && !lower
            .match_indices(needle)
            .any(|(index, _)| index != 0 && lower.as_bytes()[index - 1].is_ascii_alphanumeric())
}
fn city(key: &str) -> Option<&'static str> {
    // Crystal data-global/scripts/lib/a_piece_of_cake_config.lua:489-495.
    Some(match key {
        "oteryn:npc.benny_the_baker" => "Carlin",
        "oteryn:npc.biff_the_baker" => "Thais",
        "oteryn:npc.brian_the_baker" => "Edron",
        "oteryn:npc.balduin_the_baker" => "Darashia",
        "oteryn:npc.billy_the_baker" => "Liberty Bay",
        _ => return None,
    })
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
    city(&context.2)?;
    // Crystal data-global/npc/biff_the_baker.lua:63-69 preserves branch order.
    let topic = if keyword(&intent.text, "mass") {
        Topic::Mass
    } else if keyword(&intent.text, "base") {
        Topic::Base
    } else if keyword(&intent.text, "substance") || keyword(&intent.text, "deliver") {
        Topic::Deliver
    } else {
        return None;
    };
    Some(Step { context, topic })
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
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
    if !admission
        .states
        .player_trades
        .physical_service(command.game_session_id)
    {
        return rejected();
    }
    let _revision = admission
        .revision_sequencer
        .acquire(fence.character_id)
        .await;
    let Ok(Some(current)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if current.item_fence() != fence {
        return rejected();
    }
    let current_context = async {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        !states.is_dead(actor)
            && states.spell_idle(&runtime, actor, command.game_session_id)
            && admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) == Some(step.context.clone())
            && runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
    };
    if !current_context.await {
        return rejected();
    }
    if !matches!(step.topic, Topic::Deliver) {
        let progress = match admission
            .root
            .cake_world_progress(admission.world_id, admission.reset_epoch)
            .await
        {
            Ok(progress) => progress,
            Err(_) => return unknown(),
        };
        let (substances, golems) =
            progress.map_or((0, 0), |progress| (progress.substances, progress.golems));
        let line = match step.topic {
            Topic::Mass => format!(
                "So far, {substances} of the 200 Sweet and Sugary Substances have been delivered here in {}.",
                city(&step.context.2).unwrap_or("")
            ),
            Topic::Base => {
                format!("So far, {golems} of the 200 Cake Golems have been lured to the machines.")
            }
            Topic::Deliver => unreachable!(),
        };
        return NpcConversations::bank_reply(&intent, line);
    }
    let inventory = match admission
        .root
        .read_item_use_inventory(admission.character, admission.holder, fence)
        .await
    {
        Ok(inventory) => inventory,
        Err(_) => return unknown(),
    };
    let source = inventory
        .equipment
        .iter()
        .chain(
            inventory
                .backpack
                .iter()
                .flat_map(|backpack| backpack.entries.iter().map(|entry| &entry.item)),
        )
        .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
        .find(|item| {
            item.definition.family == "Item"
                && item.definition.production_key == "oteryn:item.tibia.i12143"
                && item.definition.revision_ref == "definition-r1"
        })
        .map(|item| item.item_instance_id);
    let Ok(bytes) = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&intent) else {
        return rejected();
    };
    // An absent source still reaches the existing receipt lookup, so a replay
    // after all carried stacks were consumed returns the original native reply.
    let request = ConsumeTransformRequest {
        command: CommandRef::new(command.game_session_id, id),
        source: source.unwrap_or([0; 16]),
        target: None,
        intent: bytes,
    };
    let context = CakeBakerContext {
        npc_key: step.context.2,
        expected_epoch: admission.reset_epoch,
    };
    let content = super::super::item_ref_admission::NativeItemWeights(native)
        .with_wheel(admission.premium_coordinator);
    let result = admission
        .root
        .consume_cake_substances(
            &permit,
            admission.character,
            admission.holder,
            fence,
            request,
            native,
            &content,
            &context,
        )
        .await;
    match result {
        Ok(Some(result)) if result.disposition == UseDisposition::Committed => {
            let (Some(accepted), Some(after)) = (
                result
                    .effects
                    .get("accepted")
                    .and_then(serde_json::Value::as_u64),
                result
                    .effects
                    .get("after")
                    .and_then(serde_json::Value::as_u64),
            ) else {
                return unknown();
            };
            // Crystal data-global/npc/biff_the_baker.lua:92-98.
            let mut outcome = NpcConversations::bank_reply(
                &intent,
                format!(
                    "Wonderful, thank you! You delivered {accepted} substance(s). ({after}/200 total delivered)"
                ),
            );
            outcome.inventory_changed = true;
            outcome
        }
        Err(_) => unknown(),
        _ => {
            let progress = match admission
                .root
                .cake_world_progress(admission.world_id, admission.reset_epoch)
                .await
            {
                Ok(progress) => progress,
                Err(_) => return unknown(),
            };
            // Crystal biff_the_baker.lua:70-89: stage, carried count, then goal.
            let line = if progress.is_none_or(|progress| !progress.phase_one_active()) {
                "I'm not collecting anything right now, but thanks for asking!"
            } else if source.is_none() {
                "You don't have any Sweet and Sugary Substances on you. Go defeat some Cake Golems!"
            } else if progress.is_some_and(|progress| progress.substances >= 200) {
                "We already have all the substances we need, thank you though!"
            } else {
                return rejected();
            };
            NpcConversations::bank_reply(&intent, line.into())
        }
    }
}
