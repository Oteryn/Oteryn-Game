//! W09: the Cube reuses the item-use TRANSFORM, its fences and Type 2 audit.
//! Crystal 201189f8 `data/scripts/actions/items/drome_cube.lua:27-39`.

use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::item_loop_move::ItemLoopContent;
use super::super::item_transfer::read_derived_transfer_capacity_centi_oz;
use super::*;
use crate::foundation::CommandRef;
use oteryn_gameplay_items::drome_cube::{DROME_CUBE_KEY, reward, reward_index};
use oteryn_gameplay_items::stacking::stack_maximum;

mod plan;
pub use plan::plan_output;

pub const DROME_CUBE_CAUSE: u32 = 578;

/// A Cube-specific capacity refusal, or an unchanged failure from the shared item-use owner.
#[derive(Debug)]
pub enum DromeCubeError {
    Capacity,
    ItemUse(ItemUseError),
}

impl From<ItemUseError> for DromeCubeError {
    fn from(error: ItemUseError) -> Self {
        Self::ItemUse(error)
    }
}

impl From<DurabilityError> for DromeCubeError {
    fn from(error: DurabilityError) -> Self {
        Self::ItemUse(ItemUseError::Unavailable(error))
    }
}

impl From<sqlx::Error> for DromeCubeError {
    fn from(error: sqlx::Error) -> Self {
        Self::ItemUse(ItemUseError::from(error))
    }
}

impl std::fmt::Display for DromeCubeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capacity => formatter.write_str("Cube output has no carried capacity"),
            Self::ItemUse(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for DromeCubeError {}

pub(super) type Result<T> = std::result::Result<T, DromeCubeError>;

/// Historical result, not authority to change inventory or a replacement for live fences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DromeCubeReceipt {
    pub reward_index: u8,
}

