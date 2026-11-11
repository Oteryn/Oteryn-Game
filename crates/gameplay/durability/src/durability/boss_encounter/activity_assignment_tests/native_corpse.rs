//! Native corpse-owner component proof; production encounter qualification is separate.
use super::{
    AssignmentCommand, TestResult, committed, fixture, id, readiness, request, runtime_fence,
};
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, CorpseContainerPlacement, CorpseLootMintRequest,
    GroundPlacement, ItemMintCause, ItemMintError, ItemMintOutcome, ItemMintRequest,
    TypedDefinitionRef,
};
use crate::durability::item_mint_audit;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{
    ChannelId, CreatureDeathOccurrenceKey, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use oteryn_gameplay_entities::carrier::CombatDeathFixture;

fn on_pg_thread(run: impl FnOnce() -> TestResult + Send + 'static) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || run().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "native corpse PostgreSQL test thread panicked")?
        .map_err(Into::into)
}

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "fixture-r1".into(),
    }
}

fn physical_death(
    scope: RuntimeScopeRefV1,
    generation: u64,
) -> TestResult<(CreatureDeathOccurrenceKey, GroundPlacement)> {
    let mut native = CombatDeathFixture::new_for_scope(
        scope,
        ScopeOwnershipGeneration::new(generation).map_err(fixture::debug)?,
    )
    .map_err(fixture::debug)?;
    native
        .strike("fixture:instance-corpse.strike", CombatDeathFixture::HEALTH)
        .map_err(fixture::debug)?;
    let corpse = native.project_death_projection().map_err(fixture::debug)?;
    let death = corpse.occurrence().death_key().map_err(fixture::debug)?;
    let at = corpse.position();
    let mut spatial_position = Vec::new();
    spatial_position.extend_from_slice(&at.x.to_be_bytes());
    spatial_position.extend_from_slice(&at.y.to_be_bytes());
    spatial_position.extend_from_slice(&at.floor.to_be_bytes());
    let (frame, map, content) = corpse.context_markers();
    let mut placement = Vec::new();
    for value in [frame, map, content] {
        placement.extend_from_slice(&value.to_be_bytes());
    }
    Ok((
        death,
        GroundPlacement {
            spatial_position,
            corpse_ref: death.native_bytes(),
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            native_room_placement_context: placement,
        },
    ))
}

fn corpse_request(
    scope: RuntimeScopeRefV1,
    generation: u64,
) -> TestResult<(CreatureDeathOccurrenceKey, ItemMintRequest)> {
    let (death, ground) = physical_death(scope, generation)?;
    let item = definition("fixture:instance-corpse");
    Ok((
        death,
        ItemMintRequest {
            cause: ItemMintCause::from_creature_death(
                death,
                item.clone(),
                CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
                0,
            )
            .map_err(fixture::debug)?,
            item,
            quantity: 1,
            ground,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        },
    ))
}

