//! Durable consumption of one unit of a stack in the main backpack (ITEM-USE-1b, migration
//! 0081; decision `ITEM-USE-0` §4-§5, DUR-03 item-use amendment).
//!
//! [`DurabilityRoot::commit_item_use`] is one item-only one-item transaction under the holder's
//! current gameplay fence (composition rule 2: recovery fence, admission relation locks, the
//! live session, lease and scope, the scope assignment held by the current node incarnation).
//! It never advances the CharacterRevision. One record per CommandRef (game session, command
//! id) is kept forever: an exact replay answers from the record and never burns twice; the same
//! CommandRef with another request, or another TransactionId, conflicts and writes nothing.
//!
//! The caller plans the whole use at PREPARE (decision §4.1): the used stack `S`, its quantity,
//! and for a potion the flask output (a compatible stack `E` and its quantity, or a fresh item
//! `F` with its identity and ordinal, or the missing flask). This writer checks only those same
//! rows and answers [`ItemUseError::StaleState`] when any of them moved; it never searches
//! again. Nothing is minted: the unit leaves `S` and a flask unit is added elsewhere in the same
//! backpack. The record, the item changes, the entry changes and the one audit event (type 2,
//! `item_use`) commit together; the database guards of 0081 admit each change only with them.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    uuid_text,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::{
    self as mint_audit, ITEM_LIFECYCLE_LIVE, OneItemStateV1, SelectedType2Tuple,
    check_technical_text, check_uuid_v7,
};
use super::item_transfer::definition_message;
use super::item_transfer_audit::{
    GAMEITEM01_STACK_QUANTITY_MAX, ITEM_LIFECYCLE_RETIRED, OneItemCommandRefV1,
    OneItemContainerEntryV1,
};
use super::item_use_audit::{
    ITEM_USE_CAUSE_FOOD, ITEM_USE_CAUSE_POTION, ItemUseEventIdentity, ItemUseShape,
    OneItemUseCauseV1, OneItemUseFlaskV1, OneItemUseV1, encode_item_use_event,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::postgres::PgConnection;

/// The closed `ItemUseCause` kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemUseKind {
    Food,
    Potion,
}

impl ItemUseKind {
    const fn code(self) -> u32 {
        match self {
            Self::Food => ITEM_USE_CAUSE_FOOD,
            Self::Potion => ITEM_USE_CAUSE_POTION,
        }
    }
}

/// The flask output reserved at PREPARE (decision §4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemUseFlaskPlan {
    /// Food, or a potion whose content names no flask.
    None,
    /// A potion whose flask has no room (`PARITY_PENDING` until ITEM-MOVE-2b).
    Missing,
    /// One unit added to a compatible stack `E` of the same backpack.
    Stack {
        item_instance_id: [u8; 16],
        definition: TypedDefinitionRef,
        quantity_before: u32,
    },
    /// A fresh flask item `F` of quantity 1 in a new entry at `placement_ordinal`.
    New {
        item_instance_id: [u8; 16],
        definition: TypedDefinitionRef,
        placement_ordinal: u64,
    },
}

/// One use of one unit of the stack `source_item_instance_id`. The TransactionId, the event id
/// and the occurrence time are fixed by the caller and reused on retry. The session of the
/// CommandRef is the fence's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemUseRequest {
    pub kind: ItemUseKind,
    pub command_id: u64,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: String,
    pub source_item_instance_id: [u8; 16],
    pub source_definition: TypedDefinitionRef,
    pub source_quantity_before: u32,
    pub flask: ItemUseFlaskPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemUseOutcome {
    /// Written in the caller's transaction; durable only when it commits.
    Written { shape: ItemUseShape },
    /// The CommandRef already committed with this request: nothing more is written.
    AlreadyCommitted { shape: ItemUseShape },
}

#[derive(Debug)]
pub enum ItemUseError {
    InvalidInput,
    /// The holder's current gameplay fence does not hold; nothing was written.
    AuthorityRejected,
    /// The holder's root is not at the fence's expected CharacterRevision.
    CharacterRevisionMismatch,
    /// A row of the PREPARE plan moved (`STALE_STATE`); nothing was written.
    StaleState,
    /// The CommandRef or the TransactionId was committed by another request.
    ConflictingCommand,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for ItemUseError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for ItemUseError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}