impl DurabilityRoot {
    /// Read the original USE result before resolving current handles. Its session,
    /// Character, World, Channel and bounded intent must all match the historical record.
    pub async fn read_drome_cube_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        binding: [u8; 32],
    ) -> super::Result<Option<DromeCubeReceipt>> {
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
                        "SELECT p.wire_binding, p.reward_index, \
                                uuid_send(u.character_id) AS character, \
                                uuid_send(u.world_id) AS world, \
                                uuid_send(u.channel_id) AS channel \
                           FROM game_item_use_records u \
                           LEFT JOIN game_drome_cube_uses p USING(transaction_id) \
                          WHERE u.game_session_id=encode($1,'hex')::uuid \
                            AND u.command_id=$2::text::numeric(20,0) AND u.cause_kind=578",
                    )
                    .bind(command.game_session_id().as_bytes().as_slice())
                    .bind(command.command_id().get().to_string())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let result = match row {
                        None => Ok(None),
                        Some(row) => {
                            let RuntimeScopeRefV1::Channel {
                                world_id,
                                channel_id,
                            } = fence.runtime_scope
                            else {
                                return Ok(Err(ItemUseError::AuthorityRejected));
                            };
                            let wire: Vec<u8> = row.try_get("wire_binding")?;
                            let character: Vec<u8> = row.try_get("character")?;
                            let world: Vec<u8> = row.try_get("world")?;
                            let channel: Vec<u8> = row.try_get("channel")?;
                            if wire.as_slice() != binding
                                || character.as_slice() != fence.character_id.as_bytes()
                                || world.as_slice() != world_id.as_bytes()
                                || channel.as_slice() != channel_id.as_bytes()
                            {
                                Err(ItemUseError::ConflictingCommand)
                            } else {
                                let index = u8::try_from(row.try_get::<i16, _>("reward_index")?)
                                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                                if reward(index).is_none() {
                                    return Err(DurabilityError::InvalidStoredState);
                                }
                                Ok(Some(DromeCubeReceipt {
                                    reward_index: index,
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

    /// Capacity and output provenance are checked under the current Character root lock.
    /// The proof, source debit, output placement and existing audit commit together.
    pub async fn commit_drome_cube(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemUseRequest,
        content: &dyn ItemLoopContent,
    ) -> Result<ItemUseOutcome> {
        if !matches!(request.kind, ItemUseKind::DromeCube { .. }) {
            return Err(ItemUseError::InvalidInput.into());
        }
        self.commit_use_inner(authority, node, fence, request, Some(content))
            .await
    }
}

/// Called by the existing transaction after its current recovery/session/root/equipment
/// fences. Refusals drop the whole transaction, including a prepared proof.
pub(super) async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
    content: Option<&dyn ItemLoopContent>,
) -> Result<()> {
    let ItemUseKind::DromeCube { wire_binding } = request.kind else {
        return Err(ItemUseError::InvalidInput.into());
    };
    let content = content.ok_or(ItemUseError::InvalidInput)?;
    let index = reward_index(request.source_item_instance_id, request.source_quantity_before)
        .ok_or(ItemUseError::InvalidInput)?;
    let selected = reward(index).ok_or(ItemUseError::InvalidInput)?;
    let output = flask_definition(request).ok_or(ItemUseError::InvalidInput)?;
    if request.source_definition.family != "Item"
        || request.source_definition.production_key != DROME_CUBE_KEY
        || output.family != "Item"
        || output.production_key != selected.production_key()
    {
        return Err(ItemUseError::InvalidInput.into());
    }
    let source_facts = content
        .loop_facts(&request.source_definition)
        .ok_or(ItemUseError::InvalidInput)?;
    let output_facts = content
        .loop_facts(output)
        .ok_or(ItemUseError::InvalidInput)?;
    if source_facts.definition != request.source_definition
        || output_facts.definition != *output
        || source_facts.container_capacity.is_some()
        || output_facts.container_capacity.is_some()
    {
        return Err(ItemUseError::InvalidInput.into());
    }
    let (source_maximum, _) =
        stack_maximum(source_facts.stack).map_err(|_| ItemUseError::InvalidInput)?;
    let (maximum, stackable) =
        stack_maximum(output_facts.stack).map_err(|_| ItemUseError::InvalidInput)?;
    if request.source_quantity_before > source_maximum {
        return Err(ItemUseError::InvalidInput.into());
    }
    if let ItemUseFlaskPlan::Stack { quantity_before, .. } = request.flask
        && (!stackable || quantity_before >= maximum)
    {
        return Err(DromeCubeError::Capacity);
    }
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
        .chain(inventory.nested.iter().map(|(_, entry)| entry));
    let source = entries
        .clone()
        .find(|entry| entry.item.item_instance_id == request.source_item_instance_id)
        .ok_or(ItemUseError::StaleState)?;
    if source.item.definition != request.source_definition
        || source.item.quantity != request.source_quantity_before
    {
        return Err(ItemUseError::StaleState.into());
    }
    let capacity = content
        .loop_facts(&bag.backpack.definition)
        .and_then(|facts| facts.container_capacity)
        .ok_or(ItemUseError::InvalidInput)?;
    plan::check_room(
        &inventory,
        source,
        capacity,
        matches!(request.flask, ItemUseFlaskPlan::New { .. }),
    )?;
    let weight = |definition: &TypedDefinitionRef| {
        content
            .weight_of(definition)
            .map(u64::from)
            .map_err(|_| ItemUseError::InvalidInput)
    };
    let mut total = weight(&bag.backpack.definition)?
        .checked_mul(u64::from(bag.backpack.quantity))
        .ok_or(ItemUseError::InvalidInput)?;
    for item in entries.map(|entry| &entry.item).chain(inventory.equipment.iter()) {
        let stack_weight = weight(&item.definition)?
            .checked_mul(u64::from(item.quantity))
            .ok_or(ItemUseError::InvalidInput)?;
        total = total
            .checked_add(stack_weight)
            .ok_or(ItemUseError::InvalidInput)?;
    }
    let after = total
        .checked_sub(weight(&source.item.definition)?)
        .and_then(|value| value.checked_add(u64::from(output_facts.weight?)))
        .ok_or(ItemUseError::InvalidInput)?;
    if after > read_derived_transfer_capacity_centi_oz(tx, fence.character_id).await? {
        return Err(DromeCubeError::Capacity);
    }
    sqlx::query(
        "INSERT INTO game_drome_cube_uses(transaction_id,wire_binding,reward_index) \
         VALUES(encode($1,'hex')::uuid,$2,$3)",
    )
    .bind(request.transaction_id.as_slice())
    .bind(wire_binding.as_slice())
    .bind(i16::from(index))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
