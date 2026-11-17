//! Bulk BUY shares the NPC catalogue, transaction, replay fence and native Ground owner.
use super::super::item_loop_move::{ITEM_LOOP_DROP_REF_PREFIX, ItemLoopContent, LoopGround};
use super::super::item_mint_audit::OneItemGroundV1;
use super::super::npc_trade_audit::purchase::{
    OneItemNpcBulkPurchaseOutputV1, OneItemNpcBulkPurchaseV1,
};
use super::super::spell_owner_commit::SpellLanePermit;
use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use oteryn_gameplay_economy::npc_purchase::{
    PurchaseInput, PurchaseRefusal, plan_purchase, purchase_message,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseSelection {
    pub amount: u32,
    pub in_backpacks: bool,
    pub ignore_cap: bool,
    pub category: u8,
    pub ground: LoopGround,
    pub item_name: String,
}
impl PurchaseSelection {
    pub(super) fn bind(&self, binding: Vec<u8>) -> Vec<u8> {
        Sha256::new()
            .chain_update(b"oteryn:npc-purchase:v1\0")
            .chain_update(binding)
            .chain_update(self.amount.to_be_bytes())
            .chain_update([u8::from(self.in_backpacks), u8::from(self.ignore_cap)])
            .finalize()
            .to_vec()
    }
}

pub(super) struct PreparedPurchase {
    pub(super) plan: Plan,
    pub(super) total: u64,
    pub(super) units: u32,
    audit: OneItemNpcBulkPurchaseV1,
    bag: Option<TypedDefinitionRef>,
    text: String,
}
impl PreparedPurchase {
    pub(super) fn output(&self, line: &NpcTradeLine) -> Option<&OneItemNpcBulkPurchaseOutputV1> {
        self.audit
            .outputs
            .iter()
            .find(|output| output.item_instance_id == line.item_instance_id)
    }
    pub(super) fn definition(&self, line: &NpcTradeLine) -> Option<&TypedDefinitionRef> {
        self.audit
            .outputs
            .iter()
            .position(|output| output.item_instance_id == line.item_instance_id)
            .filter(|index| self.audit.in_backpacks && index % 21 == 0)
            .and(self.bag.as_ref())
    }
    pub(super) fn annotate(&self, message: &mut OneItemNpcTradeV1) {
        message.purchase = Some(self.audit.clone());
        for (line, planned) in message.lines.iter_mut().zip(&self.plan.lines) {
            if let Some(definition) = self.definition(planned)
                && let Some(after) = &mut line.after
            {
                after.definition = Some(definition_message(definition));
            }
        }
    }
}
fn definition_message(value: &TypedDefinitionRef) -> OneItemTypedDefinitionRevisionV1 {
    OneItemTypedDefinitionRevisionV1 {
        family: value.family.clone(),
        production_key: value.production_key.clone(),
        revision_ref: value.revision_ref.clone(),
    }
}
fn identity(request: &NpcTradeRequest, index: u32) -> [u8; 16] {
    let digest = Sha256::new()
        .chain_update(b"oteryn:npc-purchase-output:v1\0")
        .chain_update(request.transaction_id)
        .chain_update(index.to_be_bytes())
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}
fn row_definition(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<TypedDefinitionRef, DurabilityError> {
    Ok(TypedDefinitionRef {
        family: row.try_get("definition_family")?,
        production_key: row.try_get("definition_production_key")?,
        revision_ref: row.try_get("definition_revision_ref")?,
    })
}
#[allow(clippy::too_many_arguments, reason = "one fenced purchase preparation")]
pub(super) async fn prepare(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterItemFence,
    request: &NpcTradeRequest,
    selection: &PurchaseSelection,
    content: &dyn ItemLoopContent,
    permit: &SpellLanePermit,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    backpack: [u8; 16],
    entries: &[Entry],
    capacity: usize,
    tuple: Type2EventTuple,
) -> Result<PreparedPurchase> {
    if request.cause.side != NpcTradeSide::Buy
        || request.quantity != 1
        || !(1..=100).contains(&request.offer_count)
        || !selection.amount.is_multiple_of(request.offer_count)
        || !(1..=5).contains(&selection.category)
    {
        return Err(NpcTradeError::InvalidInput);
    }
    permit
        .check_scope(fence.runtime_scope)
        .map_err(|_| NpcTradeError::AuthorityRejected)?;
    let (world, channel) = scope_of(fence).map_err(|_| NpcTradeError::AuthorityRejected)?;
    let bag_ref = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i21411".into(),
        revision_ref: "definition-r1".into(),
    };
    let bag = if selection.in_backpacks {
        Some(
            content
                .loop_facts(&bag_ref)
                .filter(|facts| {
                    facts.container_capacity == Some(20)
                        && facts.stack == super::super::item_transfer::ItemStackClass::NonStackable
                })
                .ok_or(NpcTradeError::InvalidInput)?,
        )
    } else {
        None
    };
    let (stack_size, _) =
        stack_maximum(request.item.stack).map_err(|_| NpcTradeError::InvalidInput)?;
    let bags = if selection.in_backpacks {
        selection.amount.div_ceil(stack_size).div_ceil(20)
    } else {
        0
    };
    let total = request
        .unit_price
        .checked_mul(u64::from(selection.amount / request.offer_count))
        .and_then(|cost| cost.checked_add(u64::from(bags) * 20))
        .ok_or(NpcTradeError::InvalidInput)?;
    // Preserve the ordinary deterministic coin fee plan. Replace only its singleton item MINT.
    let mut plan = super::plan_buy(
        request,
        total,
        1,
        usize::MAX,
        entries,
        tuple,
        super::super::bank::fee_payer_is_junior(fence.character_id),
    )?;
    plan.lines
        .retain(|line| line.kind != NpcTradeLineKind::ItemMint);
    let whole = plan
        .lines
        .iter()
        .filter(|line| line.kind == NpcTradeLineKind::CoinBurn && line.quantity_after == 0)
        .count();
    let change = plan
        .lines
        .iter()
        .filter(|line| line.kind == NpcTradeLineKind::CoinMint)
        .count();
    let mut next_main = plan
        .lines
        .iter()
        .map(|line| line.placement_ordinal)
        .chain(entries.iter().map(|entry| entry.placement_ordinal))
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(NpcTradeError::DoesNotFit)?;
    let main_free = capacity.saturating_sub(entries.len().saturating_sub(whole) + change);
    let rows = super::super::item_transfer::load_carried_rows(&mut *tx, fence.character_id).await?;
    let mut carried = 0_u64;
    let mut ids = std::collections::BTreeSet::new();
    for row in &rows {
        let id = uuid_text(&row.try_get::<String, _>("item_instance_id")?)?;
        if row.try_get::<bool, _>("cycle")?
            || !ids.insert(id)
            || row.try_get::<i16, _>("lifecycle")? != 1
            || uuid_text(&row.try_get::<String, _>("item_world")?)? != *world.as_bytes()
            || uuid_text(&row.try_get::<String, _>("custody_world")?)? != *world.as_bytes()
        {
            return Err(NpcTradeError::Unavailable(
                DurabilityError::InvalidStoredState,
            ));
        }
        let quantity = u64::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| NpcTradeError::InvalidInput)?;
        let unit = content
            .weight_of(&row_definition(row)?)
            .map_err(|_| NpcTradeError::InvalidInput)?;
        carried = carried
            .checked_add(u64::from(unit) * quantity)
            .ok_or(NpcTradeError::InvalidInput)?;
    }
    carried = carried
        .checked_add(
            super::super::item_loop_move::pouch_weight(
                &mut *tx,
                fence.character_id,
                *world.as_bytes(),
                content,
            )
            .await?
            .map_err(|_| NpcTradeError::InvalidInput)?,
        )
        .ok_or(NpcTradeError::InvalidInput)?;
    for line in &plan.lines {
        let definition = request
            .coins
            .of(line.coin.ok_or(NpcTradeError::InvalidInput)?);
        let unit = u64::from(
            content
                .weight_of(definition)
                .map_err(|_| NpcTradeError::InvalidInput)?,
        );
        if line.kind == NpcTradeLineKind::CoinBurn {
            carried = carried
                .checked_sub(unit * u64::from(line.quantity_before - line.quantity_after))
                .ok_or(NpcTradeError::InvalidInput)?;
        } else {
            carried = carried
                .checked_add(unit * u64::from(line.quantity_after))
                .ok_or(NpcTradeError::InvalidInput)?;
        }
    }
    let max_weight = super::super::item_transfer::read_derived_transfer_capacity_centi_oz(
        &mut *tx,
        fence.character_id,
        content,
    )
    .await?;
    let mut target = None;
    if !selection.in_backpacks {
        let selected:Option<String>=sqlx::query_scalar("SELECT container_id::text FROM game_character_obtain_containers WHERE character_id=encode($1,'hex')::uuid AND category IN(0,$2) ORDER BY category DESC LIMIT 1")
            .bind(fence.character_id.as_bytes().as_slice()).bind(i16::from(selection.category)).fetch_optional(&mut ***tx).await?;
        if let Some(selected) = selected {
            let id = uuid_text(&selected)?;
            if id != backpack
                && ids.contains(&id)
                && let Some(row) = rows.iter().find(|row| {
                    row.try_get::<String, _>("item_instance_id")
                        .ok()
                        .and_then(|value| uuid_text(&value).ok())
                        == Some(id)
                })
                && let Some(capacity) = content
                    .loop_facts(&row_definition(row)?)
                    .and_then(|facts| facts.container_capacity)
            {
                let(count,last):(i64,Option<String>)=sqlx::query_as("SELECT count(*),max(placement_ordinal)::text FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(id.as_slice()).fetch_one(&mut ***tx).await?;
                let free = capacity
                    .saturating_sub(u32::try_from(count).map_err(|_| NpcTradeError::InvalidInput)?);
                let next = last
                    .map(|value| value.parse::<u64>())
                    .transpose()
                    .map_err(|_| NpcTradeError::InvalidInput)?
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or(NpcTradeError::DoesNotFit)?;
                target = Some((id, free, next));
            }
        }
    }
    let tile_items:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND spatial_position=$3")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&selection.ground.spatial_position).fetch_one(&mut ***tx).await?;
    let input = PurchaseInput {
        amount: selection.amount,
        stack_size,
        unit_price: 0,
        in_backpacks: selection.in_backpacks,
        ignore_cap: selection.ignore_cap,
        free_slots: u32::try_from(main_free)
            .map_err(|_| NpcTradeError::InvalidInput)?
            .saturating_add(target.map_or(0, |(_, free, _)| free)),
        tile_items: u32::try_from(tile_items).map_err(|_| NpcTradeError::InvalidInput)?,
        free_capacity: max_weight.saturating_sub(carried),
        item_weight: u64::from(
            content
                .weight_of(&request.item.definition)
                .map_err(|_| NpcTradeError::InvalidInput)?,
        ),
        bag_weight: match &bag {
            Some(bag) => u64::from(bag.weight.ok_or(NpcTradeError::InvalidInput)?),
            None => 0,
        },
        funds: total,
    };
    let packing = plan_purchase(input).map_err(|error| match error {
        PurchaseRefusal::Capacity => NpcTradeError::OverCapacity,
        PurchaseRefusal::Room => NpcTradeError::DoesNotFit,
        PurchaseRefusal::InsufficientFunds => NpcTradeError::InsufficientFunds,
        PurchaseRefusal::InvalidInput => NpcTradeError::InvalidInput,
    })?;
    let mut outputs = Vec::new();
    let mut output_index = 0;
    let mut current_bag = None;
    let mut root_index = 0_u32;
    let mut ground_checked = false;
    let ground = OneItemGroundV1 {
        world_id: world.as_bytes().to_vec(),
        channel_id: channel.as_bytes().to_vec(),
        runtime_scope_ownership_generation: fence.scope_ownership_generation.get(),
        spatial_position: selection.ground.spatial_position.clone(),
        corpse_ref: [ITEM_LOOP_DROP_REF_PREFIX, request.transaction_id.as_slice()].concat(),
        map_revision: selection.ground.map_revision.clone(),
        content_revision: selection.ground.content_revision.clone(),
        native_room_placement_context: selection.ground.placement_context.clone(),
        instance_id: None,
    };
    for (stack_index, count) in packing.stacks.iter().enumerate() {
        let new_bag = selection.in_backpacks && stack_index % 20 == 0;
        if new_bag || !selection.in_backpacks {
            let (parent, on_ground, ordinal) = if !packing.carried_roots[root_index as usize] {
                (None, Some(ground.clone()), u64::from(root_index) + 1)
            } else if let Some((id, free, next)) = &mut target
                && *free > 0
            {
                let ordinal = *next;
                *next += 1;
                *free -= 1;
                (Some(id.to_vec()), None, ordinal)
            } else {
                let ordinal = next_main;
                next_main += 1;
                (Some(backpack.to_vec()), None, ordinal)
            };
            let id = identity(request, output_index);
            output_index += 1;
            if on_ground.is_some() {
                let placement = super::super::item_mint::GroundPlacement {
                    spatial_position: ground.spatial_position.clone(),
                    corpse_ref: ground.corpse_ref.clone(),
                    map_revision: ground.map_revision.clone(),
                    content_revision: ground.content_revision.clone(),
                    native_room_placement_context: ground.native_room_placement_context.clone(),
                };
                super::super::ground_cell_limit::check_ground_cell_root_insert_with_recovery(
                    &mut *tx,
                    permit,
                    recovery,
                    node,
                    (fence.runtime_scope, fence.scope_ownership_generation.get()),
                    &placement,
                    &id,
                )
                .await
                .map_err(|error| error.into_item_mint_error())
                .map_err(|error| match error {
                    super::super::item_mint::ItemMintError::Unavailable(error) => {
                        NpcTradeError::Unavailable(error)
                    }
                    _ => NpcTradeError::DoesNotFit,
                })?;
                if !ground_checked {
                    // Recount under the existing native Ground writer admission before any mint.
                    // Its caller retains the native Channel lock for this transaction.
                    let current: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND spatial_position=$3")
                        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                        .bind(&ground.spatial_position).fetch_one(&mut ***tx).await?;
                    let needed = i64::try_from(
                        packing
                            .carried_roots
                            .iter()
                            .filter(|carried| !**carried)
                            .count(),
                    )
                    .map_err(|_| NpcTradeError::InvalidInput)?;
                    if current.checked_add(needed).is_none_or(|count| count > 30) {
                        return Err(NpcTradeError::DoesNotFit);
                    }
                    ground_checked = true;
                }
            }
            plan.lines.push(NpcTradeLine {
                kind: NpcTradeLineKind::ItemMint,
                item_instance_id: id,
                placement_ordinal: ordinal,
                coin: None,
                quantity_before: 0,
                quantity_after: if new_bag { 1 } else { *count },
            });
            outputs.push(OneItemNpcBulkPurchaseOutputV1 {
                item_instance_id: id.to_vec(),
                parent,
                ground: on_ground,
            });
            if new_bag {
                current_bag = Some(id);
            } else if stack_index == 0 {
                plan.item_instance_id = id;
            }
            root_index += 1;
        }
        if selection.in_backpacks {
            let id = identity(request, output_index);
            output_index += 1;
            plan.lines.push(NpcTradeLine {
                kind: NpcTradeLineKind::ItemMint,
                item_instance_id: id,
                placement_ordinal: (stack_index % 20 + 1) as u64,
                coin: None,
                quantity_before: 0,
                quantity_after: *count,
            });
            outputs.push(OneItemNpcBulkPurchaseOutputV1 {
                item_instance_id: id.to_vec(),
                parent: current_bag.map(|id| id.to_vec()),
                ground: None,
            });
            if stack_index == 0 {
                plan.item_instance_id = id;
            }
        }
    }
    if outputs.len() > 128 {
        return Err(NpcTradeError::CapacityExceeded);
    }
    let audit = OneItemNpcBulkPurchaseV1 {
        offer_count: request.offer_count,
        in_backpacks: selection.in_backpacks,
        ignore_cap: selection.ignore_cap,
        stack_size,
        item_definition: Some(definition_message(&request.item.definition)),
        bag_definition: bag.as_ref().map(|bag| definition_message(&bag.definition)),
        outputs,
    };
    Ok(PreparedPurchase {
        plan,
        total,
        units: selection.amount,
        audit,
        bag: bag.map(|bag| bag.definition),
        text: purchase_message(selection.amount, &selection.item_name, bags, total),
    })
}

pub(super) async fn store(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &NpcTradeRequest,
    prepared: &PreparedPurchase,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_npc_purchase_details(transaction_id,in_backpacks,ignore_cap,stack_size,bag_count,response_text,offer_count) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7)")
        .bind(request.transaction_id.as_slice()).bind(prepared.audit.in_backpacks).bind(prepared.audit.ignore_cap).bind(i32::try_from(prepared.audit.stack_size).map_err(|_|DurabilityError::InvalidStoredState)?).bind(prepared.bag.as_ref().map_or(0,|_|prepared.units.div_ceil(prepared.audit.stack_size).div_ceil(20)) as i32).bind(&prepared.text).bind(i64::from(request.offer_count)).execute(&mut **tx).await?;
    for (index, output) in prepared.audit.outputs.iter().enumerate() {
        let line = prepared
            .plan
            .lines
            .iter()
            .find(|line| output.item_instance_id == line.item_instance_id)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let definition = prepared
            .definition(line)
            .unwrap_or(&request.item.definition);
        let ground = output.ground.as_ref();
        sqlx::query("INSERT INTO game_npc_purchase_outputs(transaction_id,purchase_index,item_instance_id,parent_id,placement_ordinal,quantity,definition_key,definition_revision,is_bag,ground_position,corpse_ref,map_revision,content_revision,placement_context) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
            .bind(request.transaction_id.as_slice()).bind(index as i32).bind(&output.item_instance_id).bind(output.parent.as_deref()).bind(line.placement_ordinal.to_string()).bind(i64::from(line.quantity_after)).bind(&definition.production_key).bind(&definition.revision_ref).bind(prepared.definition(line).is_some()).bind(ground.map(|g|&g.spatial_position)).bind(ground.map(|g|&g.corpse_ref)).bind(ground.map(|g|&g.map_revision)).bind(ground.map(|g|&g.content_revision)).bind(ground.map(|g|&g.native_room_placement_context)).execute(&mut **tx).await?;
    }
    Ok(())
}
pub(super) async fn place(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterItemFence,
    request: &NpcTradeRequest,
    line: &NpcTradeLine,
    output: &OneItemNpcBulkPurchaseOutputV1,
    backpack: &[u8; 16],
) -> std::result::Result<(), DurabilityError> {
    let (world, channel) = scope_of(fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    if let Some(ground) = &output.ground {
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9)")
            .bind(line.item_instance_id.as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string()).bind(&ground.spatial_position).bind(&ground.corpse_ref).bind(&ground.map_revision).bind(&ground.content_revision).bind(&ground.native_room_placement_context).execute(&mut **tx).await?;
    } else {
        let parent = output
            .parent
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        if parent == backpack {
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
                .bind(line.item_instance_id.as_slice()).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(parent).bind(line.placement_ordinal.to_string()).bind(request.transaction_id.as_slice()).execute(&mut **tx).await?;
        } else {
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid)")
                .bind(line.item_instance_id.as_slice()).bind(world.as_bytes().as_slice()).bind(parent).bind(line.placement_ordinal.to_string()).bind(request.transaction_id.as_slice()).execute(&mut **tx).await?;
        }
    }
    Ok(())
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "existing NPC transaction with its native content and Ground permit"
    )]
    pub async fn commit_npc_bulk_purchase(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: NpcTradeRequest,
        selection: PurchaseSelection,
        content: &dyn ItemLoopContent,
        permit: &SpellLanePermit,
        yasir_position: Option<(u16, u16, i8)>,
    ) -> Result<NpcTradeOutcome> {
        self.commit_trade(
            authority,
            node,
            fence,
            request,
            TradeCommitOptions {
                purchase: Some((selection, content, permit)),
                yasir_position,
                ..TradeCommitOptions::default()
            },
        )
        .await
    }
    /// Historical answer before conversation and current capacity admission, bound to the full wire intent.
    pub async fn read_npc_purchase(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        command: CommandRef,
        occurrence: [u8; 16],
        intent: oteryn_protocol_oteryn::npc_purchase::NpcPurchaseIntent,
    ) -> Result<Option<String>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| NpcTradeError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_occurrence(&mut tx,&occurrence).await?;
            let row=sqlx::query("SELECT t.occurrence_id::text,t.catalogue_revision::text,t.offer_index,t.quantity,t.unit_price::text,p.in_backpacks,p.ignore_cap,p.response_text,p.offer_count FROM game_npc_trades t JOIN game_npc_purchase_details p USING(transaction_id) WHERE t.character_id=encode($1,'hex')::uuid AND t.game_session_id=encode($2,'hex')::uuid AND t.command_id=$3::text::numeric")
                .bind(character.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
            let answer=if let Some(row)=row {
                if uuid_text(&row.try_get::<String,_>("occurrence_id")?)?!=occurrence || numeric_u64(&row,"catalogue_revision")?!=intent.offer.catalogue_revision || row.try_get::<i32,_>("offer_index")?!=intent.offer.offer_index as i32 || row.try_get::<i64,_>("quantity")?!=i64::from(intent.amount) * row.try_get::<i64,_>("offer_count")? || numeric_u64(&row,"unit_price")?!=u64::from(intent.offer.expected_unit_price) || row.try_get::<bool,_>("in_backpacks")?!=intent.in_backpacks || row.try_get::<bool,_>("ignore_cap")?!=intent.ignore_cap {return Ok(Err(NpcTradeError::ConflictingOccurrence));}
                Some(row.try_get("response_text")?)
            } else {None};
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(answer))
        })).await?
    }
}
