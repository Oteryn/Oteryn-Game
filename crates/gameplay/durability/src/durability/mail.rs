//! MAIL recipient resolution and atomic posting of letters and empty parcels.

use super::DurabilityError;
use super::character_progression::uuid_text;
use crate::domain::character_name::{CharacterName, NAMING_POLICY_REVISION};
use crate::domain::{CharacterId, WorldId};
use sqlx::{Postgres, Row, Transaction};

pub const MAIL_POSTS_PER_SECOND: u64 = 1;
pub const MAIL_INBOX_ITEM_CEILING: u64 = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailPostRefusal {
    UnknownRecipient,
    NotMailable,
    RecipientInboxFull,
    Exhausted,
    ReplayConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailPostOutcome {
    Posted,
    AlreadyPosted,
    Refused(MailPostRefusal),
}

/// Atomically transfers one already-qualified letter/empty parcel into the
/// recipient Inbox and stamps it without changing its ItemInstanceId.
///
/// The caller owns this transaction and must first run
/// `character_item_fence_is_current`, resolve the mailbox reach/binding and
/// current Content stamp rule. Consequently a crash before commit preserves
/// the sender location, while a retry after commit observes the occurrence.
#[allow(clippy::too_many_arguments)]
pub async fn post_mail_item(
    tx: &mut Transaction<'_, Postgres>,
    mail_id: [u8; 16],
    game_session_id: [u8; 16],
    command_id: u64,
    world: WorldId,
    channel_id: [u8; 16],
    sender: CharacterId,
    recipient: CharacterId,
    item: [u8; 16],
    stamped_production_key: &str,
    mailbox_placement_key: &[u8],
    request_sha256: [u8; 32],
    transaction_id: [u8; 16],
    event_payload: &[u8],
) -> Result<MailPostOutcome, DurabilityError> {
    // Receipts are immutable; replay needs no UPDATE privilege for a row lock.
    if let Some(row) = sqlx::query(
        "SELECT request_sha256 FROM game_mail_operations WHERE \
         game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric",
    )
    .bind(game_session_id.as_slice())
    .bind(command_id.to_string())
    .fetch_optional(&mut **tx)
    .await?
    {
        let stored_hash: Vec<u8> = row.try_get("request_sha256")?;
        return Ok(if stored_hash.as_slice() == request_sha256 {
            MailPostOutcome::AlreadyPosted
        } else {
            MailPostOutcome::Refused(MailPostRefusal::ReplayConflict)
        });
    }
    // The sender root is held by the current-fence proof. The recipient lock
    // is retained to commit, excluding rename, deletion and World transfer.
    let live = sqlx::query(
        "SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND \
         world_id=encode($2,'hex')::uuid AND lifecycle=1 FOR SHARE",
    )
    .bind(recipient.as_bytes().as_slice())
    .bind(world.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    if live.is_none() {
        return Ok(MailPostOutcome::Refused(MailPostRefusal::UnknownRecipient));
    }
    let recent: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_mail_operations WHERE \
         sender_character_id=encode($1,'hex')::uuid AND occurred_at>clock_timestamp()-interval '1 \
         second')",
    )
    .bind(sender.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if recent {
        return Ok(MailPostOutcome::Refused(MailPostRefusal::Exhausted));
    }
    // Lock the item before checking its children or changing custody. All native
    // child writers serialize through the parent item lock.
    let stored = sqlx::query(
        "SELECT to_jsonb(i) AS before_item FROM game_item_instances i WHERE \
         item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND \
         lifecycle=1 AND quantity=1 AND definition_family='Item' FOR UPDATE",
    )
    .bind(item.as_slice())
    .bind(world.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(stored) = stored else {
        return Ok(MailPostOutcome::Refused(MailPostRefusal::NotMailable));
    };
    let source = sqlx::query(
        "SELECT to_jsonb(e) AS source_entry FROM game_item_container_entries e WHERE \
         item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND \
         world_id=encode($3,'hex')::uuid",
    )
    .bind(item.as_slice())
    .bind(sender.as_bytes().as_slice())
    .bind(world.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(source) = source else {
        return Ok(MailPostOutcome::Refused(MailPostRefusal::NotMailable));
    };
    let has_children: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE \
         parent_item_instance_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM \
         game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) OR \
         EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE \
         parent_item_instance_id=encode($1,'hex')::uuid)",
    )
    .bind(item.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if has_children {
        return Ok(MailPostOutcome::Refused(MailPostRefusal::NotMailable));
    }
    // The Inbox owner grants no direct runtime writes to its tables. MAIL's
    // narrowly granted SQL owner locks the counter and applies the complete
    // transaction; its boolean refusal is returned before any custody change.
    let applied = sqlx::query_scalar::<_, bool>(
        "SELECT game_mail_posting_apply(encode($1,'hex')::uuid,encode($2,'hex')::uuid, \
         $3::text::numeric,encode($4,'hex')::uuid,encode($5,'hex')::uuid, \
         encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid, \
         $9,$10,$11,encode($12,'hex')::uuid,$13,$14,$15)",
    )
    .bind(mail_id.as_slice())
    .bind(game_session_id.as_slice())
    .bind(command_id.to_string())
    .bind(world.as_bytes().as_slice())
    .bind(channel_id.as_slice())
    .bind(sender.as_bytes().as_slice())
    .bind(recipient.as_bytes().as_slice())
    .bind(item.as_slice())
    .bind(stamped_production_key)
    .bind(mailbox_placement_key)
    .bind(request_sha256.as_slice())
    .bind(transaction_id.as_slice())
    .bind(event_payload)
    .bind(stored.try_get::<serde_json::Value, _>("before_item")?)
    .bind(source.try_get::<serde_json::Value, _>("source_entry")?)
    .fetch_one(&mut **tx)
    .await;
    match applied {
        Ok(true) => Ok(MailPostOutcome::Posted),
        Ok(false) => Ok(MailPostOutcome::Refused(
            MailPostRefusal::RecipientInboxFull,
        )),
        Err(error)
            if error.as_database_error().and_then(|e| e.code()).as_deref() == Some("OTM02") =>
        {
            Err(DurabilityError::InvalidStoredState)
        }
        Err(error) => Err(error.into()),
    }
}

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

#[cfg(any(test, feature = "test-support"))]
#[allow(
    dead_code,
    reason = "PostgreSQL exercises are registered in the canonical PG target; the library unit target has no database"
)]
pub mod tests;
