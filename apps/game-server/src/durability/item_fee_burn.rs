//! In-transaction coin fee BURN with its change MINT (GOLD-FEE-1a/1b, migrations 0023 and 0031;
//! decision
//! `CHARACTER-GOLD-FEE-BOUNDARY-V1` §4, owner decisions D174-D178).
//!
//! A fee source (CHARM-6 `CharmUnassign` first) calls [`burn_fee_in_transaction`] inside its own
//! Character transaction, after its gameplay fence locked the Character root at
//! `fence.expected_character_revision`. The source advances the root to the next revision with
//! its one receipt in the same transaction, before or after this call; the database refuses the
//! burn at commit otherwise. The source runs in its Character's revision slot (CHAR-REV-SEQ-1), so the
//! burn is sequenced with it; only `charm_state.rs` may call it (structural test in
//! `character_revision_sequencer.rs`). The burn plans over the gold, platinum and crystal coin stacks in
//! direct entries of the equipped main backpack (decision §4.2), writes its record, its BURN
//! lines, the item changes, the whole-burn entry removals, the change MINT (at most a platinum
//! and a gold stack, each a fresh item in a new entry after the burn lines) and its one audit
//! event, and never commits: any error leaves the source's transaction to roll back, so a
//! rejection writes nothing.

use super::DurabilityError;
use super::character_progression::{CurrentCharacterGameplayFence, numeric_u64, uuid_text};
use super::charm_state::CharmCommandOccurrence;
use super::item_fee_burn_audit::{
    FEE_GOLD_UNITS_MAX, FeeBurnCauseV1, FeeBurnEventIdentity, OneItemCharmUnassignV1,
    OneItemFeeBurnCauseV1, OneItemFeeBurnLineV1, OneItemFeeBurnV1, OneItemFeeChangeMintV1,
    encode_fee_burn_event,
};
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::{
    self as mint_audit, ITEM_LIFECYCLE_LIVE, OneItemStateV1, OneItemTypedDefinitionRevisionV1,
    check_technical_text, check_uuid_v7,
};
use super::item_transfer::ItemDefinitionFacts;
use super::item_transfer_audit::ITEM_LIFECYCLE_RETIRED;
use crate::domain::charm::CharmKey;
use crate::domain::currency::{
    BACKPACK_ENTRIES_MAX, COIN_DEFINITION_FAMILY, Coin, CoinStack, FeePlanError, plan_fee_within,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::postgres::PgConnection;

const REQUEST_BINDING_VERSION: u8 = 1;
const CAUSE_CHARM_UNASSIGN: i16 = 1;

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
    pub change: FeeChangeFacts,
}

/// What the change MINT needs (decision §4.2 step 4, §4.3), supplied by the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeChangeFacts {
    /// Platinum, then gold: the two output identity slots, fixed with the TransactionId before
    /// the first attempt and reused on every retry. A slot the plan does not need stays unused.
    pub item_instance_ids: [[u8; 16]; 2],
    /// The current compatible definitions of the three coins: every live coin stack of the
    /// backpack must be at its coin's revision (else the fee is refused), and the change is
    /// minted at the platinum and gold ones.
    pub platinum: TypedDefinitionRef,
    pub gold: TypedDefinitionRef,
    pub crystal: TypedDefinitionRef,
    /// The main backpack's definition: its declared capacity bounds the change.
    pub backpack: ItemDefinitionFacts,
}

/// One change output: a fresh live coin stack in a new backpack entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MintedCoinStack {
    pub item_instance_id: [u8; 16],
    pub coin: Coin,
    pub quantity: u32,
    pub placement_ordinal: u64,
}

