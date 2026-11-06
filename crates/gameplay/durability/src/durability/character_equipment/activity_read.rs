//! Read-only equipment under a current, shared-owner admitted activity Instance.
use super::*;

pub(super) async fn assert_read_authority(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    content_digest: [u8; 32],
) -> Result<EquipmentAuthority> {
    let crate::foundation::RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = fence.runtime_scope
    else {
        return Err(rejected("equipment activity Instance"));
    };
    if content_digest == [0; 32] {
        return Err(rejected("equipment activity Content pin"));
    }
    let character = sqlx::query("SELECT character_revision::text,account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let revision = number(character.try_get("character_revision")?)?;
    let gameplay = super::super::character_progression::CurrentCharacterGameplayFence {
        character_id: fence.character_id,
        game_session_id: fence.game_session_id,
        connection_generation: fence.connection_generation,
        character_lease_generation: fence.character_lease_generation,
        runtime_scope: fence.runtime_scope,
        scope_ownership_generation: fence.scope_ownership_generation,
        expected_character_revision: crate::domain::CharacterRevision::new(revision)
            .map_err(|_| rejected("equipment activity Character revision"))?,
    };
    super::super::character_progression::assert_gameplay_fence(tx, &gameplay, node)
        .await?
        .map_err(|_| rejected("equipment current activity fence"))?;
    // Retained shared entry, rather than caller-supplied origin metadata, owns the
    // Instance's content and the Channel whose equipment custody it carries.
    let origin: Option<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(p.origin_channel_id) FROM game_activity_handoff_groups g JOIN game_activity_handoff_participants p USING(handoff_id) JOIN game_activity_runtime_entries e USING(handoff_id) WHERE g.state=2 AND g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid AND g.ownership_generation=$3::text::numeric AND p.character_id=encode($4,'hex')::uuid AND p.destination_game_session_id=encode($5,'hex')::uuid AND e.destination_server_artifact_digest=$6 AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE r.handoff_id=p.handoff_id AND r.character_id=p.character_id)")
        .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice())
        .bind(fence.scope_ownership_generation.get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(content_digest.as_slice()).fetch_optional(&mut **tx).await?;
    let channel: [u8; 16] = origin
        .ok_or_else(|| rejected("equipment committed activity entry"))?
        .try_into()
        .map_err(|_| rejected("equipment activity origin"))?;
    let authority = EquipmentAuthority {
        physical_transaction: sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut **tx)
            .await?,
        fence: *fence,
        command: None,
        world: *world_id.as_bytes(),
        channel,
        content_digest,
        character_revision: revision,
        account: uuid(character.try_get("account_id")?)?,
    };
    Ok(authority)
}

impl DurabilityRoot {
    /// Read the real custody graph after the shared owner proves the current Instance.
    /// The sealed authority never escapes this read transaction or authorizes a move.
    pub async fn read_activity_equipment(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        content_digest: [u8; 32],
    ) -> Result<MovementEquipmentSnapshot> {
        let record = recovery
            .record_for(self)
            .map_err(|_| rejected("equipment recovery root"))?;
        let node = node.clone();
        let fence = *fence;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::super::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    super::super::db::lock_admission_relations(&mut tx).await?;
                    let authority = assert_read_authority(&mut tx, &node, &fence, content_digest)
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                    let equipment = read_equipment_in_transaction(&mut tx, &authority)
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(MovementEquipmentSnapshot { fence, equipment })
                })
            })
            .await
            .map_err(EquipmentError::from)
    }
}
