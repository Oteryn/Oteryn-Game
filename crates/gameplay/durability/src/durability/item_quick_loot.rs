//! Manual loot is a batch of the existing item loop's conserved moves, in one transaction.
use super::*;
use crate::durability::character_quick_loot as settings_owner;
use oteryn_gameplay_items::quick_loot::{
    Corpse, LootCategory, LootItem, LootSettings, MAX_NEARBY_CORPSES, MAX_TRANSFERS_PER_COMMAND,
    ManualLootMode, Recipient, TransferCommit, execute_bounded_plan, plan_manual_loot,
};
use oteryn_protocol_oteryn::quick_loot::{
    self as wire, QuickLootAction, QuickLootCategory, QuickLootItemResult, QuickLootRecipient,
    QuickLootDestination, QuickLootDestinations, QuickLootRequest, QuickLootResult,
};

#[derive(Clone, Copy)]
pub(super) struct MoveContext {
    pub(super) source: MoveSource,
    pub(super) ordinal: i16,
}

#[derive(Clone, Copy)]
pub(super) enum MoveSource {
    Corpse {
        id: [u8; 16],
        cell: [u8; 10],
        stack_ordinal: i64,
    },
    Carried([u8; 16]),
}

pub fn is_loot_coin(definition: &TypedDefinitionRef) -> bool {
    definition.family == "Item"
        && matches!(
            definition.production_key.as_str(),
            "oteryn:item.tibia.i3031" | "oteryn:item.tibia.i3035" | "oteryn:item.tibia.i3043"
        )
}

const _: () = {
    assert!(LootCategory::ALL.len() == QuickLootCategory::ALL.len());
    let mut index = 0;
    while index < LootCategory::ALL.len() {
        assert!(LootCategory::ALL[index] as usize == index);
        assert!(QuickLootCategory::ALL[index] as usize == index + 1);
        index += 1;
    }
};

fn category(value: QuickLootCategory) -> LootCategory {
    LootCategory::ALL[(value as usize) - 1]
}

pub(super) async fn assert_corpse_source(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    item: [u8; 16],
    corpse: [u8; 16],
    cell: [u8; 10],
    stack_ordinal: i64,
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
          AND (g.runtime_scope_ownership_generation=$5::text::numeric OR (g.runtime_scope_ownership_generation<$5::text::numeric AND (game_ordinary_corpse_loop_ground(g.item_instance_id) OR game_ordinary_corpse_drag_source(g.item_instance_id)))) AND g.spatial_position=$6 AND g.stack_ordinal=$8
          AND i.lifecycle=1 AND r.loot_purpose_key='CORPSE_MATERIALIZATION' AND r.draw_ordinal=0
          AND (floor(extract(epoch FROM clock_timestamp())*1000)::bigint >= r.materialized_at + 10000
            OR r.corpse_top_damage_character_id=encode($7,'hex')::uuid
            OR EXISTS (SELECT 1 FROM game_party_members a JOIN game_party_members b USING(party_id)
                WHERE a.character_id=r.corpse_top_damage_character_id AND b.character_id=encode($7,'hex')::uuid)))")
        .bind(item.as_slice()).bind(corpse.as_slice()).bind(world.as_bytes().as_slice())
        .bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(cell.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(stack_ordinal).fetch_one(&mut **tx).await?;
    if allowed {
        Ok(())
    } else {
        refuse(LoopMoveRefusal::Stale)
    }
}

pub(super) async fn recipient<C: ItemLoopContent>(
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

/// The receipt keeps the original wire binding for handle-bearing commands.
pub enum QuickLootInput<'a> {
    Legacy(QuickLootRequest),
    Managed { command_type: u32, payload: &'a [u8], request: Option<QuickLootRequest> },
}

/// One manual command, retaining the input bytes' identity separately from Channel resolution.
pub struct QuickLootCommand<'a> {
    pub command: CommandRef,
    pub cell: [u8; 10],
    pub loop_map_revision: Option<String>,
    pub premium: bool,
    pub input: QuickLootInput<'a>,
    pub selected_corpse: Option<[u8; 16]>,
}