/// One burned stack: the whole stack when `quantity_after` is 0 (the item retires and its entry
/// ends), otherwise the last line, keeping its entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BurnedCoinStack {
    pub item_instance_id: [u8; 16],
    pub coin: Coin,
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
    /// Burned worth minus the fee.
    pub change_gold_units: u64,
    /// Platinum first, then gold; each only when its count is positive.
    pub change: Vec<MintedCoinStack>,
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
        || !valid_change_facts(request)
    {
        return Err(FeeBurnError::InvalidInput);
    }
    let binding = request_binding(fence, request);

    let stored = sqlx::query(
        "SELECT transaction_id::text, event_id::text, request_binding, \
                committed_character_revision::text, fee_gold_units, change_gold_units, \
                change_platinum_item_instance_id::text, change_gold_item_instance_id::text, \
                change_placement_ordinal::text \
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
        let change_gold_units = u64::try_from(row.try_get::<i64, _>("change_gold_units")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let change = match row.try_get::<Option<String>, _>("change_placement_ordinal")? {
            None => Vec::new(),
            Some(ordinal) => change_outputs(
                [
                    uuid_text(&row.try_get::<String, _>("change_platinum_item_instance_id")?)?,
                    uuid_text(&row.try_get::<String, _>("change_gold_item_instance_id")?)?,
                ],
                change_gold_units,
                ordinal
                    .parse()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            ),
        };
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
            change_gold_units,
            change,
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
        "SELECT i.item_instance_id::text, i.definition_family, i.definition_production_key, \
                i.definition_revision_ref \
           FROM game_item_container_slots s \
           JOIN game_item_instances i ON i.item_instance_id = s.item_instance_id \
          WHERE s.character_id = encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut *connection)
    .await?;
    let Some(backpack) = backpack else {
        return Err(FeeBurnError::InsufficientFunds);
    };
    // The change must fit the backpack's declared capacity (the source's current Content facts
    // of the equipped backpack).
    let declared = &request.change.backpack.definition;
    if backpack.try_get::<String, _>("definition_family")? != declared.family
        || backpack.try_get::<String, _>("definition_production_key")? != declared.production_key
        || backpack.try_get::<String, _>("definition_revision_ref")? != declared.revision_ref
    {
        return Err(FeeBurnError::InvalidInput);
    }
    let capacity = request
        .change
        .backpack
        .container_capacity
        .and_then(|capacity| usize::try_from(capacity).ok())
        .ok_or(FeeBurnError::InvalidInput)?;
    let backpack = uuid_text(&backpack.try_get::<String, _>("item_instance_id")?)?;

    // Every direct entry counts against the capacity; the coins are the inputs.
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
        let Some(coin) = Coin::from_production_key(&key) else {
            continue;
        };
        if family != COIN_DEFINITION_FAMILY || row.try_get::<i16, _>("lifecycle")? != 1 {
            continue;
        }
        if uuid_text(&row.try_get::<String, _>("world_id")?)? != *world_id.as_bytes() {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        // Decision §4.2: an eligible input is at the compatible definition revision. A stack
        // at another one is not skipped (the database plan guard counts every coin stack).
        let compatible = match coin {
            Coin::Gold => &request.change.gold,
            Coin::Platinum => &request.change.platinum,
            Coin::Crystal => &request.change.crystal,
        };
        if row.try_get::<String, _>("definition_revision_ref")? != compatible.revision_ref {
            return Err(FeeBurnError::InvalidInput);
        }
        let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        inputs.push((
            CoinStack {
                coin,
                quantity,
                placement_ordinal: numeric_u64(row, "placement_ordinal")?,
            },
            uuid_text(&row.try_get::<String, _>("item_instance_id")?)?,
            row.try_get::<String, _>("definition_revision_ref")?,
        ));
    }
    let stacks: Vec<CoinStack> = inputs.iter().map(|(stack, _, _)| *stack).collect();
    let plan = match plan_fee_within(request.fee_gold_units, &stacks, entries.len(), capacity) {
        Ok(plan) => plan,
        Err(FeePlanError::InsufficientFunds) => return Err(FeeBurnError::InsufficientFunds),
        Err(FeePlanError::CapacityExceeded) => return Err(FeeBurnError::CapacityExceeded),
        Err(FeePlanError::ChangeDoesNotFit) => return Err(FeeBurnError::ChangeDoesNotFit),
        // Stored stacks outside 1..=100 or repeated ordinals are not a caller error.
        Err(FeePlanError::InvalidInput) => return Err(DurabilityError::InvalidStoredState.into()),
    };
    // Placed after the burn lines: the highest ordinal the backpack held before the burn, plus
    // one (entries are read highest first).
    let first_ordinal = match entries.first() {
        Some(row) => numeric_u64(row, "placement_ordinal")?,
        None => 0,
    }
    .checked_add(1)
    .unwrap_or(0);
    let change = change_outputs(request.change.item_instance_ids, plan.change, first_ordinal);
    if change.iter().any(|output| output.placement_ordinal == 0) {
        return Err(FeeBurnError::ChangeDoesNotFit);
    }

    let lines: Vec<BurnedCoinStack> = plan
        .lines
        .iter()
        .map(|line| {
            let (stack, item, _) = &inputs[line.input];
            BurnedCoinStack {
                item_instance_id: *item,
                coin: stack.coin,
                placement_ordinal: stack.placement_ordinal,
                quantity_before: stack.quantity,
                quantity_after: stack.quantity - line.burned,
            }
        })
        .collect();
    let state = |item: &[u8; 16], coin: Coin, revision: &str, quantity: u32| OneItemStateV1 {
        item_instance_id: item.to_vec(),
        world_id: world_id.as_bytes().to_vec(),
        definition: Some(OneItemTypedDefinitionRevisionV1 {
            family: COIN_DEFINITION_FAMILY.into(),
            production_key: coin.production_key().into(),
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
            let (stack, item, revision) = &inputs[planned.input];
            OneItemFeeBurnLineV1 {
                before: Some(state(item, stack.coin, revision, line.quantity_before)),
                after: Some(state(item, stack.coin, revision, line.quantity_after)),
                placement_ordinal: line.placement_ordinal,
            }
        })
        .collect();
    let change_revision = |coin: Coin| match coin {
        Coin::Platinum => request.change.platinum.revision_ref.as_str(),
        _ => request.change.gold.revision_ref.as_str(),
    };
    let audit_change = change
        .iter()
        .map(|output| OneItemFeeChangeMintV1 {
            after: Some(state(
                &output.item_instance_id,
                output.coin,
                change_revision(output.coin),
                output.quantity,
            )),
            placement_ordinal: output.placement_ordinal,
        })
        .collect();
    let burned_gold_units = request
        .fee_gold_units
        .checked_add(plan.change)
        .ok_or(FeeBurnError::InvalidInput)?;
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
            burned_gold_units,
            character_id: fence.character_id.as_bytes().to_vec(),
            committed_character_revision: committed.get(),
            world_id: world_id.as_bytes().to_vec(),
            channel_id: channel_id.as_bytes().to_vec(),
            runtime_scope_ownership_generation: fence.scope_ownership_generation.get(),
            backpack_item_instance_id: backpack.to_vec(),
            lines: audit_lines,
            change_gold_units: plan.change,
            change: audit_change,
        },
    )
    .map_err(|_| FeeBurnError::InvalidInput)?;

    let fee = i64::try_from(request.fee_gold_units).map_err(|_| FeeBurnError::InvalidInput)?;
    let gold_units = |value: u64| i64::try_from(value).map_err(|_| FeeBurnError::InvalidInput);
    sqlx::query(
        "INSERT INTO game_item_fee_burns(transaction_id, event_id, cause_kind, \
           cause_occurrence_id, charm_key, request_binding, character_id, world_id, channel_id, \
           runtime_scope_ownership_generation, committed_character_revision, \
           backpack_item_instance_id, fee_gold_units, burned_gold_units, change_gold_units, \
           line_count, occurred_at, envelope_sha256, committed_at, \
           change_platinum_item_instance_id, change_gold_item_instance_id, \
           change_placement_ordinal) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, encode($4,'hex')::uuid, \
           $5, $6, encode($7,'hex')::uuid, encode($8,'hex')::uuid, encode($9,'hex')::uuid, \
           $10::text::numeric(20,0), $11::text::numeric(20,0), encode($12,'hex')::uuid, \
           $13, $17, $18, $14, $15, sha256($16), \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint, \
           encode($19,'hex')::uuid, encode($20,'hex')::uuid, $21::text::numeric(20,0))",
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
    .bind(gold_units(burned_gold_units)?)
    .bind(gold_units(plan.change)?)
    .bind(request.change.item_instance_ids[0].as_slice())
    .bind(request.change.item_instance_ids[1].as_slice())
    .bind(
        change
            .first()
            .map(|output| output.placement_ordinal.to_string()),
    )
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
        .bind(gold_units(line.coin.worth())?)
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

    for output in &change {
        let definition = match output.coin {
            Coin::Platinum => &request.change.platinum,
            _ => &request.change.gold,
        };
        sqlx::query(
            "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
               definition_production_key, definition_revision_ref, quantity, lifecycle, \
               minted_transaction_id) \
             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, $6, 1, \
               encode($7,'hex')::uuid)",
        )
        .bind(output.item_instance_id.as_slice())
        .bind(world_id.as_bytes().as_slice())
        .bind(&definition.family)
        .bind(&definition.production_key)
        .bind(&definition.revision_ref)
        .bind(i64::from(output.quantity))
        .bind(request.transaction_id.as_slice())
        .execute(&mut *connection)
        .await?;
        sqlx::query(
            "INSERT INTO game_item_container_entries(item_instance_id, world_id, character_id, \
               parent_item_instance_id, placement_ordinal, placed_transaction_id) \
             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
               encode($4,'hex')::uuid, $5::text::numeric(20,0), encode($6,'hex')::uuid)",
        )
        .bind(output.item_instance_id.as_slice())
        .bind(world_id.as_bytes().as_slice())
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(backpack.as_slice())
        .bind(output.placement_ordinal.to_string())
        .bind(request.transaction_id.as_slice())
        .execute(&mut *connection)
        .await?;
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
        change_gold_units: plan.change,
        change,
    }))
}

