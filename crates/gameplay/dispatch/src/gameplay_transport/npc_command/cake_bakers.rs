//! Crystal 201189f8 *_the_baker.lua:58-104: mass/base queries and atomic substance hand-in.
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::seasonal_cake::{BAKERS, CakeDeliveryRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent;

#[derive(Clone, Copy)]
enum Action {
    Mass,
    Base,
    Deliver,
}
pub(super) struct Step {
    context: (u64, u64, String),
    action: Action,
}

pub(super) fn prepare(
    owner: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let context = owner.states.npc_conversations.current_npc(
        &owner.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    if !BAKERS.contains(&context.2.as_str()) {
        return None;
    }
    let text = intent.text.to_ascii_lowercase();
    let action = if text.contains("mass") {
        Action::Mass
    } else if text.contains("base") {
        Action::Base
    } else if text.contains("substance") || text.contains("deliver") {
        Action::Deliver
    } else {
        return None;
    };
    Some(Step { context, action })
}

fn reply(intent: &NpcTalkIntent, text: impl Into<String>, changed: bool) -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: false,
        inventory_changed: changed,
        disposition: NpcIntentDisposition::Ok,
        conversation: Some(NpcConversation {
            open: Some((intent.npc_actor, NpcConversationState::Open)),
            lines: vec![text.into()],
        }),
        window: None,
    }
}
fn rejected() -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: false,
        inventory_changed: false,
        disposition: NpcIntentDisposition::Rejected,
        conversation: None,
        window: None,
    }
}
fn unknown() -> NpcTalkOutcome {
    let mut result = rejected();
    result.unknown = true;
    result
}

pub(super) async fn apply(
    owner: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let (Some(fence), Ok(id)) = (command.item_fence, CommandId::new(command.command_id)) else {
        return rejected();
    };
    let Some(permit) = owner.spell_lane_permit().await else {
        return unknown();
    };
    let (generation, source) = {
        let runtime = owner.runtime.lock().await;
        if owner.states.npc_conversations.current_npc(
            &owner.states.chat,
            &runtime,
            actor,
            command.game_session_id,
            &intent,
        ) != Some(step.context.clone())
        {
            return rejected();
        }
        let source = owner
            .active_generation
            .and_then(|active| active.native_gameplay())
            .filter(|native| {
                native.source_digest() == runtime.content_pin().server_artifact_digest()
            })
            .and_then(|native| {
                native.item_policy_for_source_id(12143).map(|policy| {
                    (
                        native.source_digest(),
                        policy.record().production_definition.clone(),
                    )
                })
            });
        (runtime.binding().scope_generation(), source)
    };
    let now = super::super::seasonal_cake::now_seconds();
    if !matches!(step.action, Action::Deliver) {
        let world = match owner
            .root
            .refresh_cake_world(
                owner.character,
                owner.holder,
                owner.world_id,
                owner.channel_id,
                generation,
                now,
            )
            .await
        {
            Ok(Some(world)) => world,
            Ok(None) => {
                return reply(
                    &intent,
                    "I'm not collecting anything right now, but thanks for asking!",
                    false,
                );
            }
            Err(_) => return unknown(),
        };
        return reply(
            &intent,
            match step.action {
                Action::Mass => {
                    let city = match step.context.2.as_str() {
                        "oteryn:npc.balduin_the_baker" => "Darashia",
                        "oteryn:npc.benny_the_baker" => "Carlin",
                        "oteryn:npc.biff_the_baker" => "Thais",
                        "oteryn:npc.billy_the_baker" => "Liberty Bay",
                        _ => "Edron",
                    };
                    format!(
                        "So far, {} of the 200 Sweet and Sugary Substances have been delivered here in {city}.",
                        world.state.substances
                    )
                }
                _ => format!(
                    "So far, {} of the 200 Cake Golems have been lured to the machines.",
                    world.state.golems
                ),
            },
            false,
        );
    }
    let Some((content_digest, definition)) = source else {
        return rejected();
    };
    let Ok(bytes) = encode_npc_talk_intent(&intent) else {
        return rejected();
    };
    let request = CakeDeliveryRequest {
        command: CommandRef::new(command.game_session_id, id),
        npc_key: step.context.2,
        substance: TypedDefinitionRef {
            family: definition.family,
            production_key: definition.production_key,
            revision_ref: definition.revision_ref,
        },
        content_digest,
        intent: bytes,
        now_seconds: now,
    };
    match owner
        .root
        .deliver_cake_substance(owner.character, owner.holder, &permit, fence, request)
        .await
    {
        Ok(Some(result)) if result.accepted > 0 => reply(
            &intent,
            format!(
                "Wonderful, thank you! You delivered {} substance(s). ({}/200 total delivered)",
                result.accepted, result.substances
            ),
            true,
        ),
        Ok(Some(result)) if result.substances >= 200 => reply(
            &intent,
            "We already have all the substances we need, thank you though!",
            false,
        ),
        Ok(Some(_)) => reply(
            &intent,
            "You don't have any Sweet and Sugary Substances on you. Go defeat some Cake Golems!",
            false,
        ),
        Ok(None) => reply(
            &intent,
            "I'm not collecting anything right now, but thanks for asking!",
            false,
        ),
        Err(_) => unknown(),
    }
}
