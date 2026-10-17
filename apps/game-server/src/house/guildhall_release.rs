//! D1098 House participant, a private child of durability::house_ownership.
//! D owns the physical transaction. Nested transactions here are protected SAVEPOINTs:
//! typed refusals explicitly roll back this participant and never commit a partial D step.
//! The terminal D caller must roll back its whole physical transaction on Err.
//! Activation still requires the accepted recipient-pin amendment and actual D ordering.
//! The real D recovery/node getters and native NodeIncarnationProof::sql_possession_secret
//! are dependencies of the required upstream integration; this leaf adds no shared accessor.

use super::super::guild::{GuildDisbandTransaction, GuildError};
use crate::domain::guild::GuildId;
use crate::foundation::WorldId;
use sqlx::{Acquire, Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, GuildHallReleaseError>;

#[derive(Debug)]
pub(in crate::durability) enum GuildHallReleaseError {
    InvalidInput,
    NotCurrent,
    ResnapshotRequired,
    Occupied,
    InvalidCustody,
    MissingDependency(&'static str),
    GuildAuthority(GuildError),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for GuildHallReleaseError {
    fn from(error: sqlx::Error) -> Self {
        match error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref()
        {
            Some("OHG01") => Self::ResnapshotRequired,
            Some("OHG02") => Self::NotCurrent,
            Some("OHG03") => Self::InvalidCustody,
            Some("OHG04") => Self::Occupied,
            Some("OHG05") => Self::MissingDependency("native HOUSE-1b Guild fence ABI1"),
            _ => Self::Database(error),
        }
    }
}

/// Not Clone or constructible by consumers. The protected database marker binds the
/// physical transaction, full native census and deletion pins. A rollback losing the pins
/// also removes their marker, even when an in-memory D context retains the same xid.
pub(in crate::durability) struct RecipientReservation {
    id: [u8; 16],
    operation: [u8; 16],
    world: WorldId,
    guild: GuildId,
    step: u64,
    xid: String,
    binding: Vec<u8>,
}

pub(in crate::durability) struct GuildHallReleasedProof {
    reservation: RecipientReservation,
}

pub(in crate::durability) enum GuildHallReleaseOutcome {
    Progress { moved: u64, next_step: u64 },
    Released(GuildHallReleasedProof),
}

/// After occurrence/recovery/admission locks, BEFORE D acquires its Guild spine.
/// Inputs identify work; genuine current Guild/World authority comes from D at consumption.
/// SQL validates native14 ABI1 before reserving, then stamps evidence after sorted pins.
/// Runtime has EXECUTE on these bounded C ports, never direct C/ACL/BED table DML.
pub(in crate::durability) async fn reserve_guild_disband_recipients_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    guild: GuildId,
    operation: [u8; 16],
    step: u64,
    reservation_id: [u8; 16],
) -> Result<RecipientReservation> {
    for id in [&operation, &reservation_id] {
        super::super::item_mint_audit::check_uuid_v7(id)
            .map_err(|_| GuildHallReleaseError::InvalidInput)?;
    }
    if operation == reservation_id {
        return Err(GuildHallReleaseError::InvalidInput);
    }
    let mut savepoint = tx.begin().await?;
    let result: Result<(String, Vec<u8>)> = async {
        let xid: String = sqlx::query_scalar("SELECT game_house_guild_reserve(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,encode($5,'hex')::uuid)")
            .bind(reservation_id.as_slice()).bind(operation.as_slice()).bind(step.to_string())
            .bind(guild.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
            .fetch_one(&mut *savepoint).await?;
        let binding: Option<Vec<u8>> = sqlx::query_scalar("SELECT game_house_guild_reservation_binding(encode($1,'hex')::uuid)")
            .bind(reservation_id.as_slice()).fetch_one(&mut *savepoint).await?;
        let binding = binding.ok_or(GuildHallReleaseError::NotCurrent)?;
        if binding.len() != 32 || xid.is_empty() {
            return Err(GuildHallReleaseError::InvalidCustody);
        }
        Ok((xid, binding))
    }.await;
    let (xid, binding) = match result {
        Ok(value) => value,
        Err(error) => {
            savepoint.rollback().await?;
            return Err(error);
        }
    };
    savepoint.commit().await?; // RELEASE SAVEPOINT, never physical COMMIT.
    Ok(RecipientReservation {
        id: reservation_id,
        operation,
        world,
        guild,
        step,
        xid,
        binding,
    })
}

async fn assert_context(
    tx: &mut Transaction<'_, Postgres>,
    context: &GuildDisbandTransaction,
    reservation: &RecipientReservation,
) -> Result<()> {
    context
        .assert_current(tx)
        .await
        .map_err(GuildHallReleaseError::GuildAuthority)?;
    if context.guild_id() != reservation.guild || context.world_id() != reservation.world {
        return Err(GuildHallReleaseError::NotCurrent);
    }
    Ok(())
}

/// Before D's bid/Guild-bank/Account-bank participants. No Root/pool getter, independent
/// transaction, replacement D authority, supplied recipient or supplied drained boolean.
/// Err requires D's whole physical transaction rollback; local typed refusals are explicitly
/// rolled back before return. Progress is bounded custody work, never terminal release proof.
pub(in crate::durability) async fn advance_guild_disband_release_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    context: &GuildDisbandTransaction,
    reservation: RecipientReservation,
) -> Result<GuildHallReleaseOutcome> {
    let mut savepoint = tx.begin().await?;
    let result = advance(&mut savepoint, context, &reservation).await;
    let result = match result {
        Ok(value) => value,
        Err(error) => {
            savepoint.rollback().await?;
            return Err(error);
        }
    };
    savepoint.commit().await?;
    match result {
        Some((moved, next_step)) => Ok(GuildHallReleaseOutcome::Progress { moved, next_step }),
        None => Ok(GuildHallReleaseOutcome::Released(GuildHallReleasedProof {
            reservation,
        })),
    }
}

async fn advance(
    tx: &mut Transaction<'_, Postgres>,
    context: &GuildDisbandTransaction,
    r: &RecipientReservation,
) -> Result<Option<(u64, u64)>> {
    assert_context(tx, context, r).await?;
    // Genuine D assert_current checks xid/current authority but an xid survives SAVEPOINT
    // rollback. Forward the genuine Root-created context's expected recovery record and
    // secret-bearing native proof, never a matching record reconstructed from the DB.
    let recovery = context.house_release_recovery();
    let node = context.house_release_node();
    let fact = node.fact();
    let node_id = fact.node_id();
    let predecessor_digest =
        (recovery.predecessor_generation != 0).then_some(recovery.predecessor_digest.as_slice());
    // Native SQL UUID/hex conversion builds the exact recovery JSON from supplied fields.
    // The incarnation secret is a distinct BYTEA bind: never JSON, a marker, Debug or a log.
    // SQL independently revalidates recovery/admission/node/World, locks the real sorted
    // Guild spine at position3, and stamps protected sameTX authority LAST. Reassert D
    // before privileged advance can acquire property/anchor or perform value effects.
    sqlx::query("SELECT game_house_guild_lock_spine(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,jsonb_build_object('authority_scope_id',$8::text,'recovery_generation',$9::text,'recovery_event_id',encode($10,'hex')::uuid::text,'predecessor_generation',$11::text,'predecessor_digest',encode($12,'hex'),'issued_at',$13::text,'issuer_identity',$14::text),encode($15,'hex')::uuid,$16::text::numeric,$17)")
        .bind(r.id.as_slice()).bind(r.operation.as_slice()).bind(r.step.to_string())
        .bind(r.guild.as_bytes().as_slice()).bind(r.world.as_bytes().as_slice())
        .bind(&r.binding).bind(&r.xid)
        .bind(&recovery.authority_scope_id).bind(recovery.recovery_generation.to_string())
        .bind(recovery.recovery_event_id.as_slice()).bind(recovery.predecessor_generation.to_string())
        .bind(predecessor_digest).bind(recovery.issued_at.to_string()).bind(&recovery.issuer_identity)
        .bind(node_id.as_bytes().as_slice()).bind(fact.registration_revision().to_string())
        .bind(node.sql_possession_secret()).execute(&mut **tx).await?;
    assert_context(tx, context, r).await?;
    // The real C definer checks native14 ABI1 and all enabled native fence hooks before
    // effects, takes property+anchor FOR UPDATE before freeze/items, and delegates to the
    // native CharacterInbox function after prelocking actual sorted roots/items/counters.
    // Advance and terminal assertion require the protected authority stamp and independently
    // recheck its current recovery/node/World facts; possession is never a supplied boolean.
    let row = sqlx::query("SELECT moved,next_step::text,released FROM game_house_guild_advance(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7)")
        .bind(r.id.as_slice()).bind(r.operation.as_slice()).bind(r.step.to_string())
        .bind(r.guild.as_bytes().as_slice()).bind(r.world.as_bytes().as_slice())
        .bind(&r.binding).bind(&r.xid).fetch_one(&mut **tx).await?;
    let moved = row.try_get::<i64, _>("moved")?;
    if !(0..=100).contains(&moved) {
        return Err(GuildHallReleaseError::InvalidCustody);
    }
    let released = row.try_get::<bool, _>("released")?;
    let outcome = if released {
        assert_release_marker(tx, r).await?;
        None
    } else {
        let next_step: u64 = row
            .try_get::<String, _>("next_step")?
            .parse()
            .map_err(|_| GuildHallReleaseError::InvalidCustody)?;
        if r.step.checked_add(1) != Some(next_step) {
            return Err(GuildHallReleaseError::InvalidCustody);
        }
        Some((moved as u64, next_step))
    };
    assert_context(tx, context, r).await?;
    Ok(outcome)
}

async fn assert_release_marker(
    tx: &mut Transaction<'_, Postgres>,
    r: &RecipientReservation,
) -> Result<()> {
    let released: bool = sqlx::query_scalar("SELECT game_house_guild_assert_released(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7)")
        .bind(r.id.as_slice()).bind(r.operation.as_slice()).bind(r.step.to_string())
        .bind(r.guild.as_bytes().as_slice()).bind(r.world.as_bytes().as_slice())
        .bind(&r.binding).bind(&r.xid).fetch_one(&mut **tx).await?;
    if !released {
        return Err(GuildHallReleaseError::NotCurrent);
    }
    Ok(())
}

/// EVERY terminal consumer revalidates genuine D authority and actual protected C
/// status3/absence/journal in this same physical transaction. Retained proof alone is not
/// authority: rollback can erase its marker/journal without changing the transaction xid.
/// Any Err requires the terminal D caller's whole physical transaction rollback.
pub(in crate::durability) async fn assert_guild_disband_released_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    context: &GuildDisbandTransaction,
    proof: &GuildHallReleasedProof,
) -> Result<()> {
    assert_context(tx, context, &proof.reservation).await?;
    assert_release_marker(tx, &proof.reservation).await?;
    assert_context(tx, context, &proof.reservation).await
}
