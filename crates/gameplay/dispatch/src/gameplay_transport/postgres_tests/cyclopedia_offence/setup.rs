use super::*;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
pub(super) async fn configure(
    h: &Harness,
    deps: &mut Dependencies,
    pin: crate::content::NativeEntryContentPin,
) -> TestResult<crate::durability::item_transfer::CurrentCharacterItemFence> {
    deps.room = pin.qualified_room().clone();
    deps.combat_room = Some(deps.room.clone());
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        deps.room.map_revision_digest(),
        (2, 11, -7),
    );
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.activation_sequence().to_string())
            .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
            .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?
            .0;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        pin,
    )
    .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .bind_attacker_lease(
            deps.actor,
            deps.admitted.game_session_id,
            current.current_character_lease(),
        )
        .map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    deps.admitted.runtime_actor = Some(deps.actor);
    Ok(fence)
}

pub(super) async fn seed_foreign(h: &Harness) -> TestResult {
    sqlx::query(
        "INSERT INTO game_character_account_guards(account_id) VALUES(encode($1,'hex')::uuid)",
    )
    .bind(id(61).as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($2,'hex')::uuid,encode($3,'hex')::uuid,world_id,lifecycle,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Peer Hero' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).bind(id(60).as_slice()).bind(id(61).as_slice()).execute(&h.pool).await?;
    // Fixture copies an already committed, known-empty offline observation. Production
    // writes above always re-prove the actual session and exact equipment snapshot.
    sqlx::query("INSERT INTO game_character_cyclopedia_combat(character_id,game_session_id,character_lease_generation,connection_generation,observed_owner_micros,observed_at_unix_ms,character_revision,equipment_revision,content_digest,input_sha256,sample_revision,offence,defence,misc,offence_augments,offence_augments_revision) SELECT encode($2,'hex')::uuid,game_session_id,character_lease_generation,connection_generation,observed_owner_micros,observed_at_unix_ms,character_revision,equipment_revision,content_digest,input_sha256,sample_revision,offence,defence,misc,offence_augments,offence_augments_revision FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
    Ok(())
}

/// Match normal first-entry equipment initialization before the fenced observation.
pub(super) async fn seed_equipment(h: &Harness) -> TestResult {
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING").bind(id(41).as_slice()).execute(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    // Fixture custody only, as in parameter_spell; reads still prove live owner fences.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,encode($1,'hex')::uuid)").bind(id(112).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,3,encode($3,'hex')::uuid,encode($3,'hex')::uuid)").bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(112).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
/// A pre-extension writer advances only its own scalar sample columns.
pub(super) async fn legacy_binding(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
) -> TestResult {
    sqlx::query("UPDATE game_character_cyclopedia_combat SET sample_revision=sample_revision+1 WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
    let view = h
        .root
        .read_cyclopedia_offence(
            authority,
            &h.node,
            fence,
            fence.character_id,
            Query::default(),
        )
        .await
        .map_err(debug)?;
    assert!(
        view.facts.is_none(),
        "stale modifiers cannot attach to a newer scalar sample"
    );
    Ok(())
}
