//! Manual loot is a batch of the existing item loop's conserved moves, in one transaction.
use super::*;
use crate::durability::character_quick_loot::{self as settings_owner, DurableQuickLootSettings};
use oteryn_gameplay_items::quick_loot::{
    Corpse, LootCategory, LootItem, LootSettings, MAX_NEARBY_CORPSES, MAX_TRANSFERS_PER_COMMAND,
    ManualLootMode, Recipient, TransferCommit, execute_bounded_plan, plan_manual_loot,
};
use oteryn_protocol_oteryn::quick_loot::{
    self as wire, QuickLootAction, QuickLootCategory, QuickLootItemResult, QuickLootRecipient,
    QuickLootRequest, QuickLootResult,
};

#[derive(Clone, Copy)]
pub(super) struct MoveContext {
    pub(super) corpse: [u8; 16],
    pub(super) cell: [u8; 10],
    pub(super) ordinal: i16,
}

pub fn is_loot_coin(definition: &TypedDefinitionRef) -> bool {
    definition.family == "Item"
        && matches!(
            definition.production_key.as_str(),
            "oteryn:item.tibia.i3031" | "oteryn:item.tibia.i3035" | "oteryn:item.tibia.i3043"
        )
}

fn category(value: QuickLootCategory) -> LootCategory {
    match value {
        QuickLootCategory::Gold => LootCategory::Gold,
        QuickLootCategory::CreatureProduct => LootCategory::CreatureProduct,
        QuickLootCategory::Equipment => LootCategory::Equipment,
        QuickLootCategory::Consumable => LootCategory::Consumable,
        QuickLootCategory::Other => LootCategory::Other,
    }
}

pub(super) async fn assert_corpse_source(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    item: [u8; 16],
    context: MoveContext,
) -> LoopResult<()> {
    let (world, channel) =
        scope_of(fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
    let allowed: bool = sqlx::query_scalar("SELECT EXISTS (
        SELECT 1 FROM game_item_corpse_container_entries e
        JOIN game_item_ground_locations g ON g.item_instance_id=e.parent_item_instance_id
        JOIN game_item_instances i ON i.item_instance_id=g.item_instance_id
        JOIN game_item_mint_receipts r ON r.item_instance_id=i.item_instance_id
        WHERE e.item_instance_id=encode($1,'hex')::uuid AND e.parent_item_instance_id=encode($2,'hex')::uuid
          AND g.world_id=encode($3,'hex')::uuid AND g.channel_id=encode($4,'hex')::uuid
          AND g.runtime_scope_ownership_generation=$5::text::numeric AND g.spatial_position=$6
          AND i.lifecycle=1 AND r.loot_purpose_key='CORPSE_MATERIALIZATION' AND r.draw_ordinal=0
          AND (floor(extract(epoch FROM clock_timestamp())*1000)::bigint >= r.materialized_at + 10000
            OR r.corpse_top_damage_character_id=encode($7,'hex')::uuid
            OR EXISTS (SELECT 1 FROM game_party_members a JOIN game_party_members b USING(party_id)
                WHERE a.character_id=r.corpse_top_damage_character_id AND b.character_id=encode($7,'hex')::uuid)))")
        .bind(item.as_slice()).bind(context.corpse.as_slice()).bind(world.as_bytes().as_slice())
        .bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(context.cell.as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if allowed {
        Ok(())
    } else {
        refuse(LoopMoveRefusal::Stale)
    }
}

async fn recipient<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    world: [u8; 16],
    content: &C,
    target: Recipient<[u8; 16]>,
) -> LoopResult<LoopDestination> {
    let (id, pouch) = match target {
        Recipient::MainContainer => return Ok(LoopDestination::MainBackpack),
        Recipient::Container(id) => (id, false),
        Recipient::GoldPouch(id) => (id, true),
    };
    match loop_owner(tx, id).await? {
        Some((owner, _)) if owner == *character.as_bytes() => {}
        _ => return refuse(LoopMoveRefusal::Stale),
    }
    let Some((definition, _, _)) = lock_live_item(tx, id, world).await? else {
        return refuse(LoopMoveRefusal::Stale);
    };
    let is_pouch = definition.production_key == "oteryn:item.tibia.i23721";
    if pouch {
        let inbox: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid)")
            .bind(id.as_slice()).bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if !inbox {
            return refuse(LoopMoveRefusal::NotSupported);
        }
    }
    if pouch != is_pouch
        || content
            .loop_facts(&definition)
            .and_then(|f| f.container_capacity)
            .is_none()
    {
        return refuse(LoopMoveRefusal::NotSupported);
    }
    Ok(LoopDestination::Container(id))
}

