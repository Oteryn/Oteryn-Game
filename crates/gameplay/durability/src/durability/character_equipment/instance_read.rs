//! Read-only portable equipment under the actual committed activity Instance.
//! No equipment writer authority escapes this helper; the fence stays tagged
//! Instance, while its authenticated origin names the Content activation only.
use super::*;

pub(super) async fn read_instance_equipment_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    content_digest: [u8; 32],
) -> Result<EquipmentSnapshot> {
    let crate::foundation::RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = fence.runtime_scope
    else {
        return Err(rejected("equipment actual Instance scope"));
    };
    if content_digest == [0; 32] {
        return Err(rejected("equipment Content pin"));
    }
    assert_recovery_fence(tx, recovery).await?;
    super::super::db::lock_admission_relations(tx).await?;
    // The origin is retained committed provenance, never an invented Channel
    // interpretation of the Instance fence. Entries and origins are immutable.
    let origin=sqlx::query("SELECT uuid_send(r.origin_channel_id) AS origin,e.destination_server_artifact_digest AS digest FROM game_durability_reconnect_sessions r JOIN game_activity_runtime_entries e ON e.handoff_id=r.activity_handoff_id JOIN game_activity_handoff_groups g ON g.handoff_id=e.handoff_id WHERE r.game_session_id=encode($1,'hex')::uuid AND r.character_id=encode($2,'hex')::uuid AND r.world_id=encode($3,'hex')::uuid AND r.runtime_scope_kind=2 AND r.runtime_scope_world_id=r.world_id AND r.runtime_scope_channel_id IS NULL AND r.runtime_scope_house_key IS NULL AND r.runtime_scope_instance_id=encode($4,'hex')::uuid AND g.world_id=r.world_id AND g.instance_id=r.runtime_scope_instance_id AND g.state=2 AND r.session_state IN(1,2)")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice())
        .fetch_optional(&mut **tx).await?.ok_or(rejected("equipment committed Instance origin"))?;
    let channel: Vec<u8> = origin.try_get("origin")?;
    let channel: [u8; 16] = channel
        .try_into()
        .map_err(|_| rejected("equipment origin width"))?;
    let pinned: Vec<u8> = origin.try_get("digest")?;
    if pinned.as_slice() != content_digest.as_slice() {
        return Err(rejected("equipment Instance Content pin"));
    }
    sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea,'hex')::uuid::text || ':' || encode($2::bytea,'hex')::uuid::text,0))")
        .bind(world_id.as_bytes().as_slice()).bind(channel.as_slice()).execute(&mut **tx).await?;
    let character=sqlx::query("SELECT character_revision::text,account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
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
            .map_err(|_| rejected("equipment Character revision"))?,
    };
    // Includes the real tagged session, lease, admission guards, current node
    // incarnation/assignment and game_activity_session_origin_valid proof.
    super::super::character_progression::assert_gameplay_fence(tx, &gameplay, node)
        .await?
        .map_err(|_| rejected("equipment current Instance lifecycle fence"))?;
    let current:Option<Vec<u8>>=sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(world_id.as_bytes().as_slice()).bind(channel.as_slice()).fetch_optional(&mut **tx).await?;
    if current.as_deref() != Some(content_digest.as_slice()) {
        return Err(rejected("equipment independent active Content"));
    }
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
    read_equipment_in_transaction(tx, &authority).await
}
