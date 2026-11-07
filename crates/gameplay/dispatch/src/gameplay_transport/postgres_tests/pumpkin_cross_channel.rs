//! Test-only independent historical SessionB; no generic Channel transfer is implemented.
use super::{
    TestResult,
    fixture::{Harness, debug, id},
};
use crate::foundation::{ChannelId, GameSessionId};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

pub(super) async fn qualify(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let death_channel:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(channel_id) FROM game_boss_reward_deaths WHERE world_id=encode($1,'hex')::uuid AND boss_key='oteryn:creature.the_mutated_pumpkin' AND settlement_outcome=1")
        .bind(id(42).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        death_channel,
        id(43),
        "actual native fight remains in original ChannelA"
    );
    let (node_b, deps_b) = super::chat_world_fixture::admit_second_node(h).await?;
    let channel_b = ChannelId::decode(&id(44)).map_err(debug)?;
    assert_ne!(death_channel, channel_b.as_bytes().as_slice());
    let participant_fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("current actual participant fenceA")?;
    let mut fence = participant_fence.item_fence();
    let independent_b = deps_b
        .admitted
        .item_fence
        .ok_or("independent current ChannelB fence")?;
    fence.runtime_scope = independent_b.runtime_scope;
    fence.scope_ownership_generation = independent_b.scope_ownership_generation;
    fence.game_session_id = GameSessionId::decode(&id(111)).map_err(debug)?;
    {
        let mut runtime = deps_b.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(fence.game_session_id)
            .map_err(debug)?;
        let actor_b = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor_b)
            .map_err(debug)?;
        let character =
            crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor_b,
                fence.game_session_id,
                crate::foundation::CharacterLease::new(character, fence.character_lease_generation)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
        assert_eq!(
            runtime
                .bound_attacker_lease(actor_b, fence.game_session_id)
                .map_err(debug)?
                .character_id()
                .as_bytes(),
            fence.character_id.as_bytes()
        );
        runtime.owner_fence().map_err(debug)?;
    }
    // Fixture history only: close old SessionA and install a controlled historical SessionB.
    // Actual death, contributor, quantities and visit tables are never changed.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let original:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(s) FROM game_durability_reconnect_sessions s WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(admitted.game_session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    let writable_columns: Vec<String> = sqlx::query_scalar("SELECT column_name FROM information_schema.columns WHERE table_schema=current_schema() AND table_name='game_durability_reconnect_sessions' AND is_generated='NEVER' AND is_identity='NO' ORDER BY ordinal_position")
        .fetch_all(&mut *tx).await?;
    assert!(
        !writable_columns.is_empty(),
        "historical session has writable source columns"
    );
    let columns = writable_columns
        .iter()
        .map(|column| format!("\"{}\"", column.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(",");
    let insert = format!(
        "INSERT INTO game_durability_reconnect_sessions ({columns}) SELECT {columns} FROM jsonb_populate_record(NULL::game_durability_reconnect_sessions, $1::jsonb||jsonb_build_object('game_session_id',encode($2,'hex')::uuid,'runtime_scope_channel_id',encode($3,'hex')::uuid,'scope_ownership_generation',$4::text::numeric,'session_state',2,'fresh_replay_key',NULL,'initial_fresh_replay_key',NULL,'control_loss_epoch',1,'original_grace_deadline',floor(extract(epoch FROM clock_timestamp()))::bigint+60,'predecessor_generation',( $1::jsonb->>'current_generation')::numeric)) AS historical"
    );
    // Only quoted identifiers from this fixed fixture table enter SQL; all values are bound.
    sqlx::query(sqlx::AssertSqlSafe(insert.as_str()))
        .bind(original)
        .bind(fence.game_session_id.as_bytes().as_slice())
        .bind(channel_b.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string())
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_durability_admission_character_guards SET holder_game_session_id=encode($1,'hex')::uuid WHERE character_id=encode($2,'hex')::uuid")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_durability_admission_account_guards SET holder_game_session_id=encode($1,'hex')::uuid WHERE presence_character_id=encode($2,'hex')::uuid")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let session_current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_durability_admission_character_guards c USING(character_id) JOIN game_durability_admission_account_guards a ON a.account_id=s.account_id JOIN game_durability_admission_runtime_guards r ON r.scope_key='\\x01'::bytea||uuid_send(s.world_id)||uuid_send(s.runtime_scope_channel_id) WHERE s.game_session_id=encode($1,'hex')::uuid AND s.runtime_scope_kind=1 AND s.runtime_scope_channel_id=encode($2,'hex')::uuid AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL AND s.session_state=2 AND c.eligible AND c.holder_game_session_id=s.game_session_id AND c.lease_generation=s.character_lease_generation AND a.presence_character_id=s.character_id AND a.holder_game_session_id=s.game_session_id AND r.ready AND r.ownership_generation=s.scope_ownership_generation)")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(channel_b.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert!(
        session_current,
        "independent persisted SessionB/current lease/account/readiness is valid before negative call"
    );
    let fact = node_b.fact();
    let node_current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_runtime_scope_assignments WHERE scope_kind=1 AND world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1 AND holder_node_id=encode($3,'hex')::uuid AND holder_registration_revision=$4::text::numeric AND ownership_generation=$5::text::numeric)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(channel_b.as_bytes().as_slice()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(fence.scope_ownership_generation.get().to_string()).fetch_one(&h.pool).await?;
    assert!(
        node_current,
        "actual independently assigned NodeB is current"
    );
    let window = owner
        .root
        .pumpkin_reward_portal_window(owner.character, owner.world_id, channel_b)
        .await?;
    let visit = owner
        .root
        .admit_pumpkin_reward_visit(owner.character, &node_b, fence, [0xCB; 32])
        .await;
    let grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_pumpkin_reward_visits WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    eprintln!(
        "SRV7_PUMPKIN_INDEPENDENT_CHANNEL_B_NODE_SESSION_VALID window={window:?} visit={visit:?} persisted_visits={grants}"
    );
    assert!(
        window.is_none(),
        "deathA must not open independently current ChannelB portal"
    );
    assert!(
        matches!(visit, Ok(None)),
        "real positive participantA must not obtain visit in ChannelB"
    );
    assert_eq!(grants, 0);
    Ok(())
}
