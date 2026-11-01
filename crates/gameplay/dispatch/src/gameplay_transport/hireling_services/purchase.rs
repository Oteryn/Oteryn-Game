//! Cook and package commands keep the existing native economic owners and replay before NPC state.
use super::super::super::item_ref_admission::{
    NativeItemWeights, generation_item_facts, pinned_index,
};
use super::super::super::npc_trade_command::{derived, occurrence, unix_ms_now};
use super::*;
use crate::domain::currency::{COIN_DEFINITION_FAMILY, Coin};
use crate::domain::hireling::food::FoodPlan;
use crate::durability::house_hireling::{HirelingFoodError, HirelingFoodRequest};
use crate::durability::item_fee_burn::{FeeChangeFacts, HirelingFeeChange};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::ItemDefinitionFacts;
use crate::durability::npc_trade::{
    HirelingPackageSelection, NpcTradeCause, NpcTradeCoins, NpcTradeError, NpcTradeRequest,
};
use crate::foundation::{CommandId, CommandRef};

pub(super) fn intent_binding(intent: &NpcTalkIntent) -> [u8; 32] {
    Sha256::new()
        .chain_update(b"oteryn:hireling-talk:v1\0")
        .chain_update(intent.npc_actor.identity)
        .chain_update(intent.npc_actor.generation.to_be_bytes())
        .chain_update(intent.text.trim().to_ascii_lowercase())
        .finalize()
        .into()
}

/// Retry-stable draw through the same native decision/uniform owner as fishing.
/// No client roll or unbounded rejection loop enters the Cook command.
pub(super) fn draw(
    digest: [u8; 32],
    command: UseCommand,
    hireling: [u8; 16],
    bound: u8,
) -> Option<u8> {
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };
    if !matches!(bound, 5 | 6) {
        return None;
    }
    let occurrence = Sha256::new()
        .chain_update(b"oteryn:hireling-dish:v1\0")
        .chain_update(command.game_session_id.as_bytes())
        .chain_update(command.command_id.to_be_bytes())
        .chain_update(hireling)
        .chain_update([bound])
        .finalize();
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence[..16].try_into().ok()?);
    let decision = deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(digest),
        occurrence,
        "hireling.cook",
        0,
    )
    .ok()?;
    u8::try_from(crate::spell::uniform_draw(
        decision,
        0,
        i64::from(bound) - 1,
    ))
    .ok()
}

async fn facts(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    id: u32,
) -> Option<ItemDefinitionFacts> {
    let native = admission.active_generation?.native_gameplay()?;
    let runtime = admission.runtime.lock().await;
    let index = pinned_index(&runtime, admission.active_generation)?;
    let key = format!("oteryn:item.tibia.i{id}");
    let revision = index.revision(&key)?.to_owned();
    generation_item_facts(
        native,
        &TypedDefinitionRef {
            family: "Item".into(),
            production_key: key,
            revision_ref: revision,
        },
    )
    .ok()
}