impl DurabilityRoot {
    /// Command 114: replay, settings and all accepted transfers share one fenced item transaction.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_quick_loot<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        cell: [u8; 10],
        premium: bool,
        request: QuickLootRequest,
    ) -> std::result::Result<std::result::Result<QuickLootResult, LoopMoveRefusal>, DurabilityError>
    {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let encoded = wire::encode_quick_loot_request(&request)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let mut context = (permit, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let (permit, content) = *context;
            let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            lock_loop_command(&mut tx, command).await?;
            if let Some(row) = sqlx::query("SELECT character_id::text,request,result FROM game_quick_loot_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await? {
                if uuid_text(row.try_get("character_id")?)? != *fence.character_id.as_bytes() || row.try_get::<Vec<u8>,_>("request")? != encoded { tx.rollback().await?; return Ok(Err(LoopMoveRefusal::Rejected)); }
                let result = wire::decode_quick_loot_result(&row.try_get::<Vec<u8>,_>("result")?).map_err(|_| DurabilityError::InvalidStoredState)?;
                super::super::db::commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(result));
            }
            let attempt: LoopResult<QuickLootResult> = async {
                assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content.loop_content_digest()).await?;
                let (world, channel) = scope_of(&fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                let world = *world.as_bytes();
                let mut settings = settings_owner::load(&mut tx, fence.character_id.as_bytes()).await?;
                if matches!(request.action,QuickLootAction::FilterAdd | QuickLootAction::FilterRemove | QuickLootAction::FilterClear | QuickLootAction::FilterSelect) {
                    let mode=request.filter_mode.ok_or(LoopError::Refused(LoopMoveRefusal::Rejected))?;
                    settings.accept_only=mode==wire::QuickLootFilterMode::Accept;
                    let key=if let Some(reference)=request.filter_definition_ref {
                        let definition=content.loop_definition(reference).ok_or(LoopError::Refused(LoopMoveRefusal::Rejected))?;
                        content.loop_facts(&definition).ok_or(LoopError::Refused(LoopMoveRefusal::Rejected))?;
                        Some(definition.production_key)
                    } else {None};
                    let list=if settings.accept_only {&settings.accepted_items} else {&settings.skipped_items};
                    if request.action==QuickLootAction::FilterAdd && list.len()>=oteryn_gameplay_items::quick_loot::MAX_FILTER_ITEMS && !key.as_ref().is_some_and(|key|list.contains(key)) { return refuse(LoopMoveRefusal::NoRoom); }
                    let accepted=settings.accept_only;
                    let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                    settings_owner::store(&mut tx,fence.character_id,revision.parse().map_err(|_|DurabilityError::InvalidStoredState)?,settings).await?;
                    match request.action {
                        QuickLootAction::FilterAdd=>{sqlx::query("INSERT INTO game_character_quick_loot_filters(character_id,accepted,item_production_key) VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING").bind(fence.character_id.as_bytes().as_slice()).bind(accepted).bind(key.ok_or(DurabilityError::InvalidStoredState)?).execute(&mut *tx).await?;},
                        QuickLootAction::FilterRemove=>{sqlx::query("DELETE FROM game_character_quick_loot_filters WHERE character_id=encode($1,'hex')::uuid AND accepted=$2 AND item_production_key=$3").bind(fence.character_id.as_bytes().as_slice()).bind(accepted).bind(key.ok_or(DurabilityError::InvalidStoredState)?).execute(&mut *tx).await?;},
                        QuickLootAction::FilterClear=>{sqlx::query("DELETE FROM game_character_quick_loot_filters WHERE character_id=encode($1,'hex')::uuid AND accepted=$2").bind(fence.character_id.as_bytes().as_slice()).bind(accepted).execute(&mut *tx).await?;},
                        QuickLootAction::FilterSelect=>{},
                        _=>return refuse(LoopMoveRefusal::Rejected),
                    }
                    return Ok(QuickLootResult {items:Vec::new()});
                }
                if request.action == QuickLootAction::Assign {
                    if request.category.is_some() && !premium { return refuse(LoopMoveRefusal::Rejected); }
                    let target = match (request.recipient, request.recipient_id) {
                        (Some(QuickLootRecipient::MainContainer), None) => Recipient::MainContainer,
                        (Some(QuickLootRecipient::Container), Some(id)) => Recipient::Container(id),
                        (Some(QuickLootRecipient::GoldPouch), Some(id)) => Recipient::GoldPouch(id),
                        _ => return refuse(LoopMoveRefusal::Rejected),
                    };
                    recipient(&mut tx, fence.character_id, world, content, target).await?;
                    if let Some(value) = request.category { settings.category_targets.insert(category(value), target); }
                    else { settings.unassigned_target = target; }
                    settings.fallback_to_main = request.fallback_to_main;
                    let next: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                    let next = next.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
                    settings_owner::store(&mut tx, fence.character_id, next, settings).await?;
                    return Ok(QuickLootResult { items: Vec::new() });
                }
                // A bounded tile query, ordered before LIMIT so distant corpses cannot mask nearby ones.
                let x=i32::from_be_bytes(cell[0..4].try_into().map_err(|_| DurabilityError::InvalidStoredState)?);
                let y=i32::from_be_bytes(cell[4..8].try_into().map_err(|_| DurabilityError::InvalidStoredState)?);
                let mut cells=Vec::new();
                for dx in -1_i32..=1 { for dy in -1_i32..=1 {
                    if let (Some(x),Some(y))=(x.checked_add(dx),y.checked_add(dy)) {
                        cells.push([x.to_be_bytes().as_slice(),y.to_be_bytes().as_slice(),&cell[8..10]].concat());
                    }
                }}
                let corpses=sqlx::query("SELECT i.item_instance_id::text,g.spatial_position,(floor(extract(epoch FROM clock_timestamp())*1000)::bigint >= r.materialized_at + 10000 OR r.corpse_top_damage_character_id=encode($7,'hex')::uuid OR EXISTS(SELECT 1 FROM game_party_members a JOIN game_party_members b USING(party_id) WHERE a.character_id=r.corpse_top_damage_character_id AND b.character_id=encode($7,'hex')::uuid)) AS loot_allowed FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) JOIN game_item_mint_receipts r USING(item_instance_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND g.spatial_position=ANY($3) AND g.runtime_scope_ownership_generation=$4::text::numeric AND i.lifecycle=1 AND r.loot_purpose_key='CORPSE_MATERIALIZATION' AND r.draw_ordinal=0 AND ($5::bytea IS NULL OR i.item_instance_id=encode($5,'hex')::uuid) ORDER BY i.item_instance_id LIMIT $6 FOR UPDATE OF i")
                    .bind(world.as_slice()).bind(channel.as_bytes().as_slice()).bind(cells).bind(fence.scope_ownership_generation.get().to_string())
                    .bind(request.corpse.as_ref().map(<[u8;16]>::as_slice)).bind(MAX_NEARBY_CORPSES as i64).bind(fence.character_id.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
                let mut result=QuickLootResult { items: Vec::new() };
                for corpse in corpses {
                    let corpse_id=uuid_text(corpse.try_get("item_instance_id")?)?;
                    let corpse_cell: [u8;10]=corpse.try_get::<Vec<u8>,_>("spatial_position")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
                    let entries=sqlx::query("SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1 ORDER BY e.placement_ordinal LIMIT $2 FOR UPDATE OF i")
                        .bind(corpse_id.as_slice()).bind((MAX_TRANSFERS_PER_COMMAND-result.items.len()) as i64).fetch_all(&mut *tx).await?;
                    let entries: Vec<_> = entries.into_iter().map(|entry| Ok((uuid_text(entry.try_get("item_instance_id")?)?, definition_of(&entry)?))).collect::<LoopResult<_>>()?;
                    let policy_settings = LootSettings { category_targets: settings.category_targets.clone(), unassigned_target: settings.unassigned_target, fallback_to_main: settings.fallback_to_main };
                    let corpse = Corpse { key: corpse_id, distance: 0, age_millis: 0, top_damage_or_party: corpse.try_get("loot_allowed")?, items: entries.iter().map(|(item,definition)| LootItem { key: *item, category: content.loot_category(definition), is_coin: is_loot_coin(definition), allowed_by_filter: if settings.accept_only {settings.accepted_items.contains(&definition.production_key)} else {!settings.skipped_items.contains(&definition.production_key)} }).collect() };
                    let mode = if request.action == QuickLootAction::Click { ManualLootMode::Click } else { ManualLootMode::AllNearby };
                    let plan = plan_manual_loot(mode, request.corpse, premium, &policy_settings, &[corpse]);
                    let base_ordinal = result.items.len();
                    let mut move_context = (&mut tx, permit, &record, &node, &fence, command, content, &entries, &settings, corpse_id, corpse_cell, base_ordinal);
                    let execution = execute_bounded_plan(&plan, &mut move_context, |context, move_| Box::pin(async move {
                        let (tx, permit, record, node, fence, command, content, entries, settings, corpse_id, corpse_cell, base_ordinal) = context;
                        let (index,(_,definition)) = entries.iter().enumerate().find(|(_, (item,_))| *item == move_.item).ok_or(DurabilityError::InvalidStoredState)?;
                        let context=MoveContext { corpse:*corpse_id, cell:*corpse_cell, ordinal:i16::try_from(*base_ordinal+index+1).map_err(|_| DurabilityError::InvalidStoredState)? };
                        let accepted=attempt_move(tx, permit, record, node, fence, *command, *content, context, move_.item, definition, move_.recipient, settings).await?;
                        Ok::<_,LoopError>(if accepted { TransferCommit::Committed } else { TransferCommit::Refused })
                    })).await?;
                    for (item,_) in entries {
                        result.items.push(QuickLootItemResult { item, accepted: execution.accepted.iter().any(|m| m.item==item) });
                    }
                    if result.items.len()==MAX_TRANSFERS_PER_COMMAND { break; }
                }
                Ok(result)
            }.await;
            match attempt {
                Ok(result) => {
                    let bytes=wire::encode_quick_loot_result(&result).map_err(|_| DurabilityError::InvalidStoredState)?;
                    sqlx::query("INSERT INTO game_quick_loot_receipts(game_session_id,command_id,character_id,request,result) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4,$5)")
                        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(encoded).bind(bytes).execute(&mut *tx).await?;
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(result))
                }
                Err(error) => { tx.rollback().await?; loop_error(error).map(Err) },
            }
        })).await
    }
}