/// The change outputs of `change` gold units in `slots` (platinum, gold) from `first_ordinal`:
/// `change / 100` platinum, then `change % 100` gold, each only when positive, in consecutive
/// entries. An ordinal past `u64::MAX` is returned as 0 for the caller to refuse.
fn change_outputs(slots: [[u8; 16]; 2], change: u64, first_ordinal: u64) -> Vec<MintedCoinStack> {
    let mut ordinal = Some(first_ordinal);
    [(Coin::Platinum, change / 100), (Coin::Gold, change % 100)]
        .into_iter()
        .zip(slots)
        .filter(|((_, quantity), _)| *quantity > 0)
        .map(|((coin, quantity), item_instance_id)| {
            let placement_ordinal = ordinal.unwrap_or(0);
            ordinal = ordinal.and_then(|value| value.checked_add(1));
            MintedCoinStack {
                item_instance_id,
                coin,
                quantity: u32::try_from(quantity).unwrap_or(u32::MAX),
                placement_ordinal,
            }
        })
        .collect()
}

/// The change facts name distinct fresh slots and the platinum and gold coin definitions, and
/// the backpack declares a capacity within `GAMEITEM01-CONTAINER-ENTRIES-MAX`.
fn valid_change_facts(request: &FeeBurnRequest) -> bool {
    let facts = &request.change;
    let [platinum_slot, gold_slot] = facts.item_instance_ids;
    let coin = |definition: &TypedDefinitionRef, coin: Coin| {
        definition.family == COIN_DEFINITION_FAMILY
            && definition.production_key == coin.production_key()
    };
    facts.item_instance_ids.iter().all(|slot| {
        check_uuid_v7(slot).is_ok() && *slot != request.transaction_id && *slot != request.event_id
    }) && platinum_slot != gold_slot
        && coin(&facts.platinum, Coin::Platinum)
        && coin(&facts.gold, Coin::Gold)
        && coin(&facts.crystal, Coin::Crystal)
        && facts
            .backpack
            .container_capacity
            .is_some_and(|capacity| (1..=BACKPACK_ENTRIES_MAX as u32).contains(&capacity))
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
                quantity_after, coin_worth \
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
            let worth = u64::try_from(row.try_get::<i64, _>("coin_worth")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            Ok(BurnedCoinStack {
                item_instance_id: uuid_text(&row.try_get::<String, _>("item_instance_id")?)?,
                coin: Coin::ALL
                    .into_iter()
                    .find(|coin| coin.worth() == worth)
                    .ok_or(DurabilityError::InvalidStoredState)?,
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
    use crate::durability::item_transfer::ItemStackClass;
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
            change: change_facts(),
        }
    }

    fn definition(key: &str) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: COIN_DEFINITION_FAMILY.into(),
            production_key: key.into(),
            revision_ref: "rev-1".into(),
        }
    }

    fn change_facts() -> FeeChangeFacts {
        FeeChangeFacts {
            item_instance_ids: [id(11), id(12)],
            platinum: definition(Coin::Platinum.production_key()),
            gold: definition(Coin::Gold.production_key()),
            crystal: definition(Coin::Crystal.production_key()),
            backpack: ItemDefinitionFacts {
                definition: definition("oteryn:item.tibia.i2854"),
                stack: ItemStackClass::NonStackable,
                container_capacity: Some(20),
                container_slot_equip_pattern: true,
            },
        }
    }

    #[test]
    fn change_facts_need_fresh_slots_the_two_change_coins_and_a_bounded_capacity() {
        assert!(valid_change_facts(&request()));
        let mut cases = Vec::new();
        let mut broken = request();
        broken.change.item_instance_ids[1] = id(11);
        cases.push(("one slot twice", broken));
        let mut broken = request();
        broken.change.item_instance_ids[0] = broken.transaction_id;
        cases.push(("a slot equal to the TransactionId", broken));
        let mut broken = request();
        broken.change.platinum = definition(Coin::Crystal.production_key());
        cases.push(("crystal as platinum", broken));
        let mut broken = request();
        broken.change.gold.family = "Creature".into();
        cases.push(("gold of another family", broken));
        let mut broken = request();
        broken.change.crystal = definition(Coin::Gold.production_key());
        cases.push(("gold as crystal", broken));
        let mut broken = request();
        broken.change.backpack.container_capacity = Some(21);
        cases.push(("capacity above 20", broken));
        let mut broken = request();
        broken.change.backpack.container_capacity = None;
        cases.push(("not a container", broken));
        for (case, request) in cases {
            assert!(!valid_change_facts(&request), "{case}");
        }
    }

    #[test]
    fn change_is_platinum_then_gold_in_consecutive_entries() {
        let slots = [id(11), id(12)];
        let outputs = change_outputs(slots, 7_660, 3);
        assert_eq!(
            outputs
                .iter()
                .map(|o| (o.item_instance_id, o.coin, o.quantity, o.placement_ordinal))
                .collect::<Vec<_>>(),
            vec![(id(11), Coin::Platinum, 76, 3), (id(12), Coin::Gold, 60, 4)]
        );
        // Only the gold slot is used; no change mints nothing.
        let outputs = change_outputs(slots, 42, 9);
        assert_eq!(
            outputs
                .iter()
                .map(|o| (o.item_instance_id, o.placement_ordinal))
                .collect::<Vec<_>>(),
            vec![(id(12), 9)]
        );
        assert!(change_outputs(slots, 0, 1).is_empty());
        // An ordinal past u64::MAX is refused by the caller.
        assert_eq!(change_outputs(slots, 101, u64::MAX)[1].placement_ordinal, 0);
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
