//! Native coin/change facts for the closed source NPC payment, not a bank writer.
use super::*;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_fee_burn::FeeChangeFacts;
use crate::durability::npc_cosmetics::NpcSourcePaymentFacts;

pub(super) async fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    fence: CurrentCharacterGameplayFence,
) -> Result<NpcSourcePaymentFacts, ()> {
    let (gold, platinum, crystal, default_bag) = {
        let runtime = admission.runtime.lock().await;
        let (gold, platinum, crystal) = bank_coin_definitions(admission, &runtime).ok_or(())?;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            admission.active_generation,
        )
        .ok_or(())?;
        let revision = index.revision("oteryn:item.tibia.i2854").ok_or(())?;
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
        .map_err(|_| ())?;
    let native = admission
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or(())?;
    let definition = current
        .as_ref()
        .map(|bag| &bag.backpack.definition)
        .unwrap_or(&default_bag);
    let backpack =
        crate::gameplay_transport::item_ref_admission::generation_item_facts(native, definition)
            .map_err(|_| ())?;
    let derive = |slot: u8| {
        crate::gameplay_transport::npc_trade_command::derived(
            format!("oteryn:npc-source-payment:v1:{slot}").as_bytes(),
            command.game_session_id,
            command.command_id,
        )
    };
    Ok(NpcSourcePaymentFacts {
        occurrence: derive(0),
        transaction: derive(1),
        event: derive(2),
        occurred_at_unix_ms: crate::gameplay_transport::npc_trade_command::unix_ms_now(),
        server_build_id: "oteryn-game-server".into(),
        change: FeeChangeFacts {
            item_instance_ids: [derive(3), derive(4)],
            gold,
            platinum,
            crystal,
            backpack,
        },
    })
}