#[allow(clippy::too_many_arguments)]
async fn attempt_move<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    permit: &SpellLanePermit,
    record: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    content: &C,
    context: MoveContext,
    item: [u8; 16],
    definition: &TypedDefinitionRef,
    target: Recipient<[u8; 16]>,
    settings: &DurableQuickLootSettings,
) -> LoopResult<bool> {
    let mut targets = vec![target];
    if settings.fallback_to_main && target != Recipient::MainContainer {
        targets.push(Recipient::MainContainer);
    }
    for target in targets {
        sqlx::query("SAVEPOINT quick_loot_item")
            .execute(&mut **tx)
            .await?;
        let attempt: LoopResult<()> = async {
            if matches!(target, Recipient::GoldPouch(_)) && !is_loot_coin(definition) {
                return refuse(LoopMoveRefusal::NotSupported);
            }
            let destination = recipient(
                tx,
                fence.character_id,
                *fence.runtime_scope.world_id().as_bytes(),
                content,
                target,
            )
            .await?;
            let request = LoopMoveRequest {
                item,
                count: None,
                pickup_cell: None,
                destination,
            };
            let intent = intent_bytes(fence.character_id, &request)?;
            apply_loop_move(
                tx,
                permit,
                record,
                node,
                fence,
                command,
                content,
                &request,
                &intent,
                None,
                Some(context),
            )
            .await?;
            // Each child proves its exact before/after state before a later child may top it up.
            sqlx::query("SET CONSTRAINTS game_item_loop_line_proven IMMEDIATE")
                .execute(&mut **tx)
                .await?;
            sqlx::query("SET CONSTRAINTS game_item_loop_line_proven DEFERRED")
                .execute(&mut **tx)
                .await?;
            Ok(())
        }
        .await;
        match attempt {
            Ok(()) => {
                sqlx::query("RELEASE SAVEPOINT quick_loot_item")
                    .execute(&mut **tx)
                    .await?;
                return Ok(true);
            }
            Err(LoopError::Refused(_)) => {}
            Err(error) => return Err(error),
        }
        sqlx::query("ROLLBACK TO SAVEPOINT quick_loot_item")
            .execute(&mut **tx)
            .await?;
        sqlx::query("RELEASE SAVEPOINT quick_loot_item")
            .execute(&mut **tx)
            .await?;
    }
    Ok(false)
}