impl DurabilityRoot {
    /// Command 114: replay, settings and all accepted transfers share one fenced item transaction.
    pub async fn commit_quick_loot<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        content: &C,
        input: QuickLootCommand<'_>,
    ) -> std::result::Result<std::result::Result<Vec<u8>, LoopMoveRefusal>, DurabilityError>
    {
        let QuickLootCommand { command, cell, loop_map_revision, premium, input, selected_corpse } = input;
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let (encoded, request, legacy) = match input {
            QuickLootInput::Legacy(request) => (
                wire::encode_quick_loot_request(&request).map_err(|_| DurabilityError::InvalidStoredState)?,
                Some(request),
                true,
            ),
            QuickLootInput::Managed { command_type, payload, request } => {
                let mut encoded = command_type.to_le_bytes().to_vec();
                encoded.extend_from_slice(payload);
                (encoded, request, false)
            }
        };
        let has_corpse = request.is_some_and(|request| {
            matches!(request.action, QuickLootAction::Click | QuickLootAction::LootAll)
                && request.corpse.is_some()
        });
        if selected_corpse.is_some() != has_corpse {
            return Err(DurabilityError::InvalidStoredState);
        }
        let mut context = (permit, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let (permit, content) = *context;
            let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            lock_loop_command(&mut tx, command).await?;
            if !legacy && let Err(error) = assert_current_equipment_in_transaction(
                &mut tx, &record, &node, &fence, Some(command), content.loop_content_digest(),
            ).await {
                tx.rollback().await?;
                return loop_error(error.into()).map(Err);
            }
            if let Some(row) = sqlx::query("SELECT character_id::text,request,result FROM game_quick_loot_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await? {
                if uuid_text(row.try_get("character_id")?)? != *fence.character_id.as_bytes() || row.try_get::<Vec<u8>,_>("request")? != encoded { tx.rollback().await?; return Ok(Err(LoopMoveRefusal::Rejected)); }
                let result = row.try_get::<Vec<u8>,_>("result")?;
                super::super::db::commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(result));
            }
            let Some(request) = request else {
                tx.rollback().await?;
                return Ok(Err(LoopMoveRefusal::Stale));
            };
            let attempt: LoopResult<Vec<u8>> = async {
                if legacy {
                    assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content.loop_content_digest()).await?;
                }
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
                    if let Some(key) = key {
                        if !settings_owner::edit_filter_in_transaction(&mut tx,fence.character_id,settings,&key,
                            request.action==QuickLootAction::FilterAdd).await? {
                            return refuse(LoopMoveRefusal::NoRoom);
                        }
                    } else {
                        let accepted=settings.accept_only;
                        let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                        settings_owner::store(&mut tx,fence.character_id,revision.parse().map_err(|_|DurabilityError::InvalidStoredState)?,settings).await?;
                        if request.action==QuickLootAction::FilterClear {
                            sqlx::query("DELETE FROM game_character_quick_loot_filters WHERE character_id=encode($1,'hex')::uuid AND accepted=$2")
                                .bind(fence.character_id.as_bytes().as_slice()).bind(accepted).execute(&mut *tx).await?;
                        }
                    }
                    return Ok(Vec::new());
                }
                if !matches!(request.action, QuickLootAction::Click | QuickLootAction::LootAll) {
                    match request.action {
                        QuickLootAction::ReviewDestinations => {
                            let obtain = settings_owner::load_obtain(&mut tx, fence.character_id.as_bytes()).await?;
                            let loot = std::iter::once((None, settings.unassigned_target))
                                .chain(settings.category_targets.into_iter().map(|(key, target)| (Some(key), target)));
                            let projection = QuickLootDestinations {
                                loot: loot.map(destination_row).collect(),
                                obtain: obtain.into_iter().map(destination_row).collect(),
                                fallback_to_main: settings.fallback_to_main,
                            };
                            return wire::encode_quick_loot_destinations(&projection)
                                .map_err(|_| DurabilityError::InvalidStoredState.into());
                        }
                        QuickLootAction::Assign | QuickLootAction::AssignLoot | QuickLootAction::AssignAllLoot | QuickLootAction::UnassignLoot => {
                            if (request.category.is_some() || request.action == QuickLootAction::AssignAllLoot) && !premium {
                                return refuse(LoopMoveRefusal::Rejected);
                            }
                            let target = if request.action == QuickLootAction::UnassignLoot {
                                None
                            } else {
                                let target = assignment(request)?;
                                recipient(&mut tx, fence.character_id, world, content, target).await?;
                                settings.fallback_to_main = request.fallback_to_main;
                                Some(target)
                            };
                            match (request.action, request.category, target) {
                                (QuickLootAction::AssignAllLoot, _, Some(target)) => {
                                    settings.unassigned_target = target;
                                    for key in QuickLootCategory::ALL {
                                        settings.category_targets.insert(category(key), target);
                                    }
                                }
                                (_, Some(key), Some(target)) => {
                                    settings.category_targets.insert(category(key), target);
                                }
                                (_, Some(key), None) => {
                                    settings.category_targets.remove(&category(key));
                                }
                                (_, None, target) => {
                                    settings.unassigned_target = target.unwrap_or(Recipient::MainContainer);
                                }
                            }
                            let next: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                            let next = next.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
                            settings_owner::store(&mut tx, fence.character_id, next, settings).await?;
                            return Ok(Vec::new());
                        }
                        QuickLootAction::AssignObtain | QuickLootAction::AssignAllObtain | QuickLootAction::UnassignObtain => {
                            if !premium {
                                return refuse(LoopMoveRefusal::Rejected);
                            }
                            let target = if request.action == QuickLootAction::UnassignObtain {
                                None
                            } else {
                                let target = assignment(request)?;
                                recipient(&mut tx, fence.character_id, world, content, target).await?;
                                Some(target)
                            };
                            let categories = if request.action == QuickLootAction::AssignAllObtain {
                                std::iter::once(None).chain(QuickLootCategory::ALL.map(|key| Some(category(key)))).collect()
                            } else {
                                vec![request.category.map(category)]
                            };
                            for key in categories {
                                settings_owner::set_obtain(&mut tx, fence.character_id.as_bytes(), key, target).await?;
                            }
                            return Ok(Vec::new());
                        }
                        QuickLootAction::Distribute => {
                            if !premium {
                                return refuse(LoopMoveRefusal::Rejected);
                            }
                            let source = request.corpse.ok_or(LoopError::Refused(LoopMoveRefusal::Stale))?;
                            let mut context = (&mut tx, permit, &record, &node, &fence, command, content);
                            let result = distribute(&mut context, source).await?;
                            return wire::encode_quick_loot_result(&result)
                                .map_err(|_| DurabilityError::InvalidStoredState.into());
                        }
                        _ => return refuse(LoopMoveRefusal::Rejected),
                    }
                }
                // A bounded tile query, ordered before LIMIT so distant corpses cannot mask nearby ones.
                let x=i32::from_be_bytes(cell[0..4].try_into().map_err(|_| DurabilityError::InvalidStoredState)?);
                let y=i32::from_be_bytes(cell[4..8].try_into().map_err(|_| DurabilityError::InvalidStoredState)?);
                let mut cells=Vec::new();
                let mut loop_cells=Vec::new();
                let loop_floor=i16::from_be_bytes([cell[8],cell[9]]).checked_neg();
                for dx in -1_i32..=1 { for dy in -1_i32..=1 {
                    if let (Some(x),Some(y))=(x.checked_add(dx),y.checked_add(dy)) {
                        cells.push([x.to_be_bytes().as_slice(),y.to_be_bytes().as_slice(),&cell[8..10]].concat());
                        if let Some(floor)=loop_floor {
                            loop_cells.push([x.to_be_bytes().as_slice(),y.to_be_bytes().as_slice(),floor.to_be_bytes().as_slice()].concat());
                        }
                    }
                }}
                // The activation is locked by the equipment fence. A new Ground insertion
                // gets a unique stack ordinal, so each transfer can recheck this exact placement.
                let corpses=sqlx::query("SELECT i.item_instance_id::text,g.spatial_position,g.stack_ordinal,
                        (floor(extract(epoch FROM clock_timestamp())*1000)::bigint >= r.materialized_at + 10000
                         OR r.corpse_top_damage_character_id=encode($7,'hex')::uuid
                         OR EXISTS(SELECT 1 FROM game_party_members a JOIN game_party_members b USING(party_id)
                                   WHERE a.character_id=r.corpse_top_damage_character_id AND b.character_id=encode($7,'hex')::uuid)) AS loot_allowed
                    FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
                    JOIN game_item_mint_receipts r USING(item_instance_id)
                    CROSS JOIN (SELECT server_artifact_digest,frame_binding_digest FROM game_content_activations
                                WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid
                                ORDER BY activation_sequence DESC LIMIT 1) pin
                    WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid
                      AND g.stack_ordinal IS NOT NULL AND (g.spatial_position=ANY($3) OR g.spatial_position=ANY($8))
                      AND CASE WHEN $9::text IS NULL THEN g.runtime_scope_ownership_generation=$4::text::numeric
                                  AND g.spatial_position=ANY($3) AND NOT game_ordinary_corpse_loop_ground(g.item_instance_id)
                               ELSE g.content_revision='sha256:'||encode(pin.server_artifact_digest,'hex')
                                  AND g.native_room_placement_context=pin.frame_binding_digest
                                  AND ((g.map_revision=$9 AND g.spatial_position=ANY($8))
                                    OR (g.map_revision='sha256:'||encode(sha256(convert_to($9,'UTF8')),'hex')
                                        AND g.spatial_position=ANY($3))) END
                      AND (g.runtime_scope_ownership_generation=$4::text::numeric
                        OR (g.runtime_scope_ownership_generation<$4::text::numeric
                          AND (game_ordinary_corpse_loop_ground(g.item_instance_id) OR game_ordinary_corpse_drag_source(g.item_instance_id))))
                      AND i.lifecycle=1 AND r.loot_purpose_key='CORPSE_MATERIALIZATION' AND r.draw_ordinal=0
                      AND ($5::bytea IS NULL OR i.item_instance_id=encode($5,'hex')::uuid)
                    ORDER BY i.item_instance_id LIMIT $6 FOR UPDATE OF i")
                    .bind(world.as_slice()).bind(channel.as_bytes().as_slice()).bind(cells).bind(fence.scope_ownership_generation.get().to_string())
                    .bind(selected_corpse.as_ref().map(<[u8;16]>::as_slice)).bind(MAX_NEARBY_CORPSES as i64).bind(fence.character_id.as_bytes().as_slice()).bind(loop_cells).bind(loop_map_revision).fetch_all(&mut *tx).await?;
                let mut result=QuickLootResult { items: Vec::new() };
                for corpse in corpses {
                    let corpse_id=uuid_text(corpse.try_get("item_instance_id")?)?;
                    let stack_ordinal: i64 = corpse.try_get("stack_ordinal")?;
                    let corpse_cell: [u8;10]=corpse.try_get::<Vec<u8>,_>("spatial_position")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
                    let entries=sqlx::query("SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1 ORDER BY e.placement_ordinal LIMIT $2 FOR UPDATE OF i")
                        .bind(corpse_id.as_slice()).bind((MAX_TRANSFERS_PER_COMMAND-result.items.len()) as i64).fetch_all(&mut *tx).await?;
                    let entries: Vec<_> = entries.into_iter().map(|entry| Ok((uuid_text(entry.try_get("item_instance_id")?)?, definition_of(&entry)?))).collect::<LoopResult<_>>()?;
                    let policy_settings = LootSettings { category_targets: settings.category_targets.clone(), unassigned_target: settings.unassigned_target, fallback_to_main: settings.fallback_to_main };
                    let corpse = Corpse { key: corpse_id, distance: 0, age_millis: 0, top_damage_or_party: corpse.try_get("loot_allowed")?, items: entries.iter().map(|(item,definition)| LootItem { key: *item, category: preferred_category(content, definition, |category| settings.category_targets.contains_key(&category)), is_coin: is_loot_coin(definition), allowed_by_filter: if settings.accept_only {settings.accepted_items.contains(&definition.production_key)} else {!settings.skipped_items.contains(&definition.production_key)} }).collect() };
                    let mode = if request.action == QuickLootAction::Click { ManualLootMode::Click } else { ManualLootMode::AllNearby };
                    let plan = plan_manual_loot(mode, selected_corpse, premium, &policy_settings, &[corpse]);
                    let base_ordinal = result.items.len();
                    let source = MoveSource::Corpse { id: corpse_id, cell: corpse_cell, stack_ordinal };
                    let mut move_context = (&mut tx, permit, &record, &node, &fence, command, content, &entries, &settings, source, base_ordinal);
                    let execution = execute_bounded_plan(&plan, &mut move_context, |context, move_| Box::pin(async move {
                        let (tx, permit, record, node, fence, command, content, entries, settings, source, base_ordinal) = context;
                        let (index,(_,definition)) = entries.iter().enumerate().find(|(_, (item,_))| *item == move_.item).ok_or(DurabilityError::InvalidStoredState)?;
                        let context=MoveContext { source: *source, ordinal:i16::try_from(*base_ordinal+index+1).map_err(|_| DurabilityError::InvalidStoredState)? };
                        let accepted=attempt_move(tx, permit, record, node, fence, *command, *content, context, move_.item, definition, move_.recipient, settings.fallback_to_main).await?;
                        Ok::<_,LoopError>(if accepted { TransferCommit::Committed } else { TransferCommit::Refused })
                    })).await?;
                    for (item,_) in entries {
                        result.items.push(QuickLootItemResult { item, accepted: execution.accepted.iter().any(|m| m.item==item) });
                    }
                    if result.items.len()==MAX_TRANSFERS_PER_COMMAND { break; }
                }
                wire::encode_quick_loot_result(&result)
                    .map_err(|_| DurabilityError::InvalidStoredState.into())
            }.await;
            match attempt {
                Ok(result) => {
                    if request.action != QuickLootAction::ReviewDestinations {
                        sqlx::query("INSERT INTO game_quick_loot_receipts(game_session_id,command_id,character_id,request,result) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4,$5)")
                        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(encoded).bind(&result).execute(&mut *tx).await?;
                    }
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(result))
                }
                Err(error) => { tx.rollback().await?; loop_error(error).map(Err) },
            }
        })).await
    }
}

