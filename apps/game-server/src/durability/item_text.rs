//! MAIL-TEXT-1: native letter/label rewrites in the direct main backpack.
//!
//! This is a bounded producer with storage in migration 0101, awaiting content
//! admission, module registration and transport composition. It admits neither write-once
//! definitions nor other custody locations. The compositor must also refuse an
//! item offered in player trade (WRITE-0 §5.1); no native trade reservation port
//! exists in this base. Do not activate this writer before that integration.
//! No player text, writer name or text-derived diagnostic enters value audit.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::numeric_u64;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::check_uuid_v7;
use super::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::mail::{MailText, MailTextClass, MailTextRefusal};
use crate::foundation::CommandRef;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

// Storage is installed once by the standing MAIL migration 0101.

/// Text is intentionally absent from Debug and from receipts.
pub(crate) struct ItemTextWriteRequest {
    pub(crate) command: CommandRef,
    pub(crate) item_instance_id: [u8; 16],
    pub(crate) definition: TypedDefinitionRef,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ItemTextWriteOutcome {
    /// Zero only for an unchanged empty item which has never had text.
    pub(crate) text_revision: u64,
    pub(crate) changed: bool,
    pub(crate) replayed: bool,
}

#[derive(Debug)]
pub(crate) enum ItemTextWriteError {
    InvalidInput,
    AuthorityRejected,
    NotReachable,
    NotWritable,
    TooLong,
    Rejected,
    Exhausted,
    ConflictingCommand,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for ItemTextWriteError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<sqlx::Error> for ItemTextWriteError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}
impl std::fmt::Display for ItemTextWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Error classes only. Never render player text or a raw database detail.
        formatter.write_str(match self {
            Self::InvalidInput => "invalid text write input",
            Self::AuthorityRejected => "text write authority rejected",
            Self::NotReachable => "text item not reachable",
            Self::NotWritable => "text item not writable",
            Self::TooLong => "text too long",
            Self::Rejected => "text rejected",
            Self::Exhausted => "text write exhausted",
            Self::ConflictingCommand => "text write command conflict",
            Self::Unavailable(_) => "text write storage unavailable",
        })
    }
}
impl std::error::Error for ItemTextWriteError {}
type Result<T> = std::result::Result<T, ItemTextWriteError>;

impl ItemTextWriteRequest {
    fn binding(&self, character: &[u8; 16]) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"OTERYN_ITEM_TEXT_REWRITE/v1");
        hash.update(character);
        hash.update(self.command.game_session_id().as_bytes());
        hash.update(self.command.command_id().get().to_be_bytes());
        hash.update(self.item_instance_id);
        for value in [
            self.definition.family.as_bytes(),
            self.definition.production_key.as_bytes(),
            self.definition.revision_ref.as_bytes(),
            self.text.as_bytes(),
        ] {
            hash.update((value.len() as u64).to_be_bytes());
            hash.update(value);
        }
        hash.finalize().into()
    }
}

/// Closed leaf ABI: facts from one exact qualified Content definition. This
/// carries no current session, custody, offered-trade or activation authority.
pub(crate) struct MailTextPolicy {
    definition: TypedDefinitionRef,
    source_digest: [u8; 32],
    max_text_length: u32,
    class: MailTextClass,
    direct_backpack: MailTextDestination,
}

/// This increment admits only the direct main-backpack custody query below.
enum MailTextDestination {
    CharacterInventory,
}

impl MailTextPolicy {
    /// Sole producer: NativeGameplayState::mail_text_policy in
    /// content/mail_text_policy.rs, after exact qualified profile validation.
    /// The writer independently checks this digest against current activation.
    pub(crate) fn from_qualified_content(
        definition: TypedDefinitionRef,
        source_digest: [u8; 32],
        max_text_length: u32,
        class: MailTextClass,
    ) -> Self {
        Self {
            definition,
            source_digest,
            max_text_length,
            class,
            direct_backpack: MailTextDestination::CharacterInventory,
        }
    }
}