// An unlimited pouch needs at most one receiver row; its slot count is never a gameplay limit.
pub(super) async fn pouch_receiver(
    tx: &mut Transaction<'_, Postgres>,
    parent: [u8; 16],
    definition: &TypedDefinitionRef,
) -> LoopResult<Vec<BackpackEntry>> {
    let rows=sqlx::query("SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,n.placement_ordinal::text FROM game_item_nested_entries n JOIN game_item_instances i USING(item_instance_id) WHERE n.parent_item_instance_id=encode($1,'hex')::uuid AND i.definition_family=$2 AND i.definition_production_key=$3 AND i.definition_revision_ref=$4 AND i.quantity<100 ORDER BY n.placement_ordinal DESC LIMIT 1 FOR UPDATE OF i")
        .bind(parent.as_slice()).bind(&definition.family).bind(&definition.production_key).bind(&definition.revision_ref).fetch_all(&mut **tx).await?;
    rows.into_iter()
        .map(|row| {
            Ok(BackpackEntry {
                item: InventoryItem {
                    item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
                    definition: definition_of(&row)?,
                    quantity: stored_u32(row.try_get("quantity")?)?,
                },
                placement_ordinal: stored_u64(row.try_get("placement_ordinal")?)?,
            })
        })
        .collect()
}

