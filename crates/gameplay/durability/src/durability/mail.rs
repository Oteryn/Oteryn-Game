//! MAIL recipient resolution and atomic posting of letters and empty parcels.

use super::DurabilityError;
use super::character_progression::uuid_text;
use crate::domain::character_name::{CharacterName, NAMING_POLICY_REVISION};
use crate::domain::{CharacterId, WorldId};
use sqlx::{Postgres, Row, Transaction};
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::durability::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};
use sha2::{Digest, Sha256};

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

impl crate::durability::DurabilityRoot {
    /// Production MAIL owner. It qualifies the current session, derives the address and
    /// stamp from durable item state, resolves the same-World recipient, and commits the
    /// existing MAIL transaction. No client-supplied identity or Content key is trusted.
    #[allow(clippy::too_many_arguments)]
    pub async fn post_admitted_mail(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command_id: u64,
        item: [u8; 16],
        mailbox_placement_key: u64,
    ) -> Result<MailPostOutcome, DurabilityError> {
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            let command = CommandRef::new(fence.game_session_id, CommandId::new(command_id).map_err(|_| DurabilityError::InvalidStoredState)?);
            let RuntimeScopeRefV1::Channel { world_id, channel_id } = fence.runtime_scope else { return Ok(MailPostOutcome::Refused(MailPostRefusal::NotMailable)); };
            let world = WorldId::from_bytes(*world_id.as_bytes()).map_err(|_| DurabilityError::InvalidStoredState)?;
            if mailbox_placement_key == 0 || !character_item_fence_is_current(&mut tx, &node, &fence, command, fence.character_id, *world_id.as_bytes(), *channel_id.as_bytes()).await? {
                return Ok(MailPostOutcome::Refused(MailPostRefusal::NotMailable));
            }
            // A posted root is stamped and in the recipient's Inbox, so its address and stamp
            // cannot be derived again: the immutable receipt answers the replay first.
            if let Some(root) = sqlx::query_scalar::<_, Vec<u8>>("SELECT uuid_send(root_item_instance_id) FROM game_mail_operations WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND sender_character_id=encode($3,'hex')::uuid")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command_id.to_string()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await? {
                return Ok(if root.as_slice() == item.as_slice() { MailPostOutcome::AlreadyPosted } else { MailPostOutcome::Refused(MailPostRefusal::ReplayConflict) });
            }
            let row = sqlx::query("SELECT i.definition_production_key,t.text FROM game_item_instances i LEFT JOIN game_item_texts t USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1")
                .bind(item.as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(row) = row else { return Ok(MailPostOutcome::Refused(MailPostRefusal::NotMailable)); };
            let key: String = row.try_get("definition_production_key")?;
            let (address_text, stamp) = match key.as_str() {
                "oteryn:item.tibia.i3505" => (row.try_get::<Option<String>,_>("text")?, "oteryn:item.tibia.i3506"),
                "oteryn:item.tibia.i3503" => {
                    let text = sqlx::query_scalar::<_, String>("SELECT t.text FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) JOIN game_item_texts t USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i3507' AND t.text<>'' ORDER BY e.placement_ordinal LIMIT 1")
                        .bind(item.as_slice()).fetch_optional(&mut *tx).await?;
                    (text, "oteryn:item.tibia.i3504")
                }
                _ => return Ok(MailPostOutcome::Refused(MailPostRefusal::NotMailable)),
            };
            let Some(first) = address_text.as_deref().and_then(|s| s.lines().next()).map(str::trim).filter(|s| !s.is_empty()) else { return Ok(MailPostOutcome::Refused(MailPostRefusal::UnknownRecipient)); };
            let Ok(address) = CharacterName::parse(first) else { return Ok(MailPostOutcome::Refused(MailPostRefusal::UnknownRecipient)); };
            let Some(recipient) = mail_recipient_hint(&mut tx, world, &address).await? else { return Ok(MailPostOutcome::Refused(MailPostRefusal::UnknownRecipient)); };
            if !lock_mail_recipient_root(&mut tx, world, recipient, &address).await? { return Ok(MailPostOutcome::Refused(MailPostRefusal::UnknownRecipient)); }
            let ids: (Vec<u8>, Vec<u8>) = sqlx::query_as("SELECT uuid_send(game_character_uuid_v7()),uuid_send(game_character_uuid_v7())").fetch_one(&mut *tx).await?;
            let mut mail_id=[0;16]; mail_id.copy_from_slice(&ids.0);
            let mut transaction_id=[0;16]; transaction_id.copy_from_slice(&ids.1);
            let hash: [u8; 32] = Sha256::new()
                .chain_update(fence.game_session_id.as_bytes())
                .chain_update(command_id.to_be_bytes())
                .chain_update(item)
                .chain_update(mailbox_placement_key.to_be_bytes())
                .chain_update(address.comparison_key().as_bytes())
                .chain_update(recipient.as_bytes())
                .chain_update(stamp.as_bytes())
                .finalize()
                .into();
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
            let outcome=post_mail_item(&mut tx,mail_id,*fence.game_session_id.as_bytes(),command_id,world,*channel_id.as_bytes(),fence.character_id,recipient,item,key == "oteryn:item.tibia.i3503",stamp,&mailbox_placement_key.to_be_bytes(),hash,transaction_id,b"MAIL_POSTED").await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(outcome)
        })).await
    }
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
    qualified_parcel_tree: bool,
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
    if has_children && !qualified_parcel_tree {
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
