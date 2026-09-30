//! In-transaction gold fee BURN (GOLD-FEE-1a, migration 0023; decision
//! `CHARACTER-GOLD-FEE-BOUNDARY-V1` §4, owner decisions D174-D178).
//!
//! A fee source (CHARM-6 `CharmUnassign` first) calls [`burn_fee_in_transaction`] inside its own
//! Character transaction, after its gameplay fence locked the Character root at
//! `fence.expected_character_revision`. The source advances the root to the next revision with
//! its one receipt in the same transaction, before or after this call; the database refuses the
//! burn at commit otherwise. The burn plans over the gold coin stacks in direct entries of the
//! equipped main backpack (decision §4.2), writes its record, its BURN lines, the item changes,
//! the whole-burn entry removals and its one audit event, and never commits: any error leaves
//! the source's transaction to roll back, so a rejection writes nothing.
//!
//! GOLD-FEE-1a admits gold coins only. Gold has the lowest worth, so a burn committed here is
//! exactly the decision's plan and never has change; a fee the gold cannot pay is refused with
//! [`FeeBurnError::InsufficientFunds`] until GOLD-FEE-1b admits platinum and crystal with their
//! change MINT.

use super::DurabilityError;
use super::character_progression::{CurrentCharacterGameplayFence, numeric_u64, uuid_text};
use super::charm_state::CharmCommandOccurrence;
use super::item_fee_burn_audit::{
    FEE_GOLD_UNITS_MAX, FeeBurnCauseV1, FeeBurnEventIdentity, OneItemCharmUnassignV1,
    OneItemFeeBurnCauseV1, OneItemFeeBurnLineV1, OneItemFeeBurnV1, encode_fee_burn_event,
};
use super::item_mint_audit::{
    self as mint_audit, ITEM_LIFECYCLE_LIVE, OneItemStateV1, OneItemTypedDefinitionRevisionV1,
    check_technical_text, check_uuid_v7,
};
use super::item_transfer_audit::ITEM_LIFECYCLE_RETIRED;
use crate::domain::charm::CharmKey;
use crate::domain::currency::{COIN_DEFINITION_FAMILY, Coin, CoinStack, FeePlanError, plan_fee};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::postgres::PgConnection;

const REQUEST_BINDING_VERSION: u8 = 1;
const CAUSE_CHARM_UNASSIGN: i16 = 1;
/// GOLD-FEE-1a: the admitted inputs (decision §4.2 burns gold first in any case).
const ADMITTED_COIN: Coin = Coin::Gold;

/// The closed `FeeBurnCause` (decision §4.4): one variant per admitted fee source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeeBurnCause {
    CharmUnassign {
        charm: CharmKey,
        occurrence: CharmCommandOccurrence,
    },
}

/// One fee. The TransactionId and EventId are fixed by the source before its first attempt and
/// reused on every retry of the same occurrence (decision §4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeBurnRequest {
    pub cause: FeeBurnCause,
    /// The fee `F` in gold units, computed by the source from facts read after the root lock.
    pub fee_gold_units: u64,
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: String,
}

/// One burned stack: the whole stack when `quantity_after` is 0 (the item retires and its entry
/// ends), otherwise the last line, keeping its entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BurnedCoinStack {
    pub item_instance_id: [u8; 16],
    pub placement_ordinal: u64,
    pub quantity_before: u32,
    pub quantity_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedFeeBurn {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub character_id: CharacterId,
    pub committed_character_revision: CharacterRevision,
    pub fee_gold_units: u64,
    pub lines: Vec<BurnedCoinStack>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeeBurnOutcome {
    /// Written in the caller's transaction; durable only when the caller commits.
    Burned(CommittedFeeBurn),
    /// The occurrence already burned with the same binding: the caller must return its own
    /// retained outcome and must not commit a second Character change.
    AlreadyBurned(CommittedFeeBurn),
}

#[derive(Debug)]
pub enum FeeBurnError {
    InvalidInput,
    /// The locked root is not the fenced Character at the expected or next revision.
    CharacterMismatch,
    InsufficientFunds,
    CapacityExceeded,
    ChangeDoesNotFit,
    ConflictingOccurrence,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for FeeBurnError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for FeeBurnError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}

impl std::fmt::Display for FeeBurnError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid fee BURN input"),
            Self::CharacterMismatch => formatter.write_str("fee BURN Character does not match"),
            Self::InsufficientFunds => formatter.write_str("insufficient coins for the fee"),
            Self::CapacityExceeded => formatter.write_str("fee BURN needs too many coin stacks"),
            Self::ChangeDoesNotFit => formatter.write_str("fee change does not fit"),
            Self::ConflictingOccurrence => {
                formatter.write_str("fee occurrence was reused with a different binding")
            }
            Self::Unavailable(error) => write!(formatter, "fee BURN storage failed: {error:?}"),
        }
    }
}

