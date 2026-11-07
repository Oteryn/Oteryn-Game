//! Crystal201189f871d23597b82caf88fd611454a2ddefb8:
//! data/scripts/actions/items/{blueberry,strawberry}_cupcake.lua:4-13 and
//! lemon_cupcake.lua:1-5,10-19; three independent600s exhaustions, one-unit USE.
use super::super::character_condition_snapshot::{self, CharacterConditionSnapshot};
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::*;

pub const CUPCAKE_CAUSE: u32 = 579;
pub fn cupcake_id(key: &str) -> Option<u32> {
    match key {
        "oteryn:item.tibia.i28484" => Some(28484),
        "oteryn:item.tibia.i28485" => Some(28485),
        "oteryn:item.tibia.i28486" => Some(28486),
        _ => None,
    }
}
fn source_conditions(item_id: u32, saved: &CharacterConditionSnapshot) -> bool {
    if !saved.valid() {
        return false;
    }
    if item_id != 28486 {
        return true;
    }
    use oteryn_gameplay_domain::domain::condition::{
        ConditionValues, ConflictKey,
        attributes::{AttributeModifier, AttributeModifiers},
    };
    let Ok(store) =
        oteryn_gameplay_foundation::foundation::condition::ConditionStore::from_persistent_snapshot(
            &saved.conditions,
            0,
        )
    else {
        return false;
    };
    let Some(condition) = store.get(ConflictKey::Attributes) else {
        return false;
    };
    let modifiers = AttributeModifiers {
        distance: Some(AttributeModifier::Add(10)),
        ..AttributeModifiers::default()
    };
    condition.definition().key() == "item.seasonal.cupcake.lemon"
        && condition.definition().values()
            == ConditionValues::SourceAttributes {
                modifiers,
                duration_ms: 3_600_000,
            }
        && condition
            .remaining_duration_ms(0)
            .is_some_and(|remaining| (1..=3_600_000).contains(&remaining))
}
#[derive(Debug, Clone)]
pub struct CupcakeUseRequest {
    pub item: ItemUseRequest,
    pub wire_binding: [u8; 32],
    pub conditions: CharacterConditionSnapshot,
}
async fn receipt(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    command: u64,
    wire: [u8; 32],
) -> Result<bool> {
    let row=sqlx::query("SELECT wire_binding,character_id=encode($3,'hex')::uuid AS same_character FROM game_seasonal_cupcake_uses WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(connection).await?;
    let Some(row) = row else {
        return Ok(false);
    };
    if !row.try_get::<bool, _>("same_character")?
        || row.try_get::<Vec<u8>, _>("wire_binding")?.as_slice() != wire
    {
        return Err(ItemUseError::ConflictingCommand);
    }
    Ok(true)
}
impl DurabilityRoot {
    /// Historical bytes identify a committed use without fresh item/cooldown authority.
    pub async fn read_cupcake_use_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        command: u64,
        wire: [u8; 32],
    ) -> Result<bool> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:cupcake-command:'||encode($1,'hex')||':'||$2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string()).execute(&mut *tx).await?;
            let result=receipt(&mut tx,&fence,command,wire).await;
            if result.is_ok(){commit_semantic_transaction(tx,deadline).await?;}
            Ok(result)
        })).await?
    }
    /// Current source eligibility, native burn, immutable cooldown and condition snapshot commit together.
    pub async fn commit_cupcake_use(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: CupcakeUseRequest,
    ) -> Result<ItemUseOutcome> {
        request.item.validate()?;
        let item_id = cupcake_id(&request.item.source_definition.production_key)
            .ok_or(ItemUseError::InvalidInput)?;
        let ItemUseKind::Cupcake { origin_channel } = request.item.kind else {
            return Err(ItemUseError::InvalidInput);
        };
        if request.item.flask != ItemUseFlaskPlan::None
            || !source_conditions(item_id, &request.conditions)
            || request.conditions.lease_generation != fence.character_lease_generation
        {
            return Err(ItemUseError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_type2_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:cupcake-command:'||encode($1,'hex')||':'||$2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.item.command_id.to_string()).execute(&mut **tx).await?;
            match receipt(&mut tx,&fence,request.item.command_id,request.wire_binding).await{
                Ok(true)=>{let exact:bool=sqlx::query_scalar("SELECT conditions_sha256=sha256(convert_to($3::jsonb::text,'UTF8')) FROM game_seasonal_cupcake_uses WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric").bind(fence.game_session_id.as_bytes().as_slice()).bind(request.item.command_id.to_string()).bind(&request.conditions.conditions).fetch_one(&mut **tx).await?;if !exact{return Ok(Err(ItemUseError::ConflictingCommand));}let existing=match replay(&mut tx,&fence,&request.item).await { Ok(Some(existing))=>existing,Ok(None)=>return Err(DurabilityError::InvalidStoredState),Err(ItemUseError::Unavailable(error))=>return Err(error),Err(error)=>return Ok(Err(error)) };tx.commit(deadline).await?;return Ok(Ok(existing));},Ok(false)=>{},Err(ItemUseError::Unavailable(error))=>return Err(error),Err(error)=>return Ok(Err(error)),
            }
            let current=match fence.runtime_scope{
                crate::foundation::RuntimeScopeRefV1::Instance{..}=>super::super::house_hireling::fence::assert_house_custody_fence(&mut tx,&node,&fence,origin_channel).await?.is_some(),
                _=>super::super::house_hireling::fence::assert_inventory_fence(&mut tx,&node,&fence,origin_channel).await?,
            };
            if !current{return Ok(Err(ItemUseError::AuthorityRejected));}
            let revisions=sqlx::query("SELECT content_revision,ruleset_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if revisions.try_get::<String,_>("content_revision")?!=request.item.content_revision || revisions.try_get::<String,_>("ruleset_revision")?!=request.item.ruleset_revision{return Ok(Err(ItemUseError::StaleState));}
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:cupcake:'||encode($1,'hex')||':'||$2,0))")
                .bind(fence.character_id.as_bytes().as_slice()).bind(item_id.to_string()).execute(&mut **tx).await?;
            let cooling:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_seasonal_cupcake_uses WHERE character_id=encode($1,'hex')::uuid AND item_id=$2 AND used_at+interval '600 seconds'>public.game_daily_effect_now())")
                .bind(fence.character_id.as_bytes().as_slice()).bind(i64::from(item_id)).fetch_one(&mut **tx).await?;
            if cooling{return Ok(Err(ItemUseError::Cooldown));}
            let tuple=tx.tuple();let outcome=match write(&mut tx,&fence,&request.item,tuple).await{
                Ok(ItemUseOutcome::Written{shape})=>ItemUseOutcome::Written{shape},Ok(_)=>return Err(DurabilityError::InvalidStoredState),Err(ItemUseError::Unavailable(error))=>return Err(error),Err(error)=>return Ok(Err(error)),
            };
            sqlx::query("INSERT INTO game_seasonal_cupcake_uses(transaction_id,source_kind,use_transaction_id,character_id,game_session_id,command_id,wire_binding,item_id,used_at,conditions_sha256) VALUES(encode($1,'hex')::uuid,1,encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,public.game_daily_effect_now(),sha256(convert_to($7::jsonb::text,'UTF8')))")
                .bind(request.item.transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.item.command_id.to_string()).bind(request.wire_binding.as_slice()).bind(i64::from(item_id)).bind(&request.conditions.conditions).execute(&mut **tx).await?;
            character_condition_snapshot::write_in_transaction(&mut tx,&fence,&request.conditions).await?;
            if item_id==28486{
                // FORCEUPDATE replaces the default attribute family; preserve unrelated haste and its cooldown.
                sqlx::query("UPDATE game_hireling_gourmet_state SET attributes_item=NULL,attributes_ms=0 WHERE character_id=encode($1,'hex')::uuid AND attributes_item IS NOT NULL")
                    .bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?;
            }
            tx.commit(deadline).await?;Ok(Ok(outcome))
        })).await?
    }
}