fn qualified_text(policy: &MailTextPolicy, request: &ItemTextWriteRequest) -> Result<MailText> {
    let maximum = match policy.class {
        MailTextClass::Letter => 1_999,
        MailTextClass::Label => 79,
    };
    if policy.definition != request.definition || policy.max_text_length != maximum {
        return Err(ItemTextWriteError::NotWritable);
    }
    MailText::parse(policy.class, &request.text).map_err(|error| match error {
        MailTextRefusal::TooLong => ItemTextWriteError::TooLong,
        MailTextRefusal::ControlCharacter => ItemTextWriteError::Rejected,
    })
}

/// Empty deletion preserves the revision tombstone. An unchanged rewrite does
/// not consume a revision or a rate token. Overflow never wraps or deletes text.
fn next_revision(last: u64, previous: Option<&str>, next: &str) -> Result<(u64, bool)> {
    if previous.unwrap_or_default() == next {
        return Ok((last, false));
    }
    last.checked_add(1)
        .map(|revision| (revision, true))
        .ok_or(ItemTextWriteError::Exhausted)
}

impl DurabilityRoot {
    /// Commit a bounded rewrite; exact terminal replay never reacquires item
    /// authority. Every new write independently proves the live content digest,
    /// recovery, node, session, admission, root and current direct custody.
    #[expect(
        dead_code,
        reason = "MAIL-TEXT-1 awaits CP wire composition, admitted content, migration and real offered-trade exclusion"
    )]
    pub(crate) async fn commit_item_text_rewrite(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        policy: MailTextPolicy,
        request: ItemTextWriteRequest,
    ) -> Result<ItemTextWriteOutcome> {
        if request.command.game_session_id() != fence.game_session_id
            || check_uuid_v7(&request.item_instance_id).is_err()
        {
            return Err(ItemTextWriteError::InvalidInput);
        }
        if request.text.len() > 7_996 {
            return Err(ItemTextWriteError::TooLong);
        }
        let recovery = recovery
            .record_for(self)
            .map_err(|_| ItemTextWriteError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match rewrite(&mut tx, &node, &fence, &policy, &request).await {
                        Ok(outcome) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(outcome))
                        }
                        Err(ItemTextWriteError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

async fn rewrite(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    policy: &MailTextPolicy,
    request: &ItemTextWriteRequest,
) -> Result<ItemTextWriteOutcome> {
    let character = fence.character_id.as_bytes();
    let session = request.command.game_session_id();
    let command = request.command.command_id().get().to_string();
    let binding = request.binding(character);
    let mut key = session.as_bytes().to_vec();
    key.extend_from_slice(&request.command.command_id().get().to_be_bytes());
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('oteryn:item-text:' || encode($1,'hex'),0))",
    )
    .bind(key)
    .execute(&mut **tx)
    .await?;
    if let Some(row) = sqlx::query("SELECT character_id=encode($3,'hex')::uuid AS same_character, request_binding, text_revision::text, changed FROM game_item_text_write_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
        .bind(session.as_bytes().as_slice()).bind(&command).bind(character.as_slice())
        .fetch_optional(&mut **tx).await?
    {
        if !row.try_get::<bool,_>("same_character")?
            || row.try_get::<Vec<u8>,_>("request_binding")? != binding
        {
            return Err(ItemTextWriteError::ConflictingCommand);
        }
        return Ok(ItemTextWriteOutcome {
            text_revision: numeric_u64(&row, "text_revision")?,
            changed: row.try_get("changed")?,
            replayed: true,
        });
    }
    let (world, channel) = scope_of(fence).map_err(|_| ItemTextWriteError::AuthorityRejected)?;
    // Same lock used by game_content_record_activation and the equipment owner.
    sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea,'hex')::uuid::text || ':' || encode($2::bytea,'hex')::uuid::text,0))")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .execute(&mut **tx).await?;
    if !character_item_fence_is_current(
        tx,
        node,
        fence,
        request.command,
        fence.character_id,
        *world.as_bytes(),
        *channel.as_bytes(),
    )
    .await?
    {
        return Err(ItemTextWriteError::AuthorityRejected);
    }
    let digest: Option<Vec<u8>> = sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    if digest.as_deref() != Some(policy.source_digest.as_slice()) {
        return Err(ItemTextWriteError::AuthorityRejected);
    }
    let text = qualified_text(policy, request)?;
    // Lock first, then read custody in a separate snapshot after any wait. A
    // joined locking SELECT could retain an old snapshot of the location row.
    let locked = sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(request.item_instance_id.as_slice()).fetch_optional(&mut **tx).await?;
    if locked.is_none() {
        return Err(ItemTextWriteError::NotReachable);
    }
    let owned: bool = match policy.direct_backpack {
        MailTextDestination::CharacterInventory => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) JOIN game_item_container_slots s ON s.character_id=e.character_id AND s.item_instance_id=e.parent_item_instance_id AND s.world_id=e.world_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND e.character_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family=$4 AND i.definition_production_key=$5 AND i.definition_revision_ref=$6)")
        .bind(request.item_instance_id.as_slice()).bind(world.as_bytes().as_slice()).bind(character.as_slice())
        .bind(&request.definition.family).bind(&request.definition.production_key).bind(&request.definition.revision_ref)
        .fetch_one(&mut **tx).await?,
    };
    if !owned {
        return Err(ItemTextWriteError::NotReachable);
    }
    persist_rewrite(tx, character, request, &text).await
}

