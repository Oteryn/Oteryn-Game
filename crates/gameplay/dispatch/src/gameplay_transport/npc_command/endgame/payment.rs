//! Bloodshade's double-confirmed purchase uses the established wallet/item fee owner.
use super::*;
use crate::durability::item_fee_burn::FeeChangeFacts;

pub(super) async fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    fence: CurrentCharacterGameplayFence,
) -> Result<RottenBloodAccessPayment, Box<NpcTalkOutcome>> {
    prepare_with_domain(
        admission,
        command,
        fence,
        "oteryn:rotten-blood-access-purchase:v1",
    )
    .await
}

pub(in crate::gameplay_transport::npc_command) async fn prepare_reward_fee(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    fence: CurrentCharacterGameplayFence,
) -> Result<crate::durability::reward_claim_mint::QuestRewardFeePayment, Box<NpcTalkOutcome>> {
    let payment =
        prepare_with_domain(admission, command, fence, "oteryn:quest-reward-fee:v1").await?;
    Ok(
        crate::durability::reward_claim_mint::QuestRewardFeePayment {
            command: payment.command,
            occurrence: payment.occurrence,
            transaction_id: payment.transaction_id,
            event_id: payment.event_id,
            coins: payment.coins,
            expected_backpack: payment.expected_backpack,
            occurred_at_unix_ms: payment.occurred_at_unix_ms,
        },
    )
}

async fn prepare_with_domain(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    fence: CurrentCharacterGameplayFence,
    domain: &str,
) -> Result<RottenBloodAccessPayment, Box<NpcTalkOutcome>> {
    let rejected = || Box::new(NpcTalkOutcome::only(NpcIntentDisposition::Rejected));
    let (gold, platinum, crystal, default_backpack) = {
        let runtime = admission.runtime.lock().await;
        let (gold, platinum, crystal) =
            bank_coin_definitions(admission, &runtime).ok_or_else(rejected)?;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            admission.active_generation,
        )
        .ok_or_else(rejected)?;
        let revision = index
            .revision("oteryn:item.tibia.i2854")
            .ok_or_else(rejected)?;
        (
            gold,
            platinum,
            crystal,
            crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: revision.into(),
            },
        )
    };
    let current = admission
        .root
        .read_character_backpack(admission.character, fence.character_id)
        .await
        .map_err(|_| Box::new(unknown()))?;
    let native = admission
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or_else(rejected)?;
    let definition = current
        .as_ref()
        .map(|bag| &bag.backpack.definition)
        .unwrap_or(&default_backpack);
    let backpack =
        crate::gameplay_transport::item_ref_admission::generation_item_facts(native, definition)
            .map_err(|_| rejected())?;
    let derive = |slot: u8| {
        crate::gameplay_transport::npc_trade_command::derived(
            format!("{domain}:{slot}").as_bytes(),
            command.game_session_id,
            command.command_id,
        )
    };
    Ok(RottenBloodAccessPayment {
        command: CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| rejected())?,
        ),
        occurrence: derive(0),
        transaction_id: derive(1),
        event_id: derive(2),
        coins: FeeChangeFacts {
            item_instance_ids: [derive(3), derive(4)],
            gold,
            platinum,
            crystal,
            backpack,
        },
        expected_backpack: current.as_ref().map(|bag| bag.backpack.item_instance_id),
        occurred_at_unix_ms: crate::gameplay_transport::npc_trade_command::unix_ms_now(),
    })
}

pub(super) fn reply(intent: &NpcTalkIntent, result: &QuestTransitionOutcome) -> NpcTalkOutcome {
    match result {
        QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_) => {
            NpcTalkOutcome {
                inventory_changed: true,
                ..NpcConversations::bank_reply(
                    intent,
                    "The bargain has been made. You may enter the sacred blood.".into(),
                )
            }
        }
        QuestTransitionOutcome::Refused(_) => NpcConversations::bank_reply(
            intent,
            "You already have access, or do not have enough gold and room for the payment.".into(),
        ),
        QuestTransitionOutcome::ObligationClosed => {
            NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }
    }
}