/// Source payload retained by the existing Ground/House prepared effect owner.
#[derive(Debug, Clone)]
pub struct CupcakeTransformContext {
    pub item_id: u32,
    pub wire_binding: [u8; 32],
    pub conditions: CharacterConditionSnapshot,
}
pub(crate) async fn check_transform_cooldown(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &crate::durability::consume_transform::ConsumeTransformRequest,
    context: &CupcakeTransformContext,
) -> std::result::Result<bool, DurabilityError> {
    if !matches!(context.item_id, 28484..=28486)
        || !source_conditions(context.item_id, &context.conditions)
        || context.conditions.lease_generation != fence.character_lease_generation
        || request.command.game_session_id() != fence.game_session_id
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:cupcake:'||encode($1,'hex')||':'||$2,0))").bind(fence.character_id.as_bytes().as_slice()).bind(context.item_id.to_string()).execute(&mut *connection).await?;
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_seasonal_cupcake_uses WHERE character_id=encode($1,'hex')::uuid AND item_id=$2 AND used_at+interval '600 seconds'>public.game_daily_effect_now())").bind(fence.character_id.as_bytes().as_slice()).bind(i64::from(context.item_id)).fetch_one(&mut *connection).await?;
    Ok(!active)
}
pub(crate) async fn insert_transform(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &crate::durability::consume_transform::ConsumeTransformRequest,
    context: &CupcakeTransformContext,
) -> std::result::Result<(), DurabilityError> {
    let row=sqlx::query("INSERT INTO game_seasonal_cupcake_uses(transaction_id,source_kind,transform_transaction_id,character_id,game_session_id,command_id,wire_binding,item_id,used_at,conditions_sha256) SELECT transaction_id,2,transaction_id,character_id,game_session_id,command_id,$4,$5,public.game_daily_effect_now(),sha256(convert_to($6::jsonb::text,'UTF8')) FROM game_item_consume_transform_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND character_id=encode($3,'hex')::uuid AND source_item_instance_id=encode($7,'hex')::uuid AND created_xact_id=pg_current_xact_id() AND consume_source AND source_count=1 AND before_item->>'definition_production_key'='oteryn:item.tibia.i'||$5::text AND recipe='oteryn:seasonal.cupcake.'||$5::text AND outputs='[]'::jsonb RETURNING transaction_id")
 .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(context.wire_binding.as_slice()).bind(i64::from(context.item_id)).bind(&context.conditions.conditions).bind(request.source.as_slice()).fetch_optional(&mut *connection).await?;
    if row.is_none() {
        return Err(DurabilityError::InvalidStoredState);
    }
    character_condition_snapshot::write_in_transaction(connection, fence, &context.conditions)
        .await?;
    if context.item_id == 28486 {
        sqlx::query("UPDATE game_hireling_gourmet_state SET attributes_item=NULL,attributes_ms=0 WHERE character_id=encode($1,'hex')::uuid AND attributes_item IS NOT NULL").bind(fence.character_id.as_bytes().as_slice()).execute(&mut *connection).await?;
    }
    Ok(())
}
