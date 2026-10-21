//! MAIL-0 §5/§6 recipient resolution against existing native tables.
//! No mail/value writer, new schema, Inbox cause, content definition or authority grant.

use super::DurabilityError;
use super::character_progression::uuid_text;
use crate::domain::character_name::{CharacterName, NAMING_POLICY_REVISION};
use crate::domain::{CharacterId, WorldId};
use sqlx::{Postgres, Row, Transaction};

/// A candidate only, never a recipient proof. The caller must independently fence
/// the sender and derive `world` from that current native fence, not client data.
/// No result may be exposed by an unfenced lookup endpoint. Cross-World, dead,
/// renamed, held-only and missing names all return `None`, without other details.
pub async fn mail_recipient_hint(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    address: &CharacterName,
) -> Result<Option<CharacterId>, DurabilityError> {
    let row = sqlx::query(
        "SELECT r.character_id::text AS character_id \
           FROM game_character_name_reservations n \
           JOIN game_character_roots r ON r.character_id=n.character_id \
          WHERE n.name_key=$1 AND n.naming_policy_revision=$3 \
            AND r.name_key=$1 AND r.world_id=encode($2,'hex')::uuid \
            AND r.lifecycle=1",
    )
    .bind(address.comparison_key())
    .bind(world.as_bytes().as_slice())
    .bind(i32::from(NAMING_POLICY_REVISION))
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|row| {
        CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)
    })
    .transpose()
}

/// Recheck a hint under the recipient root's actual FOR SHARE lock, retained by
/// this transaction. A stale hint never becomes fresh authority by matching itself.
/// Sender/recipient roots must be acquired in CharacterId order (one row for
/// self-mail) by the eventual writer, before item/container/Inbox counter locks.
/// `false` has only the privacy-safe UNKNOWN_RECIPIENT meaning. Account, online
/// state, Channel and junior status do not exclude letter recipients (MAIL-0 §5).
/// This does not authorize delivery, survive commit, or fence the sender.
pub async fn lock_mail_recipient_root(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    candidate: CharacterId,
    address: &CharacterName,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query(
        "SELECT 1 FROM game_character_roots \
          WHERE character_id=encode($1,'hex')::uuid \
            AND world_id=encode($2,'hex')::uuid AND lifecycle=1 AND name_key=$3 \
          FOR SHARE",
    )
    .bind(candidate.as_bytes().as_slice())
    .bind(world.as_bytes().as_slice())
    .bind(address.comparison_key())
    .fetch_optional(&mut **tx)
    .await?
    .is_some())
}

#[cfg(test)]
#[allow(
    dead_code,
    reason = "PostgreSQL exercises are registered in the canonical PG target; the library unit target has no database"
)]
pub mod tests;
