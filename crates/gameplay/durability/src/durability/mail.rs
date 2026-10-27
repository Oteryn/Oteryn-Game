//! MAIL-0 §5/§6 recipient resolution against existing native tables.
//! No mail/value writer, new schema, Inbox cause, content definition or authority grant.

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
pub enum MailPostOutcome { Posted, AlreadyPosted, Refused(MailPostRefusal) }

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
    if let Some(row) = sqlx::query("SELECT request_sha256 FROM game_mail_operations WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric FOR SHARE")
        .bind(game_session_id.as_slice()).bind(command_id.to_string()).fetch_optional(&mut **tx).await? {
        return Ok(if row.try_get::<Vec<u8>,_>("request_sha256")?.as_slice()==request_sha256 { MailPostOutcome::AlreadyPosted } else { MailPostOutcome::Refused(MailPostRefusal::ReplayConflict) });
    }
    // The sender root is held by the current-fence proof. The recipient lock
    // is retained to commit, excluding rename, deletion and World transfer.
    let live = sqlx::query("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND lifecycle=1 FOR SHARE")
        .bind(recipient.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    if live.is_none() { return Ok(MailPostOutcome::Refused(MailPostRefusal::UnknownRecipient)); }
    let recent: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_mail_operations WHERE sender_character_id=encode($1,'hex')::uuid AND occurred_at>clock_timestamp()-interval '1 second')")
        .bind(sender.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if recent { return Ok(MailPostOutcome::Refused(MailPostRefusal::Exhausted)); }
    let counter = sqlx::query("SELECT committed::text,next_ordinal::text FROM game_character_inbox_counters WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(recipient.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(counter)=counter else { return Ok(MailPostOutcome::Refused(MailPostRefusal::RecipientInboxFull)); };
    let committed: u64=counter.try_get::<String,_>("committed")?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
    if committed>=MAIL_INBOX_ITEM_CEILING { return Ok(MailPostOutcome::Refused(MailPostRefusal::RecipientInboxFull)); }
    let ordinal=counter.try_get::<String,_>("next_ordinal")?;
    let removed=sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid RETURNING world_id::text")
        .bind(item.as_slice()).bind(sender.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    if removed.is_none() { return Ok(MailPostOutcome::Refused(MailPostRefusal::NotMailable)); }
    sqlx::query("UPDATE game_item_instances SET definition_production_key=$2 WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($3,'hex')::uuid")
        .bind(item.as_slice()).bind(stamped_production_key).bind(world.as_bytes().as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_character_inbox_deliveries(item_instance_id,cause_kind,cause_ref,world_id,character_id,ordinal) VALUES(encode($1,'hex')::uuid,'MAIL_POST',$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric)")
        .bind(item.as_slice()).bind(mail_id.iter().map(|byte| format!("{byte:02x}")).collect::<String>()).bind(world.as_bytes().as_slice()).bind(recipient.as_bytes().as_slice()).bind(&ordinal).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_item_character_inbox_locations(item_instance_id,world_id,character_id,ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric)")
        .bind(item.as_slice()).bind(world.as_bytes().as_slice()).bind(recipient.as_bytes().as_slice()).bind(&ordinal).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_character_inbox_counters SET next_ordinal=next_ordinal+1 WHERE character_id=encode($1,'hex')::uuid")
        .bind(recipient.as_bytes().as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_mail_operations(mail_id,game_session_id,command_id,world_id,channel_id,sender_character_id,recipient_character_id,root_item_instance_id,mailbox_placement_key,item_count,request_sha256,transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,1,$10,encode($11,'hex')::uuid)")
        .bind(mail_id.as_slice()).bind(game_session_id.as_slice()).bind(command_id.to_string()).bind(world.as_bytes().as_slice()).bind(channel_id.as_slice()).bind(sender.as_bytes().as_slice()).bind(recipient.as_bytes().as_slice()).bind(item.as_slice()).bind(mailbox_placement_key).bind(request_sha256.as_slice()).bind(transaction_id.as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_mail_outbox(mail_id,event_type,event_payload) VALUES(encode($1,'hex')::uuid,'MAIL_POSTED',$2)")
        .bind(mail_id.as_slice()).bind(event_payload).execute(&mut **tx).await?;
    Ok(MailPostOutcome::Posted)
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