// Unlimited pouch rows are aggregated into at most three definition groups.
pub(in super::super) async fn pouch_weight(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    world: [u8; 16],
    weights: &dyn ItemWeightLookup,
) -> std::result::Result<std::result::Result<u64, ItemWeightRefused>, DurabilityError> {
    let rows=sqlx::query("SELECT i.definition_family,i.definition_production_key,i.definition_revision_ref,sum(i.quantity)::text AS quantity FROM game_item_character_inbox_locations p JOIN game_item_instances pouch ON pouch.item_instance_id=p.item_instance_id JOIN game_item_nested_entries n ON n.parent_item_instance_id=p.item_instance_id JOIN game_item_instances i ON i.item_instance_id=n.item_instance_id WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid AND pouch.definition_production_key='oteryn:item.tibia.i23721' GROUP BY i.definition_family,i.definition_production_key,i.definition_revision_ref LIMIT 4")
        .bind(character.as_bytes().as_slice()).bind(world.as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() > 3 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut total = 0_u64;
    for row in rows {
        let definition = TypedDefinitionRef {
            family: row.try_get("definition_family")?,
            production_key: row.try_get("definition_production_key")?,
            revision_ref: row.try_get("definition_revision_ref")?,
        };
        if !is_loot_coin(&definition) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let quantity: u64 = row
            .try_get::<String, _>("quantity")?
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let unit = match weights.weight_of(&definition) {
            Ok(v) => v,
            Err(e) => return Ok(Err(e)),
        };
        total = total
            .checked_add(
                quantity
                    .checked_mul(u64::from(unit))
                    .ok_or(DurabilityError::InvalidStoredState)?,
            )
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    Ok(Ok(total))
}
