//! Read-only advisory view over the exact Forge custody/rule readers; no reservation or rolls.
use super::*;
use crate::domain::{currency::Coin, item_definition::ItemDefinitionFacts};
use oteryn_gameplay_content::content::{ReferenceItemField, item_ref::ItemDefinitionIndex};
use oteryn_protocol_oteryn::forge_preview::{Cost, Resources};
#[derive(Debug, Clone, Copy)]
pub struct ForgePreviewSelection {
    pub items: ForgeResolvedItems,
    pub operation: ForgeOperation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgePreviewItem {
    pub instance: [u8; 16],
    pub definition: TypedDefinitionRef,
    pub definition_ref: u32,
    pub name: String,
    pub classification: u8,
    pub tier: u8,
    pub maximum_tier: u8,
    pub container: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgePreviewQuote {
    pub cost: Cost,
    pub target_tier: u8,
    pub success_percent: u8,
    pub protection_percent: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgePreviewSnapshot {
    pub items: Vec<ForgePreviewItem>,
    pub resources: Resources,
    pub quote: Option<ForgePreviewQuote>,
}
fn display(
    native: &NativeGameplayState,
    index: &ItemDefinitionIndex,
    id: [u8; 16],
    facts: &ItemDefinitionFacts,
    forge: Option<ForgeItem<'_>>,
    container: bool,
) -> Option<ForgePreviewItem> {
    let definition = &facts.definition;
    let record = native
        .item_policy(&definition.production_key, &definition.revision_ref)?
        .record();
    let ReferenceItemField::Known(presentation) = &record.semantics.presentation else {
        return None;
    };
    let ReferenceItemField::Known(name) = &presentation.name else {
        return None;
    };
    if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
        return None;
    }
    let (classification, tier, maximum_tier) = if let Some(item) = forge {
        (
            item.profile.classification,
            item.tier.get(),
            item.profile.max_tier,
        )
    } else if container {
        (0, 0, 0)
    } else {
        return None;
    };
    Some(ForgePreviewItem {
        instance: id,
        definition: definition.clone(),
        definition_ref: index
            .definition_ref(
                &definition.family,
                &definition.production_key,
                &definition.revision_ref,
            )?
            .get(),
        name: name.clone(),
        classification,
        tier,
        maximum_tier,
        container: forge.is_none() && container,
    })
}
impl DurabilityRoot {
    /// Current immutable quote only; the paid writer independently resolves and rechecks everything.
    pub async fn read_forge_preview(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        native: &NativeGameplayState,
        selection: Option<ForgePreviewSelection>,
    ) -> LimitResult<ForgePreviewSnapshot> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ForgeDustLimitError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = native;
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move|holder,deadline,native|{
            let native=*native;
            Box::pin(async move {
                let mut tx=begin_semantic_transaction(holder,deadline).await?;
                assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
                let Ok((world,channel))=scope_of(&fence) else{return Ok(Err(ForgeDustLimitError::AuthorityRejected));};
                if !lock_dust_limit_current(&mut tx,&node,&fence,command,*world.as_bytes(),*channel.as_bytes()).await? {return Ok(Err(ForgeDustLimitError::AuthorityRejected));}
                if current_dust_limit_content(&mut tx,*world.as_bytes(),*channel.as_bytes()).await?!=Some(native.source_digest()) {return Ok(Err(ForgeDustLimitError::MissingActiveContent));}
                let Some(inventory)=inventory::read(&mut tx,fence,native).await? else{return Ok(Err(ForgeDustLimitError::InvalidInput));};
                let Some(index)=native.item_index() else{return Ok(Err(ForgeDustLimitError::MissingActiveContent));};
                let dust=read_forge_dust(&mut tx,fence.character_id).await?;
                let Some(core)=inventory::current_definition(native,oteryn_gameplay_items::forge::EXALTED_CORE_KEY) else{return Ok(Err(ForgeDustLimitError::MissingActiveContent));};
                let Some(sliver)=inventory::current_definition(native,oteryn_gameplay_items::forge::FORGE_SLIVER_KEY) else{return Ok(Err(ForgeDustLimitError::MissingActiveContent));};
                let cores=inventory::material_count(&mut tx,fence,&inventory,&core).await?;
                let slivers=inventory::material_count(&mut tx,fence,&inventory,&sliver).await?;
                let carried_gold=inventory.items.iter().filter(|item|item.plain && item.location["kind"].as_i64()==Some(1)).try_fold(0_u64,|total,item| {
                    let worth=Coin::from_production_key(&item.facts.definition.production_key).map_or(0,Coin::worth);
                    total.checked_add(worth.checked_mul(u64::from(item.quantity))?)
                }).ok_or(DurabilityError::InvalidStoredState)?;
                let account:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                let bank:Option<i64>=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR SHARE")
                    .bind(account).bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                let bank_gold=u64::try_from(bank.unwrap_or(0)).map_err(|_|DurabilityError::InvalidStoredState)?;
                let resources=Resources {carried_gold,bank_gold,bank_spendable:!crate::durability::bank::fee_payer_is_junior(fence.character_id),dust:u32::from(dust.balance()),dust_limit:u32::from(dust.limit().get()),cores,slivers};
                let mut items=Vec::with_capacity(inventory.items.len()+1);
                if let Some(bag)=display(native,index,uuid_text(&inventory.bag_identity)?,&inventory.bag_facts,None,true){items.push(bag);}
                for item in &inventory.items {
                    if let Some(row)=display(native,index,item.id,&item.facts,item.forge(native),item.facts.container_capacity.is_some()){items.push(row);}
                }
                let quote=selection.and_then(|selection|{
                    let target=inventory.items.iter().find(|item|item.id==selection.items.first)?;
                    let source=inventory.items.iter().find(|item|item.id==selection.items.second)?;
                    let quote=ForgeQuote::try_from(ForgeRequest {operation:selection.operation,target:target.forge(native)?,source:source.forge(native)?}).ok()?;
                    let (cost,success_percent,target_tier)=quote.preview();
                    Some(ForgePreviewQuote {cost:Cost {gold:cost.gold,dust:cost.dust,cores:cost.cores},target_tier,success_percent,protection_percent:if matches!(selection.operation,ForgeOperation::Fusion {protect:true,..}){oteryn_gameplay_items::forge::FORGE_PROTECTION_PERCENT}else{0}})
                });
                commit_semantic_transaction(tx,deadline).await?;
                Ok(Ok(ForgePreviewSnapshot {items,resources,quote}))
            })
        }).await?
    }
}
