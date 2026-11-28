//! Recovery of Ground state saved before carried drops acquired the scope cap.
use super::*;

#[test]
fn legacy_over_cap_scope_retires_its_due_corpse_postgres() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async move {
        let h =
            fixture::Harness::create_with_admission_seed(admin, "legacy_corpse_cap", true, false)
                .await?;
        let result = drain_legacy_scope(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn drain_legacy_scope(h: &fixture::Harness) -> TestResult {
    let fence = fixture::fence(1)?;
    let scope = fence.runtime_scope;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let mut carried = None;
    let (_, template) = corpse_request(scope, 1)?;
    for actor in 1..=65u32 {
        let mut request = template.clone();
        // The other 64 roots never retire. The old drop could restore its
        // saved root on a free tile after the last ordinary admission.
        let key = if actor == 1 {
            "fixture:legacy"
        } else {
            "oteryn:item.tibia.i33793"
        };
        request.item = TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        };
        request.ground.spatial_position[..4].copy_from_slice(&actor.to_be_bytes());
        request.cause = ItemMintCause::for_test(
            scope.world_id(),
            scope.channel_id().ok_or("channel")?,
            fence.scope_ownership_generation,
            actor,
            1,
            request.item.clone(),
            CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
            0,
        );
        let mut mint = h
            .root
            .freeze_item_mint(&authority, &h.node, request)
            .await
            .map_err(fixture::debug)?;
        let permit = mint.fresh_death_lane_permit().await;
        h.root
            .commit_corpse_mint(
                &permit,
                &authority,
                &h.node,
                &mut mint,
                *fence.character_id.as_bytes(),
            )
            .await
            .map_err(fixture::debug)?;
        if actor == 1 {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_item_mint_receipts SET materialized_at=materialized_at-61000 WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(mint.item_instance_id().as_slice()).execute(&mut *tx).await?;
            let row: serde_json::Value = sqlx::query_scalar("DELETE FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid RETURNING to_jsonb(g)")
                .bind(mint.item_instance_id().as_slice()).fetch_one(&mut *tx).await?;
            tx.commit().await?;
            carried = Some((*mint.item_instance_id(), row));
        }
    }
    let (due, location) = carried.ok_or("saved corpse")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_ground_locations SELECT * FROM jsonb_populate_record(NULL::game_item_ground_locations,$1)")
        .bind(location).execute(&mut *tx).await?;
    tx.commit().await?;
    let schedule = h
        .root
        .read_scoped_corpse_decay_schedule(&authority, scope)
        .await
        .map_err(fixture::debug)?;
    assert_eq!(schedule.corpses.len(), 64);
    assert!(schedule.corpses.iter().any(|c| {
        c.corpse_item_instance_id == due
            && c.decay_at_unix_ms
                .is_some_and(|d| d <= schedule.database_now_unix_ms)
    }));
    let permit = SpellLanePermit::of_fresh_scope(scope).await?;
    h.root
        .retire_decayed_corpse(
            &permit,
            &authority,
            &h.node,
            crate::durability::item_decay_retire::CorpseDecayFence {
                runtime_scope: scope,
                scope_ownership_generation: fence.scope_ownership_generation,
            },
            due,
        )
        .await
        .map_err(fixture::debug)?;
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(remaining, 64);
    Ok(())
}

#[test]
fn sculpture_reconciliation_reads_the_commit_before_lock_acquisition_postgres() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async move {
        let h =
            fixture::Harness::create_with_admission_seed(admin, "sculpture_read_lock", true, false)
                .await?;
        let result = read_after_writer(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn read_after_writer(h: &fixture::Harness) -> TestResult {
    let fence = fixture::fence(1)?;
    let scope = fence.runtime_scope;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let (death, mut request) = corpse_request(scope, 1)?;
    request.item = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i4024".into(),
        revision_ref: "definition-r1".into(),
    };
    request.cause = ItemMintCause::from_creature_death(
        death,
        request.item.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    )
    .map_err(fixture::debug)?;
    let mut mint = h
        .root
        .freeze_item_mint(&authority, &h.node, request)
        .await
        .map_err(fixture::debug)?;
    let permit = mint.fresh_death_lane_permit().await;
    let barrier = std::sync::Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
    let reader = crate::durability::item_loop_move::BEFORE_SCULPTURE_LOCK.scope(
        barrier.clone(),
        h.root.read_item_loop_ground_after_sculpture(scope),
    );
    let writer = async {
        barrier.0.notified().await;
        let result = h
            .root
            .commit_corpse_mint(
                &permit,
                &authority,
                &h.node,
                &mut mint,
                *fence.character_id.as_bytes(),
            )
            .await;
        barrier.1.notify_one();
        result
    };
    let (rows, committed) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(reader, writer)
    })
    .await?;
    assert!(matches!(
        committed.map_err(fixture::debug)?,
        ItemMintOutcome::Committed(_)
    ));
    assert!(
        rows?
            .iter()
            .any(|r| r.item.item_instance_id == *mint.item_instance_id()),
        "protected census must see the preceding commit"
    );
    Ok(())
}

#[test]
fn takeover_skinning_observes_only_receipt_backed_ground_postgres() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async move {
        let h = fixture::Harness::create_with_admission_seed(
            admin,
            "takeover_skinning_ground",
            true,
            false,
        )
        .await?;
        let result = read_prior_corpse(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn read_prior_corpse(h: &fixture::Harness) -> TestResult {
    let mut fence = fixture::fence(1)?.item_fence();
    let scope = fence.runtime_scope;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let (death, mut request) = corpse_request(scope, 1)?;
    request.item = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i5969".into(),
        revision_ref: "definition-r1".into(),
    };
    request.cause = ItemMintCause::from_creature_death(
        death,
        request.item.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    )
    .map_err(fixture::debug)?
    .with_corpse_race(
        crate::domain::bestiary::BestiaryRace::new(
            "oteryn:creature.minotaur",
            "definition-r1",
            vec![1],
        )
        .map_err(fixture::debug)?,
    )
    .map_err(fixture::debug)?;
    let mut mint = h
        .root
        .freeze_item_mint(&authority, &h.node, request)
        .await
        .map_err(fixture::debug)?;
    let permit = mint.fresh_death_lane_permit().await;
    h.root
        .commit_corpse_mint(
            &permit,
            &authority,
            &h.node,
            &mut mint,
            *fence.character_id.as_bytes(),
        )
        .await
        .map_err(fixture::debug)?;
    let item = *mint.item_instance_id();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_runtime_scope_assignments SET ownership_generation=2 WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
        .bind(scope.world_id().as_bytes().as_slice()).bind(scope.channel_id().ok_or("channel")?.as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    fence.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(fixture::debug)?;
    assert!(
        h.root
            .read_skinning_ground_source(&authority, fence, item)
            .await?
            .is_some(),
        "the current writer must see the previous writer's proven corpse"
    );
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_ground_locations SET corpse_ref=decode('00','hex') WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(
        h.root
            .read_skinning_ground_source(&authority, fence, item)
            .await?
            .is_none()
    );
    Ok(())
}
