//! CP option (a): crash-left sessions of ended holders, under a sealed reset.
use super::*;
use crate::durability::DurabilityRoot;
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::durability::runtime_scope_assignment::{
    NodeIncarnationProof, prove_current_incarnation,
};
use crate::durability::world_reset::RetiringWorldReset;
use sqlx::Row;

#[derive(Clone)]
pub(super) struct RetiredHolderProof {
    node: NodeIncarnationProof,
    reset: RetiringWorldReset,
}

// This is re-run inside the existing terminal-release transaction, after its
// current session and admission locks. An earlier batch read grants nothing.
pub(super) async fn validate(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    proof: &RetiredHolderProof,
    current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
) -> Result<bool> {
    if current.commit().world_id() != proof.reset.world
        || !super::super::world_reset::loss_binding_current(tx, proof.reset.loss_cause()).await?
        || !prove_current_incarnation(tx, &proof.node).await?
    {
        return Ok(false);
    }
    let fact = proof.node.fact();
    // The old holder comes from the immutable assignment receipt of the
    // session's exact generation. The independently current assignment must
    // belong to this proven incarnation and have a strictly newer generation.
    let rows = sqlx::query(
        "SELECT s.session_state,s.control_loss_epoch::text,s.original_grace_deadline, \
         floor(extract(epoch FROM clock_timestamp()))::bigint AS now \
         FROM game_durability_reconnect_sessions s \
         JOIN game_runtime_scope_assignments a ON a.scope_key=CASE s.runtime_scope_kind \
          WHEN 1 THEN decode('01','hex')||uuid_send(s.runtime_scope_world_id)||uuid_send(s.runtime_scope_channel_id) \
          WHEN 2 THEN game_house_scope_key(s.runtime_scope_world_id,s.runtime_scope_house_key) END \
         JOIN game_runtime_scope_assignment_receipts r ON r.scope_key=a.scope_key \
          AND r.ownership_generation=s.scope_ownership_generation AND r.state=1 \
         JOIN game_node_registrations n ON n.node_id=r.holder_node_id \
          AND n.registration_revision=r.holder_registration_revision \
         WHERE s.game_session_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid \
          AND s.session_state<>3 AND a.world_id=s.world_id AND s.runtime_scope_world_id=s.world_id AND a.state=1 AND a.ownership_generation>s.scope_ownership_generation \
          AND a.holder_node_id=encode($3,'hex')::uuid AND a.holder_registration_revision=$4::text::numeric \
          AND n.state<>1 AND EXISTS(SELECT 1 FROM game_node_registration_endings e WHERE e.node_id=n.node_id) \
         LIMIT 2 FOR SHARE OF a,n",
    )
    .bind(current.current_game_session_id().as_bytes().as_slice())
    .bind(proof.reset.world.as_bytes().as_slice())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let Some(row) = rows.first() else {
        return Ok(false);
    };
    let now = row.try_get::<i64, _>("now")?;
    match row.try_get::<i16, _>("session_state")? {
        2 => Ok(true),
        1 => Ok(row
            .try_get::<Option<String>, _>("control_loss_epoch")?
            .is_some()
            && row
                .try_get::<Option<i64>, _>("original_grace_deadline")?
                .is_some_and(|deadline| now >= deadline)),
        _ => Ok(false),
    }
}

pub(in crate::durability) async fn release_retired_holder_sessions(
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    reset: RetiringWorldReset,
) -> Result<()> {
    let proof = RetiredHolderProof {
        node: node.clone(),
        reset,
    };
    let mut store = FreshAdmissionStore::from_root(root.clone());
    store.retired_holder = Some(proof.clone());
    let mut after: Option<[u8; 16]> = None;
    loop {
        let proof_for_pass = proof.clone();
        let rows = root.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            super::super::db::lock_admission_relations(&mut tx).await?;
            if !super::super::world_reset::loss_binding_current(&mut tx, reset.loss_cause()).await?
                || !prove_current_incarnation(&mut tx, &proof_for_pass.node).await? {
                return Err(DurabilityError::Unavailable);
            }
            let rows = sqlx::query("SELECT s.game_session_id::text,s.account_id::text FROM game_durability_reconnect_sessions s WHERE s.world_id=encode($1,'hex')::uuid AND s.session_state<>3 AND (s.session_state=2 OR (s.session_state=1 AND s.control_loss_epoch IS NOT NULL AND s.original_grace_deadline<=floor(extract(epoch FROM clock_timestamp()))::bigint)) AND ($2::bytea IS NULL OR s.game_session_id>encode($2,'hex')::uuid) AND EXISTS(SELECT 1 FROM game_runtime_scope_assignment_receipts r JOIN game_node_registrations n ON n.node_id=r.holder_node_id AND n.registration_revision=r.holder_registration_revision WHERE r.scope_key=CASE s.runtime_scope_kind WHEN 1 THEN decode('01','hex')||uuid_send(s.runtime_scope_world_id)||uuid_send(s.runtime_scope_channel_id) WHEN 2 THEN game_house_scope_key(s.runtime_scope_world_id,s.runtime_scope_house_key) END AND r.ownership_generation=s.scope_ownership_generation AND r.state=1 AND n.state<>1) ORDER BY s.game_session_id LIMIT 64")
                .bind(reset.world.as_bytes().as_slice()).bind(after.map(|id|id.to_vec()))
                .fetch_all(&mut *tx).await?;
            let rows = rows.iter().map(|row| Ok((super::super::item_mint::uuid_text(row.try_get("game_session_id")?)?,row.try_get::<String,_>("account_id")?)))
                .collect::<Result<Vec<_>>>()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(rows)
        })).await?;
        if rows.is_empty() {
            return Ok(());
        }
        for (session, account) in rows {
            after = Some(session);
            let session =
                GameSessionId::decode(&session).map_err(|_| DurabilityError::InvalidStoredState)?;
            let (current, now) = store.current_session_at(session).await?;
            if current.session_state() == GameSessionState::Terminal {
                continue;
            }
            // `release` repeats the proof under its deciding transaction. A
            // refusal, lost reply or concurrent replacement uses its existing
            // lifecycle reconciliation rather than inventing another release.
            let probe = proof.clone();
            let valid = root
                .try_issue_semantic_pass()?
                .run(move |holder, deadline| {
                    Box::pin(async move {
                        let mut tx = begin_semantic_transaction(holder, deadline).await?;
                        super::super::db::lock_admission_relations(&mut tx).await?;
                        let valid = validate(&mut tx, &probe, current).await?;
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(valid)
                    })
                })
                .await?;
            if valid {
                store.release_current_claims(current, now, &account).await?;
            }
        }
    }
}