async fn qualify(
    h: &fixture::Harness,
    migration_preservation: bool,
    legacy_reservation: bool,
) -> TestResult {
    // Crystal 201189f the_primal_menace.lua:79,94 requires a real corpse even
    // when the activity boss's personal reward owner is a separate descendant.
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
    assert_eq!(
        h.root
            .publish_runtime_readiness(&h.node, &readiness(h, scope, 1, 1).await?)
            .await
            .map_err(fixture::debug)?,
        GuardPublicationDisposition::Applied
    );
    assert!(runtime_fence(h, scope, 1, &h.node).await?);
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let (death, request) = corpse_request(scope, 1)?;
    assert!(
        matches!(
            h.root
                .freeze_item_mint(&authority, &h.node, request.clone())
                .await,
            Err(ItemMintError::AuthorityRejected)
        ),
        "an assigned, ready Instance without a committed participant is not a MINT owner"
    );
    activity_entry::enter(h, scope).await?;
    let expected_ground = if legacy_reservation {
        let mut bytes = death.world_id().as_bytes().to_vec();
        bytes.extend_from_slice(b"instance:");
        bytes.extend_from_slice(&id(88));
        bytes.extend_from_slice(&death.scope_ownership_generation().get().to_be_bytes());
        bytes.extend_from_slice(&death.actor_local_id().to_be_bytes());
        bytes.extend_from_slice(&death.actor_local_generation().to_be_bytes());
        assert_eq!(bytes.len(), 61);
        bytes
    } else {
        death.native_bytes()
    };
    let mut corpse = if legacy_reservation {
        let mut legacy_request = request.clone();
        legacy_request.ground.corpse_ref = expected_ground.clone();
        h.root
            .freeze_item_mint_legacy_instance_for_test(&authority, &h.node, legacy_request)
            .await
    } else {
        h.root
            .freeze_item_mint(&authority, &h.node, request.clone())
            .await
    }
    .map_err(fixture::debug)?;
    let corpse_id = *corpse.item_instance_id();
    if legacy_reservation {
        let frozen_before:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(v) FROM game_item_mint_reservations v WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse_id.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse_id.as_slice()).fetch_one(&h.pool).await?,0,"resume must exercise pending reservation before first commit");
        let resumed = h
            .root
            .freeze_item_mint(&authority, &h.node, request.clone())
            .await
            .map_err(fixture::debug)?;
        assert_eq!(resumed.item_instance_id(), &corpse_id);
        assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(v) FROM game_item_mint_reservations v WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse_id.as_slice()).fetch_one(&h.pool).await?,frozen_before,"pending public53 resume retains exact legacy61 frozen bytes");
        corpse = resumed;
    }

    let alias =
        RuntimeScopeRefV1::channel(world, ChannelId::decode(&id(88)).map_err(fixture::debug)?);
    let wrong = SpellLanePermit::of_fresh_scope(alias).await?;
    assert!(
        h.root
            .commit_corpse_mint(
                &wrong,
                &authority,
                &h.node,
                &mut corpse,
                *character.as_bytes()
            )
            .await
            .is_err()
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse_id.as_slice()).fetch_one(&h.pool).await?,0);
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
        ItemMintOutcome::AlreadyCommitted(_)
    ));
    let stored = h
        .root
        .read_item_instance(&authority, corpse_id)
        .await
        .map_err(fixture::debug)?
        .ok_or("missing Instance corpse")?;
    assert_eq!(stored.runtime_scope, scope);
    assert_eq!(stored.ground.corpse_ref, expected_ground);
    let loot_request = CorpseLootMintRequest {
        cause: ItemMintCause::from_creature_death(
            death,
            TypedDefinitionRef {
                family: "LootTable".into(),
                production_key: "fixture:instance-loot".into(),
                revision_ref: "fixture-r1".into(),
            },
            "fixture:entry".into(),
            1,
        )
        .map_err(fixture::debug)?,
        item: definition("fixture:instance-loot"),
        quantity: 1,
        placement: CorpseContainerPlacement {
            corpse_item_instance_id: corpse_id,
            placement_ordinal: 1,
            fluid_subtype: None,
        },
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    };
    let mut loot = if legacy_reservation {
        h.root
            .freeze_corpse_loot_mint_legacy_instance_for_test(
                &authority,
                &h.node,
                loot_request.clone(),
            )
            .await
    } else {
        h.root
            .freeze_corpse_loot_mint(&authority, &h.node, loot_request.clone())
            .await
    }
    .map_err(fixture::debug)?;
    let loot_id = *loot.item_instance_id();
    if legacy_reservation {
        let frozen_before:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(v) FROM game_item_mint_reservations v WHERE item_instance_id=encode($1,'hex')::uuid").bind(loot_id.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid").bind(loot_id.as_slice()).fetch_one(&h.pool).await?,0,"loot resume must precede first commit");
        let resumed = h
            .root
            .freeze_corpse_loot_mint(&authority, &h.node, loot_request.clone())
            .await
            .map_err(fixture::debug)?;
        assert_eq!(resumed.item_instance_id(), &loot_id);
        assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(v) FROM game_item_mint_reservations v WHERE item_instance_id=encode($1,'hex')::uuid").bind(loot_id.as_slice()).fetch_one(&h.pool).await?,frozen_before,"pending public child resume retains exact legacyv1 frozen bytes");
        loot = resumed;
    }

    assert!(matches!(
        h.root
            .commit_corpse_loot_mint(&permit, &authority, &h.node, &mut loot)
            .await
            .map_err(fixture::debug)?,
        ItemMintOutcome::Committed(_)
    ));
    assert!(matches!(
        h.root
            .commit_corpse_loot_mint(&permit, &authority, &h.node, &mut loot)
            .await
            .map_err(fixture::debug)?,
        ItemMintOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid").bind(loot_id.as_slice()).bind(corpse_id.as_slice()).fetch_one(&h.pool).await?,1);
    for rejected in [
        corpse_request(alias, 1)?.1,
        corpse_request(scope, 2)?.1,
        corpse_request(
            RuntimeScopeRefV1::instance(world, id(89)).map_err(fixture::debug)?,
            1,
        )?
        .1,
    ] {
        assert!(matches!(
            h.root.freeze_item_mint(&authority, &h.node, rejected).await,
            Err(ItemMintError::AuthorityRejected)
        ));
    }
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_mint_receipts WHERE death_scope_key=game_activity_scope_key(encode($1,'hex')::uuid,encode($2,'hex')::uuid) AND death_scope_kind=2 AND death_channel_id IS NULL AND death_instance_id=encode($2,'hex')::uuid").bind(world.as_bytes().as_slice()).bind(id(88).as_slice()).fetch_one(&h.pool).await?,2);
    // The same migrated owner retains the legacy Channel cause and framing.
    let channel_scope = fixture::fence(1)?.runtime_scope;
    let (channel_death, channel_request) = corpse_request(channel_scope, 1)?;
    assert_eq!(channel_death.native_bytes().len(), 52);
    let mut channel_corpse = h
        .root
        .freeze_item_mint(&authority, &h.node, channel_request.clone())
        .await
        .map_err(fixture::debug)?;
    let channel_permit = channel_corpse.fresh_death_lane_permit().await;
    assert!(matches!(
        h.root
            .commit_corpse_mint(
                &channel_permit,
                &authority,
                &h.node,
                &mut channel_corpse,
                *character.as_bytes()
            )
            .await
            .map_err(fixture::debug)?,
        ItemMintOutcome::Committed(_)
    ));
    let stored_channel = h
        .root
        .read_item_instance(&authority, *channel_corpse.item_instance_id())
        .await
        .map_err(fixture::debug)?
        .ok_or("missing Channel control corpse")?;
    assert_eq!(stored_channel.runtime_scope, channel_scope);
    assert_eq!(
        stored_channel.ground.corpse_ref,
        channel_death.native_bytes()
    );
    let channel_v1: bool = sqlx::query_scalar("SELECT death_scope_kind=1 AND death_channel_id IS NOT NULL AND death_instance_id IS NULL AND get_byte(intent_binding,0)=1 FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(channel_corpse.item_instance_id().as_slice()).fetch_one(&h.pool).await?;
    assert!(channel_v1);
    if migration_preservation {
        preserve_committed_causes_across_1152(h).await?;
        let replay_rows_before:serde_json::Value=sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(r) ORDER BY transaction_id) FROM game_item_mint_reservations r").fetch_one(&h.pool).await?;
        let replay_receipts_before: serde_json::Value = sqlx::query_scalar(
            "SELECT jsonb_agg(to_jsonb(r) ORDER BY transaction_id) FROM game_item_mint_receipts r",
        )
        .fetch_one(&h.pool)
        .await?;
        let mut replay_corpse = h
            .root
            .freeze_item_mint(&authority, &h.node, request.clone())
            .await
            .map_err(fixture::debug)?;
        assert_eq!(replay_corpse.item_instance_id(), &corpse_id);
        assert!(matches!(
            h.root
                .commit_corpse_mint(
                    &permit,
                    &authority,
                    &h.node,
                    &mut replay_corpse,
                    *character.as_bytes()
                )
                .await
                .map_err(fixture::debug)?,
            ItemMintOutcome::AlreadyCommitted(_)
        ));
        let mut replay_loot = h
            .root
            .freeze_corpse_loot_mint(&authority, &h.node, loot_request)
            .await
            .map_err(fixture::debug)?;
        assert_eq!(replay_loot.item_instance_id(), &loot_id);
        assert!(matches!(
            h.root
                .commit_corpse_loot_mint(&permit, &authority, &h.node, &mut replay_loot)
                .await
                .map_err(fixture::debug)?,
            ItemMintOutcome::AlreadyCommitted(_)
        ));
        let mut replay_channel = h
            .root
            .freeze_item_mint(&authority, &h.node, channel_request)
            .await
            .map_err(fixture::debug)?;
        assert_eq!(
            replay_channel.item_instance_id(),
            channel_corpse.item_instance_id()
        );
        assert!(matches!(
            h.root
                .commit_corpse_mint(
                    &channel_permit,
                    &authority,
                    &h.node,
                    &mut replay_channel,
                    *character.as_bytes()
                )
                .await
                .map_err(fixture::debug)?,
            ItemMintOutcome::AlreadyCommitted(_)
        ));
        assert!(
            h.root
                .commit_corpse_mint(
                    &wrong,
                    &authority,
                    &h.node,
                    &mut replay_corpse,
                    *character.as_bytes()
                )
                .await
                .is_err()
        );
        //DUR03-RL-08 (0319): every commit/reconciliation attempt, including
        //AlreadyCommitted, charges one durable work unit. Only this counter is
        //mutable; compare every other frozen field byte-for-byte independently.
        let replay_rows_after:serde_json::Value=sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(r) ORDER BY transaction_id) FROM game_item_mint_reservations r").fetch_one(&h.pool).await?;
        let before_rows = replay_rows_before
            .as_array()
            .ok_or("reservation rows before replay")?;
        let after_rows = replay_rows_after
            .as_array()
            .ok_or("reservation rows after replay")?;
        assert_eq!(after_rows.len(), before_rows.len());
        for (before, after) in before_rows.iter().zip(after_rows) {
            let mut before = before
                .as_object()
                .ok_or("before reservation object")?
                .clone();
            let mut after = after.as_object().ok_or("after reservation object")?.clone();
            let before_used = before
                .remove("work_units_used")
                .and_then(|value| value.as_u64())
                .ok_or("before work budget")?;
            let after_used = after
                .remove("work_units_used")
                .and_then(|value| value.as_u64())
                .ok_or("after work budget")?;
            assert_eq!(
                after_used,
                before_used + 1,
                "one replay attempt charges exactly one work unit"
            );
            assert!(after_used <= u64::from(item_mint_audit::RL08_RETRY_WORK_UNITS_MAX));
            assert_eq!(
                after, before,
                "public replay cannot rewrite any immutable frozen reservation field"
            );
        }
        assert_eq!(
            replay_corpse.work_units_used(),
            item_mint_audit::RL08_RETRY_WORK_UNITS_MAX
        );
        assert_eq!(
            replay_loot.work_units_used(),
            item_mint_audit::RL08_RETRY_WORK_UNITS_MAX
        );
        assert!(matches!(
            h.root
                .commit_corpse_mint(
                    &permit,
                    &authority,
                    &h.node,
                    &mut replay_corpse,
                    *character.as_bytes()
                )
                .await,
            Err(ItemMintError::CapacityExceeded)
        ));
        assert!(matches!(
            h.root
                .commit_corpse_loot_mint(&permit, &authority, &h.node, &mut replay_loot)
                .await,
            Err(ItemMintError::CapacityExceeded)
        ));
        assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT jsonb_agg(to_jsonb(r) ORDER BY transaction_id) FROM game_item_mint_reservations r").fetch_one(&h.pool).await?,replay_rows_after,"fourth corpse/loot attempts leave entire frozen rows unchanged");
        assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT jsonb_agg(to_jsonb(r) ORDER BY transaction_id) FROM game_item_mint_receipts r").fetch_one(&h.pool).await?,replay_receipts_before,"public replay cannot rewrite immutable receipt identities");
        let replay_ground = h
            .root
            .read_item_instance(&authority, corpse_id)
            .await
            .map_err(fixture::debug)?
            .ok_or("missing replay physical corpse")?;
        assert_eq!(
            replay_ground.ground.corpse_ref, expected_ground,
            "recognized alternate request must retain exact accepted physical bytes"
        );
        for rejected in [
            corpse_request(alias, 1)?.1,
            corpse_request(scope, 2)?.1,
            corpse_request(
                RuntimeScopeRefV1::instance(world, id(89)).map_err(fixture::debug)?,
                1,
            )?
            .1,
        ] {
            assert!(matches!(
                h.root.freeze_item_mint(&authority, &h.node, rejected).await,
                Err(ItemMintError::AuthorityRejected)
            ));
        }
    }
    let envelopes:Vec<Vec<u8>>=sqlx::query_scalar("SELECT envelope FROM game_item_audit_outbox WHERE uuid_send(item_instance_id)=ANY($1::bytea[]) ORDER BY item_instance_id")
        .bind(vec![corpse_id.to_vec(),loot_id.to_vec()]).fetch_all(&h.pool).await?;
    assert_eq!(envelopes.len(), 2);
    for wire in envelopes {
        let (envelope, mint) = item_mint_audit::decode_envelope(&wire).map_err(fixture::debug)?;
        assert_eq!(envelope.instance_id, Some(id(88).to_vec()));
        assert!(envelope.channel_id.is_none());
        let source = mint
            .source
            .ok_or("missing source")?
            .death_occurrence
            .ok_or("missing death")?;
        assert_eq!(source.runtime_scope().map_err(fixture::debug)?, scope);
    }
    Ok(())
}

