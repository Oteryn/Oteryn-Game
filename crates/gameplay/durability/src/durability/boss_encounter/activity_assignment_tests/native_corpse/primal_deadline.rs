//! M29: physical Primal corpse, immutable donor deadlines and exact Instance owner.
use super::*;

#[test]
fn primal_ordeal_instance_corpse_postgres_deadline_and_scope_fences() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    on_pg_thread(move || {
        fixture::runtime()?.block_on(async move {
            let started = std::time::Instant::now();
            let h = fixture::Harness::create_with_admission_seed(
                admin,
                "encx_primal_deadline",
                true,
                false,
            )
            .await?;
            let result = prove(&h).await;
            h.cleanup().await?;
            eprintln!(
                "M29 real PostgreSQL Primal Instance corpse deadline: {:?}",
                started.elapsed()
            );
            result
        })
    })
}

async fn prove(h: &fixture::Harness) -> TestResult {
    let character = fixture::fence(1)?.character_id;
    let world = fixture::fence(1)?.runtime_scope.world_id();
    let scope = RuntimeScopeRefV1::instance(world, id(88)).map_err(fixture::debug)?;
    sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice()).bind(id(88).as_slice()).execute(&h.pool).await?;
    committed(
        h.writer
            .submit(&request(
                144,
                AssignmentCommand::Assign {
                    scope,
                    target: h.node.fact(),
                },
            )?)
            .await,
    )?;
    h.root
        .publish_runtime_readiness(&h.node, &readiness(h, scope, 1, 1).await?)
        .await
        .map_err(fixture::debug)?;
    assert!(runtime_fence(h, scope, 1, &h.node).await?);
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let (death, mut request) = corpse_request(scope, 1)?;
    request.item = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i39530".into(),
        revision_ref: "definition-r1".into(),
    };
    request.cause = ItemMintCause::from_creature_death(
        death,
        request.item.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    )
    .map_err(fixture::debug)?;
    assert!(
        matches!(
            h.root
                .freeze_item_mint(&authority, &h.node, request.clone())
                .await,
            Err(ItemMintError::AuthorityRejected)
        ),
        "a ready allocation alone cannot admit a Primal corpse"
    );
    activity_entry::enter(h, scope).await?;
    let mut corpse = h
        .root
        .freeze_item_mint(&authority, &h.node, request)
        .await
        .map_err(fixture::debug)?;
    let item = *corpse.item_instance_id();
    let permit = corpse.fresh_death_lane_permit().await;
    assert!(matches!(
        h.root
            .commit_corpse_mint(
                &permit,
                &authority,
                &h.node,
                &mut corpse,
                *character.as_bytes()
            )
            .await
            .map_err(fixture::debug)?,
        ItemMintOutcome::Committed(_)
    ));
    // Crystal 201189f data/items/items.xml:73328-73347: 10s, 300s, 300s;
    // the terminal 39527 survives another 60s. No test accelerates these clocks.
    let deadlines: Vec<(i32, i64)> = sqlx::query_as("SELECT p.source_stage,p.expires_at_unix_ms-m.materialized_at FROM game_spell_field_temporal_schedules p JOIN game_item_mint_receipts m USING(item_instance_id) WHERE p.item_instance_id=encode($1,'hex')::uuid ORDER BY p.source_stage")
        .bind(item.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(deadlines, vec![(1, 10000), (2, 310000), (3, 610000)]);
    let schedule = h
        .root
        .read_scoped_corpse_decay_schedule(&authority, scope)
        .await
        .map_err(fixture::debug)?;
    assert_eq!(schedule.corpses.len(), 1);
    assert_eq!(schedule.corpses[0].corpse_item_instance_id, item);
    assert_eq!(
        schedule.corpses[0].decay_at_unix_ms,
        Some(schedule.corpses[0].materialized_at_unix_ms + 670000)
    );
    assert_eq!(
        h.root
            .drain_spell_item_deadlines(&permit, &authority, &h.node, scope, 1)
            .await
            .map_err(fixture::debug)?,
        0
    );
    let current: (bool, bool, bool) = sqlx::query_as("SELECT game_instance_corpse_stage_current(g.item_instance_id,g.world_id,g.instance_id,1,g.stack_ordinal),game_instance_corpse_stage_current(g.item_instance_id,g.world_id,encode($2,'hex')::uuid,1,g.stack_ordinal),game_instance_corpse_stage_current(g.item_instance_id,g.world_id,g.instance_id,2,g.stack_ordinal) FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).bind(id(89).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        current,
        (true, false, false),
        "SQL proof independently rejects foreign identity and non-current generation"
    );
    assert!(sqlx::query("UPDATE game_spell_field_temporal_schedules SET expires_at_unix_ms=expires_at_unix_ms-1 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).execute(&h.pool).await.is_err(), "source deadlines remain immutable");
    // Wait for the genuine source deadline; never rewrite an immutable schedule.
    tokio::time::sleep(std::time::Duration::from_millis(10100)).await;
    let alias =
        RuntimeScopeRefV1::channel(world, ChannelId::decode(&id(88)).map_err(fixture::debug)?);
    // Both owners are genuinely assigned and ready. The UUID, world, node and
    // generation match; only the Channel/Instance namespace differs.
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice()).bind(id(88).as_slice()).execute(&h.pool).await?;
    committed(
        h.writer
            .submit(&super::request(
                145,
                AssignmentCommand::Assign {
                    scope: alias,
                    target: h.node.fact(),
                },
            )?)
            .await,
    )?;
    h.root
        .publish_runtime_readiness(&h.node, &readiness(h, alias, 1, 1).await?)
        .await
        .map_err(fixture::debug)?;
    assert!(runtime_fence(h, alias, 1, &h.node).await?);
    let wrong_permit = SpellLanePermit::of_fresh_scope(alias).await?;
    assert_eq!(
        h.root
            .drain_spell_item_deadlines(&wrong_permit, &authority, &h.node, alias, 1)
            .await
            .map_err(fixture::debug)?,
        0,
        "a current Channel owner must not consume its same-UUID Instance corpse deadline"
    );
    let untouched: (String, String) = sqlx::query_as("SELECT definition_production_key,state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(untouched, ("oteryn:item.tibia.i39530".into(), "1".into()));
    assert!(
        h.root
            .drain_spell_item_deadlines(&wrong_permit, &authority, &h.node, scope, 1)
            .await
            .is_err()
    );
    assert!(
        h.root
            .drain_spell_item_deadlines(&permit, &authority, &h.node, scope, 2)
            .await
            .is_err()
    );
    assert_eq!(
        h.root
            .drain_spell_item_deadlines(&permit, &authority, &h.node, scope, 1)
            .await
            .map_err(fixture::debug)?,
        1
    );
    assert_eq!(
        h.root
            .drain_spell_item_deadlines(&permit, &authority, &h.node, scope, 1)
            .await
            .map_err(fixture::debug)?,
        0
    );
    let stored: (String, i64) = sqlx::query_as("SELECT i.definition_production_key,count(a.*) FROM game_item_instances i JOIN game_spell_field_temporal_receipts r USING(item_instance_id) JOIN game_spell_field_temporal_audit a USING(transaction_id) WHERE i.item_instance_id=encode($1,'hex')::uuid GROUP BY i.definition_production_key")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(stored, ("oteryn:item.tibia.i39529".into(), 1));
    Ok(())
}
