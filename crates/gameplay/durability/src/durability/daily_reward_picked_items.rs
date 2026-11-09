//! One Inbox mint for selected consumables/exercise weapons and the existing temple scroll.
use super::{
    DurabilityError,
    character_inbox::*,
    daily_reward_claim::DailyRewardClaimRequest,
    item_mint::{TypedDefinitionRef, uuid_text},
    item_mint_audit::Type2Transaction,
    item_transfer::{CurrentCharacterItemFence, ItemDefinitionFacts, ItemStackClass},
};
use oteryn_protocol_oteryn::daily_reward::picker::Claim;
pub(super) async fn mint(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: DailyRewardClaimRequest,
    day: u8,
    temple: u8,
    selection: Option<&Claim>,
) -> Result<Option<String>, DurabilityError> {
    let mut lines = Vec::new();
    if temple > 0 {
        lines.push((25718, u32::from(temple), false));
    }
    if let Some(selection) = selection {
        for pick in &selection.picks {
            let exercise = (day - 1) % 7 + 1 == 6;
            if exercise {
                for _ in 0..pick.count {
                    lines.push((pick.item, 1, true));
                }
            } else {
                lines.push((pick.item, u32::from(pick.count), false));
            }
        }
    }
    if lines.is_empty() {
        return Ok(None);
    }
    let mut ids = Vec::new();
    let mut strings = Vec::new();
    let mut outputs = Vec::new();
    for (line, (source, quantity, exercise)) in lines.iter().enumerate() {
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
                stack: if *exercise || *source == 25718 {
                    ItemStackClass::NonStackable
                } else {
                    ItemStackClass::Stackable {
                        proven_maximum: Some(100),
                    }
                },
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
    for (index, (item, (source, quantity, exercise))) in strings.iter().zip(&lines).enumerate() {
        sqlx::query("INSERT INTO public.game_item_bindings(item_instance_id,world_id,character_id,cause) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,'DAILY_REWARD')").bind(item).bind(world_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).execute(&mut ***tx).await?;
        if temple > 0 && index == 0 {
            continue;
        }
        sqlx::query("INSERT INTO public.game_daily_reward_picks(claim_id,line,item_instance_id,source_id,quantity,charges) VALUES(encode($1,'hex')::uuid,$2,$3::uuid,$4,$5,$6)").bind(request.transaction_id.as_slice()).bind(index as i16).bind(item).bind(i64::from(*source)).bind(*quantity as i16).bind(if *exercise {Some(50i16)}else{None}).execute(&mut ***tx).await?;
    }
    Ok(if temple > 0 {
        strings.first().cloned()
    } else {
        None
    })
}
