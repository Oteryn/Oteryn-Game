//! Crystal201189f brutus_the_baker.lua:59-162; the NPC talk owner resolves
//! source choices and the normal audited MINT owner consumes annual categories.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{RewardClaimMintError, RewardClaimMintRequest};
use crate::foundation::{CommandId, CommandRef};

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<(u64, u64, String)> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    (context.2 == "oteryn:npc.brutus_the_baker").then_some(context)
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    context: (u64, u64, String),
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let keyword = |word| source_keyword(&intent.text, word);
    let selected = if keyword("help") || keyword("reward") || keyword("yes") || keyword("one") {
        None
    } else if keyword("cabinet") {
        Some((1_i16, 14756))
    } else if keyword("backpack") {
        Some((1, 20347))
    } else if keyword("tapestry") {
        Some((1, 20350))
    } else if keyword("cupcake") {
        None
    } else if keyword("blueberry") {
        Some((2, 28484))
    } else if keyword("strawberry") {
        Some((2, 28485))
    } else if keyword("lemon") {
        Some((2, 28486))
    } else {
        return NpcConversations::bank_reply(&intent, "Say help, reward, one or cupcake.".into());
    };
    let Some((category, id)) = selected else {
        let reply = if keyword("help") {
            "Step on the cake base on the isle between visits to all five extraction machines: Thais, Carlin, Darashia, Liberty Bay and Edron."
        } else if keyword("cupcake") {
            "You can have a blueberry cupcake, a strawberry cupcake or a lemon cupcake."
        } else {
            "Choose a cake cabinet, cake backpack or cake tapestry. I will also give you a cupcake."
        };
        return NpcConversations::bank_reply(&intent, reply.into());
    };
    let Some(expected) = command.item_fence else {
        return rejected();
    };
    let Some(_permit) = admission.spell_lane_permit().await else {
        return rejected();
    };
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return rejected();
    };
    if fence.item_fence() != expected {
        return rejected();
    }
    let Some(native) = admission
        .active_generation
        .and_then(|active| active.native_gameplay())
    else {
        return rejected();
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
                &intent,
            ) != Some(context)
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || !admission
                .states
                .player_trades
                .physical_service(command.game_session_id)
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return rejected();
        }
    }
    let Ok(Some(year)) = admission.root.current_cake_year(admission.world_id).await else {
        return rejected();
    };
    let Ok(Some(bag)) = admission
        .root
        .read_character_backpack(admission.character, fence.character_id)
        .await
    else {
        return rejected();
    };
    let facts = |definition: &TypedDefinitionRef| {
        super::super::item_ref_admission::generation_item_facts(native, definition)
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    };
    let (Ok(item), Ok(backpack)) = (facts(&definition), facts(&bag.backpack.definition)) else {
        return rejected();
    };
    let Ok(command_id) = CommandId::new(command.command_id) else {
        return rejected();
    };
    let name = if category == 1 { "item" } else { "cupcake" };
    let request = RewardClaimMintRequest {
        backpack_outputs: false,
        root_charges_remaining: None,
        root_fluid_subtype: None,
        command: CommandRef::new(command.game_session_id, command_id),
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: format!("oteryn:reward-claim.cake-brutus-{name}.{year}"),
            revision_ref: "oteryn:cake-brutus.v1".into(),
        },
        source_placement: "oteryn:npc.brutus_the_baker".into(),
        item,
        backpack,
        quantity: 1,
        content_revision: format!("sha256:{}", hex(native.source_digest())),
        ruleset_revision: "oteryn:cake-brutus.v1".into(),
        sim_revision: "oteryn:cake-brutus.v1".into(),
        additional_items: Vec::new(),
        contents: Vec::new(),
        root_text: None,
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    let weights = super::super::item_ref_admission::NativeItemWeights(native)
        .with_wheel(admission.premium_coordinator);
    match admission
        .root
        .claim_cake_reward(
            admission.character,
            admission.holder,
            expected,
            (year, category),
            request,
            &weights,
        )
        .await
    {
        Ok(_) => {
            admission.states.loop_ground.invalidate();
            let mut reply = NpcConversations::bank_reply(
                &intent,
                if category == 1 {
                    "Here it is. Handle it with care."
                } else {
                    "Here it is. Enjoy your meal!"
                }
                .into(),
            );
            reply.inventory_changed = true;
            reply
        }
        Err(RewardClaimMintError::Unavailable(_)) => {
            let mut reply = rejected();
            reply.unknown = true;
            reply
        }
        Err(_) => NpcConversations::bank_reply(
            &intent,
            "You have not earned this reward, already claimed it this year, or cannot carry it."
                .into(),
        ),
    }
}
fn hex(bytes: [u8; 32]) -> String {
    bytes.into_iter().map(|b| format!("{b:02x}")).collect()
}

// Crystal npclib/npc.lua:13-19 uses a left word frontier, with no right boundary.
fn source_keyword(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase()
        .match_indices(needle)
        .any(|(index, _)| index == 0 || !text.as_bytes()[index - 1].is_ascii_alphanumeric())
}
