//! Exact royal base-purchase reward on the mature native Inbox mint owner.
use crate::durability::character_inbox::{
    RewardClaimInboxMintCandidate, RewardClaimInboxMintOutput, RewardClaimInboxMintRequest,
    RewardClaimInboxPlan,
};
use crate::durability::item_mint::{TypedDefinitionRef, uuid_text};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
use crate::durability::{DurabilityError, npc_cosmetics::NpcProgramContext};
use sha2::{Digest, Sha256};

pub(crate) async fn mint_royal_kit(
    tx: &mut Type2Transaction<'_>,
    ctx: &NpcProgramContext<'_>,
) -> Result<[u8; 16], DurabilityError> {
    let source_row = match (ctx.npc_key, ctx.source_key) {
        ("oteryn:npc.king_tibianus", "oteryn:npc-cosmetic/king_tibianus/golden-buy-armor") => {
            2_000_000
        }
        ("oteryn:npc.queen_eloise", "oteryn:npc-cosmetic/queen_eloise/golden-buy-armor") => {
            2_000_001
        }
        ("oteryn:npc.emperor_kruzak", "oteryn:npc-cosmetic/emperor_kruzak/golden-buy-armor") => {
            2_000_002
        }
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let fee = ctx
        .fee_transaction_id
        .ok_or(DurabilityError::InvalidStoredState)?;
    let source_current:bool=sqlx::query_scalar("SELECT game_npc_source_parent_current(encode($1,'hex')::uuid,$2) AND EXISTS(SELECT 1 FROM game_npc_cosmetic_receipts n JOIN game_item_fee_burns f ON f.transaction_id=n.fee_transaction_id WHERE n.transaction_id=encode($1,'hex')::uuid AND n.source_key=$2 AND n.npc_key=$3 AND n.fee_transaction_id=encode($4,'hex')::uuid AND n.program_sha256=$5 AND n.plan_sha256=$6 AND f.character_id=n.character_id AND f.fee_gold_units=500000000 AND f.created_xact_id=pg_current_xact_id())")
        .bind(ctx.npc_transaction.as_slice()).bind(ctx.source_key).bind(ctx.npc_key).bind(fee.as_slice()).bind(ctx.program_sha256.as_slice()).bind(ctx.plan_sha256.as_slice()).fetch_one(&mut ***tx).await?;
    if !source_current {
        return Err(DurabilityError::InvalidStoredState);
    }
    let slot: bool =
        sqlx::query_scalar("SELECT game_npc_royal_inbox_admit(encode($1,'hex')::uuid,$2)")
            .bind(ctx.npc_transaction.as_slice())
            .bind(ctx.source_key)
            .fetch_one(&mut ***tx)
            .await?;
    if !slot {
        return Err(DurabilityError::InvalidStoredState);
    }
    let policy = ctx
        .native
        .item_policy_for_source_id(23398)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let def = &policy.record().production_definition;
    if !policy.record().admission.materializable
        || def.family != "Item"
        || def.production_key != "oteryn:item.tibia.i23398"
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let (mint, item): (String, String) =
        sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text")
            .fetch_one(&mut ***tx)
            .await?;
    let mint = uuid_text(&mint)?;
    let item_id = uuid_text(&item)?;
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = ctx.fence.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let (content,ruleset):(String,String)=sqlx::query_as("SELECT content_revision,ruleset_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(ctx.fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let binding:[u8;32]=Sha256::digest(serde_json::to_vec(&serde_json::json!({"parent":ctx.npc_transaction,"source":ctx.source_key,"program":ctx.program_sha256,"plan":ctx.plan_sha256,"kit":23398,"target":31510,"quantity":1})).map_err(|_|DurabilityError::InvalidStoredState)?).into();
    RewardClaimInboxMintCandidate::mint_atomic_in_transaction(
        tx,
        ctx.fence.item_fence(),
        RewardClaimInboxPlan {
            command: ctx.command,
            character_id: ctx.fence.character_id,
            world_id,
            channel_id,
            transaction_id: mint,
            source_binding: binding,
            item_instance_ids: vec![item_id],
        },
        RewardClaimInboxMintRequest {
            claim: TypedDefinitionRef {
                family: "NpcCosmetic".into(),
                production_key: ctx.source_key.into(),
                revision_ref: ctx
                    .program_sha256
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect(),
            },
            content_revision: content,
            ruleset_revision: ruleset,
            sim_revision: "npc-source-program-v1".into(),
            outputs: vec![RewardClaimInboxMintOutput {
                source_line: 0,
                item: ItemDefinitionFacts {
                    weight: None,
                    definition: TypedDefinitionRef {
                        family: def.family.clone(),
                        production_key: def.production_key.clone(),
                        revision_ref: def.revision_ref.clone(),
                    },
                    stack: ItemStackClass::NonStackable,
                    container_capacity: None,
                    container_slot_equip_pattern: false,
                },
                quantity: 1,
                key_number: None,
            }],
        },
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_npc_royal_kit_mints(parent_transaction_id,fee_transaction_id,mint_transaction_id,item_instance_id,source_row,source_key,program_sha256,plan_sha256,target_key) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8,'oteryn:item.tibia.i31510')").bind(ctx.npc_transaction.as_slice()).bind(fee.as_slice()).bind(mint.as_slice()).bind(item_id.as_slice()).bind(source_row).bind(ctx.source_key).bind(ctx.program_sha256.as_slice()).bind(ctx.plan_sha256.as_slice()).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_house_furniture_kit_origins(item_instance_id,mint_transaction_id,source_row,target_key,target_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'oteryn:item.tibia.i31510',$4)").bind(item_id.as_slice()).bind(mint.as_slice()).bind(source_row).bind(&def.revision_ref).execute(&mut ***tx).await?;
    Ok(item_id)
}

// Append inside the existing closed Royal kit owner after its real Look baseline RED.
impl crate::durability::DurabilityRoot {
    /// Description of the still-wrapped, immutable Royal base-purchase reward.
    /// Availability/visibility remains with the existing Look observation owner.
    pub async fn royal_furniture_kit_description(
        &self,
        authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
        item: [u8; 16],
    ) -> Result<Option<String>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=crate::durability::db::begin_semantic_transaction(holder,deadline).await?;
            crate::durability::character_authority::assert_recovery_fence(&mut tx,&recovery).await?;
            let royal:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_house_furniture_kit_origins o ON o.item_instance_id=i.item_instance_id JOIN game_npc_royal_kit_mints k ON k.item_instance_id=i.item_instance_id JOIN game_npc_cosmetic_receipts p ON p.transaction_id=k.parent_transaction_id JOIN game_item_fee_burns f ON f.transaction_id=k.fee_transaction_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i23398' AND NOT EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts wrapped WHERE wrapped.item_instance_id=i.item_instance_id AND wrapped.action=1) AND o.source_row BETWEEN 2000000 AND 2000002 AND o.source_row=k.source_row AND o.target_key='oteryn:item.tibia.i31510' AND k.target_key=o.target_key AND o.mint_transaction_id=k.mint_transaction_id AND i.minted_transaction_id=k.mint_transaction_id AND p.source_key=k.source_key AND p.program_sha256=k.program_sha256 AND p.plan_sha256=k.plan_sha256 AND p.fee_transaction_id=f.transaction_id AND f.fee_gold_units=500000000 AND f.cause_kind=474 AND ((p.npc_key='oteryn:npc.king_tibianus' AND k.source_row=2000000 AND p.source_key='oteryn:npc-cosmetic/king_tibianus/golden-buy-armor') OR(p.npc_key='oteryn:npc.queen_eloise' AND k.source_row=2000001 AND p.source_key='oteryn:npc-cosmetic/queen_eloise/golden-buy-armor') OR(p.npc_key='oteryn:npc.emperor_kruzak' AND k.source_row=2000002 AND p.source_key='oteryn:npc-cosmetic/emperor_kruzak/golden-buy-armor')))").bind(item.as_slice()).fetch_one(&mut *tx).await?;
            crate::durability::db::commit_semantic_transaction(tx,deadline).await?;
            // Crystal King137–139; Queen/Emperor's legacy kit encoding is
            // normalized to the same working Royal reward, as recorded for MINT.
            Ok(royal.then(||"Unwrap it in your own house to create a golden outfit display.".to_owned()))
        })).await
    }
}
