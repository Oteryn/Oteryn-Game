//! Cancellation and exact two-item refund use the mature atomic native Inbox MINT.
use super::super::character_inbox::{
    RewardClaimInboxMintCandidate, RewardClaimInboxMintOutput, RewardClaimInboxMintRequest,
    RewardClaimInboxPlan,
};
use super::*;
pub(super) async fn mint(
    tx: &mut Type2Transaction<'_>,
    fence: super::super::item_transfer::CurrentCharacterItemFence,
    command: CommandRef,
    parent: sqlx::types::Uuid,
    binding: [u8; 32],
    native: &NativeGameplayState,
) -> Result<(), DurabilityError> {
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let index = native
        .item_index()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut ids = Vec::new();
    let mut outputs = Vec::new();
    for (ordinal, (source_id, count)) in rules::MATERIALS.into_iter().enumerate() {
        let key = format!("oteryn:item.tibia.i{source_id}");
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.clone(),
            revision_ref: index
                .revision(&key)
                .ok_or(DurabilityError::InvalidStoredState)?
                .into(),
        };
        let facts = oteryn_gameplay_items::item_ref_admission::generation_inbox_item_facts(
            native,
            &definition,
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        let id: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
            .fetch_one(&mut ***tx)
            .await?;
        ids.push(*id.as_bytes());
        outputs.push(RewardClaimInboxMintOutput {
            source_line: u32::try_from(ordinal).map_err(|_| DurabilityError::InvalidStoredState)?,
            item: facts,
            quantity: count,
            key_number: None,
        });
    }
    let (content_revision,ruleset_revision):(String,String)=sqlx::query_as("SELECT content_revision,ruleset_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let positions = RewardClaimInboxMintCandidate::mint_atomic_in_transaction(
        tx,
        fence,
        RewardClaimInboxPlan {
            command,
            character_id: fence.character_id,
            world_id,
            channel_id,
            transaction_id: *parent.as_bytes(),
            source_binding: binding,
            item_instance_ids: ids,
        },
        RewardClaimInboxMintRequest {
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: format!("oteryn:reward_claim.lynda.wedding_cancel.{parent}"),
                revision_ref: "crystal-201189f-lynda-v1".into(),
            },
            content_revision,
            ruleset_revision,
            sim_revision: "lynda-wedding-refund-v1".into(),
            outputs,
        },
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    if positions.len() != 2 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
