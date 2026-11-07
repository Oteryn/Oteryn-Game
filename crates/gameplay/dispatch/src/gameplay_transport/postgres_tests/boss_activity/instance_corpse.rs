//! Physical corpse source, custody and audit after a real W02 handoff.
use super::*;

pub(super) async fn assert_mint(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    policy: &crate::foundation::CompiledCreaturePolicy,
) -> TestResult<(Vec<u8>, Vec<u8>)> {
    let started = std::time::Instant::now();
    let fence = admitted.item_fence.ok_or("Instance corpse fence")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("actual Instance admission required".into());
    };
    let purpose = crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts r JOIN game_item_ground_locations g USING(item_instance_id) WHERE r.loot_purpose_key=$1 AND r.materialized_at IS NOT NULL AND g.scope_kind=2 AND g.instance_id=encode($2,'hex')::uuid AND g.channel_id IS NULL")
        .bind(purpose).bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    eprintln!(
        "INSTANCE_CORPSE POSTGRES_CASE_OBSERVED corpse_mints={count} elapsed_ms={}",
        started.elapsed().as_millis()
    );
    assert_eq!(
        count, 1,
        "a native Instance death materializes one physical corpse"
    );
    let (item, generation, bytes): (Vec<u8>, String, Vec<u8>) = sqlx::query_as("SELECT uuid_send(r.item_instance_id),g.runtime_scope_ownership_generation::text,v.envelope FROM game_item_mint_receipts r JOIN game_item_mint_reservations v USING(item_instance_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE r.loot_purpose_key=$1 AND g.scope_kind=2 AND g.instance_id=encode($2,'hex')::uuid")
        .bind(purpose).bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        generation.parse::<u64>()?,
        fence.scope_ownership_generation.get()
    );
    let (event, mint) =
        crate::durability::item_mint_audit::decode_envelope(&bytes).map_err(debug)?;
    assert_eq!(
        event.world_id.as_deref(),
        Some(fence.runtime_scope.world_id().as_bytes().as_slice())
    );
    assert!(event.channel_id.is_none());
    assert_eq!(event.instance_id.as_deref(), Some(instance_id.as_slice()));
    assert!(
        event.command_id.is_none(),
        "server corpse MINT invents no player command"
    );
    assert!(mint.before_semantically_absent);
    assert!(mint.corpse_container_entry.is_none());
    let ground = mint.destination.ok_or("actual corpse Ground")?;
    assert_eq!(ground.runtime_scope().map_err(debug)?, fence.runtime_scope);
    assert_eq!(
        ground.runtime_scope_ownership_generation,
        fence.scope_ownership_generation.get()
    );
    assert_eq!(ground.corpse_ref.len(), 53);
    assert_eq!(ground.corpse_ref[0], 2);
    assert_eq!(&ground.corpse_ref[17..33], instance_id.as_slice());
    let state = mint.after.ok_or("physical corpse after-state")?;
    assert_eq!(state.item_instance_id, item);
    assert_eq!(state.quantity, 1);
    let visible = owner.runtime.lock().await.visible_corpses();
    assert_eq!(
        visible
            .iter()
            .filter(|corpse| corpse.item.item_instance_id.as_slice() == item.as_slice())
            .count(),
        1,
        "the committed Instance corpse is bound into the native runtime view"
    );
    let definition = state.definition.ok_or("typed corpse definition")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native content")?;
    let row = native
        .reward_table()
        .row(&policy.definition_key)
        .map_err(debug)?;
    assert_eq!(definition.family, row.corpse_item.family);
    assert_eq!(definition.production_key, row.corpse_item.production_key);
    assert_eq!(definition.revision_ref, row.corpse_item.revision_ref);
    let source = mint.source.ok_or("native corpse source")?;
    assert_eq!(source.loot_purpose_key, purpose);
    assert_eq!(source.draw_ordinal, 0);
    let death = source.death_occurrence.ok_or("sealed death source")?;
    assert_eq!(death.runtime_scope().map_err(debug)?, fence.runtime_scope);
    assert!(
        death.channel_id.is_empty(),
        "Instance death cannot borrow an origin Channel"
    );
    assert_eq!(
        death.scope_ownership_generation,
        fence.scope_ownership_generation.get()
    );
    // Even the schema owner cannot change a reserved scope during an otherwise
    // legal work charge. The generated key must not weaken raw-field immutability.
    for assignment in [
        "death_instance_id = game_character_uuid_v7()",
        "death_scope_kind = 1, death_channel_id = death_instance_id, death_instance_id = NULL",
    ] {
        let mut transaction = h.pool.begin().await?;
        let query = format!(
            "UPDATE game_item_mint_reservations SET work_units_used = work_units_used + 1, {assignment} WHERE item_instance_id = encode($1,'hex')::uuid"
        );
        let error = sqlx::query(sqlx::AssertSqlSafe(query))
            .bind(&item)
            .execute(&mut *transaction)
            .await
            .expect_err("reserved death namespace is immutable");
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        transaction.rollback().await?;
    }
    eprintln!(
        "INSTANCE_CORPSE POSTGRES_CASE_COMPLETED scoped_native_mint_audit elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok((item, bytes))
}
