//! Authenticated Character creation, including creation sex.
use super::*;

impl DurabilityRoot {
    /// Consume one authenticated `CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1`
    /// decision. In one transaction, or not at all: the sealed recovery fence,
    /// the current #415 incarnation, the current allowed S1/S2 account-security
    /// floor for the intent's AccountId and the unexpired intent are proven; the
    /// intent source high-water advances; and the Character root, receipt with
    /// the complete intent binding, audit event and pending outbox commit.
    /// The intent's world and interpretation are requested context: the world
    /// must currently have an assigned Channel (#415) and the revisions must
    /// equal the current Game-owned interpretation, which is resolved from the
    /// durable configuration inside this transaction, never from the caller.
    /// The requested name must be unreserved in the global namespace; the root
    /// insert reserves its comparison key (migration 0022).
    /// An exact retry returns the committed result; changed reuse conflicts.
    pub async fn bootstrap_character(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        intent: &CharacterBootstrapIntentV1,
    ) -> Result<CharacterAuthorityRecord> {
        let operation = intent.operation_id();
        let intent_binding = intent.binding();
        if !uuid_v7_shape(&operation)
            || intent_binding.len() > MAX_OPERATION_BINDING_BYTES
            || !intent.interpretation().into_iter().all(bounded)
        {
            return Err(CharacterAuthorityError::Rejected);
        }
        let requested = intent.clone();
        let proof = proof.clone();
        let account = *intent.account_id().as_bytes();
        let world = *intent.target_world_id().as_bytes();
        let [profile, ruleset, content, starter] = intent.interpretation().map(str::to_owned);
        let sex = match intent.requested_sex() {
            crate::domain::appearance::Sex::Female => 1_i16,
            crate::domain::appearance::Sex::Male => 2_i16,
        };
        let name = intent.requested_name().as_str().to_owned();
        let name_key = intent.requested_name().comparison_key();
        let decision = intent.issuer_decision_id();
        let source_revision = intent.source_revision();
        let issued_at = intent.issued_at_source();
        let expires_at = intent.expires_at_source();
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            // The write must come from the currently registered process incarnation.
            if !prove_current_incarnation(&mut tx, &proof).await? {
                return Err(DurabilityError::Unavailable);
            }
            // Serialize the operation identity before any account-scoped lock, so a
            // concurrent reuse with another account observes the committed receipt.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-operation:' || encode($1, 'hex'), 0))")
                .bind(operation.as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES (encode($1, 'hex')::uuid) ON CONFLICT DO NOTHING")
                .bind(account.as_slice()).execute(&mut *tx).await?;
            sqlx::query("SELECT account_id FROM game_character_account_guards WHERE account_id = encode($1, 'hex')::uuid FOR UPDATE")
                .bind(account.as_slice()).fetch_one(&mut *tx).await?;
            // One operation identity is held by at most one receipt table (migration 0087).
            if sqlx::query("SELECT 1 FROM game_character_command_receipts WHERE operation_id = encode($1, 'hex')::uuid")
                .bind(operation.as_slice()).fetch_optional(&mut *tx).await?.is_some() {
                return Ok(Err(CharacterAuthorityError::Conflict));
            }
            if let Some(row) = sqlx::query("SELECT o.command_binding, o.character_id::text, o.event_id::text, o.transaction_id::text FROM game_character_operation_receipts o WHERE o.operation_id = encode($1, 'hex')::uuid")
                .bind(operation.as_slice()).fetch_optional(&mut *tx).await? {
                // Reconciliation of the committed decision, never a new authorization.
                let old: Vec<u8> = row.try_get("command_binding")?;
                if old != intent_binding { return Ok(Err(CharacterAuthorityError::Conflict)); }
                let record = read_record_row(&row, account, world)?;
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(record));
            }
            // Source-time validity against trusted Game time; arrival or retry
            // time never refreshes the intent.
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
                .fetch_one(&mut *tx).await?;
            if issued_at > now || expires_at <= now {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            // Game-owned current interpretation, share-locked against a
            // concurrent reconfiguration.
            sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:character-interpretation', 0))")
                .execute(&mut *tx).await?;
            match current_interpretation(&mut tx).await? {
                Some((_, current)) if current.admits(&requested) => {}
                _ => return Ok(Err(CharacterAuthorityError::Rejected)),
            }
            // Game-owned current world: at least one currently assigned Channel
            // (scope_kind 1; a house scope is not a Channel), share-locked so a
            // concurrent revocation serializes with this commit.
            if sqlx::query("SELECT scope_key FROM game_runtime_scope_assignments WHERE world_id = encode($1, 'hex')::uuid AND scope_kind = 1 AND state = 1 LIMIT 1 FOR SHARE")
                .bind(world.as_slice()).fetch_optional(&mut *tx).await?.is_none() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            // Independent prerequisite: current allowed Platform account security.
            if !current_account_security_allows(&mut tx, &account, now).await? {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            // Global name namespace: serialize on the comparison key, then a
            // concurrent same-name bootstrap sees the committed reservation.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-name:' || $1, 0))")
                .bind(&name_key).execute(&mut *tx).await?;
            if sqlx::query("SELECT 1 FROM game_character_name_reservations WHERE name_key = $1")
                .bind(&name_key).fetch_optional(&mut *tx).await?.is_some() {
                return Ok(Err(CharacterAuthorityError::NameUnavailable));
            }
            // Retained issuer/variant source high-water: a lower revision is stale
            // and an equal one is another decision (no receipt matched this one).
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-bootstrap-intent-floor', 0))")
                .execute(&mut *tx).await?;
            let floor: Option<i64> = sqlx::query_scalar("SELECT source_revision FROM game_character_bootstrap_intent_floors WHERE issuer_scope = 1 FOR UPDATE")
                .fetch_optional(&mut *tx).await?;
            if floor.is_some_and(|floor| source_revision <= floor) {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            sqlx::query("INSERT INTO game_character_bootstrap_intent_floors(issuer_scope, source_revision, issuer_decision_id, intent_binding) VALUES (1, $1, encode($2,'hex')::uuid, $3) ON CONFLICT (issuer_scope) DO UPDATE SET source_revision = EXCLUDED.source_revision, issuer_decision_id = EXCLUDED.issuer_decision_id, intent_binding = EXCLUDED.intent_binding")
                .bind(source_revision).bind(decision.as_slice()).bind(&intent_binding).execute(&mut *tx).await?;
            let ids = sqlx::query("SELECT game_character_uuid_v7()::text AS character_id, game_character_uuid_v7()::text AS event_id, game_character_uuid_v7()::text AS transaction_id, floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at")
                .fetch_one(&mut *tx).await?;
            let character = uuid_text(ids.try_get("character_id")?)?;
            let event = uuid_text(ids.try_get("event_id")?)?;
            let transaction = uuid_text(ids.try_get("transaction_id")?)?;
            let occurred_at: i64 = ids.try_get("occurred_at")?;
            let payload = encode_bootstrap(&account, &character, &world);
            sqlx::query("INSERT INTO game_character_roots(character_id, account_id, world_id, lifecycle, character_revision, profile_revision, ruleset_revision, content_revision, starter_template_revision, name, sex) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, 1, 1, $4, $5, $6, $7, $8, $9)")
                .bind(character.as_slice()).bind(account.as_slice()).bind(world.as_slice()).bind(profile).bind(ruleset).bind(content).bind(starter).bind(name).bind(sex).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_audit_outbox(event_id, transaction_id, transaction_ordinal, transaction_count, event_type_id, schema_revision, retention_profile_id, character_id, occurred_at, expires_at, payload, payload_sha256, server_build_id, publication_state) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, encode($6,'hex')::uuid, $9, $9 + 7776000000, $7, sha256($7), $8, 1)")
                .bind(event.as_slice()).bind(transaction.as_slice()).bind(i64::from(EVENT_TYPE_CHARACTER_AUTHORITY_BOOTSTRAPPED)).bind(i64::from(EVENT_SCHEMA_REVISION)).bind(RETENTION_PROFILE).bind(character.as_slice()).bind(&payload).bind(SERVER_BUILD_ID).bind(occurred_at).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_operation_receipts(operation_id, command_binding, account_id, character_id, world_id, character_revision, event_id, transaction_id, server_build_id, occurred_at, issuer_decision_id, intent_source_revision, issued_at_source, expires_at_source) VALUES (encode($1,'hex')::uuid, $2, encode($3,'hex')::uuid, encode($4,'hex')::uuid, encode($5,'hex')::uuid, 1, encode($6,'hex')::uuid, encode($7,'hex')::uuid, $8, $9, encode($10,'hex')::uuid, $11, $12, $13)")
                .bind(operation.as_slice()).bind(&intent_binding).bind(account.as_slice()).bind(character.as_slice()).bind(world.as_slice()).bind(event.as_slice()).bind(transaction.as_slice()).bind(SERVER_BUILD_ID).bind(occurred_at).bind(decision.as_slice()).bind(source_revision).bind(issued_at).bind(expires_at).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(CharacterAuthorityRecord { account_id: AccountId::from_bytes(account).map_err(|_| DurabilityError::Unavailable)?, character_id: CharacterId::from_bytes(character).map_err(|_| DurabilityError::Unavailable)?, world_id: WorldId::from_bytes(world).map_err(|_| DurabilityError::Unavailable)?, revision: CharacterRevision::new(1).map_err(|_| DurabilityError::Unavailable)?, event_id: event, transaction_id: transaction, payload }))
        })).await?
    }
}
