//! Native cmd9 adapts the existing House writer; snapshots are never authority.
use super::*;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
#[derive(Debug, Clone, Copy)]
pub enum NativeHouseTarget {
    Backpack,
    Position([i32; 3]),
}
/// Read-only input descriptors; every Root call reproves the live named-House fence.
#[derive(Clone)]
pub struct NativeHouseMoveContext {
    pub fence: CurrentCharacterItemFence,
    pub origin_channel: [u8; 16],
    pub house: HouseId,
    pub command: CommandRef,
}
pub struct NativeHousePrepared {
    pub request: HouseItemRequest,
    pub source_position: Option<[i32; 3]>,
}
pub enum NativeHouseRecovery {
    Absent,
    Pending(Box<HouseItemCandidate>),
    Committed(HouseItemReceipt),
}
impl HouseItemCandidate {
    pub fn native_request(&self) -> &HouseItemRequest {
        &self.frozen.request
    }
}
fn bounded_wire(raw: &[u8]) -> WriterResult<()> {
    if raw.is_empty()
        || raw.len() > oteryn_protocol_oteryn::item_view::MAX_ITEM_MOVE_INTENT_STACK_COUNT_BYTES
    {
        return Err(HouseWriterError::InvalidInput);
    }
    Ok(())
}
pub(super) fn with_wire(mut p: Value, wire: Option<&[u8]>) -> WriterResult<Value> {
    if let Some(wire) = wire {
        bounded_wire(wire)?;
        p["native_move"] = hex(wire).into();
    }
    Ok(p)
}
fn text<'a>(p: &'a Value, key: &str) -> WriterResult<&'a str> {
    p[key].as_str().ok_or(HouseWriterError::InvalidInput)
}
fn integer(p: &Value, key: &str) -> WriterResult<u64> {
    text(p, key)?
        .parse()
        .map_err(|_| HouseWriterError::InvalidInput)
}
fn bytes<const N: usize>(p: &Value, key: &str) -> WriterResult<[u8; N]> {
    let raw = text(p, key)?;
    if raw.len() != N * 2 || !raw.is_ascii() {
        return Err(HouseWriterError::InvalidInput);
    }
    let mut result = [0; N];
    for (i, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&raw[i * 2..i * 2 + 2], 16)
            .map_err(|_| HouseWriterError::InvalidInput)?;
    }
    Ok(result)
}
fn position(p: &Value, key: &str) -> WriterResult<Vec<u8>> {
    Ok(bytes::<12>(p, key)?.to_vec())
}
fn request_from_context(
    p: &Value,
    fence: CurrentCharacterItemFence,
    house: HouseId,
    command: CommandRef,
) -> WriterResult<HouseItemRequest> {
    if text(p, "session")? != hex(command.game_session_id().as_bytes())
        || integer(p, "command")? != command.command_id().get()
        || text(p, "character")? != hex(fence.character_id.as_bytes())
        || text(p, "world")? != hex(house.world_id().as_bytes())
        || text(p, "house")? != house.house_key()
    {
        return Err(HouseWriterError::ConflictingOccurrence);
    }
    let shape = p["shape"].as_i64().ok_or(HouseWriterError::InvalidInput)?;
    let action = match shape {
        5 => HouseItemAction::Place {
            source: match p["source_kind"].as_i64() {
                Some(1) => HouseCharacterSource::Inventory {
                    parent: bytes(p, "source_parent")?,
                    ordinal: integer(p, "source_ordinal")?,
                },
                Some(2) => HouseCharacterSource::ContainerSlot,
                Some(3) => HouseCharacterSource::Equipment {
                    slot: u8::try_from(
                        p["source_slot"]
                            .as_u64()
                            .ok_or(HouseWriterError::InvalidInput)?,
                    )
                    .map_err(|_| HouseWriterError::InvalidInput)?,
                },
                _ => return Err(HouseWriterError::InvalidInput),
            },
            position: position(p, "destination_position")?,
        },
        6 => HouseItemAction::Take {
            placement: bytes(p, "source_placement")?,
            provenance_revision: integer(p, "source_provenance_revision")?,
        },
        7 => HouseItemAction::TileMove {
            placement: bytes(p, "source_placement")?,
            provenance_revision: integer(p, "source_provenance_revision")?,
            position: position(p, "destination_position")?,
        },
        _ => return Err(HouseWriterError::InvalidInput),
    };
    let backpack = if shape == 6 {
        Some(HouseBackpackFacts {
            definition: TypedDefinitionRef {
                family: text(p, "backpack_family")?.into(),
                production_key: text(p, "backpack_key")?.into(),
                revision_ref: text(p, "backpack_revision")?.into(),
            },
            capacity: u32::try_from(
                p["backpack_capacity"]
                    .as_u64()
                    .ok_or(HouseWriterError::InvalidInput)?,
            )
            .map_err(|_| HouseWriterError::InvalidInput)?,
        })
    } else {
        None
    };
    Ok(HouseItemRequest {
        command,
        house,
        item_instance_id: bytes(p, "item")?,
        action,
        expected_acl_revision: integer(p, "acl_revision")?,
        expected_guild_revisions: p.get("guild_revisions").filter(|v| !v.is_null()).cloned(),
        backpack,
        content_revision: text(p, "content_revision")?.into(),
        ruleset_revision: text(p, "ruleset_revision")?.into(),
        sim_revision: text(p, "sim_revision")?.into(),
    })
}
impl DurabilityRoot {
    pub async fn freeze_native_house_item(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: HouseItemRequest,
        wire: &[u8],
    ) -> WriterResult<HouseItemCandidate> {
        bounded_wire(wire)?;
        self.freeze_house_item_inner(authority, node, fence, request, Some(wire.to_vec()))
            .await
    }
    pub async fn recover_native_house_item(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        context: NativeHouseMoveContext,
        wire: &[u8],
    ) -> WriterResult<NativeHouseRecovery> {
        let NativeHouseMoveContext {
            fence,
            house,
            command,
            ..
        } = context;
        bounded_wire(wire)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseWriterError::NotCurrent)?;
        let node = node.clone();
        let wire = wire.to_vec();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;cause_lock(&mut tx,command).await?;
   let row=sqlx::query("SELECT transaction_id::text,event_id::text,character_id::text,occurred_at,intent_binding,house_intent FROM game_item_transfer_reservations WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND source_kind>0 FOR UPDATE")
    .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
   let Some(row)=row else {return Ok(Ok(NativeHouseRecovery::Absent))};let p:Value=row.try_get("house_intent")?;
   if p.get("native_move").and_then(Value::as_str)!=Some(hex(&wire).as_str()){return Ok(Err(HouseWriterError::ConflictingOccurrence))}
   let request=match request_from_context(&p,fence,house,command){Ok(request)=>request,Err(e)=>return Ok(Err(e))};
   let binding:Vec<u8>=row.try_get("intent_binding")?;if binding!=intent_binding(&p){return Err(DurabilityError::InvalidStoredState)}
   let result=match receipt(&mut tx,command,&binding).await? {
    Ok(Some(r))=>NativeHouseRecovery::Committed(r),Err(e)=>return Ok(Err(e)),Ok(None)=>{if !named_house_current(&mut tx,&node,fence,&request).await? {return Ok(Err(HouseWriterError::NotCurrent))} NativeHouseRecovery::Pending(Box::new(HouseItemCandidate{frozen:FrozenHouseItem{request,character:fence.character_id,binding,transaction:uuid_text(row.try_get("transaction_id")?)?,event:uuid_text(row.try_get("event_id")?)?,occurred_at:row.try_get("occurred_at")?,native_intent:Some(wire)}}))},
   };commit_semantic_transaction(tx,deadline).await?;Ok(Ok(result))
  })).await?
    }
    pub async fn prepare_native_house_item(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        context: NativeHouseMoveContext,
        item: [u8; 16],
        target: NativeHouseTarget,
        native: &NativeGameplayState,
    ) -> WriterResult<Option<NativeHousePrepared>> {
        let NativeHouseMoveContext {
            fence,
            origin_channel,
            house,
            command,
        } = context;
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseWriterError::NotCurrent)?;
        let node = node.clone();
        let mut native = native;
        self.try_issue_semantic_pass()?.run_with_context(&mut native,move|holder,deadline,native|Box::pin(async move {
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
   if house.runtime_scope()?!=fence.runtime_scope||!super::super::house_hireling::fence::assert_house_inventory_fence(&mut tx,&node,&fence,origin_channel).await? {return Ok(Err(HouseWriterError::NotCurrent))}
   let row=sqlx::query("SELECT r.content_revision,r.ruleset_revision,a.acl_revision::text,a.guild_revisions FROM game_character_roots r CROSS JOIN LATERAL game_house_access(r.world_id,$2,r.character_id) a WHERE r.character_id=encode($1,'hex')::uuid")
    .bind(fence.character_id.as_bytes().as_slice()).bind(house.house_key()).fetch_one(&mut *tx).await?;
   let inventory=super::super::item_use::read_use_inventory(&mut tx,&fence,row.try_get("content_revision")?,row.try_get("ruleset_revision")?).await.map_err(|_|DurabilityError::InvalidStoredState)?;
   let interior=load_house_interior_in_transaction(&mut tx,&house,fence.scope_ownership_generation.get(),&node).await.map_err(|_|DurabilityError::InvalidStoredState)?;
   let floor=interior.items.iter().find(|i|i.item_instance_id==item);
   let main=inventory.backpack.as_ref().and_then(|bag|bag.entries.iter().find(|entry|entry.item.item_instance_id==item).map(|entry|(bag,entry)));
   let(action,source_position)=match(target,floor,main) {
    (NativeHouseTarget::Backpack,Some(source),_) =>(HouseItemAction::Take{placement:source.placement_transaction_id,provenance_revision:source.provenance_revision},Some(super::super::consume_transform::HouseActionContext::decode_spatial_position(&source.spatial_position).ok_or(DurabilityError::InvalidStoredState)?)),
    (NativeHouseTarget::Position(at),Some(source),_) =>(HouseItemAction::TileMove{placement:source.placement_transaction_id,provenance_revision:source.provenance_revision,position:at.into_iter().flat_map(i32::to_be_bytes).collect()},Some(super::super::consume_transform::HouseActionContext::decode_spatial_position(&source.spatial_position).ok_or(DurabilityError::InvalidStoredState)?)),
    (NativeHouseTarget::Position(at),None,Some((bag,entry))) =>(HouseItemAction::Place{source:HouseCharacterSource::Inventory{parent:bag.backpack.item_instance_id,ordinal:entry.placement_ordinal},position:at.into_iter().flat_map(i32::to_be_bytes).collect()},None),
    _=>return Ok(Ok(None)),
   };
   let backpack=if matches!(action,HouseItemAction::Take{..}) {
    let Some(bag)=inventory.backpack.as_ref() else{return Ok(Err(HouseWriterError::InventoryFull))};
    let Some(capacity)=native.item_policy(&bag.backpack.definition.production_key,&bag.backpack.definition.revision_ref).and_then(|p|super::super::spell_items_abi::QualifiedItemDefinition::from_native_policy(p).ok()).and_then(|p|p.container_capacity).map(u32::from) else{return Ok(Err(HouseWriterError::InventoryFull))};
    Some(HouseBackpackFacts{definition:bag.backpack.definition.clone(),capacity})
   } else {None};
   let request=HouseItemRequest{command,house,item_instance_id:item,action,expected_acl_revision:row.try_get::<String,_>("acl_revision")?.parse().map_err(|_|DurabilityError::InvalidStoredState)?,expected_guild_revisions:row.try_get("guild_revisions")?,backpack,content_revision:row.try_get("content_revision")?,ruleset_revision:row.try_get("ruleset_revision")?,sim_revision:SERVER_BUILD_ID.into()};
   commit_semantic_transaction(tx,deadline).await?;Ok(Ok(Some(NativeHousePrepared{request,source_position})))
  })).await?
    }
}
