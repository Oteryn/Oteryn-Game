//! W21: one session-fenced transaction for a private difficulty draft and its replay receipt.
use super::admission_authority_guards::AdmissionGuardStore;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_item_only_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::{NodeIncarnationProof, assignment_scope_key};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
};
use oteryn_protocol_oteryn::boss_difficulty::{self as wire, Refusal, Reply, Request};
use sqlx::{Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, DurabilityError>;

impl DurabilityRoot {
    /// The client may identify a catalogue key or a context. It supplies no revision, owner,
    /// scope, stat multiplier or encounter grant. Start is composed with native room admission.
    pub async fn boss_difficulty_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command_id: u64,
        payload: &[u8],
    ) -> Result<Option<Vec<u8>>> {
        let Ok(request) = wire::decode_request(payload) else {
            return Ok(None);
        };
        if command_id == 0 {
            return Ok(None);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let payload = payload.to_vec();
        let guards = AdmissionGuardStore::from_root(self.clone());
        let admissions = super::fresh_admission::FreshAdmissionStore::from_root(self.clone());
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let character_id = crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if !admissions.committed_session_use(&mut tx, character_id, fence.game_session_id).await? {
                return Ok(None);
            }
            let character = match guards.load_locked(&mut tx, &AdmissionAuthorityGuardKeyV1::Character(character_id)).await {
                Ok(Some(value)) => value,
                Ok(None) | Err(DurabilityError::InvalidStoredState) => return Ok(None),
                Err(error) => return Err(error),
            };
            let AdmissionAuthorityGuardStateV1::Character { account_id, .. } = character.state else { return Ok(None); };
            for key in [AdmissionAuthorityGuardKeyV1::Account { account_id }, AdmissionAuthorityGuardKeyV1::Runtime(fence.runtime_scope)] {
                match guards.load_locked(&mut tx, &key).await {
                    Ok(Some(_)) => {},
                    Ok(None) | Err(DurabilityError::InvalidStoredState) => return Ok(None),
                    Err(error) => return Err(error),
                }
            }
            if assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() { return Ok(None); }
            // Serializes initial receipt absence and the one-open-context replacement together.
            let root: Option<i32> = sqlx::query_scalar("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            if root.is_none() { return Ok(None); }
            let previous: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as(
                "SELECT request,reply FROM game_boss_difficulty_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND connection_generation=$3::text::numeric AND command_id=$4::text::numeric"
            ).bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                .bind(fence.connection_generation.get().to_string()).bind(command_id.to_string()).fetch_optional(&mut *tx).await?;
            if let Some((old_request, reply)) = previous {
                if old_request != payload {
                    return Ok(Some(encode(&Reply::Refused { context: request.context(), reason: Refusal::ReplayConflict })?));
                }
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Some(reply));
            }
            let reply = match &request {
                Request::Open { boss_key } => open(&mut tx, fence, boss_key, false).await?,
                Request::OpenWithProgress { boss_key } => open(&mut tx, fence, boss_key, true).await?,
                _ => act(&mut tx, fence, command_id, &request).await?,
            };
            let reply = encode(&reply)?;
            sqlx::query("INSERT INTO game_boss_difficulty_receipts(character_id,game_session_id,connection_generation,command_id,request,reply) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5,$6)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                .bind(fence.connection_generation.get().to_string()).bind(command_id.to_string())
                .bind(&payload).bind(&reply).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Some(reply))
        })).await
    }
}

fn encode(reply: &Reply) -> Result<Vec<u8>> {
    wire::encode_reply(reply).map_err(|_| DurabilityError::InvalidStoredState)
}