impl std::fmt::Display for ItemUseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid item use input"),
            Self::AuthorityRejected => formatter.write_str("item use authority rejected"),
            Self::CharacterRevisionMismatch => {
                formatter.write_str("item use CharacterRevision mismatch")
            }
            Self::StaleState => formatter.write_str("item use plan is stale"),
            Self::ConflictingCommand => {
                formatter.write_str("item use command was committed by another request")
            }
            Self::Unavailable(error) => write!(formatter, "item use storage failed: {error:?}"),
        }
    }
}

impl std::error::Error for ItemUseError {}

type Result<T> = std::result::Result<T, ItemUseError>;

fn valid_definition(definition: &TypedDefinitionRef) -> bool {
    (1..=128).contains(&definition.family.len())
        && (1..=512).contains(&definition.production_key.len())
        && (1..=512).contains(&definition.revision_ref.len())
}

impl ItemUseRequest {
    fn validate(&self) -> Result<()> {
        let flask_valid = match (&self.flask, self.kind) {
            (ItemUseFlaskPlan::None, _) => true,
            (_, ItemUseKind::Food) => false,
            (ItemUseFlaskPlan::Missing, ItemUseKind::Potion) => true,
            (
                ItemUseFlaskPlan::Stack {
                    item_instance_id,
                    definition,
                    quantity_before,
                },
                ItemUseKind::Potion,
            ) => {
                check_uuid_v7(item_instance_id).is_ok()
                    && *item_instance_id != self.source_item_instance_id
                    && valid_definition(definition)
                    && *definition != self.source_definition
                    && (1..GAMEITEM01_STACK_QUANTITY_MAX).contains(quantity_before)
            }
            (
                ItemUseFlaskPlan::New {
                    item_instance_id,
                    definition,
                    placement_ordinal,
                },
                ItemUseKind::Potion,
            ) => {
                check_uuid_v7(item_instance_id).is_ok()
                    && *item_instance_id != self.source_item_instance_id
                    && valid_definition(definition)
                    && *definition != self.source_definition
                    && *placement_ordinal != 0
            }
        };
        if !flask_valid
            || self.command_id == 0
            || check_uuid_v7(&self.source_item_instance_id).is_err()
            || check_uuid_v7(&self.transaction_id).is_err()
            || check_uuid_v7(&self.event_id).is_err()
            || self.transaction_id == self.event_id
            || check_technical_text(&self.server_build_id).is_err()
            || self.occurred_at_unix_ms <= 0
            || !valid_definition(&self.source_definition)
            || !(1..=GAMEITEM01_STACK_QUANTITY_MAX).contains(&self.source_quantity_before)
        {
            return Err(ItemUseError::InvalidInput);
        }
        Ok(())
    }