/// Storage half only, after the caller acquired the real owner/item locks and
/// proved current content/custody. Private so it cannot bypass the native gate.
async fn persist_rewrite(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    request: &ItemTextWriteRequest,
    text: &MailText,
) -> Result<ItemTextWriteOutcome> {
    let session = request.command.game_session_id();
    let command = request.command.command_id().get().to_string();
    let binding = request.binding(character);
    let revision = sqlx::query("SELECT last_revision::text FROM game_item_text_revisions WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(request.item_instance_id.as_slice()).fetch_optional(&mut **tx).await?;
    let last = revision
        .as_ref()
        .map(|row| numeric_u64(row, "last_revision"))
        .transpose()?
        .unwrap_or(0);
    let previous = sqlx::query("SELECT text,text_revision::text FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(request.item_instance_id.as_slice()).fetch_optional(&mut **tx).await?;
    if previous
        .as_ref()
        .is_some_and(|row| numeric_u64(row, "text_revision").ok() != Some(last))
    {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let previous = previous
        .as_ref()
        .map(|row| row.try_get::<String, _>("text"))
        .transpose()?;
    let (revision, changed) = next_revision(last, previous.as_deref(), text.as_str())?;
    // Server time is sampled after all blocking owner/item locks. Never trust a
    // client timestamp, a prepared request time or the transaction-start clock.
    let now: String = sqlx::query_scalar("SELECT clock_timestamp()::text")
        .fetch_one(&mut **tx)
        .await?;
    if changed {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM (SELECT 1 FROM game_item_text_write_receipts WHERE character_id=encode($1,'hex')::uuid AND changed AND written_at>$2::text::timestamptz-interval '1 second' LIMIT 2) recent")
            .bind(character.as_slice()).bind(&now).fetch_one(&mut **tx).await?;
        if count >= 2 {
            return Err(ItemTextWriteError::Exhausted);
        }
        sqlx::query("INSERT INTO game_item_text_revisions(item_instance_id,last_revision) VALUES(encode($1,'hex')::uuid,$2::text::numeric(20,0)) ON CONFLICT(item_instance_id) DO UPDATE SET last_revision=EXCLUDED.last_revision")
            .bind(request.item_instance_id.as_slice()).bind(revision.to_string()).execute(&mut **tx).await?;
        if text.as_str().is_empty() {
            sqlx::query(
                "DELETE FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(request.item_instance_id.as_slice())
            .execute(&mut **tx)
            .await?;
        } else {
            sqlx::query("INSERT INTO game_item_texts(item_instance_id,text,writer_character_id,written_at,text_revision) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::text::timestamptz,$5::text::numeric(20,0)) ON CONFLICT(item_instance_id) DO UPDATE SET text=EXCLUDED.text,writer_character_id=EXCLUDED.writer_character_id,written_at=EXCLUDED.written_at,text_revision=EXCLUDED.text_revision")
                .bind(request.item_instance_id.as_slice()).bind(text.as_str()).bind(character.as_slice()).bind(&now).bind(revision.to_string()).execute(&mut **tx).await?;
        }
    }
    sqlx::query("INSERT INTO game_item_text_write_receipts(game_session_id,command_id,character_id,request_binding,text_revision,changed,written_at) VALUES(encode($1,'hex')::uuid,$2::text::numeric(20,0),encode($3,'hex')::uuid,$4,$5::text::numeric(20,0),$6,$7::text::timestamptz)")
        .bind(session.as_bytes().as_slice()).bind(command).bind(character.as_slice()).bind(binding.as_slice()).bind(revision.to_string()).bind(changed).bind(now).execute(&mut **tx).await?;
    Ok(ItemTextWriteOutcome {
        text_revision: revision,
        changed,
        replayed: false,
    })
}

#[cfg(test)]
#[allow(
    dead_code,
    reason = "PostgreSQL exercises are registered in the canonical PG target; the library unit target has no database"
)]
pub(crate) mod tests {
    use super::*;
    use crate::foundation::{CommandId, GameSessionId};
    use sqlx::Connection;

    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn id(tag: u8) -> [u8; 16] {
        let mut value = [0; 16];
        value[6] = 0x70;
        value[8] = 0x80;
        value[15] = tag;
        value
    }

    fn request(command: u64, text: &str) -> TestResult<ItemTextWriteRequest> {
        Ok(ItemTextWriteRequest {
            command: CommandRef::new(GameSessionId::decode(&id(1))?, CommandId::new(command)?),
            item_instance_id: id(2),
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i3505".into(),
                revision_ref: "definition-r1".into(),
            },
            text: text.into(),
        })
    }

    #[test]
    fn replay_binding_covers_private_text_item_definition_and_character() -> TestResult {
        let original = request(1, "Aldric\nprivate")?;
        let binding = original.binding(&id(3));
        assert_ne!(binding, request(1, "Aldric\nchanged")?.binding(&id(3)));
        assert_ne!(binding, request(2, "Aldric\nprivate")?.binding(&id(3)));
        assert_ne!(binding, original.binding(&id(4)));
        let mut changed = request(1, "Aldric\nprivate")?;
        changed.item_instance_id = id(9);
        assert_ne!(binding, changed.binding(&id(3)));
        changed.item_instance_id = original.item_instance_id;
        changed.definition.revision_ref = "definition-r2".into();
        assert_ne!(binding, changed.binding(&id(3)));
        Ok(())
    }

    #[test]
    fn empty_delete_then_rewrite_never_reuses_revision() -> TestResult {
        assert_eq!(next_revision(0, None, "")?, (0, false));
        assert_eq!(next_revision(0, None, "first")?, (1, true));
        assert_eq!(next_revision(1, Some("first"), "")?, (2, true));
        assert_eq!(next_revision(2, None, "second")?, (3, true));
        assert_eq!(next_revision(3, Some("second"), "second")?, (3, false));
        Ok(())
    }

    #[test]
    fn revision_exhaustion_refuses_change_and_allows_unchanged() -> TestResult {
        assert!(matches!(
            next_revision(u64::MAX, Some("private"), ""),
            Err(ItemTextWriteError::Exhausted)
        ));
        assert_eq!(
            next_revision(u64::MAX, Some("same"), "same")?,
            (u64::MAX, false)
        );
        Ok(())
    }

    // These are storage tests, not fabricated native content/fence proofs. Seed
    // only native roots/items under explicit fixture trigger bypass. Production
    // admission, reach and the complete writer still require separate PG cases
    // when MAIL-CONTENT and the real trade consumer are available.
    async fn storage_fixture(pool: &sqlx::PgPool) -> TestResult<[u8; 16]> {
        let mut connection = pool.acquire().await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut *connection)
            .await?;
        assert_eq!(version, "170006", "native PostgreSQL 17.6 required");
        let mut tx = connection.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let account: String = sqlx::query_scalar("INSERT INTO game_character_account_guards(account_id) VALUES(game_character_uuid_v7()) RETURNING account_id::text")
            .fetch_one(&mut *tx).await?;
        let character: String = sqlx::query_scalar("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) VALUES(game_character_uuid_v7(),$1::uuid,encode($2,'hex')::uuid,1,1,'p1','r1','c1','s1','Aldric') RETURNING character_id::text")
            .bind(account).bind(id(3).as_slice()).fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3505','definition-r1',1,1,game_character_uuid_v7())")
            .bind(id(2).as_slice()).bind(id(3).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(super::super::character_progression::uuid_text(&character)?)
    }

    async fn store(
        pool: &sqlx::PgPool,
        character: &[u8; 16],
        command: u64,
        body: &str,
    ) -> TestResult<ItemTextWriteOutcome> {
        let mut tx = pool.begin().await?;
        // Same real root/item locks as the production path. No claim that
        // locking fixture rows supplies current session/content authority.
        sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(character.as_slice()).execute(&mut *tx).await?;
        sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(id(2).as_slice()).execute(&mut *tx).await?;
        let text =
            MailText::parse(MailTextClass::Letter, body).map_err(|_| "invalid fixture text")?;
        let outcome = persist_rewrite(&mut tx, character, &request(command, body)?, &text).await?;
        tx.commit().await?;
        Ok(outcome)
    }

    pub(crate) async fn text_storage_delete_retains_tombstone_and_rewrite_advances(
        pool: sqlx::PgPool,
    ) -> TestResult {
        let character = storage_fixture(&pool).await?;
        assert_eq!(
            store(&pool, &character, 1, "private").await?.text_revision,
            1
        );
        assert_eq!(store(&pool, &character, 2, "").await?.text_revision, 2);
        let texts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_texts")
            .fetch_one(&pool)
            .await?;
        assert_eq!(texts, 0);
        // Fixture clock advance only; do not sleep or alter a host clock.
        sqlx::query("UPDATE game_item_text_write_receipts SET written_at=clock_timestamp()-interval '2 seconds'")
            .execute(&pool).await?;
        assert_eq!(store(&pool, &character, 3, "new").await?.text_revision, 3);
        let revision: String =
            sqlx::query_scalar("SELECT last_revision::text FROM game_item_text_revisions")
                .fetch_one(&pool)
                .await?;
        assert_eq!(revision, "3");
        Ok(())
    }

    pub(crate) async fn text_storage_rate_refusal_preserves_text_and_revision(
        pool: sqlx::PgPool,
    ) -> TestResult {
        let character = storage_fixture(&pool).await?;
        store(&pool, &character, 1, "one").await?;
        store(&pool, &character, 2, "two").await?;
        // Privileged storage fixture only: retain both persisted tokens in the
        // guard's window despite arbitrary CI delays. Future timestamps remain
        // conservative after a server clock rollback; infinity makes this case
        // independent of elapsed wall time. Production still samples its own
        // clock. This case proves refusal/state preservation, not clock expiry.
        sqlx::query("UPDATE game_item_text_write_receipts SET written_at='infinity'::timestamptz")
            .execute(&pool)
            .await?;
        let mut tx = pool.begin().await?;
        let text = MailText::parse(MailTextClass::Letter, "three").map_err(|_| "fixture")?;
        assert!(matches!(
            persist_rewrite(&mut tx, &character, &request(3, "three")?, &text).await,
            Err(ItemTextWriteError::Exhausted)
        ));
        tx.rollback().await?;
        let row = sqlx::query("SELECT text,text_revision::text FROM game_item_texts")
            .fetch_one(&pool)
            .await?;
        assert_eq!(row.try_get::<String, _>("text")?, "two");
        assert_eq!(row.try_get::<String, _>("text_revision")?, "2");
        // Unchanged text remains a no-op even at the rate limit.
        let unchanged = store(&pool, &character, 4, "two").await?;
        assert!(!unchanged.changed);
        assert_eq!(unchanged.text_revision, 2);
        Ok(())
    }

    pub(crate) async fn text_storage_rollback_loses_neither_atomicity_nor_command_retry(
        pool: sqlx::PgPool,
    ) -> TestResult {
        let character = storage_fixture(&pool).await?;
        let text = MailText::parse(MailTextClass::Letter, "private").map_err(|_| "fixture")?;
        let mut tx = pool.begin().await?;
        persist_rewrite(&mut tx, &character, &request(1, "private")?, &text).await?;
        tx.rollback().await?;
        for query in [
            "SELECT count(*) FROM game_item_texts",
            "SELECT count(*) FROM game_item_text_revisions",
            "SELECT count(*) FROM game_item_text_write_receipts",
        ] {
            assert_eq!(
                sqlx::query_scalar::<_, i64>(query).fetch_one(&pool).await?,
                0
            );
        }
        assert_eq!(
            store(&pool, &character, 1, "private").await?.text_revision,
            1
        );
        Ok(())
    }
}