fn assignment(request: QuickLootRequest) -> LoopResult<Recipient<[u8; 16]>> {
    match (request.recipient, request.recipient_id) {
        (Some(QuickLootRecipient::MainContainer), None) => Ok(Recipient::MainContainer),
        (Some(QuickLootRecipient::Container), Some(id)) => Ok(Recipient::Container(id)),
        (Some(QuickLootRecipient::GoldPouch), Some(id)) => Ok(Recipient::GoldPouch(id)),
        _ => refuse(LoopMoveRefusal::Rejected),
    }
}

fn destination_row((category, target): (Option<LootCategory>, Recipient<[u8; 16]>)) -> QuickLootDestination {
    let category = category.map(|key| QuickLootCategory::ALL[key as usize]);
    let (recipient, recipient_id) = match target {
        Recipient::MainContainer => (QuickLootRecipient::MainContainer, None),
        Recipient::Container(id) => (QuickLootRecipient::Container, Some(id)),
        Recipient::GoldPouch(id) => (QuickLootRecipient::GoldPouch, Some(id)),
    };
    QuickLootDestination { category, recipient, recipient_id }
}

pub(in super::super) fn preferred_category<C: ItemLoopContent>(
    content: &C,
    definition: &TypedDefinitionRef,
    assigned: impl FnOnce(LootCategory) -> bool,
) -> LootCategory {
    content.managed_item_category(definition).filter(|&category| assigned(category))
        .unwrap_or_else(|| content.loot_category(definition))
}

