//! The native Guildhall participant runs inside the existing House owner cycle.
use super::*;
use crate::durability::spell_owner_commit::SpellLanePermit;

pub(super) async fn drain(
    root: &DurabilityRoot,
    permit: &SpellLanePermit,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    generation: u64,
) -> Result<()> {
    let recovery = authority
        .record_for(root)
        .map_err(|_| HouseDispositionError::Rejected)?;
    let p = scope_payload(scope, generation)?;
    let discover_p = p.clone();
    let discover_node = node.clone();
    let discover_recovery = recovery.clone();
    let mut context = permit;
    let candidates = root
        .try_issue_semantic_pass()?
        .run_with_context(&mut context, move |holder, deadline, permit| {
            Box::pin(async move {
                let mut tx = super::super::spell_item_transaction::begin_spell_owner_transaction(
                    holder, deadline,
                )
                .await?;
                super::super::spell_item_transaction::assert_spell_item_scope_with_recovery(
                    &mut tx,
                    permit,
                    &discover_recovery,
                    &discover_node,
                    scope,
                    generation,
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                let result = job(&mut tx, &discover_node, "candidates", &discover_p).await?;
                let candidates = result
                    .as_array()
                    .ok_or(DurabilityError::InvalidStoredState)?
                    .iter()
                    .map(|h| {
                        h.as_str()
                            .map(str::to_owned)
                            .ok_or(DurabilityError::InvalidStoredState)
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(candidates)
            })
        })
        .await?;
    for house in candidates {
        let node = node.clone();
        let recovery = recovery.clone();
        let mut p = p.clone();
        p["house"] = house.into();
        root.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, permit| {
                Box::pin(async move {
                    let mut tx =
                        super::super::spell_item_transaction::begin_spell_owner_transaction(
                            holder, deadline,
                        )
                        .await?;
                    super::super::spell_item_transaction::assert_spell_item_scope_with_recovery(
                        &mut tx, permit, &recovery, &node, scope, generation,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let result = job(&mut tx, &node, "advance", &p).await?;
                    if let Some(operation) = result.as_str() {
                        audit::write(&mut tx, &node, &p, operation).await?;
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(())
                })
            })
            .await?;
    }
    Ok(())
}
async fn job(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    mode: &str,
    p: &Value,
) -> std::result::Result<Value, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT game_guildhall_job_port($1,$2,encode($3,'hex')::uuid,$4::text::numeric,$5)",
    )
    .bind(mode)
    .bind(p)
    .bind(node.fact().node_id().as_bytes().as_slice())
    .bind(node.fact().registration_revision().to_string())
    .bind(node.sql_possession_secret())
    .fetch_one(&mut **tx)
    .await?)
}
