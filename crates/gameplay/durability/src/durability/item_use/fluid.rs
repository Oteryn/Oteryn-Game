//! I10a: fill, pour or empty carried fluid containers (migration 0560).
//!
//! Crystal (201189f8) `data/scripts/actions/items/fluids.lua` keeps a container's content in the
//! item's sub-type and changes it in place: drinking sets it to 0, and using a filled container
//! on an empty one moves the fluid (`fluids.lua:39-48`). Here the sub-type is a
//! `game_item_fluid_states` row (none = empty) changed only with one
//! `game_item_fluid_transitions` record per changed container and CommandRef, under the holder's
//! current item fence and with each container's backpack path locked as an item use locks it.
use super::*;

/// One container's planned change from its current sub-type `fluid_before` (0 = empty) to
/// `fluid_after`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFluidChange {
    pub item_instance_id: [u8; 16],
    pub definition: TypedDefinitionRef,
    pub fluid_before: u8,
    pub fluid_after: u8,
}

/// One fluid command: the drunk container alone, or a pour from one container into another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFluidRequest {
    pub command_id: u64,
    pub transaction_id: [u8; 16],
    pub change: ItemFluidChange,
    pub other: Option<ItemFluidChange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemFluidOutcome {
    Written,
    /// The CommandRef already committed this request; nothing more is written.
    AlreadyCommitted,
}

impl ItemFluidChange {
    fn valid(&self) -> bool {
        check_uuid_v7(&self.item_instance_id).is_ok()
            && valid_definition(&self.definition)
            && self.fluid_before != self.fluid_after
    }
}

impl ItemFluidRequest {
    fn changes(&self) -> impl Iterator<Item = &ItemFluidChange> {
        std::iter::once(&self.change).chain(&self.other)
    }

    fn validate(&self) -> Result<()> {
        // A pour moves the whole fluid: one side empties, the other takes its sub-type.
        let pour = match &self.other {
            None => true,
            Some(other) => {
                other.item_instance_id != self.change.item_instance_id
                    && other.fluid_before == self.change.fluid_after
                    && other.fluid_after == self.change.fluid_before
                    && (self.change.fluid_before == 0 || self.change.fluid_after == 0)
            }
        };
        if self.command_id == 0
            || check_uuid_v7(&self.transaction_id).is_err()
            || !pour
            || !self.changes().all(ItemFluidChange::valid)
        {
            return Err(ItemUseError::InvalidInput);
        }
        Ok(())
    }
}

