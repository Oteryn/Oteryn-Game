//! Native opaque Wrap delegates to the existing complete House wrap owner.
use super::*;
use oteryn_gameplay_content::content::ReferenceItemField as F;
use oteryn_protocol_oteryn::house_wrap as wire;

type PreviousWrapReceipt = (Option<Vec<u8>>, Vec<u8>, Vec<u8>, String, String);

pub struct OpaqueHouseWrapResult {
    pub reply: wire::Reply,
    pub fresh: bool,
}
impl DurabilityRoot {
    /// The current native owner supplies context and the definition it issued.
    /// Original opaque intent is retained before the handle can retire.
    #[allow(
        clippy::too_many_arguments,
        reason = "Node, House fence, command binding and observed definition are independent proofs"
    )]
    pub async fn wrap_house_furniture_opaque(
        &self,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
        command: crate::foundation::CommandRef,
        intent: Vec<u8>,
        source: Option<[u8; 16]>,
        observed: TypedDefinitionRef,
        native: &NativeGameplayState,
        prepared: Option<(u64, [i32; 3])>,
    ) -> Result<OpaqueHouseWrapResult, DurabilityError> {
        if command.game_session_id() != context.fence.game_session_id
            || intent.len() != 13
            || !matches!(intent[0], 3 | 4)
        {
            return Ok(OpaqueHouseWrapResult {
                reply: wire::Reply::Refused(wire::Refusal::Stale),
                fresh: false,
            });
        }
        let carried = intent[0] == 4;
        let node = node.clone();
        let mut invocation = (native, false);
        let reply=self.try_issue_semantic_pass()?.run_with_context(&mut invocation,move|holder,deadline,invocation|Box::pin(async move {
            let (native,fresh)=invocation;
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            crate::durability::db::lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx,&node).await? || !fenced(CURRENT,&node,&context)
                .fetch_one(&mut **tx).await?.try_get::<bool,_>(0)? {
                return Ok(wire::Reply::Refused(wire::Refusal::Stale));
            }
            // Replay is independent of the current opaque handle/observed item.
            let previous:Option<PreviousWrapReceipt>=sqlx::query_as(
                "SELECT use_intent,uuid_send(character_id),uuid_send(world_id),house_key,((before_item->>'state_revision')::numeric+1)::text FROM game_house_furniture_wrap_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(context.fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .fetch_optional(&mut **tx).await?;
            if let Some((original,character,world,house,revision))=previous {
                let reply=if original.as_deref()==Some(intent.as_slice()) && character==context.fence.character_id.as_bytes()
                    && world==context.house.world_id().as_bytes() && house==context.house.house_key() {
                    wire::Reply::Committed(revision.parse().map_err(|_|DurabilityError::InvalidStoredState)?)
                } else { wire::Reply::Refused(wire::Refusal::Stale) };
                tx.commit(deadline).await?;
                return Ok(reply);
            }
            let Some(source)=source else{return Ok(wire::Reply::Refused(wire::Refusal::Stale))};
            let Some(row)=fenced(WRAP_SOURCE,&node,&context).bind(source.to_vec()).fetch_optional(&mut **tx).await? else {
                return Ok(wire::Reply::Refused(wire::Refusal::Stale));
            };
            let before:Value=row.try_get("before_item")?;
            let position:Option<Vec<u8>>=row.try_get("spatial_position")?;
            let spatial=position.and_then(|v|HouseActionContext::decode_spatial_position(&v));
            let position=match (carried,spatial) {
                (true,None)=>context.position,
                (false,Some(position))=>position,
                _=>return Ok(wire::Reply::Refused(wire::Refusal::NotWrapable)),
            };
            if prepared != Some((before.get("state_revision").and_then(Value::as_u64).ok_or(DurabilityError::InvalidStoredState)?,position)) {
                return Ok(wire::Reply::Refused(wire::Refusal::Stale));
            }
            if position[2]!=context.position[2] || position[0].abs_diff(context.position[0])>1 || position[1].abs_diff(context.position[1])>1 {
                return Ok(wire::Reply::Refused(wire::Refusal::Blocked));
            }
            if before.get("definition_family").and_then(Value::as_str)!=Some(observed.family.as_str())
                || before.get("definition_production_key").and_then(Value::as_str)!=Some(observed.production_key.as_str())
                || before.get("definition_revision_ref").and_then(Value::as_str)!=Some(observed.revision_ref.as_str()) {
                return Ok(wire::Reply::Refused(wire::Refusal::Stale));
            }
            let Some(id)=observed.production_key.strip_prefix("oteryn:item.tibia.i").and_then(|id|id.parse::<u32>().ok()) else {
                return Ok(wire::Reply::Refused(wire::Refusal::NotWrapable));
            };
            if carried || before.get("quantity").and_then(Value::as_u64).is_some_and(|quantity| quantity > 1) {
                let Some(policy)=native.item_policy(&observed.production_key,&observed.revision_ref) else {return Ok(wire::Reply::Refused(wire::Refusal::NotWrapable))};
                let F::Known(physical)=&policy.record().semantics.physical else{return Ok(wire::Reply::Refused(wire::Refusal::NotWrapable))};
                if physical.pickupable!=F::Known(true){return Ok(wire::Reply::Refused(wire::Refusal::NotWrapable))}
                let quantity=before.get("quantity").and_then(Value::as_u64).ok_or(DurabilityError::InvalidStoredState)?;
                let F::Known(stack)=&policy.record().semantics.stack else{return Ok(wire::Reply::Refused(wire::Refusal::NotWrapable))};
                let maximum=match (&stack.stackable,&stack.stack_max){(F::Known(true),F::Known(maximum))=>u64::from(*maximum),(F::Known(false),_)=>1,_=>0};
                if quantity==0 || quantity>maximum{return Ok(wire::Reply::Refused(wire::Refusal::NotWrapable))}
            }
            let target:Option<i32>=sqlx::query_scalar("SELECT result_item FROM game_house_furniture_rules WHERE action=1 AND $1 BETWEEN first_item AND last_item").bind(i32::try_from(id).map_err(|_|DurabilityError::InvalidStoredState)?).fetch_optional(&mut **tx).await?;
            let Some(target)=target else {
                return Ok(wire::Reply::Refused(wire::Refusal::NotWrapable));
            };
            let target=format!("oteryn:item.tibia.i{target}");
            if native.item_policy(&observed.production_key,&observed.revision_ref).is_none_or(|p|!p.record().admission.materializable)
                || native.item_policy(&target,"definition-r1").is_none_or(|p|!p.record().admission.materializable) {
                return Ok(wire::Reply::Refused(wire::Refusal::NotWrapable));
            }
            let acl:String=sqlx::query_scalar("SELECT acl_revision::text FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
                .bind(context.house.world_id().as_bytes().as_slice()).bind(context.house.house_key()).fetch_one(&mut **tx).await?;
            let backpack:Option<String>=sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_container_slots b JOIN game_item_instances i USING(item_instance_id,world_id) WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1'")
                .bind(context.fence.character_id.as_bytes().as_slice()).bind(context.house.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            let capacity=backpack.as_ref().and_then(|key|native.item_policy(key,"definition-r1")).and_then(|p|match &p.record().semantics.container {
                F::Known(c)=>match c.capacity {F::Known(n)=>Some(i32::from(n)),_=>None},_=>None
            });
            let request=wire::Request{action:wire::Action::Wrap,item:source,
                expected_revision:before.get("state_revision").and_then(Value::as_u64).ok_or(DurabilityError::InvalidStoredState)?,
                acl_revision:acl.parse().map_err(|_|DurabilityError::InvalidStoredState)?,position};
            let revision=fenced("SELECT game_house_furniture_construct(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9,$10::text::numeric,encode($11,'hex')::uuid,$12::text::numeric,$13::text::numeric,$14::smallint,$15,$16,$17,$18,$19,$20)::text",&node,&context)
                .bind(command.command_id().get().to_string()).bind(source.to_vec()).bind(request.expected_revision.to_string())
                .bind(request.acl_revision.to_string()).bind(1_i16).bind(position.iter().flat_map(|n|n.to_be_bytes()).collect::<Vec<_>>())
                .bind(request.encode()).bind(target).bind(capacity).bind(backpack).bind(intent).fetch_one(&mut **tx).await;
            let revision:String=match revision {
                Ok(row)=>row.try_get(0)?,
                Err(error)=>return match error.as_database_error().map(|e|e.message()) {
                    Some("PODIUM_STALE_SESSION"|"WRAP_STALE")=>Ok(wire::Reply::Refused(wire::Refusal::Stale)),
                    Some("WRAP_NO_ACCESS")=>Ok(wire::Reply::Refused(wire::Refusal::NoAccess)),
                    Some("WRAP_NOT_WRAPABLE")=>Ok(wire::Reply::Refused(wire::Refusal::NotWrapable)),
                    Some("WRAP_BLOCKED")=>Ok(wire::Reply::Refused(wire::Refusal::Blocked)),
                    Some("WRAP_INVENTORY_FULL")=>Ok(wire::Reply::Refused(wire::Refusal::InventoryFull)),
                    _=>Err(error.into()),
                }
            };
            let transaction:String=sqlx::query_scalar("SELECT transaction_id::text FROM game_house_furniture_wrap_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(context.fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_one(&mut **tx).await?;
            sqlx::query("SELECT game_house_furniture_timer_apply($1::uuid)").bind(transaction).execute(&mut **tx).await?;
            let revision=revision.parse().map_err(|_|DurabilityError::InvalidStoredState)?;
            tx.commit(deadline).await?;
            *fresh=true;
            Ok(wire::Reply::Committed(revision))
        })).await?;
        Ok(OpaqueHouseWrapResult {
            reply,
            fresh: invocation.1,
        })
    }
}

const WRAP_SOURCE: &str = "SELECT * FROM game_house_furniture_wrap_source(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9,encode($10,'hex')::uuid)";
pub struct HouseWrapSourcePreview {
    pub definition: TypedDefinitionRef,
    pub state_revision: u64,
    pub position: [i32; 3],
    pub placed: bool,
}
impl DurabilityRoot {
    pub async fn read_house_furniture_wrap_source(
        &self,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
        item: [u8; 16],
    ) -> Result<Option<HouseWrapSourcePreview>, DurabilityError> {
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        crate::durability::db::begin_semantic_transaction(holder, deadline).await?;
                    if !prove_current_incarnation(&mut tx, &node).await?
                        || !fenced(CURRENT, &node, &context)
                            .fetch_one(&mut *tx)
                            .await?
                            .try_get::<bool, _>(0)?
                    {
                        return Ok(None);
                    }
                    let Some(row) = fenced(WRAP_SOURCE, &node, &context)
                        .bind(item.to_vec())
                        .fetch_optional(&mut *tx)
                        .await?
                    else {
                        return Ok(None);
                    };
                    let before: Value = row.try_get("before_item")?;
                    let position: Option<Vec<u8>> = row.try_get("spatial_position")?;
                    let placed = position.is_some();
                    let position = position
                        .as_ref()
                        .map(|p| {
                            HouseActionContext::decode_spatial_position(p)
                                .ok_or(DurabilityError::InvalidStoredState)
                        })
                        .transpose()?
                        .unwrap_or(context.position);
                    let source = HouseWrapSourcePreview {
                        definition: TypedDefinitionRef {
                            family: before
                                .get("definition_family")
                                .and_then(Value::as_str)
                                .ok_or(DurabilityError::InvalidStoredState)?
                                .into(),
                            production_key: before
                                .get("definition_production_key")
                                .and_then(Value::as_str)
                                .ok_or(DurabilityError::InvalidStoredState)?
                                .into(),
                            revision_ref: before
                                .get("definition_revision_ref")
                                .and_then(Value::as_str)
                                .ok_or(DurabilityError::InvalidStoredState)?
                                .into(),
                        },
                        state_revision: before
                            .get("state_revision")
                            .and_then(Value::as_u64)
                            .ok_or(DurabilityError::InvalidStoredState)?,
                        position,
                        placed,
                    };
                    crate::durability::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(source))
                })
            })
            .await
    }
}