    /// SHA-256 over the semantic request: a changed request for the same CommandRef conflicts.
    fn binding(&self) -> [u8; 32] {
        fn text(hasher: &mut Sha256, value: &str) {
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
        fn definition(hasher: &mut Sha256, value: &TypedDefinitionRef) {
            text(hasher, &value.family);
            text(hasher, &value.production_key);
            text(hasher, &value.revision_ref);
        }
        let mut hasher = Sha256::new();
        hasher.update(b"oteryn.item_use.request.v1");
        hasher.update(self.kind.code().to_be_bytes());
        hasher.update(self.command_id.to_be_bytes());
        text(&mut hasher, &self.content_revision);
        text(&mut hasher, &self.ruleset_revision);
        text(&mut hasher, &self.sim_revision);
        hasher.update(self.source_item_instance_id);
        definition(&mut hasher, &self.source_definition);
        hasher.update(self.source_quantity_before.to_be_bytes());
        match &self.flask {
            ItemUseFlaskPlan::None => hasher.update([0_u8]),
            ItemUseFlaskPlan::Missing => hasher.update([1_u8]),
            ItemUseFlaskPlan::Stack {
                item_instance_id,
                definition: flask,
                quantity_before,
            } => {
                hasher.update([2_u8]);
                hasher.update(item_instance_id);
                definition(&mut hasher, flask);
                hasher.update(quantity_before.to_be_bytes());
            }
            ItemUseFlaskPlan::New {
                item_instance_id,
                definition: flask,
                placement_ordinal,
            } => {
                hasher.update([3_u8]);
                hasher.update(item_instance_id);
                definition(&mut hasher, flask);
                hasher.update(placement_ordinal.to_be_bytes());
            }
        }
        hasher.finalize().into()
    }

    const fn shape(&self) -> ItemUseShape {
        match self.flask {
            ItemUseFlaskPlan::None => ItemUseShape::Burn,
            ItemUseFlaskPlan::Missing => ItemUseShape::BurnFlaskMissing,
            ItemUseFlaskPlan::Stack { .. } => ItemUseShape::FlaskStack,
            ItemUseFlaskPlan::New { .. } => ItemUseShape::FlaskNew,
        }
    }
}

const fn shape_code(shape: ItemUseShape) -> i16 {
    match shape {
        ItemUseShape::Burn => 1,
        ItemUseShape::BurnFlaskMissing => 2,
        ItemUseShape::FlaskStack => 3,
        ItemUseShape::FlaskNew => 4,
    }
}

fn shape_from_code(code: i16) -> std::result::Result<ItemUseShape, DurabilityError> {
    Ok(match code {
        1 => ItemUseShape::Burn,
        2 => ItemUseShape::BurnFlaskMissing,
        3 => ItemUseShape::FlaskStack,
        4 => ItemUseShape::FlaskNew,
        _ => return Err(DurabilityError::InvalidStoredState),
    })
}

impl DurabilityRoot {
    /// Commit one item use under its holder's current gameplay fence (see the module
    /// documentation). An exact replay of a committed CommandRef returns its result without
    /// reacquiring gameplay authority; every refusal writes nothing.
    pub async fn commit_item_use(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: ItemUseRequest,
    ) -> Result<ItemUseOutcome> {
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
                    match replay(&mut tx, &fence, &request).await {
                        Ok(Some(outcome)) => {
                            tx.commit(deadline).await?;
                            return Ok(Ok(outcome));
                        }
                        Ok(None) => {}
                        Err(ItemUseError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => {
                            if root.content_revision != request.content_revision
                                || root.ruleset_revision != request.ruleset_revision
                            {
                                return Ok(Err(ItemUseError::StaleState));
                            }
                        }
                        Err(CharacterProgressionError::CharacterRevisionMismatch) => {
                            return Ok(Err(ItemUseError::CharacterRevisionMismatch));
                        }
                        Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
                        Err(_) => return Ok(Err(ItemUseError::AuthorityRejected)),
                    }
                    let tuple = tx.tuple();
                    match write(&mut tx, &fence, &request, tuple).await {
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

/// The recorded result of an exact replay, a conflict for a CommandRef or TransactionId
/// committed by another request, or `None` for a new command.
async fn replay(
    connection: &mut PgConnection,
    fence: &CurrentCharacterGameplayFence,
    request: &ItemUseRequest,
) -> Result<Option<ItemUseOutcome>> {
    let rows = sqlx::query(
        "SELECT transaction_id::text AS transaction_id, event_id::text AS event_id, \
                request_binding, shape, \
                (game_session_id = encode($1,'hex')::uuid \
                 AND command_id = $2::text::numeric(20,0)) AS same_command \
           FROM game_item_use_records \
          WHERE (game_session_id = encode($1,'hex')::uuid \
                 AND command_id = $2::text::numeric(20,0)) \
             OR transaction_id = encode($3,'hex')::uuid \
             OR event_id = encode($4,'hex')::uuid",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(request.command_id.to_string())
    .bind(request.transaction_id.as_slice())
    .bind(request.event_id.as_slice())
    .fetch_all(&mut *connection)
    .await?;
    let [row] = rows.as_slice() else {
        return if rows.is_empty() {
            Ok(None)
        } else {
            Err(ItemUseError::ConflictingCommand)
        };
    };
    let transaction_id = uuid_text(&row.try_get::<String, _>("transaction_id")?)?;
    let event_id = uuid_text(&row.try_get::<String, _>("event_id")?)?;
    let binding: Vec<u8> = row.try_get("request_binding")?;
    if row.try_get::<bool, _>("same_command")?
        && transaction_id == request.transaction_id
        && event_id == request.event_id
        && binding.as_slice() == request.binding().as_slice()
    {
        Ok(Some(ItemUseOutcome::AlreadyCommitted {
            shape: shape_from_code(row.try_get("shape")?)?,
        }))
    } else {
        Err(ItemUseError::ConflictingCommand)
    }
}

struct LockedStack {
    quantity: u32,
    parent: [u8; 16],
    ordinal: u64,
}

/// Lock the item and read its entry of the holder's backpack when it is live, in the world, at
/// the planned definition and quantity. Anything else is a stale plan.
async fn lock_stack(
    connection: &mut PgConnection,
    holder: &[u8; 16],
    backpack: &[u8; 16],
    world_id: &[u8; 16],
    item_instance_id: &[u8; 16],
    definition: &TypedDefinitionRef,
    quantity_before: u32,
) -> Result<LockedStack> {
    let row = sqlx::query(
        "SELECT i.definition_family, i.definition_production_key, i.definition_revision_ref, \
                i.lifecycle, i.quantity, i.world_id::text AS world_id, \
                e.parent_item_instance_id::text AS parent, e.placement_ordinal::text AS ordinal \
           FROM game_item_instances i \
           JOIN game_item_container_entries e \
             ON e.item_instance_id = i.item_instance_id \
            AND e.character_id = encode($2,'hex')::uuid \
          WHERE i.item_instance_id = encode($1,'hex')::uuid FOR UPDATE OF i",
    )
    .bind(item_instance_id.as_slice())
    .bind(holder.as_slice())
    .fetch_optional(&mut *connection)
    .await?;
    let Some(row) = row else {
        return Err(ItemUseError::StaleState);
    };
    let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let parent = uuid_text(&row.try_get::<String, _>("parent")?)?;
    if row.try_get::<i16, _>("lifecycle")? != 1
        || quantity != quantity_before
        || parent != *backpack
        || uuid_text(&row.try_get::<String, _>("world_id")?)? != *world_id
        || row.try_get::<String, _>("definition_family")? != definition.family
        || row.try_get::<String, _>("definition_production_key")? != definition.production_key
        || row.try_get::<String, _>("definition_revision_ref")? != definition.revision_ref
    {
        return Err(ItemUseError::StaleState);
    }
    Ok(LockedStack {
        quantity,
        parent,
        ordinal: numeric_u64(&row, "ordinal")?,
    })
}

/// The use body, after the fence locked the holder's root in this transaction.
#[allow(clippy::too_many_lines)]
async fn write(
    connection: &mut PgConnection,
    fence: &CurrentCharacterGameplayFence,
    request: &ItemUseRequest,
    tuple: SelectedType2Tuple,
) -> Result<ItemUseOutcome> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(ItemUseError::AuthorityRejected);
    };
    let holder = *fence.character_id.as_bytes();
    let world = *world_id.as_bytes();
    if let Some(outcome) = replay(connection, fence, request).await? {
        return Ok(outcome);
    }
    let backpack = sqlx::query(
        "SELECT item_instance_id::text AS item_instance_id FROM game_item_container_slots \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(holder.as_slice())
    .fetch_optional(&mut *connection)
    .await?
    .map(|row| uuid_text(&row.try_get::<String, _>("item_instance_id")?))
    .transpose()?
    .ok_or(ItemUseError::StaleState)?;

    let source = lock_stack(
        connection,
        &holder,
        &backpack,
        &world,
        &request.source_item_instance_id,
        &request.source_definition,
        request.source_quantity_before,
    )
    .await?;
    let source_retires = source.quantity == 1;

    let flask_entry = match &request.flask {
        ItemUseFlaskPlan::Stack {
            item_instance_id,
            definition,
            quantity_before,
        } => {
            let stack = lock_stack(
                connection,
                &holder,
                &backpack,
                &world,
                item_instance_id,
                definition,
                *quantity_before,
            )
            .await?;
            Some(stack.ordinal)
        }
        ItemUseFlaskPlan::New {
            item_instance_id,
            placement_ordinal,
            ..
        } => {
            // The planned ordinal is above every ordinal of the backpack, the backpack keeps
            // room for the entry (ceiling 20), and the planned item does not exist yet.
            let state = sqlx::query(
                "SELECT COALESCE(max(placement_ordinal), 0)::text AS highest, count(*) AS entries, \
                        EXISTS (SELECT 1 FROM game_item_instances \
                                 WHERE item_instance_id = encode($2,'hex')::uuid) AS taken \
                   FROM game_item_container_entries \
                  WHERE parent_item_instance_id = encode($1,'hex')::uuid",
            )
            .bind(backpack.as_slice())
            .bind(item_instance_id.as_slice())
            .fetch_one(&mut *connection)
            .await?;
            let entries: i64 = state.try_get("entries")?;
            if state.try_get::<bool, _>("taken")?
                || *placement_ordinal <= numeric_u64(&state, "highest")?
                || entries + 1 - i64::from(source_retires) > 20
            {
                return Err(ItemUseError::StaleState);
            }
            Some(*placement_ordinal)
        }
        ItemUseFlaskPlan::None | ItemUseFlaskPlan::Missing => None,
    };

    let state =
        |item: &[u8; 16], definition: &TypedDefinitionRef, quantity: u32, lifecycle: u32| {
            OneItemStateV1 {
                item_instance_id: item.to_vec(),
                world_id: world.to_vec(),
                definition: Some(definition_message(definition)),
                quantity,
                lifecycle,
            }
        };
    let entry = |ordinal: u64| OneItemContainerEntryV1 {
        parent_item_instance_id: backpack.to_vec(),
        placement_ordinal: ordinal,
    };
    let source_after = source.quantity - 1;
    let flask = match &request.flask {
        ItemUseFlaskPlan::Stack {
            item_instance_id,
            definition,
            quantity_before,
        } => Some(OneItemUseFlaskV1 {
            before: Some(state(
                item_instance_id,
                definition,
                *quantity_before,
                ITEM_LIFECYCLE_LIVE,
            )),
            after: Some(state(
                item_instance_id,
                definition,
                quantity_before + 1,
                ITEM_LIFECYCLE_LIVE,
            )),
            entry: flask_entry.map(entry),
        }),
        ItemUseFlaskPlan::New {
            item_instance_id,
            definition,
            placement_ordinal,
        } => Some(OneItemUseFlaskV1 {
            before: None,
            after: Some(state(item_instance_id, definition, 1, ITEM_LIFECYCLE_LIVE)),
            entry: Some(entry(*placement_ordinal)),
        }),
        ItemUseFlaskPlan::None | ItemUseFlaskPlan::Missing => None,
    };
    let item_use = OneItemUseV1 {
        source_before: Some(state(
            &request.source_item_instance_id,
            &request.source_definition,
            source.quantity,
            ITEM_LIFECYCLE_LIVE,
        )),
        source_after: Some(state(
            &request.source_item_instance_id,
            &request.source_definition,
            source_after,
            if source_retires {
                ITEM_LIFECYCLE_RETIRED
            } else {
                ITEM_LIFECYCLE_LIVE
            },
        )),
        source_entry: Some(entry(source.ordinal)),
        cause: Some(OneItemUseCauseV1 {
            kind: request.kind.code(),
            content_revision: request.content_revision.clone(),
            ruleset_revision: request.ruleset_revision.clone(),
            sim_revision: request.sim_revision.clone(),
            command_ref: Some(OneItemCommandRefV1 {
                game_session_id: fence.game_session_id.as_bytes().to_vec(),
                command_id: request.command_id,
            }),
        }),
        character_id: holder.to_vec(),
        world_id: world.to_vec(),
        channel_id: channel_id.as_bytes().to_vec(),
        runtime_scope_ownership_generation: fence.scope_ownership_generation.get(),
        expected_session_generation: fence.connection_generation.get(),
        expected_game_session_id: fence.game_session_id.as_bytes().to_vec(),
        flask,
        flask_missing: request.flask == ItemUseFlaskPlan::Missing,
    };
    let envelope = encode_item_use_event(
        ItemUseEventIdentity {
            event_id: request.event_id,
            transaction_id: request.transaction_id,
            occurred_at_unix_ms: request.occurred_at_unix_ms,
            server_build_id: &request.server_build_id,
        },
        item_use,
        tuple.tuple(),
    )
    .map_err(|_| ItemUseError::InvalidInput)?;
    let shape = request.shape();

    // The record first: the item guards admit each change only with it. Its flask reference is
    // checked at commit, after the fresh flask item is created.
    sqlx::query(
        "INSERT INTO game_item_use_records (\
           transaction_id, event_id, cause_kind, game_session_id, command_id, content_revision, \
           ruleset_revision, sim_revision, request_binding, character_id, world_id, channel_id, \
           runtime_scope_ownership_generation, expected_session_generation, shape, \
           source_item_instance_id, source_definition_family, source_definition_production_key, \
           source_definition_revision_ref, source_parent_item_instance_id, \
           source_placement_ordinal, source_quantity_before, source_quantity_after, \
           flask_item_instance_id, flask_definition_family, flask_definition_production_key, \
           flask_definition_revision_ref, flask_placement_ordinal, flask_quantity_before, \
           flask_quantity_after, occurred_at, envelope_sha256) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, encode($4,'hex')::uuid, \
           $5::text::numeric(20,0), $6, $7, $8, $9, encode($10,'hex')::uuid, \
           encode($11,'hex')::uuid, encode($12,'hex')::uuid, $13::text::numeric(20,0), \
           $14::text::numeric(20,0), $15, encode($16,'hex')::uuid, $17, $18, $19, \
           encode($20,'hex')::uuid, $21::text::numeric(20,0), $22, $23, \
           encode($24,'hex')::uuid, $25, $26, $27, $28::text::numeric(20,0), $29, $30, $31, \
           sha256($32))",
    )
    .bind(request.transaction_id.as_slice())
    .bind(request.event_id.as_slice())
    .bind(i16::try_from(request.kind.code()).map_err(|_| ItemUseError::InvalidInput)?)
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(request.command_id.to_string())
    .bind(&request.content_revision)
    .bind(&request.ruleset_revision)
    .bind(&request.sim_revision)
    .bind(request.binding().as_slice())
    .bind(holder.as_slice())
    .bind(world.as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(fence.connection_generation.get().to_string())
    .bind(shape_code(shape))
    .bind(request.source_item_instance_id.as_slice())
    .bind(&request.source_definition.family)
    .bind(&request.source_definition.production_key)
    .bind(&request.source_definition.revision_ref)
    .bind(source.parent.as_slice())
    .bind(source.ordinal.to_string())
    .bind(i64::from(source.quantity))
    .bind(i64::from(source_after))
    .bind(match &request.flask {
        ItemUseFlaskPlan::Stack {
            item_instance_id, ..
        }
        | ItemUseFlaskPlan::New {
            item_instance_id, ..
        } => Some(item_instance_id.to_vec()),
        _ => None,
    })
    .bind(flask_definition(request).map(|definition| &definition.family))
    .bind(flask_definition(request).map(|definition| &definition.production_key))
    .bind(flask_definition(request).map(|definition| &definition.revision_ref))
    .bind(flask_entry.map(|ordinal| ordinal.to_string()))
    .bind(match &request.flask {
        ItemUseFlaskPlan::Stack {
            quantity_before, ..
        } => Some(i64::from(*quantity_before)),
        _ => None,
    })
    .bind(match &request.flask {
        ItemUseFlaskPlan::Stack {
            quantity_before, ..
        } => Some(i64::from(*quantity_before) + 1),
        ItemUseFlaskPlan::New { .. } => Some(1_i64),
        _ => None,
    })
    .bind(request.occurred_at_unix_ms)
    .bind(envelope.as_slice())
    .execute(&mut *connection)
    .await?;

    // The used stack: one unit leaves, the identity is kept or the stack retires at zero.
    let changed = sqlx::query(
        "UPDATE game_item_instances \
            SET quantity = $2, lifecycle = $3, last_transaction_id = encode($4,'hex')::uuid \
          WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1",
    )
    .bind(request.source_item_instance_id.as_slice())
    .bind(i64::from(source_after))
    .bind(if source_retires { 2_i16 } else { 1_i16 })
    .bind(request.transaction_id.as_slice())
    .execute(&mut *connection)
    .await?
    .rows_affected()
        == 1;
    if !changed {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    if source_retires {
        let removed = sqlx::query(
            "DELETE FROM game_item_container_entries \
              WHERE item_instance_id = encode($1,'hex')::uuid \
                AND character_id = encode($2,'hex')::uuid",
        )
        .bind(request.source_item_instance_id.as_slice())
        .bind(holder.as_slice())
        .execute(&mut *connection)
        .await?
        .rows_affected()
            == 1;
        if !removed {
            return Err(DurabilityError::InvalidStoredState.into());
        }
    }

    match &request.flask {
        ItemUseFlaskPlan::Stack {
            item_instance_id,
            quantity_before,
            ..
        } => {
            let grown = sqlx::query(
                "UPDATE game_item_instances \
                    SET quantity = $2, last_transaction_id = encode($3,'hex')::uuid \
                  WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1",
            )
            .bind(item_instance_id.as_slice())
            .bind(i64::from(*quantity_before) + 1)
            .bind(request.transaction_id.as_slice())
            .execute(&mut *connection)
            .await?
            .rows_affected()
                == 1;
            if !grown {
                return Err(DurabilityError::InvalidStoredState.into());
            }
        }
        ItemUseFlaskPlan::New {
            item_instance_id,
            definition,
            placement_ordinal,
        } => {
            sqlx::query(
                "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
                   definition_production_key, definition_revision_ref, quantity, lifecycle, \
                   minted_transaction_id) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, 1, 1, \
                   encode($6,'hex')::uuid)",
            )
            .bind(item_instance_id.as_slice())
            .bind(world.as_slice())
            .bind(&definition.family)
            .bind(&definition.production_key)
            .bind(&definition.revision_ref)
            .bind(request.transaction_id.as_slice())
            .execute(&mut *connection)
            .await?;
            sqlx::query(
                "INSERT INTO game_item_container_entries(item_instance_id, world_id, \
                   character_id, parent_item_instance_id, placement_ordinal, \
                   placed_transaction_id) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, \
                   encode($3,'hex')::uuid, encode($4,'hex')::uuid, $5::text::numeric(20,0), \
                   encode($6,'hex')::uuid)",
            )
            .bind(item_instance_id.as_slice())
            .bind(world.as_slice())
            .bind(holder.as_slice())
            .bind(backpack.as_slice())
            .bind(placement_ordinal.to_string())
            .bind(request.transaction_id.as_slice())
            .execute(&mut *connection)
            .await?;
        }
        ItemUseFlaskPlan::None | ItemUseFlaskPlan::Missing => {}
    }

    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
           transaction_count, event_type_id, schema_revision, retention_profile_id, \
           item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
           publication_state) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, \
           encode($6,'hex')::uuid, $7, $7 + $8, $9, sha256($9), 1)",
    )
    .bind(request.event_id.as_slice())
    .bind(request.transaction_id.as_slice())
    .bind(i64::from(mint_audit::EVENT_TYPE_ID))
    .bind(i64::from(tuple.schema_revision()))
    .bind(tuple.retention_profile_id())
    .bind(request.source_item_instance_id.as_slice())
    .bind(request.occurred_at_unix_ms)
    .bind(mint_audit::AUDIT_RETENTION_P90D_MS)
    .bind(envelope.as_slice())
    .execute(&mut *connection)
    .await?;
    Ok(ItemUseOutcome::Written { shape })
}

fn flask_definition(request: &ItemUseRequest) -> Option<&TypedDefinitionRef> {
    match &request.flask {
        ItemUseFlaskPlan::Stack { definition, .. } | ItemUseFlaskPlan::New { definition, .. } => {
            Some(definition)
        }
        _ => None,
    }
}
