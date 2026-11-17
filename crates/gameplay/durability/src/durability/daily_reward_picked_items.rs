//! One Inbox mint for the daily temple scroll. Winter Update 2026 retired the item picker,
//! so no selected consumable or exercise weapon is minted (migration 1711 keeps history).
use super::{
    DurabilityError,
    character_inbox::*,
    daily_reward_claim::DailyRewardClaimRequest,
    item_mint::{TypedDefinitionRef, uuid_text},
    item_mint_audit::Type2Transaction,
    item_transfer::{CurrentCharacterItemFence, ItemDefinitionFacts, ItemStackClass},
};
pub(super) async fn mint(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: DailyRewardClaimRequest,
    day: u8,
    temple: u8,
) -> Result<Option<String>, DurabilityError> {
    if temple == 0 {
        return Ok(None);
    }
    let lines = [(25718_u32, u32::from(temple))];
    let mut ids = Vec::new();
    let mut strings = Vec::new();
    let mut outputs = Vec::new();
    for (line, (source, quantity)) in lines.iter().enumerate() {
        let text: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut ***tx)
            .await?;
        ids.push(uuid_text(&text)?);
        strings.push(text);
        outputs.push(RewardClaimInboxMintOutput {
            source_line: line as u32,
            item: ItemDefinitionFacts {
                weight: None,
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: format!("oteryn:item.tibia.i{source}"),
                    revision_ref: "definition-r1".into(),
                },
                stack: ItemStackClass::NonStackable,
                container_capacity: None,
                container_slot_equip_pattern: false,
            },
            quantity: *quantity,
            key_number: None,
        });
    }
    let (content_revision,ruleset_revision):(String,String)=sqlx::query_as("SELECT content_revision,ruleset_revision FROM public.game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    RewardClaimInboxMintCandidate::mint_atomic_in_transaction(
        tx,
        fence,
        RewardClaimInboxPlan {
            command: request.command,
            character_id: fence.character_id,
            world_id,
            channel_id,
            transaction_id: request.transaction_id,
            source_binding: request.request_binding,
            item_instance_ids: ids,
        },
        RewardClaimInboxMintRequest {
            claim: TypedDefinitionRef {
                family: "DailyReward".into(),
                production_key: format!("oteryn:daily_reward.day{day}"),
                revision_ref:
                    oteryn_gameplay_content::content::daily_reward::DailyRewardContent::catalogue()
                        .ok_or(DurabilityError::InvalidStoredState)?
                        .revision
                        .clone(),
            },
            content_revision,
            ruleset_revision,
            sim_revision: "daily-reward-v2".into(),
            outputs,
        },
    )
    .await
    .map_err(|e| match e {
        RewardClaimInboxError::Unavailable(e) => e,
        _ => DurabilityError::InvalidStoredState,
    })?;
    for item in &strings {
        sqlx::query("INSERT INTO public.game_item_bindings(item_instance_id,world_id,character_id,cause) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,'DAILY_REWARD')").bind(item).bind(world_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).execute(&mut ***tx).await?;
    }
    Ok(strings.first().cloned())
}
