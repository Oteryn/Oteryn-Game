pub(super) async fn has_owning_loss_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: &[u8],
    epoch: u64,
) -> Result<bool> {
    if session.len() != 16 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut key = b"owning-loss-v1".to_vec();
    key.extend_from_slice(session);
    key.extend_from_slice(&epoch.to_be_bytes());
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_admission_lifecycle_receipts WHERE operation_key = $1)")
        .bind(key).fetch_one(&mut **tx).await.map_err(DurabilityError::from)
}

fn json_text<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or(DurabilityError::InvalidStoredState)
}
fn json_u64(value: &serde_json::Value, key: &str) -> Result<u64> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .ok_or(DurabilityError::InvalidStoredState)
}
fn decode_hex(text: &str) -> Result<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return Err(DurabilityError::InvalidStoredState);
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| checked(u8::from_str_radix(checked(std::str::from_utf8(pair))?, 16)))
        .collect()
}
fn json_uuid(value: &serde_json::Value, key: &str) -> Result<[u8; 16]> {
    checked(decode_hex(&json_text(value, key)?.replace('-', ""))?.try_into())
}
fn json_bytea(value: &serde_json::Value, key: &str) -> Result<Vec<u8>> {
    decode_hex(
        json_text(value, key)?
            .strip_prefix("\\x")
            .ok_or(DurabilityError::InvalidStoredState)?,
    )
}

struct SessionUseState {
    revision: u64,
    complete: bool,
}

async fn session_use_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> Result<SessionUseState> {
    use sqlx::Row;
    let row = sqlx::query("SELECT version, complete, revision::text, revision_floor::text FROM game_durability_session_use_ledgers WHERE character_id = encode($1, 'hex')::uuid FOR UPDATE")
        .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let (revision, complete) = if let Some(row) = row {
        let revision = checked(row.try_get::<String, _>("revision")?.parse::<u64>())?;
        let floor = checked(row.try_get::<String, _>("revision_floor")?.parse::<u64>())?;
        if row.try_get::<i16, _>("version")? != 1
            || revision != floor
            || revision > GAME_SESSION_USE_LEDGER_CAPACITY_V1
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        (revision, row.try_get::<bool, _>("complete")?)
    } else {
        let existing: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions WHERE character_id = encode($1, 'hex')::uuid)")
            .bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if existing {
            return Err(DurabilityError::InvalidStoredState);
        }
        (0, true)
    };
    let (count, maximum): (i64, Option<String>) = sqlx::query_as("SELECT count(*), max(membership_revision)::text FROM game_durability_session_use_memberships WHERE character_id = encode($1, 'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let maximum = maximum
        .map(|n| checked(n.parse::<u64>()))
        .transpose()?
        .unwrap_or(0);
    if checked(u64::try_from(count))? != revision || maximum != revision {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(SessionUseState { revision, complete })
}

impl FreshAdmissionStore {
    /// Retained session identity-use proof for a current participant consumer. For
    /// an original fresh session, also validate its bounded admission receipt and
    /// its exact membership/retained-origin binding. Legacy and replacement sessions
    /// retain the identity-use check; this does not prove their admission lineage.
    /// The caller holds `lock_admission_relations` in its bounded transaction.
    /// This never proves an activity handoff or current lease.
    pub(super) async fn committed_session_use(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character: CharacterId,
        session: GameSessionId,
    ) -> Result<bool> {
        use sqlx::Row;
        let state = match session_use_state(tx, character).await {
            Ok(state) => state,
            Err(DurabilityError::InvalidStoredState) => return Ok(false),
            Err(error) => return Err(error),
        };
        if !state.complete || state.revision == 0 {
            return Ok(false);
        }
        // session_use_state already proves the bounded, gap-free revision range.
        // NULL bindings are legacy observations, not committed issuance evidence.
        // Find an original fresh receipt by its immutable, unique session identity,
        // independently of the live projection's origin pointers. A damaged
        // projection must not hide that retained origin by dropping both pointers.
        let row = sqlx::query(
            "SELECT m.operation_binding,s.fresh_replay_key,s.initial_fresh_replay_key, \
                    r.replay_key AS issued_replay_key \
             FROM game_durability_session_use_memberships m \
             JOIN game_durability_reconnect_sessions s USING (game_session_id) \
             LEFT JOIN game_durability_fresh_admission_receipts r USING (game_session_id) \
             WHERE m.game_session_id=encode($1,'hex')::uuid \
               AND m.character_id=encode($2,'hex')::uuid \
               AND m.operation_binding IS NOT NULL \
               AND m.operation_binding <> decode(repeat('00',16),'hex')",
        )
        .bind(session.as_bytes().as_slice())
        .bind(character.as_bytes().as_slice())
        .fetch_optional(&mut **tx)
        .await?;
        let Some(row) = row else {
            return Ok(false);
        };
        let activity_origin: bool = sqlx::query_scalar("SELECT COALESCE(game_activity_session_origin_valid(encode($1,'hex')::uuid,encode($2,'hex')::uuid),false)")
            .bind(session.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if !activity_origin {
            return Ok(false);
        }
        let Some(replay) = row.try_get::<Option<Vec<u8>>, _>("issued_replay_key")? else {
            return Ok(true);
        };
        let fresh: Option<Vec<u8>> = row.try_get("fresh_replay_key")?;
        let initial: Option<Vec<u8>> = row.try_get("initial_fresh_replay_key")?;
        if fresh.as_ref().or(initial.as_ref()) != Some(&replay)
            || (fresh.is_some() && initial.is_some() && fresh != initial)
        {
            return Ok(false);
        }
        let receipt = match self.receipt_locked(tx, &replay).await {
            Ok(Some(receipt)) => receipt,
            Ok(None) | Err(DurabilityError::InvalidStoredState) => return Ok(false),
            Err(error) => return Err(error),
        };
        let binding: Vec<u8> = row.try_get("operation_binding")?;
        let origin = receipt.binding();
        Ok(origin.candidate_session == session
            && checked(origin.initial_commit())?.character_id() == character
            && origin.transport.to_bytes().as_slice() == binding.as_slice())
    }
}

/// Consumed only inside the caller's already serialized session transaction.
/// No membership can survive a rollback of the corresponding session effects.
pub(super) async fn commit_session_use(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    candidate: GameSessionId,
    operation_binding: [u8; 16],
    expected_revision: u64,
) -> Result<()> {
    let state = session_use_state(tx, character).await?;
    if !state.complete || state.revision != expected_revision || operation_binding == [0; 16] {
        return Err(DurabilityError::InvalidStoredState);
    }
    let revision = state
        .revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if revision > GAME_SESSION_USE_LEDGER_CAPACITY_V1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("INSERT INTO game_durability_session_use_ledgers (character_id, version, complete, revision, revision_floor) VALUES (encode($1,'hex')::uuid,1,TRUE,0,0) ON CONFLICT DO NOTHING")
        .bind(character.as_bytes().as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_durability_session_use_memberships (game_session_id, character_id, membership_revision, operation_binding) VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),$4)")
        .bind(candidate.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).bind(revision.to_string()).bind(operation_binding.as_slice()).execute(&mut **tx).await?;
    let updated = sqlx::query("UPDATE game_durability_session_use_ledgers SET revision = $2::text::numeric(20,0), revision_floor = $2::text::numeric(20,0) WHERE character_id = encode($1,'hex')::uuid AND revision = $3::text::numeric(20,0) AND revision_floor = $3::text::numeric(20,0) AND complete")
        .bind(character.as_bytes().as_slice()).bind(revision.to_string()).bind(expected_revision.to_string()).execute(&mut **tx).await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