async fn open(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    boss_key: &str,
    progress_details: bool,
) -> Result<Reply> {
    let revision: Option<String> = sqlx::query_scalar(
        "SELECT content_revision FROM game_boss_room_catalogue WHERE boss_key=$1 AND enabled FOR SHARE"
    ).bind(boss_key).fetch_optional(&mut **tx).await?;
    let Some(content_revision) = revision else {
        return Ok(Reply::Refused {
            context: None,
            reason: Refusal::BossUnavailable,
        });
    };
    let (personal_highest, unlocked_max, maximum_difficulty, boss_creature_key) =
        progress(tx, fence, boss_key, progress_details).await?;
    let selected_difficulty = u16::from(unlocked_max > 0);
    // At most one row is affected, enforced by the partial unique index.
    sqlx::query("UPDATE game_boss_difficulty_contexts SET closed=true WHERE character_id=encode($1,'hex')::uuid AND NOT closed")
        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    let context: Vec<u8> = sqlx::query_scalar(
        "INSERT INTO game_boss_difficulty_contexts(context_id,character_id,game_session_id,connection_generation,character_lease_generation,scope_key,scope_ownership_generation,boss_key,content_revision,selected_difficulty,progress_details) VALUES(gen_random_uuid(),encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5,$6::text::numeric,$7,$8,$9,$10) RETURNING decode(replace(context_id::text,'-',''),'hex')"
    ).bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .bind(assignment_scope_key(fence.runtime_scope).as_slice()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(boss_key).bind(&content_revision).bind(i16::try_from(selected_difficulty).map_err(|_|DurabilityError::InvalidStoredState)?).bind(progress_details).fetch_one(&mut **tx).await?;
    Ok(Reply::Window {
        context: context
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        boss_key: boss_key.to_owned(),
        boss_creature_key,
        content_revision,
        selected_difficulty,
        personal_highest,
        unlocked_max,
        maximum_difficulty,
    })
}

async fn act(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    command_id: u64,
    request: &Request,
) -> Result<Reply> {
    let context = request
        .context()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let refusal = |reason| Reply::Refused {
        context: Some(context),
        reason,
    };
    let row = sqlx::query(
        "SELECT boss_key,content_revision,closed,progress_details FROM game_boss_difficulty_contexts WHERE context_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND game_session_id=encode($3,'hex')::uuid AND connection_generation=$4::text::numeric AND character_lease_generation=$5::text::numeric AND scope_key=$6 AND scope_ownership_generation=$7::text::numeric FOR UPDATE"
    ).bind(context.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .bind(assignment_scope_key(fence.runtime_scope).as_slice()).bind(fence.scope_ownership_generation.get().to_string())
        .fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(refusal(Refusal::ContextMissing));
    };
    if row.try_get::<bool, _>("closed")? {
        return Ok(refusal(Refusal::ContextClosed));
    }
    if matches!(request, Request::Cancel { .. }) {
        sqlx::query("UPDATE game_boss_difficulty_contexts SET closed=true WHERE context_id=encode($1,'hex')::uuid")
            .bind(context.as_slice()).execute(&mut **tx).await?;
        return Ok(Reply::Closed { context });
    }
    let boss_key: String = row.try_get("boss_key")?;
    let content_revision: String = row.try_get("content_revision")?;
    let current: Option<String> = sqlx::query_scalar(
        "SELECT content_revision FROM game_boss_room_catalogue WHERE boss_key=$1 AND enabled FOR SHARE"
    ).bind(&boss_key).fetch_optional(&mut **tx).await?;
    if current.as_ref() != Some(&content_revision) {
        return Ok(refusal(Refusal::ContentChanged));
    }
    let (personal_highest, unlocked_max, maximum_difficulty, boss_creature_key) =
        progress(tx, fence, &boss_key, row.try_get("progress_details")?).await?;
    match request {
        Request::Start { .. } if unlocked_max == 0 => Ok(refusal(Refusal::NoAcceptedProfile)),
        Request::Start { difficulty, .. } if *difficulty == 0 || *difficulty > unlocked_max => {
            Ok(refusal(Refusal::DifficultyLocked))
        }
        Request::Preview { difficulty, .. } | Request::Start { difficulty, .. } => {
            let difficulty = (*difficulty).min(maximum_difficulty);

            sqlx::query("UPDATE game_boss_difficulty_contexts SET selected_difficulty=$2,start_command_id=$3::text::numeric WHERE context_id=encode($1,'hex')::uuid")
                .bind(context.as_slice()).bind(i16::try_from(difficulty).map_err(|_| DurabilityError::InvalidStoredState)?)
                .bind(matches!(request, Request::Start { .. }).then(|| command_id.to_string()))
                .execute(&mut **tx).await?;
            Ok(Reply::Window {
                context,
                boss_key,
                boss_creature_key,
                content_revision,
                selected_difficulty: difficulty,
                personal_highest,
                unlocked_max,
                maximum_difficulty,
            })
        }
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

async fn progress(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    boss: &str,
    progress_details: bool,
) -> Result<(u16, u16, u16, String)> {
    let creature: String = sqlx::query_scalar(
        "SELECT boss_creature_key FROM game_boss_room_catalogue WHERE boss_key=$1",
    )
    .bind(boss)
    .fetch_one(&mut **tx)
    .await?;
    if !progress_details {
        return Ok((0, 0, 25, creature));
    }
    let maximum = wire::boss_maximum(boss).or_else(|| wire::boss_maximum(&creature));
    let highest: Option<i16> = sqlx::query_scalar("SELECT highest_defeated FROM game_character_boss_difficulty_records WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(boss).fetch_optional(&mut **tx).await?;
    let highest =
        u16::try_from(highest.unwrap_or(0)).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok((
        highest,
        maximum.map_or(0, |m| (highest + 1).min(m)),
        maximum.unwrap_or(25),
        creature,
    ))
}