impl DurabilityRoot {
    /// Commit one fluid command on containers in the holder's backpack tree. Each container must
    /// be live, of the planned definition, a single item, and hold its `fluid_before`; anything
    /// else is `StaleState` with nothing written.
    pub async fn commit_item_fluid(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemFluidRequest,
    ) -> Result<ItemFluidOutcome> {
        request.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match assert_item_only_fence(&mut tx, &fence, &node).await? {
                        Ok(_) => {}
                        Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
                        Err(_) => return Ok(Err(ItemUseError::AuthorityRejected)),
                    }
                    match write_fluid(&mut tx, &fence, &request).await {
                        Ok(outcome) => {
                            tx.commit(deadline).await?;
                            Ok(Ok(outcome))
                        }
                        Err(ItemUseError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

/// The recorded transitions of this CommandRef or TransactionId as (transaction, item, before,
/// after); the request replays only when they are exactly its own.
async fn replay(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemFluidRequest,
    holder: &[u8; 16],
) -> Result<Option<ItemFluidOutcome>> {
    let rows = sqlx::query(
        "SELECT transaction_id::text AS transaction_id, character_id::text AS character_id, \
                item_instance_id::text AS item_instance_id, fluid_before, fluid_after \
           FROM game_item_fluid_transitions \
          WHERE (game_session_id = encode($1,'hex')::uuid \
                 AND command_id = $2::text::numeric(20,0)) \
             OR transaction_id = encode($3,'hex')::uuid \
          ORDER BY item_instance_id",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(request.command_id.to_string())
    .bind(request.transaction_id.as_slice())
    .fetch_all(&mut *connection)
    .await?;
    if rows.is_empty() {
        return Ok(None);
    }
    let mut recorded = Vec::with_capacity(rows.len());
    for row in &rows {
        if uuid_text(&row.try_get::<String, _>("transaction_id")?)? != request.transaction_id
            || uuid_text(&row.try_get::<String, _>("character_id")?)? != *holder
        {
            return Err(ItemUseError::ConflictingCommand);
        }
        recorded.push((
            uuid_text(&row.try_get::<String, _>("item_instance_id")?)?,
            row.try_get::<i16, _>("fluid_before")?,
            row.try_get::<i16, _>("fluid_after")?,
        ));
    }
    let mut planned: Vec<_> = request
        .changes()
        .map(|change| {
            (
                change.item_instance_id,
                i16::from(change.fluid_before),
                i16::from(change.fluid_after),
            )
        })
        .collect();
    planned.sort_unstable();
    if recorded == planned {
        Ok(Some(ItemFluidOutcome::AlreadyCommitted))
    } else {
        Err(ItemUseError::ConflictingCommand)
    }
}

async fn write_fluid(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemFluidRequest,
) -> Result<ItemFluidOutcome> {
    let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
        return Err(ItemUseError::AuthorityRejected);
    };
    let holder = *fence.character_id.as_bytes();
    let world = *world_id.as_bytes();
    if let Some(outcome) = replay(connection, fence, request, &holder).await? {
        return Ok(outcome);
    }
    let backpack = sqlx::query_scalar::<_, String>(
        "SELECT item_instance_id::text FROM game_item_container_slots \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(holder.as_slice())
    .fetch_optional(&mut *connection)
    .await?
    .map(|id| uuid_text(&id))
    .transpose()?
    .ok_or(ItemUseError::StaleState)?;
    // Both containers and their ancestry are locked in one UUID-ordered pass.
    carried_source::lock_carried_path(
        connection,
        &holder,
        &backpack,
        &world,
        &request.change.item_instance_id,
        request.other.as_ref().map(|other| other.item_instance_id),
        None,
    )
    .await?;
    for change in request.changes() {
        let current = sqlx::query(
            "SELECT i.definition_family, i.definition_production_key, \
                    i.definition_revision_ref, i.quantity, \
                    (SELECT f.fluid_subtype FROM game_item_fluid_states f \
                      WHERE f.item_instance_id = i.item_instance_id FOR UPDATE) AS fluid \
               FROM game_item_instances i \
              WHERE i.item_instance_id = encode($1,'hex')::uuid AND i.lifecycle = 1",
        )
        .bind(change.item_instance_id.as_slice())
        .fetch_optional(&mut *connection)
        .await?
        .ok_or(ItemUseError::StaleState)?;
        let definition = TypedDefinitionRef {
            family: current.try_get("definition_family")?,
            production_key: current.try_get("definition_production_key")?,
            revision_ref: current.try_get("definition_revision_ref")?,
        };
        let fluid = current.try_get::<Option<i16>, _>("fluid")?.unwrap_or(0);
        if definition != change.definition
            || current.try_get::<i64, _>("quantity")? != 1
            || fluid != i16::from(change.fluid_before)
        {
            return Err(ItemUseError::StaleState);
        }
    }
    for change in request.changes() {
        write_change(connection, fence, request, change, &holder, &world).await?;
    }
    Ok(ItemFluidOutcome::Written)
}

async fn write_change(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemFluidRequest,
    change: &ItemFluidChange,
    holder: &[u8; 16],
    world: &[u8; 16],
) -> Result<()> {
    sqlx::query(
        "INSERT INTO game_item_fluid_transitions(transaction_id, game_session_id, command_id, \
           character_id, item_instance_id, fluid_before, fluid_after) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3::text::numeric(20,0), \
                 encode($4,'hex')::uuid, encode($5,'hex')::uuid, $6, $7)",
    )
    .bind(request.transaction_id.as_slice())
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(request.command_id.to_string())
    .bind(holder.as_slice())
    .bind(change.item_instance_id.as_slice())
    .bind(i16::from(change.fluid_before))
    .bind(i16::from(change.fluid_after))
    .execute(&mut *connection)
    .await?;
    let write = match (change.fluid_before, change.fluid_after) {
        (_, 0) => sqlx::query(
            "DELETE FROM game_item_fluid_states WHERE item_instance_id = encode($1,'hex')::uuid",
        )
        .bind(change.item_instance_id.as_slice()),
        (0, after) => sqlx::query(
            "INSERT INTO game_item_fluid_states(item_instance_id, world_id, fluid_subtype, \
               last_transaction_id) \
             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, encode($4,'hex')::uuid)",
        )
        .bind(change.item_instance_id.as_slice())
        .bind(world.as_slice())
        .bind(i16::from(after))
        .bind(request.transaction_id.as_slice()),
        (_, after) => sqlx::query(
            "UPDATE game_item_fluid_states SET fluid_subtype = $3, \
               last_transaction_id = encode($4,'hex')::uuid \
             WHERE item_instance_id = encode($1,'hex')::uuid AND world_id = encode($2,'hex')::uuid",
        )
        .bind(change.item_instance_id.as_slice())
        .bind(world.as_slice())
        .bind(i16::from(after))
        .bind(request.transaction_id.as_slice()),
    };
    if write.execute(&mut *connection).await?.rows_affected() != 1 {
        return Err(ItemUseError::StaleState);
    }
    Ok(())
}
