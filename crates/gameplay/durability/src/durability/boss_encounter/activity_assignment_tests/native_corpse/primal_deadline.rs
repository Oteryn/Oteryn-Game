//! M29: physical Primal corpse, immutable donor deadlines and exact Instance owner.
use super::*;

#[test]
fn primal_ordeal_instance_corpse_postgres_deadline_and_scope_fences() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
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
}

async fn ready_instance(h: &fixture::Harness) -> TestResult<RuntimeScopeRefV1> {
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
    Ok(scope)
}

async fn prove(h: &fixture::Harness) -> TestResult {
    let character = fixture::fence(1)?.character_id;
    let scope = ready_instance(h).await?;
    let world = scope.world_id();
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

#[test]
fn a_carried_corpse_uses_its_current_instance_timer_postgres() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    let worker =
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || -> Result<(), String> {
                fixture::runtime()
                    .map_err(fixture::debug)?
                    .block_on(async move {
                        let h = fixture::Harness::create_with_admission_seed(
                            admin,
                            "ordinary_corpse_instance_timer",
                            true,
                            false,
                        )
                        .await?;
                        let result = prove_carried_corpse(&h).await;
                        h.cleanup().await?;
                        result
                    })
                    .map_err(fixture::debug)
            })?;
    worker
        .join()
        .map_err(|_| "carried corpse timer worker panicked")?
        .map_err(Into::into)
}

async fn prove_carried_corpse(h: &fixture::Harness) -> TestResult {
    let origin = fixture::fence(1)?;
    let world = origin.runtime_scope.world_id();
    let instance = ready_instance(h).await?;
    activity_entry::enter(h, instance).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let (death, mut request) = corpse_request(origin.runtime_scope, 1)?;
    request.item = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i5013".into(),
        revision_ref: "definition-r1".into(),
    };
    request.cause = ItemMintCause::from_creature_death(
        death,
        request.item.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    )
    .map_err(fixture::debug)?;
    let mut corpse = h
        .root
        .freeze_item_mint(&authority, &h.node, request)
        .await
        .map_err(fixture::debug)?;
    let item = *corpse.item_instance_id();
    let channel_permit = corpse.fresh_death_lane_permit().await;
    assert!(matches!(
        h.root
            .commit_corpse_mint(
                &channel_permit,
                &authority,
                &h.node,
                &mut corpse,
                *origin.character_id.as_bytes(),
            )
            .await
            .map_err(fixture::debug)?,
        ItemMintOutcome::Committed(_)
    ));

    // Restore saved backpack custody with the real MINT and its original
    // source duration already elapsed. This fixture keeps the source age and
    // deadline in sync without waiting out the authored sixty seconds.
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    let bag: String = sqlx::query_scalar("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i2815','definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id::text")
        .bind(world.as_bytes().as_slice()).fetch_one(&mut *fixture).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,$2::uuid,encode($3,'hex')::uuid,game_character_uuid_v7())")
        .bind(origin.character_id.as_bytes().as_slice()).bind(&bag).bind(world.as_bytes().as_slice()).execute(&mut *fixture).await?;
    sqlx::query(
        "DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .execute(&mut *fixture)
    .await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::uuid,1,game_character_uuid_v7())")
        .bind(item.as_slice()).bind(world.as_bytes().as_slice()).bind(origin.character_id.as_bytes().as_slice()).bind(&bag).execute(&mut *fixture).await?;
    let (target, duration): (String, i64) = sqlx::query_as("SELECT target_production_key,duration_millis FROM game_spell_field_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid AND source_stage=1")
        .bind(item.as_slice()).fetch_one(&mut *fixture).await?;
    sqlx::query("UPDATE game_item_mint_receipts SET materialized_at=materialized_at-$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).bind(duration).execute(&mut *fixture).await?;
    let deadline: i64 = sqlx::query_scalar("UPDATE game_spell_field_temporal_schedules SET expires_at_unix_ms=expires_at_unix_ms-$2 WHERE item_instance_id=encode($1,'hex')::uuid RETURNING expires_at_unix_ms")
        .bind(item.as_slice()).bind(duration).fetch_one(&mut *fixture).await?;
    fixture.commit().await?;

    assert_eq!(
        h.root
            .drain_spell_item_deadlines(
                &channel_permit,
                &authority,
                &h.node,
                origin.runtime_scope,
                1
            )
            .await
            .map_err(fixture::debug)?,
        0
    );
    drop(channel_permit);
    let instance_permit = SpellLanePermit::of_fresh_scope(instance).await?;
    assert_eq!(
        h.root
            .drain_spell_item_deadlines(&instance_permit, &authority, &h.node, instance, 1)
            .await
            .map_err(fixture::debug)?,
        1
    );
    assert_eq!(
        h.root
            .drain_spell_item_deadlines(&instance_permit, &authority, &h.node, instance, 1)
            .await
            .map_err(fixture::debug)?,
        0
    );
    let instance_id = instance.instance_id().ok_or("private activity scope")?;
    let outcome: (String, String, i64, bool, bool, bool) = sqlx::query_as(r#"
        SELECT i.definition_production_key,e.parent_item_instance_id::text,p.expires_at_unix_ms,
            r.channel_id IS NULL,r.instance_id=encode($2,'hex')::uuid,
            r.game_session_id=s.game_session_id AND r.character_lease_generation=s.character_lease_generation
                AND r.connection_generation=s.current_generation
        FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id)
        JOIN game_spell_field_temporal_schedules p USING(item_instance_id)
        JOIN game_spell_field_temporal_receipts r USING(item_instance_id,source_stage)
        JOIN game_durability_reconnect_sessions s ON s.character_id=r.character_id AND s.session_state IN (1,2)
        WHERE i.item_instance_id=encode($1,'hex')::uuid
    "#).bind(item.as_slice()).bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(outcome, (target, bag, deadline, true, true, true));
    let snapshot =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(
                crate::foundation::GameSessionId::decode(&id(90)).map_err(fixture::debug)?,
            )
            .await?
            .0;
    let fence = crate::durability::item_transfer::CurrentCharacterItemFence {
        character_id: origin.character_id,
        game_session_id: snapshot.current_game_session_id(),
        character_lease_generation: snapshot.current_character_lease().generation(),
        connection_generation: snapshot.current_connection_generation(),
        scope_ownership_generation: snapshot.current_scope_generation(),
        runtime_scope: instance,
    };
    reject_legacy_pickup(h, &authority, &instance_permit, &fence).await?;
    let command = crate::foundation::CommandRef::new(
        fence.game_session_id,
        crate::foundation::CommandId::new(5).map_err(fixture::debug)?,
    );
    let result = h
        .root
        .commit_item_loop_move(
            &instance_permit,
            &authority,
            &h.node,
            &fence,
            command,
            &ButterflyContent,
            crate::durability::item_loop_move::LoopMoveRequest {
                item,
                count: None,
                pickup_cell: None,
                destination: crate::durability::item_loop_move::LoopDestination::Ground(
                    crate::durability::item_loop_move::LoopGround {
                        spatial_position: [
                            200_i32.to_be_bytes().as_slice(),
                            201_i32.to_be_bytes().as_slice(),
                            (-7_i16).to_be_bytes().as_slice(),
                        ]
                        .concat(),
                        map_revision: format!("sha256:{}", "22".repeat(32)),
                        content_revision: format!("sha256:{}", "33".repeat(32)),
                        placement_context: vec![0x11; 32],
                    },
                ),
            },
        )
        .await?;
    assert!(result.is_ok(), "drop after carried decay: {result:?}");
    let ground: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid AND instance_id=encode($2,'hex')::uuid)")
        .bind(item.as_slice()).bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert!(ground);
    eprintln!("Ordinary corpse PostgreSQL carried Instance deadline PASS");
    Ok(())
}