#[test]
fn native_instance_corpse_mint_postgres_physical_death_loot_replay_and_scope_fences() -> TestResult
{
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    on_pg_thread(move || {
        fixture::runtime()?.block_on(async move {
        let h=fixture::Harness::create_with_admission_seed(admin,"encx_native_corpse",true,false).await?;
        eprintln!("Canonical PostgreSQL 18.6; physical Instance death, native corpse/loot owner, replay and namespace fences");
        let result=qualify(&h,false,false).await;
        h.cleanup().await?;
        result
    })
    })
}

// Reproduce the real1100/1101/1103 prefix using committed physical deaths, not
// synthetic cause rows. Only1152's additive projections are removed/rebuilt;
// every immutability/admission/current-fence guard remains installed.
async fn preserve_committed_causes_across_1152(h: &fixture::Harness) -> TestResult {
    use sqlx::Executor;
    let signatures = [
        "game_item_mint_receipt_materialize_corpse()",
        "game_item_mint_consistency_guard()",
        "game_corpse_source_race_admit()",
        "game_kill_nested_loot_mint_proven(uuid)",
        "game_item_ground_insertion_guard()",
        "game_spell_corpse_entry_owner_lock()",
        "game_item_mint_reservation_guard()",
        "game_item_mint_receipt_guard()",
    ];
    const PROOFS: &str = "SELECT signature,pg_get_functiondef(signature::regprocedure) FROM unnest($1::text[]) AS names(signature) ORDER BY signature";
    const PRIMARY_KEYS: &str = "SELECT conrelid::regclass::text,pg_get_constraintdef(oid) FROM pg_constraint WHERE contype='p' AND conrelid IN('game_item_mint_reservations'::regclass,'game_item_mint_receipts'::regclass) ORDER BY conrelid::regclass::text";
    let proofs_before: Vec<(String, String)> = sqlx::query_as(PROOFS)
        .bind(signatures.to_vec())
        .fetch_all(&h.pool)
        .await?;
    let keys_before: Vec<(String, String)> =
        sqlx::query_as(PRIMARY_KEYS).fetch_all(&h.pool).await?;
    assert_eq!(proofs_before.len(), 8);
    assert_eq!(keys_before.len(), 2);
    assert!(
        keys_before
            .iter()
            .all(|(_, definition)| definition.contains("death_scope_id"))
    );
    let tables = [
        "game_item_mint_reservations",
        "game_item_mint_receipts",
        "game_item_instances",
        "game_item_ground_locations",
        "game_item_corpse_container_entries",
        "game_item_audit_outbox",
    ];
    let mut rows_before = Vec::new();
    for table in tables {
        let query = sqlx::AssertSqlSafe(format!(
            "SELECT coalesce(jsonb_agg(core ORDER BY core::text),'[]'::jsonb) FROM (SELECT to_jsonb(t)-ARRAY['death_scope_kind','death_scope_key'] AS core FROM {table} t) captured"
        ));
        rows_before.push(
            sqlx::query_scalar::<_, serde_json::Value>(query)
                .fetch_one(&h.pool)
                .await?,
        );
    }
    assert!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_item_mint_reservations WHERE work_units_used>0"
        )
        .fetch_one(&h.pool)
        .await?
            >= 3
    );
    let mut tx = h.pool.begin().await?;
    for table in ["game_item_mint_reservations", "game_item_mint_receipts"] {
        // Drop only the compatibility capture, never a proof/immutability guard.
        (&mut *tx)
            .execute(sqlx::AssertSqlSafe(format!(
                "DROP TRIGGER game_item_mint_death_scope_capture ON {table}"
            )))
            .await?;
        (&mut *tx)
            .execute(sqlx::AssertSqlSafe(format!(
                "DROP INDEX {table}_scope_key_cause"
            )))
            .await?;
        (&mut *tx).execute(sqlx::AssertSqlSafe(format!("ALTER TABLE {table} DROP CONSTRAINT {table}_scope_key_exact, DROP CONSTRAINT {table}_death_scope, DROP COLUMN death_scope_kind, DROP COLUMN death_scope_key"))).await?;
    }
    (&mut *tx)
        .execute("DROP FUNCTION game_item_native_instance_corpse_proven(UUID)")
        .await?;
    (&mut *tx)
        .execute("DROP FUNCTION game_item_mint_death_scope_capture()")
        .await?;
    (&mut *tx)
        .execute("DROP FUNCTION game_item_death_scope_key(UUID,SMALLINT,UUID,UUID)")
        .await?;
    (&mut *tx)
        .execute(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/game-server/migrations/1152_instance_creature_death_mints.sql"
        )))
        .await?;
    tx.commit().await?;
    assert_eq!(
        sqlx::query_as::<_, (String, String)>(PROOFS)
            .bind(signatures.to_vec())
            .fetch_all(&h.pool)
            .await?,
        proofs_before,
        "all1100/1101/1103 proof bodies and security settings remain byte-identical"
    );
    assert_eq!(
        sqlx::query_as::<_, (String, String)>(PRIMARY_KEYS)
            .fetch_all(&h.pool)
            .await?,
        keys_before,
        "original exact-cause primary keys are not replaced"
    );
    for (table, before) in tables.into_iter().zip(rows_before) {
        let query = sqlx::AssertSqlSafe(format!(
            "SELECT coalesce(jsonb_agg(core ORDER BY core::text),'[]'::jsonb) FROM (SELECT to_jsonb(t)-ARRAY['death_scope_kind','death_scope_key'] AS core FROM {table} t) captured"
        ));
        assert_eq!(
            sqlx::query_scalar::<_, serde_json::Value>(query)
                .fetch_one(&h.pool)
                .await?,
            before,
            "immutable core rows changed in {table}"
        );
    }
    for table in ["game_item_mint_reservations", "game_item_mint_receipts"] {
        // Independently check both actual byte layouts, not the new helper itself.
        let exact = sqlx::AssertSqlSafe(format!(
            "SELECT coalesce(bool_and(octet_length(death_scope_id)=17 AND octet_length(death_scope_key)=33 AND substring(death_scope_key FROM 2 FOR 16)=uuid_send(death_world_id) AND CASE WHEN death_channel_id IS NOT NULL THEN death_scope_kind=1 AND death_instance_id IS NULL AND get_byte(death_scope_id,0)=1 AND get_byte(death_scope_key,0)=1 AND substring(death_scope_id FROM 2)=uuid_send(death_channel_id) AND substring(death_scope_key FROM 18)=uuid_send(death_channel_id) ELSE death_scope_kind=2 AND get_byte(death_scope_id,0)=2 AND get_byte(death_scope_key,0)=3 AND substring(death_scope_id FROM 2)=uuid_send(death_instance_id) AND substring(death_scope_key FROM 18)=uuid_send(death_instance_id) END),false) FROM {table}"
        ));
        assert!(
            sqlx::query_scalar::<_, bool>(exact)
                .fetch_one(&h.pool)
                .await?,
            "exact Channel/Instance projections: {table}"
        );
    }
    eprintln!("NATIVE_CORPSE_1152_EXISTING_TYPED_SCHEMA_PRESERVED");
    Ok(())
}

#[test]
fn native_instance_corpse_migration_preserves_committed_causes_and_replay_postgres() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    on_pg_thread(move || {
        fixture::runtime()?.block_on(async move {
        let h=fixture::Harness::create_with_admission_seed(admin,"encx_corpse_migration",true,false).await?;
        eprintln!("Canonical PostgreSQL18.6:1152 preserves genuine committed Channel/Instance corpse and loot causes");
        let result=qualify(&h,true,false).await;
        h.cleanup().await?;
        result
    })
    })
}

#[test]
fn native_instance_corpse_migration_preserves_legacy_v1_corpse_and_loot_replay_postgres()
-> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    on_pg_thread(move || {
        fixture::runtime()?.block_on(async move {
        let h=fixture::Harness::create_with_admission_seed(admin,"encx_corpse_legacy_migration",true,false).await?;
        eprintln!("Canonical PostgreSQL18.6:legacyv1/61 nativeInstance corpse+loot survives1152 and publicv2/53 replay");
        let result=qualify(&h,true,true).await;
        h.cleanup().await?;
        result
    })
    })
}

#[path = "native_corpse/activity_entry.rs"]
mod activity_entry;

#[path = "native_corpse/primal_deadline.rs"]
mod primal_deadline;