impl DurabilityRoot {
    /// Advisory only. No command receipt, Type2 write or reservation is made.
    pub async fn house_furniture_wrap_affordance(
        &self,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
        source: [u8; 16],
        observed: TypedDefinitionRef,
        native: &NativeGameplayState,
    ) -> Result<bool, DurabilityError> {
        let node = node.clone();
        let mut native = native;
        self.try_issue_semantic_pass()?.run_with_context(&mut native,move|holder,deadline,native|Box::pin(async move{
            let mut tx=crate::durability::db::begin_semantic_transaction(holder,deadline).await?;
            if !prove_current_incarnation(&mut tx,&node).await? || !fenced(CURRENT,&node,&context).fetch_one(&mut *tx).await?.try_get::<bool,_>(0)? {return Ok(false)}
            let role:Option<String>=sqlx::query_scalar("SELECT role FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid) WHERE NOT content_fenced").bind(context.house.world_id().as_bytes().as_slice()).bind(context.house.house_key()).bind(context.fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            if !matches!(role.as_deref(),Some("OWNER"|"SUBOWNER")){return Ok(false)}
            let Some(row)=fenced(WRAP_SOURCE,&node,&context).bind(source.to_vec()).fetch_optional(&mut *tx).await? else{return Ok(false)};
            let position:Option<Vec<u8>>=row.try_get("spatial_position")?;
            if position.is_some(){return Ok(false)}
            let before:Value=row.try_get("before_item")?;
            let location:Value=row.try_get("location")?;
            if !matches!(location.get("custody_kind").and_then(Value::as_str),Some("container"|"nested")){return Ok(false)}
            if before.get("definition_family").and_then(Value::as_str)!=Some(observed.family.as_str()) || before.get("definition_production_key").and_then(Value::as_str)!=Some(observed.production_key.as_str()) || before.get("definition_revision_ref").and_then(Value::as_str)!=Some(observed.revision_ref.as_str()){return Ok(false)}
            let Some(policy)=native.item_policy(&observed.production_key,&observed.revision_ref) else{return Ok(false)};
            if !policy.record().admission.materializable{return Ok(false)}
            let F::Known(physical)=&policy.record().semantics.physical else{return Ok(false)};
            if physical.pickupable!=F::Known(true){return Ok(false)}
            let F::Known(stack)=&policy.record().semantics.stack else{return Ok(false)};
            let maximum=match(&stack.stackable,&stack.stack_max){(F::Known(true),F::Known(max))=>u64::from(*max),(F::Known(false),_)=>1,_=>return Ok(false)};
            let quantity=before.get("quantity").and_then(Value::as_u64).ok_or(DurabilityError::InvalidStoredState)?;
            if quantity==0 || quantity>maximum{return Ok(false)}
            let qualified:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_furniture_portable_wrap_sources p JOIN game_house_furniture_rules r ON r.action=1 AND p.item_id BETWEEN r.first_item AND r.last_item WHERE $1='oteryn:item.tibia.i'||p.item_id::text) AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid) AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid) AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid) AND NOT EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=encode($2,'hex')::uuid) AND NOT EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=encode($2,'hex')::uuid) AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=encode($2,'hex')::uuid AND charges IS NOT NULL)").bind(&observed.production_key).bind(source.as_slice()).fetch_one(&mut *tx).await?;
            crate::durability::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(qualified)
        })).await
    }
}