async fn reject_legacy_pickup(
    h: &fixture::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    permit: &SpellLanePermit,
    fence: &crate::durability::item_transfer::CurrentCharacterItemFence,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopMoveRefusal, LoopMoveRequest};
    let (death, mut request) = corpse_request(fence.runtime_scope, 1)?;
    let cell = request.ground.spatial_position.clone();
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
        .freeze_item_mint(authority, &h.node, request)
        .await
        .map_err(fixture::debug)?;
    h.root
        .commit_corpse_mint(
            permit,
            authority,
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
    sqlx::query("UPDATE game_item_mint_receipts SET ordinary_corpse_policy=false WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).execute(&mut *tx).await?;
    let removed = sqlx::query("DELETE FROM game_spell_field_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid AND target_production_key IS NULL")
        .bind(item.as_slice()).execute(&mut *tx).await?;
    assert_eq!(removed.rows_affected(), 1);
    tx.commit().await?;
    let command = crate::foundation::CommandRef::new(
        fence.game_session_id,
        crate::foundation::CommandId::new(4).map_err(fixture::debug)?,
    );
    let result = h
        .root
        .commit_item_loop_move(
            permit,
            authority,
            &h.node,
            fence,
            command,
            &ButterflyContent,
            LoopMoveRequest {
                item,
                count: None,
                pickup_cell: Some(cell),
                destination: LoopDestination::MainBackpack,
            },
        )
        .await?;
    assert!(
        matches!(result, Err(LoopMoveRefusal::NotSupported)),
        "legacy pickup with a finite Ground-only deadline: {result:?}"
    );
    Ok(())
}

struct ButterflyContent;

impl crate::durability::item_transfer::ItemWeightLookup for ButterflyContent {
    fn generation_digest(&self) -> [u8; 32] {
        [0x33; 32]
    }
    fn weight_of(
        &self,
        _: &TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        Ok(1)
    }
}

impl crate::durability::item_loop_move::ItemLoopContent for ButterflyContent {
    fn loop_content_digest(&self) -> [u8; 32] {
        [0x33; 32]
    }
    fn loop_facts(
        &self,
        item: &TypedDefinitionRef,
    ) -> Option<crate::durability::item_transfer::ItemDefinitionFacts> {
        use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
        let bag = item.production_key == "oteryn:item.tibia.i2815";
        Some(ItemDefinitionFacts {
            definition: item.clone(),
            weight: None,
            stack: if bag || item.production_key == "oteryn:item.tibia.i4024" {
                ItemStackClass::NonStackable
            } else {
                ItemStackClass::Unknown
            },
            container_capacity: bag.then_some(20),
            container_slot_equip_pattern: bag,
        })
    }
}