impl std::error::Error for FeeBurnError {}

type Result<T> = std::result::Result<T, FeeBurnError>;

/// Burn `request.fee_gold_units` inside the caller's fenced Character transaction (see the
/// module documentation). Exact occurrence replay returns the retained outcome; a changed
/// binding conflicts. Every error requires the caller to roll back.
pub async fn burn_fee_in_transaction(
    connection: &mut PgConnection,
    fence: &CurrentCharacterGameplayFence,
    request: &FeeBurnRequest,
) -> Result<FeeBurnOutcome> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(FeeBurnError::InvalidInput);
    };
    let FeeBurnCause::CharmUnassign { charm, occurrence } = &request.cause;
    let committed = fence
        .expected_character_revision
        .get()
        .checked_add(1)
        .and_then(|value| CharacterRevision::new(value).ok())
        .ok_or(FeeBurnError::InvalidInput)?;
    if !(1..=FEE_GOLD_UNITS_MAX).contains(&request.fee_gold_units)
        || request.occurred_at_unix_ms <= 0
        || check_uuid_v7(&request.transaction_id).is_err()
        || check_uuid_v7(&request.event_id).is_err()
        || request.transaction_id == request.event_id
        || check_technical_text(&request.server_build_id).is_err()
    {
        return Err(FeeBurnError::InvalidInput);
    }
    let binding = request_binding(fence, request);

    let stored = sqlx::query(
        "SELECT transaction_id::text, event_id::text, request_binding, \
                committed_character_revision::text, fee_gold_units \
           FROM game_item_fee_burns \
          WHERE cause_kind = $1 AND cause_occurrence_id = encode($2,'hex')::uuid",
    )
    .bind(CAUSE_CHARM_UNASSIGN)
    .bind(occurrence.as_bytes().as_slice())
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(row) = stored {
        if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
            return Err(FeeBurnError::ConflictingOccurrence);
        }
        let transaction_id = uuid_text(&row.try_get::<String, _>("transaction_id")?)?;
        return Ok(FeeBurnOutcome::AlreadyBurned(CommittedFeeBurn {
            transaction_id,
            event_id: uuid_text(&row.try_get::<String, _>("event_id")?)?,
            character_id: fence.character_id,
            committed_character_revision: CharacterRevision::new(numeric_u64(
                &row,
                "committed_character_revision",
            )?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
            fee_gold_units: u64::try_from(row.try_get::<i64, _>("fee_gold_units")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            lines: load_lines(connection, &transaction_id).await?,
        }));
    }

    // The source holds this lock already; taking it again proves the row is the fenced one.
    let root = sqlx::query(
        "SELECT world_id::text, character_revision::text FROM game_character_roots \
          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(FeeBurnError::CharacterMismatch)?;
    let revision = numeric_u64(&root, "character_revision")?;
    if uuid_text(&root.try_get::<String, _>("world_id")?)? != *world_id.as_bytes()
        || (revision != fence.expected_character_revision.get() && revision != committed.get())
    {
        return Err(FeeBurnError::CharacterMismatch);
    }

    let backpack = sqlx::query(
        "SELECT item_instance_id::text FROM game_item_container_slots \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut *connection)
    .await?;
    let Some(backpack) = backpack else {
        return Err(FeeBurnError::InsufficientFunds);
    };
    let backpack = uuid_text(&backpack.try_get::<String, _>("item_instance_id")?)?;

    // Every direct entry counts against the 20 entries; the admitted coins are the inputs.
    let entries = sqlx::query(
        "SELECT i.item_instance_id::text, e.placement_ordinal::text, i.world_id::text, \
                i.definition_family, i.definition_production_key, i.definition_revision_ref, \
                i.quantity, i.lifecycle \
           FROM game_item_container_entries e \
           JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id \
          WHERE e.parent_item_instance_id = encode($1,'hex')::uuid \
          ORDER BY e.placement_ordinal DESC FOR UPDATE OF i",
    )
    .bind(backpack.as_slice())
    .fetch_all(&mut *connection)
    .await?;
    let mut inputs = Vec::new();
    for row in &entries {
        let family: String = row.try_get("definition_family")?;
        let key: String = row.try_get("definition_production_key")?;
        if family != COIN_DEFINITION_FAMILY
            || Coin::from_production_key(&key) != Some(ADMITTED_COIN)
            || row.try_get::<i16, _>("lifecycle")? != 1
        {
            continue;
        }
        if uuid_text(&row.try_get::<String, _>("world_id")?)? != *world_id.as_bytes() {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        inputs.push((
            CoinStack {
                coin: ADMITTED_COIN,
                quantity,
                placement_ordinal: numeric_u64(row, "placement_ordinal")?,
            },
            uuid_text(&row.try_get::<String, _>("item_instance_id")?)?,
            row.try_get::<String, _>("definition_revision_ref")?,
        ));
    }
    let stacks: Vec<CoinStack> = inputs.iter().map(|(stack, _, _)| *stack).collect();
    let plan = match plan_fee(request.fee_gold_units, &stacks, entries.len()) {
        Ok(plan) => plan,
        Err(FeePlanError::InsufficientFunds) => return Err(FeeBurnError::InsufficientFunds),
        Err(FeePlanError::CapacityExceeded) => return Err(FeeBurnError::CapacityExceeded),
        Err(FeePlanError::ChangeDoesNotFit) => return Err(FeeBurnError::ChangeDoesNotFit),
        // Stored stacks outside 1..=100 or repeated ordinals are not a caller error.
        Err(FeePlanError::InvalidInput) => return Err(DurabilityError::InvalidStoredState.into()),
    };
    if plan.change != 0 {
        return Err(DurabilityError::InvalidStoredState.into());
    }

    let lines: Vec<BurnedCoinStack> = plan
        .lines
        .iter()
        .map(|line| {
            let (stack, item, _) = &inputs[line.input];
            BurnedCoinStack {
                item_instance_id: *item,
                placement_ordinal: stack.placement_ordinal,
                quantity_before: stack.quantity,
                quantity_after: stack.quantity - line.burned,
            }
        })
        .collect();
    let state = |line: &BurnedCoinStack, revision: &str, quantity: u32| OneItemStateV1 {
        item_instance_id: line.item_instance_id.to_vec(),
        world_id: world_id.as_bytes().to_vec(),
        definition: Some(OneItemTypedDefinitionRevisionV1 {
            family: COIN_DEFINITION_FAMILY.into(),
            production_key: ADMITTED_COIN.production_key().into(),
            revision_ref: revision.into(),
        }),
        quantity,
        lifecycle: if quantity == 0 {
            ITEM_LIFECYCLE_RETIRED
        } else {
            ITEM_LIFECYCLE_LIVE
        },
    };
    let audit_lines = plan
        .lines
        .iter()
        .zip(&lines)
        .map(|(planned, line)| {
            let revision = &inputs[planned.input].2;
            OneItemFeeBurnLineV1 {
                before: Some(state(line, revision, line.quantity_before)),
                after: Some(state(line, revision, line.quantity_after)),
                placement_ordinal: line.placement_ordinal,
            }
        })
        .collect();
    let envelope = encode_fee_burn_event(
        FeeBurnEventIdentity {
            event_id: request.event_id,
            transaction_id: request.transaction_id,
            occurred_at_unix_ms: request.occurred_at_unix_ms,
            server_build_id: &request.server_build_id,
        },
        OneItemFeeBurnV1 {
            cause: Some(OneItemFeeBurnCauseV1 {
                cause: Some(FeeBurnCauseV1::CharmUnassign(OneItemCharmUnassignV1 {
                    charm_key: charm.as_str().into(),
                    occurrence: occurrence.as_bytes().to_vec(),
                })),
            }),
            fee_gold_units: request.fee_gold_units,
            burned_gold_units: request.fee_gold_units,
            character_id: fence.character_id.as_bytes().to_vec(),
            committed_character_revision: committed.get(),
            world_id: world_id.as_bytes().to_vec(),
            channel_id: channel_id.as_bytes().to_vec(),
            runtime_scope_ownership_generation: fence.scope_ownership_generation.get(),
            backpack_item_instance_id: backpack.to_vec(),
            lines: audit_lines,
        },
    )
    .map_err(|_| FeeBurnError::InvalidInput)?;

    let fee = i64::try_from(request.fee_gold_units).map_err(|_| FeeBurnError::InvalidInput)?;
    sqlx::query(
        "INSERT INTO game_item_fee_burns(transaction_id, event_id, cause_kind, \
           cause_occurrence_id, charm_key, request_binding, character_id, world_id, channel_id, \
           runtime_scope_ownership_generation, committed_character_revision, \
           backpack_item_instance_id, fee_gold_units, burned_gold_units, change_gold_units, \
           line_count, occurred_at, envelope_sha256, committed_at) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, encode($4,'hex')::uuid, \
           $5, $6, encode($7,'hex')::uuid, encode($8,'hex')::uuid, encode($9,'hex')::uuid, \
           $10::text::numeric(20,0), $11::text::numeric(20,0), encode($12,'hex')::uuid, \
           $13, $13, 0, $14, $15, sha256($16), \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(request.transaction_id.as_slice())
    .bind(request.event_id.as_slice())
    .bind(CAUSE_CHARM_UNASSIGN)
    .bind(occurrence.as_bytes().as_slice())
    .bind(charm.as_str())
    .bind(binding.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(committed.get().to_string())
    .bind(backpack.as_slice())
    .bind(fee)
    .bind(i16::try_from(lines.len()).map_err(|_| FeeBurnError::CapacityExceeded)?)
    .bind(request.occurred_at_unix_ms)
    .bind(envelope.as_slice())
    .execute(&mut *connection)
    .await?;

    for (ordinal, line) in (1_i16..).zip(&lines) {
        sqlx::query(
            "INSERT INTO game_item_fee_burn_lines(transaction_id, line_ordinal, item_instance_id, \
               placement_ordinal, coin_worth, quantity_before, quantity_after) \
             VALUES (encode($1,'hex')::uuid, $2, encode($3,'hex')::uuid, \
               $4::text::numeric(20,0), $5, $6, $7)",
        )
        .bind(request.transaction_id.as_slice())
        .bind(ordinal)
        .bind(line.item_instance_id.as_slice())
        .bind(line.placement_ordinal.to_string())
        .bind(i64::try_from(ADMITTED_COIN.worth()).map_err(|_| FeeBurnError::InvalidInput)?)
        .bind(i64::from(line.quantity_before))
        .bind(i64::from(line.quantity_after))
        .execute(&mut *connection)
        .await?;
        let changed = sqlx::query(
            "UPDATE game_item_instances \
                SET quantity = $2, lifecycle = $3, last_transaction_id = encode($4,'hex')::uuid \
              WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1 AND quantity = $5",
        )
        .bind(line.item_instance_id.as_slice())
        .bind(i64::from(line.quantity_after))
        .bind(if line.quantity_after == 0 { 2_i16 } else { 1 })
        .bind(request.transaction_id.as_slice())
        .bind(i64::from(line.quantity_before))
        .execute(&mut *connection)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        if line.quantity_after == 0 {
            let removed = sqlx::query(
                "DELETE FROM game_item_container_entries \
                  WHERE item_instance_id = encode($1,'hex')::uuid \
                    AND parent_item_instance_id = encode($2,'hex')::uuid \
                    AND placement_ordinal = $3::text::numeric(20,0)",
            )
            .bind(line.item_instance_id.as_slice())
            .bind(backpack.as_slice())
            .bind(line.placement_ordinal.to_string())
            .execute(&mut *connection)
            .await?;
            if removed.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState.into());
            }
        }
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
    .bind(i64::from(mint_audit::EVENT_SCHEMA_REVISION))
    .bind(mint_audit::RETENTION_PROFILE_ID)
    .bind(lines[0].item_instance_id.as_slice())
    .bind(request.occurred_at_unix_ms)
    .bind(mint_audit::AUDIT_RETENTION_P90D_MS)
    .bind(envelope.as_slice())
    .execute(&mut *connection)
    .await?;

    Ok(FeeBurnOutcome::Burned(CommittedFeeBurn {
        transaction_id: request.transaction_id,
        event_id: request.event_id,
        character_id: fence.character_id,
        committed_character_revision: committed,
        fee_gold_units: request.fee_gold_units,
        lines,
    }))
}

/// SHA-256 over the semantic request: the cause, the Character, its World and the fee. The
/// TransactionId, EventId, time and build are retry-local and excluded.
fn request_binding(fence: &CurrentCharacterGameplayFence, request: &FeeBurnRequest) -> Vec<u8> {
    let FeeBurnCause::CharmUnassign { charm, occurrence } = &request.cause;
    let mut digest = Sha256::new();
    digest.update([REQUEST_BINDING_VERSION]);
    digest.update(CAUSE_CHARM_UNASSIGN.to_be_bytes());
    digest.update(occurrence.as_bytes());
    digest.update(
        u32::try_from(charm.as_str().len())
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );
    digest.update(charm.as_str().as_bytes());
    digest.update(fence.character_id.as_bytes());
    digest.update(fence.runtime_scope.world_id().as_bytes());
    digest.update(request.fee_gold_units.to_be_bytes());
    digest.finalize().to_vec()
}

async fn load_lines(
    connection: &mut PgConnection,
    transaction_id: &[u8; 16],
) -> Result<Vec<BurnedCoinStack>> {
    let rows = sqlx::query(
        "SELECT item_instance_id::text, placement_ordinal::text, quantity_before, \
                quantity_after \
           FROM game_item_fee_burn_lines \
          WHERE transaction_id = encode($1,'hex')::uuid ORDER BY line_ordinal",
    )
    .bind(transaction_id.as_slice())
    .fetch_all(&mut *connection)
    .await?;
    rows.iter()
        .map(|row| {
            let quantity = |column: &str| -> Result<u32> {
                u32::try_from(row.try_get::<i64, _>(column)?)
                    .map_err(|_| DurabilityError::InvalidStoredState.into())
            };
            Ok(BurnedCoinStack {
                item_instance_id: uuid_text(&row.try_get::<String, _>("item_instance_id")?)?,
                placement_ordinal: numeric_u64(row, "placement_ordinal")?,
                quantity_before: quantity("quantity_before")?,
                quantity_after: quantity("quantity_after")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use crate::foundation::{
        ChannelId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration, WorldId,
    };

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    fn fence() -> CurrentCharacterGameplayFence {
        CurrentCharacterGameplayFence {
            character_id: CharacterId::from_bytes(id(1)).unwrap(),
            game_session_id: GameSessionId::decode(&id(2)).unwrap(),
            connection_generation: ConnectionGeneration::new(1).unwrap(),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&id(3)).unwrap(),
                ChannelId::decode(&id(4)).unwrap(),
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).unwrap(),
            expected_character_revision: CharacterRevision::new(1).unwrap(),
        }
    }

    fn request() -> FeeBurnRequest {
        FeeBurnRequest {
            cause: FeeBurnCause::CharmUnassign {
                charm: CharmKey::new("oteryn:charm.wound").unwrap(),
                occurrence: CharmCommandOccurrence::from_bytes(id(5)).unwrap(),
            },
            fee_gold_units: 100,
            transaction_id: id(6),
            event_id: id(7),
            occurred_at_unix_ms: 1,
            server_build_id: "build".into(),
        }
    }

    #[test]
    fn binding_covers_cause_character_world_and_fee_but_not_retry_identity() {
        let base = request_binding(&fence(), &request());
        let mut retried = request();
        retried.transaction_id = id(8);
        retried.event_id = id(9);
        retried.occurred_at_unix_ms = 2;
        retried.server_build_id = "other".into();
        assert_eq!(request_binding(&fence(), &retried), base);
        let mut other = request();
        other.fee_gold_units = 101;
        assert_ne!(request_binding(&fence(), &other), base);
        let mut other = request();
        other.cause = FeeBurnCause::CharmUnassign {
            charm: CharmKey::new("oteryn:charm.dodge").unwrap(),
            occurrence: CharmCommandOccurrence::from_bytes(id(5)).unwrap(),
        };
        assert_ne!(request_binding(&fence(), &other), base);
        let mut moved = fence();
        moved.character_id = CharacterId::from_bytes(id(10)).unwrap();
        assert_ne!(request_binding(&moved, &request()), base);
    }
}
