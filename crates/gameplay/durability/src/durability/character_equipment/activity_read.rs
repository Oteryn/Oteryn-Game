//! Read-only equipment projection for an actual committed activity participant.
//! The authority never leaves this transaction and authorizes no equipment move.
use super::*;

async fn assert_current_activity_equipment_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    content_digest: [u8; 32],
) -> Result<EquipmentAuthority> {
    let crate::foundation::RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = fence.runtime_scope
    else {
        return Err(rejected("activity equipment Instance scope"));
    };
    if content_digest == [0; 32] {
        return Err(rejected("activity equipment Content pin"));
    }
    assert_recovery_fence(tx, recovery).await?;
    super::super::db::lock_admission_relations(tx).await?;
    let revision: String = sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| rejected("activity equipment Character root"))?;
    let gameplay = super::super::character_progression::CurrentCharacterGameplayFence {
        character_id: fence.character_id,
        game_session_id: fence.game_session_id,
        connection_generation: fence.connection_generation,
        character_lease_generation: fence.character_lease_generation,
        runtime_scope: fence.runtime_scope,
        scope_ownership_generation: fence.scope_ownership_generation,
        expected_character_revision: crate::domain::CharacterRevision::new(number(revision)?)
            .map_err(|_| rejected("activity equipment Character revision"))?,
    };
    // The current Character root, session, lease, Instance assignment, holder
    // incarnation and publication are checked before any equipment is exposed.
    super::super::character_progression::assert_gameplay_fence(tx, &gameplay, node)
        .await?
        .map_err(|_| rejected("activity equipment current lifecycle fence"))?;
    let entry = sqlx::query(
        "SELECT uuid_send(p.origin_channel_id) AS origin_channel,e.destination_server_artifact_digest AS content_digest \
         FROM game_activity_handoff_groups g \
         JOIN game_activity_handoff_participants p USING(handoff_id,world_id) \
         JOIN game_activity_runtime_entries e USING(handoff_id) \
         WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid \
           AND g.state=2 AND p.character_id=encode($3,'hex')::uuid \
           AND p.destination_game_session_id=encode($4,'hex')::uuid",
    )
    .bind(world_id.as_bytes().as_slice())
    .bind(instance_id.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.game_session_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| rejected("equipment requires committed activity participation"))?;
    // An activity retains its destination generation. A later activation in
    // its origin Channel cannot reinterpret the Instance's equipment policies.
    if entry.try_get::<Vec<u8>, _>("content_digest")? != content_digest {
        return Err(rejected("activity equipment destination Content pin"));
    }
    let character = sqlx::query(
        "SELECT character_revision::text,account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    Ok(EquipmentAuthority {
        physical_transaction: sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut **tx)
            .await?,
        fence: *fence,
        command: None,
        world: *world_id.as_bytes(),
        channel: entry
            .try_get::<Vec<u8>, _>("origin_channel")?
            .try_into()
            .map_err(|_| rejected("activity equipment origin Channel identity"))?,
        content_digest,
        character_revision: number(character.try_get("character_revision")?)?,
        account: uuid(character.try_get("account_id")?)?,
    })
}

/// Keep both the read-only authority and the concrete refusal inside this owner.
pub(super) async fn read(
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    content_digest: [u8; 32],
) -> Result<MovementEquipmentSnapshot> {
    let record = recovery
        .record_for(root)
        .map_err(|_| rejected("activity equipment recovery root"))?;
    let node = node.clone();
    let fence = *fence;
    root.try_issue_semantic_pass()?
        .run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
                let authority = match assert_current_activity_equipment_in_transaction(
                    &mut tx,
                    &record,
                    &node,
                    &fence,
                    content_digest,
                )
                .await
                {
                    Ok(authority) => authority,
                    Err(error) => return Ok(Err(error)),
                };
                let equipment = match read_equipment_in_transaction(&mut tx, &authority).await {
                    Ok(equipment) => equipment,
                    Err(error) => return Ok(Err(error)),
                };
                super::super::db::commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(MovementEquipmentSnapshot { fence, equipment }))
            })
        })
        .await?
}

/// Lifecycle reads retain the caller's Character revision as well as current
/// activity membership. No equipment authority escapes this transaction.
pub(super) async fn read_lifecycle(
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    gameplay: &super::super::character_progression::CurrentCharacterGameplayFence,
    content_digest: [u8; 32],
) -> Result<RawLifecycleDurableFacts> {
    let record = recovery
        .record_for(root)
        .map_err(|_| rejected("activity lifecycle recovery root"))?;
    let node = node.clone();
    let gameplay = *gameplay;
    let fence = gameplay.item_fence();
    root.try_issue_semantic_pass()?
        .run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
                let authority = match assert_current_activity_equipment_in_transaction(
                    &mut tx,
                    &record,
                    &node,
                    &fence,
                    content_digest,
                )
                .await
                {
                    Ok(authority) => authority,
                    Err(error) => return Ok(Err(error)),
                };
                if authority.character_revision != gameplay.expected_character_revision.get() {
                    return Ok(Err(rejected("activity lifecycle Character revision")));
                }
                let facts = match read_lifecycle_facts_in_transaction(&mut tx, &authority).await {
                    Ok(facts) => facts,
                    Err(error) => return Ok(Err(error)),
                };
                super::super::db::commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(facts))
            })
        })
        .await?
}