type BatchContext<'a, 'tx, C> = (
    &'a mut Transaction<'tx, Postgres>,
    &'a SpellLanePermit,
    &'a crate::character_recovery_fence::CharacterRecoveryFenceV1,
    &'a NodeIncarnationProof,
    &'a CurrentCharacterItemFence,
    CommandRef,
    &'a C,
);

async fn distribute<C: ItemLoopContent>(
    context: &mut BatchContext<'_, '_, C>,
    source: [u8; 16],
) -> LoopResult<QuickLootResult> {
    let (tx, permit, record, node, fence, command, content) = context;
    let world = *fence.runtime_scope.world_id().as_bytes();
    recipient(tx, fence.character_id, world, *content, Recipient::Container(source)).await?;
    let targets = settings_owner::load_obtain(tx, fence.character_id.as_bytes()).await?;
    let rows = sqlx::query(
        "SELECT i.item_instance_id::text, i.definition_family, i.definition_production_key,
                i.definition_revision_ref
         FROM (
            SELECT item_instance_id, placement_ordinal FROM game_item_container_entries
            WHERE parent_item_instance_id=encode($1,'hex')::uuid
            UNION ALL
            SELECT item_instance_id, placement_ordinal FROM game_item_nested_entries
            WHERE parent_item_instance_id=encode($1,'hex')::uuid
         ) e JOIN game_item_instances i USING (item_instance_id)
         WHERE i.lifecycle=1 AND i.world_id=encode($2,'hex')::uuid
         ORDER BY e.placement_ordinal, i.item_instance_id LIMIT $3 FOR UPDATE OF i",
    )
    .bind(source.as_slice()).bind(world.as_slice()).bind(MAX_TRANSFERS_PER_COMMAND as i64)
    .fetch_all(&mut ***tx).await?;
    let mut items = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let item = uuid_text(row.try_get("item_instance_id")?)?;
        let definition = definition_of(row)?;
        let category = preferred_category(*content, &definition, |category| targets.contains_key(&Some(category)));
        let target = targets.get(&Some(category)).or_else(|| targets.get(&None));
        let accepted = if let Some(&target) = target {
            let context = MoveContext {
                source: MoveSource::Carried(source),
                ordinal: i16::try_from(index + 1).map_err(|_| DurabilityError::InvalidStoredState)?,
            };
            attempt_move(tx, permit, record, node, fence, *command, *content, context,
                item, &definition, target, false).await?
        } else {
            false
        };
        items.push(QuickLootItemResult { item, accepted });
    }
    Ok(QuickLootResult { items })
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
    fallback_to_main: bool,
) -> LoopResult<bool> {
    let mut targets = vec![target];
    if fallback_to_main && target != Recipient::MainContainer {
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
            if let MoveSource::Carried(source) = context.source {
                let destination_id = match destination {
                    LoopDestination::Container(id) => Some(id),
                    LoopDestination::MainBackpack => load_slot(tx, fence.character_id).await?.map(|slot| slot.item_instance_id),
                    LoopDestination::Ground(_) => None,
                };
                if destination_id == Some(source) {
                    return refuse(LoopMoveRefusal::NotSupported);
                }
            }
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
                Some(MoveBatch::QuickLoot(context)),
                None,
                None,
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