async fn change(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Option<HirelingFeeChange> {
    let fence = command.item_fence?;
    let native = admission.active_generation?.native_gameplay()?;
    let ids = [
        derived(
            b"hireling-food-change-platinum",
            command.game_session_id,
            command.command_id,
        ),
        derived(
            b"hireling-food-change-gold",
            command.game_session_id,
            command.command_id,
        ),
    ];
    let bag = admission
        .root
        .read_character_backpack(admission.character, fence.character_id)
        .await
        .ok()?;
    let Some(bag) = bag else {
        return Some(HirelingFeeChange::BankOnly {
            item_instance_ids: ids,
        });
    };
    let backpack = generation_item_facts(native, &bag.backpack.definition).ok()?;
    let runtime = admission.runtime.lock().await;
    let index = pinned_index(&runtime, admission.active_generation)?;
    let coin = |coin: Coin| {
        Some(TypedDefinitionRef {
            family: COIN_DEFINITION_FAMILY.into(),
            production_key: coin.production_key().into(),
            revision_ref: index.revision(coin.production_key())?.into(),
        })
    };
    Some(HirelingFeeChange::Carried(Box::new(FeeChangeFacts {
        item_instance_ids: ids,
        platinum: coin(Coin::Platinum)?,
        gold: coin(Coin::Gold)?,
        crystal: coin(Coin::Crystal)?,
        backpack,
    })))
}

fn completed(intent: &NpcTalkIntent, message: &str) -> NpcTalkOutcome {
    let mut outcome = NpcConversations::bank_reply(intent, message.into());
    outcome.inventory_changed = true;
    outcome
}

pub(super) async fn cook(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    context: Context,
    snapshot: HirelingSnapshot,
    food: FoodPlan,
) -> NpcTalkOutcome {
    let Some(item_fence) = command.item_fence else {
        return rejected();
    };
    let Some(_permit) = admission.spell_lane_permit().await else {
        return unknown();
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
    let (Some(item), Some(change)) = (
        facts(admission, food.item_id).await,
        change(admission, command).await,
    ) else {
        return rejected();
    };
    let Ok(command_id) = CommandId::new(command.command_id) else {
        return rejected();
    };
    let _slot = admission
        .revision_sequencer
        .acquire(item_fence.character_id)
        .await;
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if fence.item_fence() != item_fence {
        return rejected();
    }
    let runtime = admission.runtime.lock().await;
    if admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        &runtime,
        actor,
        command.game_session_id,
        intent,
    ) != Some(context)
        || admission.states.spell_states.lock().await.is_dead(actor)
    {
        return rejected();
    }
    let id = |tag: &[u8]| derived(tag, command.game_session_id, command.command_id);
    let request = HirelingFoodRequest {
        command: CommandRef::new(command.game_session_id, command_id),
        hireling_id: snapshot.id,
        intent_binding: intent_binding(intent),
        food,
        item,
        output_id: id(b"hireling-food-output"),
        mint_transaction: id(b"hireling-food-mint"),
        fee_transaction: id(b"hireling-food-fee"),
        fee_event: id(b"hireling-food-fee-event"),
        fee_occurrence: id(b"hireling-food-occurrence"),
        change,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
    };
    let result = admission
        .root
        .commit_hireling_food(
            admission.character,
            admission.holder,
            fence,
            admission.channel_id,
            request,
            &NativeItemWeights(native).with_wheel(admission.premium_coordinator),
        )
        .await;
    drop(runtime);
    match result {
        Ok(_) => completed(intent, "Your meal has been delivered to your Inbox."),
        Err(HirelingFoodError::Unavailable(_)) => unknown(),
        Err(HirelingFoodError::InsufficientFunds) => {
            NpcConversations::bank_reply(intent, "Sorry, you don't have enough money.".into())
        }
        Err(HirelingFoodError::InboxFull) => NpcConversations::bank_reply(
            intent,
            "Sorry, you don't have enough room in your Inbox.".into(),
        ),
        Err(HirelingFoodError::Capacity) => {
            NpcConversations::bank_reply(intent, "Sorry, you don't have enough capacity.".into())
        }
        Err(_) => rejected(),
    }
}

pub(super) async fn package(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    context: Context,
    snapshot: HirelingSnapshot,
    index: usize,
) -> NpcTalkOutcome {
    let (Some(fence), Some(package)) = (command.item_fence, catalogue::PACKAGES.get(index)) else {
        return rejected();
    };
    let Some(_permit) = admission.spell_lane_permit().await else {
        return unknown();
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
    let (Some(present), Some(change)) = (
        facts(admission, catalogue::PRESENT_ITEM_ID).await,
        change(admission, command).await,
    ) else {
        return rejected();
    };
    let HirelingFeeChange::Carried(change) = change else {
        return NpcConversations::bank_reply(intent, "You need a backpack for the present.".into());
    };
    let change = *change;
    let mut children = Vec::with_capacity(3);
    for (item, _) in package.items {
        let Some(item) = facts(admission, item).await else {
            return rejected();
        };
        children.push(item);
    }
    let Ok(children) = children.try_into() else {
        return rejected();
    };
    let Ok(command_id) = CommandId::new(command.command_id) else {
        return rejected();
    };
    let runtime = admission.runtime.lock().await;
    if admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        &runtime,
        actor,
        command.game_session_id,
        intent,
    ) != Some(context.clone())
        || admission.states.spell_states.lock().await.is_dead(actor)
    {
        return rejected();
    }
    let id = |tag: &[u8]| derived(tag, command.game_session_id, command.command_id);
    let request = NpcTradeRequest {
        cause: NpcTradeCause {
            npc_key: snapshot.npc_key(),
            catalogue_revision: context.0,
            offer_index: u32::try_from(index).unwrap_or(u32::MAX),
            side: crate::durability::npc_trade::NpcTradeSide::Buy,
            occurrence: occurrence(
                command.game_session_id,
                command.command_id,
                intent.npc_actor,
            ),
        },
        command: CommandRef::new(command.game_session_id, command_id),
        quantity: 1,
        unit_price: package.gold,
        offer_count: 1,
        item: present,
        transaction_id: id(b"hireling-package-trade"),
        event_id: id(b"hireling-package-event"),
        occurred_at_unix_ms: unix_ms_now(),
        output_item_instance_ids: [
            id(b"hireling-package-change-platinum"),
            id(b"hireling-package-change-gold"),
            id(b"hireling-package-present"),
        ],
        coins: NpcTradeCoins {
            gold: change.gold,
            platinum: change.platinum,
            crystal: change.crystal,
        },
        backpack: change.backpack,
    };
    let selection = HirelingPackageSelection {
        hireling_id: snapshot.id,
        package: index,
        origin_channel: admission.channel_id,
        children,
        intent_binding: intent_binding(intent),
        child_ids: [
            id(b"hireling-package-first"),
            id(b"hireling-package-second"),
            id(b"hireling-package-third"),
        ],
    };
    let result = admission
        .root
        .commit_hireling_package(
            admission.character,
            admission.holder,
            fence,
            request,
            selection,
            &NativeItemWeights(native).with_wheel(admission.premium_coordinator),
        )
        .await;
    drop(runtime);
    match result {
        Ok(_) => completed(intent, "Your imbuement package is in your backpack."),
        Err(NpcTradeError::Unavailable(_)) => unknown(),
        Err(NpcTradeError::InsufficientFunds) => {
            NpcConversations::bank_reply(intent, "Sorry, you don't have enough money.".into())
        }
        Err(NpcTradeError::DoesNotFit | NpcTradeError::CapacityExceeded) => {
            NpcConversations::bank_reply(
                intent,
                "You need enough capacity and a free backpack slot for the present.".into(),
            )
        }
        Err(_) => rejected(),
    }
}

pub(super) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    intent: &NpcTalkIntent,
) -> Option<NpcTalkOutcome> {
    let fence = command.item_fence?;
    let reference = CommandRef::new(
        command.game_session_id,
        CommandId::new(command.command_id).ok()?,
    );
    match admission
        .root
        .read_hireling_food_receipt(
            admission.character,
            fence.character_id,
            reference,
            intent_binding(intent),
        )
        .await
    {
        Ok(true) => {
            return Some(completed(
                intent,
                "Your meal has been delivered to your Inbox.",
            ));
        }
        Ok(false) => {}
        Err(HirelingFoodError::Unavailable(_)) => return Some(unknown()),
        Err(_) => return Some(rejected()),
    }
    match admission
        .root
        .read_hireling_package_receipt(
            admission.character,
            fence.character_id,
            reference,
            intent_binding(intent),
        )
        .await
    {
        Ok(true) => Some(completed(
            intent,
            "Your imbuement package is in your backpack.",
        )),
        Ok(false) => None,
        Err(NpcTradeError::Unavailable(_)) => Some(unknown()),
        Err(_) => Some(rejected()),
    }
}

#[cfg(test)]
mod draw_tests {
    #![allow(clippy::expect_used)]
    use super::*;
    #[test]
    fn cook_uses_retry_stable_native_decisions_with_only_source_draw_bounds() {
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = 1;
        let command = UseCommand {
            game_session_id: GameSessionId::decode(&id).expect("session"),
            command_id: 1,
            item_fence: None,
        };
        for bound in [5, 6] {
            for command_id in 1..=64 {
                let current = UseCommand {
                    command_id,
                    ..command
                };
                let first = draw([7; 32], current, id, bound).expect("native draw");
                assert!(first < bound);
                assert_eq!(draw([7; 32], current, id, bound), Some(first));
            }
        }
        assert_eq!(draw([7; 32], command, id, 0), None);
        assert_eq!(draw([7; 32], command, id, 7), None);
    }
}
