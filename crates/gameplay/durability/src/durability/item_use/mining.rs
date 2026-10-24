//! MINING-1 reuses the item-use one-unit TRANSFORM and its Type 2 audit.
//! A separate immutable proof binds the unchanged pick and original USE bytes.

use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::item_loop_move::ItemLoopContent;
use super::super::item_transfer::read_derived_transfer_capacity_centi_oz;
use super::*;
use crate::foundation::CommandRef;
use oteryn_gameplay_items::mining::{PICK, SHINY_STONE, refined_item};

pub const PICK_REFINE_CAUSE: u32 = 3;

impl DurabilityRoot {
    /// One fenced refining conversion. Capacity, tool and output are revalidated under
    /// the Character root lock before the existing atomic conversion writer runs.
    pub async fn commit_pick_refining(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemUseRequest,
        content: &dyn ItemLoopContent,
    ) -> Result<ItemUseOutcome> {
        if !matches!(request.kind, ItemUseKind::PickRefine { .. }) {
            return Err(ItemUseError::InvalidInput);
        }
        self.commit_use(authority, node, fence, request, Some(content))
            .await
    }

    /// Replay is resolved before any view handle, so a committed conversion survives a
    /// lost acknowledgement, replaced inventory projection or process restart.
    pub async fn read_pick_refining_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        binding: [u8; 32],
    ) -> Result<Option<ItemUseOutcome>> {
        if command.game_session_id() != fence.game_session_id {
            return Err(ItemUseError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(
                        "SELECT p.wire_binding, u.shape, u.character_id::text AS character, \
                        u.world_id::text AS world, u.channel_id::text AS channel \
                   FROM game_item_use_records u \
                   LEFT JOIN game_pick_refinements p USING (transaction_id) \
                  WHERE u.game_session_id=encode($1,'hex')::uuid \
                    AND u.command_id=$2::text::numeric(20,0)",
                    )
                    .bind(command.game_session_id().as_bytes().as_slice())
                    .bind(command.command_id().get().to_string())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let result = match row {
                        None => Ok(None),
                        Some(row) => {
                            let wire: Option<Vec<u8>> = row.try_get("wire_binding")?;
                            let scope = RuntimeScopeRefV1::channel(
                                crate::foundation::WorldId::decode(&uuid_text(
                                    &row.try_get::<String, _>("world")?,
                                )?)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                                crate::foundation::ChannelId::decode(&uuid_text(
                                    &row.try_get::<String, _>("channel")?,
                                )?)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                            );
                            if wire.as_deref() != Some(binding.as_slice())
                                || uuid_text(&row.try_get::<String, _>("character")?)?
                                    != *fence.character_id.as_bytes()
                                || scope != fence.runtime_scope
                            {
                                Err(ItemUseError::ConflictingCommand)
                            } else {
                                Ok(Some(ItemUseOutcome::AlreadyCommitted {
                                    shape: shape_from_code(row.try_get("shape")?)?,
                                }))
                            }
                        }
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await?
    }
}

pub(super) async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
    content: Option<&dyn ItemLoopContent>,
) -> Result<()> {
    let ItemUseKind::PickRefine {
        tool,
        wire_binding,
        percentile,
    } = request.kind
    else {
        return Err(ItemUseError::InvalidInput);
    };
    let content = content.ok_or(ItemUseError::InvalidInput)?;
    let inventory = super::carried_inventory::read_use_inventory(
        tx,
        fence,
        request.content_revision.clone(),
        request.ruleset_revision.clone(),
    )
    .await?;
    let bag = inventory
        .backpack
        .as_ref()
        .ok_or(ItemUseError::StaleState)?;
    let entries = bag
        .entries
        .iter()
        .chain(inventory.nested.iter().map(|(_, e)| e));
    let pick = entries
        .clone()
        .find(|e| e.item.item_instance_id == tool)
        .ok_or(ItemUseError::StaleState)?;
    let source = entries
        .clone()
        .find(|e| e.item.item_instance_id == request.source_item_instance_id)
        .ok_or(ItemUseError::StaleState)?;
    let output = flask_definition(request).ok_or(ItemUseError::InvalidInput)?;
    if tool == request.source_item_instance_id
        || pick.item.definition.family != "Item"
        || source.item.definition != request.source_definition
        || source.item.quantity != request.source_quantity_before
        || content.loop_facts(&pick.item.definition).is_none()
        || content.loop_facts(&source.item.definition).is_none()
        || content.loop_facts(output).is_none()
        || refined_item(
            &pick.item.definition.production_key,
            &source.item.definition.production_key,
            percentile,
        ) != Some(output.production_key.as_str())
        || source.item.definition.production_key != SHINY_STONE
        || pick.item.definition.production_key != PICK
    {
        return Err(ItemUseError::InvalidInput);
    }
    let cap = content
        .loop_facts(&bag.backpack.definition)
        .and_then(|f| f.container_capacity)
        .ok_or(ItemUseError::InvalidInput)?;
    let retiring = source.item.quantity == 1
        && bag
            .entries
            .iter()
            .any(|e| e.item.item_instance_id == source.item.item_instance_id);
    let fresh = matches!(request.flask, ItemUseFlaskPlan::New { .. });
    if bag.entries.len() + usize::from(fresh) - usize::from(retiring)
        > usize::try_from(cap).map_err(|_| ItemUseError::InvalidInput)?
    {
        return Err(ItemUseError::StaleState);
    }
    if bag.entries.len() + inventory.nested.len() + usize::from(fresh)
        - usize::from(source.item.quantity == 1)
        > 500
    {
        return Err(ItemUseError::StaleState);
    }
    let weight = |d: &TypedDefinitionRef| {
        content
            .weight_of(d)
            .map(u64::from)
            .map_err(|_| ItemUseError::InvalidInput)
    };
    let mut total = weight(&bag.backpack.definition)? * u64::from(bag.backpack.quantity);
    for item in entries.map(|e| &e.item).chain(inventory.equipment.iter()) {
        total = total
            .checked_add(weight(&item.definition)? * u64::from(item.quantity))
            .ok_or(ItemUseError::InvalidInput)?;
    }
    let after = total
        .checked_sub(weight(&source.item.definition)?)
        .and_then(|w| w.checked_add(weight(output).ok()?))
        .ok_or(ItemUseError::InvalidInput)?;
    if after > read_derived_transfer_capacity_centi_oz(tx, fence.character_id).await? {
        return Err(ItemUseError::StaleState);
    }
    // All item writers serialize on the Character root; lock both carried paths in the
    // shared writer's UUID order before recording the unchanged tool proof.
    sqlx::query("INSERT INTO game_pick_refinements(transaction_id,tool_item_instance_id,wire_binding,percentile) \
                 VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4)")
        .bind(request.transaction_id.as_slice()).bind(tool.as_slice()).bind(wire_binding.as_slice())
        .bind(i16::from(percentile)).execute(&mut **tx).await?;
    Ok(())
}
