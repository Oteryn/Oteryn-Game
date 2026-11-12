//! Generic physical corpse custody from a real Instance lethal occurrence.
//! The item definition is isolated test data, not source quest qualification.
use super::*;
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
    ItemMintRequest, TypedDefinitionRef,
};
use crate::gameplay_transport::kill_reward::TakeOutcome;

pub(super) async fn qualify(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let started = std::time::Instant::now();
    let fence = admitted.item_fence.ok_or("Instance item fence")?;
    let taken = owner
        .states
        .attack
        .lock()
        .await
        .kills()
        .take(admitted.game_session_id, fence.character_lease_generation);
    let TakeOutcome::Taken(taken) = taken else {
        return Err("native death must be queued".into());
    };
    let entry = taken.entry().ok_or("retained native death")?;
    let server: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert!(server.starts_with("18"));
    eprintln!("INSTANCE_CORPSE_MINT POSTGRES_CASE_OBSERVED server_version_num={server}");
    let mut position = Vec::with_capacity(10);
    position.extend_from_slice(&entry.facts.corpse.x.to_be_bytes());
    position.extend_from_slice(&entry.facts.corpse.y.to_be_bytes());
    position.extend_from_slice(&entry.facts.corpse.floor.to_be_bytes());
    let item = TypedDefinitionRef {
        family: "ItemType".into(),
        production_key: "fixture:instance_corpse".into(),
        revision_ref: "r1".into(),
    };
    let request = ItemMintRequest {
        cause: ItemMintCause::from_creature_death(
            entry.facts.death,
            item.clone(),
            CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
            0,
        )
        .map_err(debug)?,
        item,
        quantity: 1,
        ground: GroundPlacement {
            spatial_position: position,
            corpse_ref: b"fixture:instance-corpse".to_vec(),
            map_revision: entry.ground.map_revision.clone(),
            content_revision: entry.ground.content_revision.clone(),
            native_room_placement_context: entry.ground.native_room_placement_context.clone(),
        },
        content_revision: entry.ground.content_revision.clone(),
        ruleset_revision: entry.ground.ruleset_revision.clone(),
        sim_revision: entry.ground.sim_revision.clone(),
    };
    let mut candidate = owner
        .root
        .freeze_item_mint(owner.character, owner.holder, request)
        .await
        .map_err(debug)?;
    let instance = fence.runtime_scope.instance_id().ok_or("Instance scope")?;
    let channel_twin = RuntimeScopeRefV1::channel(
        fence.runtime_scope.world_id(),
        crate::foundation::ChannelId::decode(&instance).map_err(debug)?,
    );
    let foreign_instance =
        RuntimeScopeRefV1::instance(fence.runtime_scope.world_id(), fixture::id(231))
            .map_err(debug)?;
    for scope in [channel_twin, foreign_instance] {
        let wrong =
            crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(scope).await?;
        assert!(matches!(
            owner
                .root
                .commit_corpse_mint(
                    &wrong,
                    owner.character,
                    owner.holder,
                    &mut candidate,
                    *fence.character_id.as_bytes(),
                )
                .await,
            Err(crate::durability::item_mint::ItemMintError::Unavailable(
                crate::durability::DurabilityError::InvalidStoredState
            ))
        ));
    }
    let charged: i16 = sqlx::query_scalar("SELECT work_units_used FROM game_item_mint_reservations WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(candidate.item_instance_id().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(charged, 0, "foreign permits cannot charge the logical MINT");
    let permit = owner.spell_lane_permit().await.ok_or("Instance lane")?;
    let first = owner
        .root
        .commit_corpse_mint(
            &permit,
            owner.character,
            owner.holder,
            &mut candidate,
            *fence.character_id.as_bytes(),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(first, ItemMintOutcome::Committed(_)));
    let replay = owner
        .root
        .commit_corpse_mint(
            &permit,
            owner.character,
            owner.holder,
            &mut candidate,
            *fence.character_id.as_bytes(),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(replay, ItemMintOutcome::AlreadyCommitted(_)));
    let exact: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_mint_receipts r USING(item_instance_id) JOIN game_item_audit_outbox a USING(item_instance_id) WHERE g.item_instance_id=encode($1,'hex')::uuid AND g.scope_kind=2 AND g.channel_id IS NULL AND g.instance_id=encode($2,'hex')::uuid AND r.materialized_at IS NOT NULL AND r.corpse_top_damage_character_id=encode($3,'hex')::uuid AND a.transaction_id=r.transaction_id AND a.envelope_sha256=r.envelope_sha256)",
    ).bind(candidate.item_instance_id().as_slice())
        .bind(fence.runtime_scope.instance_id().ok_or("Instance scope")?.as_slice())
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_one(&h.pool).await?;
    assert!(
        exact,
        "corpse, custody, attribution and audit commit together"
    );
    let envelope: Vec<u8> = sqlx::query_scalar(
        "SELECT envelope FROM game_item_audit_outbox WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(candidate.item_instance_id().as_slice())
    .fetch_one(&h.pool)
    .await?;
    let (event, payload) =
        crate::durability::item_mint_audit::decode_envelope(&envelope).map_err(debug)?;
    assert_eq!(event.channel_id, None);
    assert_eq!(event.instance_id.as_deref(), Some(instance.as_slice()));
    assert_eq!(
        payload
            .source
            .ok_or("mint source")?
            .death_occurrence
            .ok_or("death occurrence")?
            .runtime_scope()
            .map_err(debug)?,
        fence.runtime_scope
    );
    drop(permit);
    taken.requeue();
    eprintln!(
        "INSTANCE_CORPSE_MINT POSTGRES_CASE_COMPLETED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

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
    assert_eq!(
        ground.corpse_ref[0], 2,
        "published Instance corpse placement tag"
    );
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
