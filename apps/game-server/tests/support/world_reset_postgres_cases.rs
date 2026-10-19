//! Exact native Root/PG cases. No fake Root, disabled trigger or invented current owner.
use crate::durability::item_decay_retire::{CorpseDecayFence, DecayRetireError, DecayRetireStep};
use crate::durability::item_decay_retire_audit::{self as audit, DecayRetireShape};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
    RuntimeScopeAssignmentWriter,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::world_reset::{
    WorldResetError, WorldResetRetirement, census_retiring_world_reset,
};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use crate::item_transfer_postgres_cases::{
    CHANNEL, Harness, SESSION, TestResult, WORLD, configured_admin, debug, id, runtime, scope,
};

#[derive(Debug)]
enum WorldResetPass {
    Retired(WorldResetRetirement),
    ChannelEmpty,
}

fn world() -> TestResult<WorldId> {
    WorldId::decode(&id(WORLD)).map_err(|e| debug(e).into())
}
fn channel() -> TestResult<ChannelId> {
    ChannelId::decode(&id(CHANNEL)).map_err(|e| debug(e).into())
}
/// D1416: migration 0131 installs the reset schema; fixtures never run DDL themselves.
async fn install(h: &Harness) -> TestResult {
    let jobs: bool = sqlx::query_scalar("SELECT to_regclass('game_world_reset_jobs') IS NOT NULL")
        .fetch_one(&h.pool)
        .await?;
    assert!(jobs, "migration 0131 must install the reset tables");
    let guards: Vec<String> = sqlx::query_scalar(
        "SELECT proname::text FROM pg_proc WHERE pronamespace = current_schema()::regnamespace \
           AND prosrc LIKE '%game_world_reset_item_proven(%.item_instance_id)%' ORDER BY 1",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        guards,
        [
            "game_item_corpse_entry_removal_proven",
            "game_item_decay_retire_consistency_guard",
            "game_item_ground_removal_proven",
            "game_item_instance_change_proven",
        ]
    );

    // The replaced guards keep main's security attribute (0015 header, 0095 ALTER).
    let definers: Vec<(String, bool)> = sqlx::query_as(
        "SELECT proname::text, prosecdef FROM pg_proc WHERE pronamespace = current_schema()::regnamespace \
           AND prosrc LIKE '%game_world_reset_item_proven(%.item_instance_id)%' ORDER BY 1",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        definers,
        [
            ("game_item_corpse_entry_removal_proven".to_owned(), false),
            ("game_item_decay_retire_consistency_guard".to_owned(), true),
            ("game_item_ground_removal_proven".to_owned(), false),
            ("game_item_instance_change_proven".to_owned(), true),
        ]
    );
    Ok(())
}
/// Explicit Control bootstrap of epoch 0 through the real initializer.
async fn initialize_epoch(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_control")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "SELECT game_world_reset_initialize_epoch(encode($1,'hex')::uuid,'map-1',$2,$3,$4,$5,1)",
    )
    .bind(id(WORLD).as_slice())
    .bind([1_u8; 32].as_slice())
    .bind([2_u8; 32].as_slice())
    .bind([3_u8; 32].as_slice())
    .bind(crate::durability::world_reset::ResetCalendarPolicy::TibiaServerSaveBerlin10.as_str())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
async fn admit(h: &Harness) -> TestResult {
    // Explicit Control bootstrap via the same operation; never seed epoch0 rows directly.
    initialize_epoch(h).await?;
    // Codex P1 4307187667: control holds no job INSERT. This legacy-shape job (no target)
    // is a fixture written as the test admin; `admit_qualified` uses the real publisher.
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,active_map_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,'map-1','RETIRING')").bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_reset_scopes SELECT world_id,0,channel_id,ownership_generation FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1").bind(id(WORLD).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn replace(h: &Harness) -> TestResult {
    replace_with(h, &h.node, 73).await
}
async fn replace_with(
    h: &Harness,
    node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
    key: u8,
) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2) ON CONFLICT DO NOTHING").bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope()?)
        .await
        .map_err(debug)?
        .ok_or("predecessor")?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "item-transfer-writer")
        .await
        .map_err(debug)?;
    let result = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([key; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Replace {
                scope: scope()?,
                predecessor,
                target: node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    assert!(matches!(result, AssignmentOutcome::Committed(_)));
    // Actual native session fact, changed by the privileged fixture after all player writes.
    // No current authority is manufactured; the writer separately checks its native recovery/node.
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=encode($1,'hex')::uuid").bind(id(SESSION).as_slice()).execute(&h.pool).await?;
    Ok(())
}
async fn pass(
    h: &Harness,
    permit: &SpellLanePermit,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<WorldResetPass> {
    match h
        .root
        .freeze_next_world_reset_item(permit, authority, &h.node, world()?, channel()?)
        .await
        .map_err(debug)?
    {
        Some(candidate) => Ok(WorldResetPass::Retired(
            h.root
                .commit_world_reset_item(permit, authority, &h.node, &candidate)
                .await
                .map_err(debug)?,
        )),
        None => Ok(WorldResetPass::ChannelEmpty),
    }
}
async fn footprint(h: &Harness) -> TestResult<Vec<i64>> {
    let mut result = Vec::new();
    for table in [
        "game_item_decay_retire_reservations",
        "game_item_decay_retire_receipts",
        "game_item_audit_outbox",
        "game_item_ground_locations",
        "game_item_corpse_container_entries",
        "game_item_house_interior_locations",
    ] {
        result.push(h.count(table).await?);
    }
    Ok(result)
}

#[test]
fn reset_native_root_retires_actual_items_reloads_history_and_never_touches_inventory() -> TestResult
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "resetroot").await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let ground = h.mint(&authority, "fixture:b3.stone", 3).await?;
        let backpack = crate::corpse_transfer_postgres_cases::equip_backpack(
            &h,
            &authority,
            crate::item_transfer_postgres_cases::fence()?,
            SESSION,
        )
        .await?;
        // The only admitted native key source is a real reward-claim MINT into inventory.
        use crate::durability::reward_claim_mint::{RewardClaimMintRequest,RewardClaimMintOutcome};
        use crate::durability::item_transfer::{ItemDefinitionFacts,ItemStackClass};
        let definition=crate::durability::item_mint::TypedDefinitionRef{family:"Item".into(),production_key:"fixture:b3.stone".into(),revision_ref:"definition-r1".into()};
        let request=RewardClaimMintRequest{command:crate::item_transfer_postgres_cases::command(20)?,claim:crate::durability::item_mint::TypedDefinitionRef{family:"RewardClaim".into(),production_key:"fixture:chest.reset-key".into(),revision_ref:"definition-r1".into()},source_placement:"fixture:placement.chest".into(),item:ItemDefinitionFacts{weight:Some(1),definition,stack:ItemStackClass::NonStackable,container_capacity:None,container_slot_equip_pattern:false},quantity:1,backpack:crate::item_transfer_postgres_cases::backpack_facts(),content_revision:"content-1".into(),ruleset_revision:"ruleset-1".into(),sim_revision:"sim-1".into(),achievement:None,quest_transition:None,key_number:Some(7)};
        let mut key_candidate=h.root.freeze_reward_claim_mint(&authority,&h.node,crate::item_transfer_postgres_cases::fence()?,request).await.map_err(debug)?;
        let key=match h.root.commit_reward_claim_mint(&authority,&h.node,crate::item_transfer_postgres_cases::fence()?,&mut key_candidate).await.map_err(debug)?{RewardClaimMintOutcome::Committed(value)|RewardClaimMintOutcome::AlreadyCommitted(value)=>value.item_instance_id};
        install(&h).await?;
        admit(&h).await?;
        let permit = SpellLanePermit::of_fresh_lane(world()?, channel()?).await;
        let before = footprint(&h).await?;
        assert!(matches!(
            h.root
                .freeze_next_world_reset_item(&permit, &authority, &h.node, world()?, channel()?)
                .await,
            Err(WorldResetError::NotCurrent)
        ));
        assert_eq!(footprint(&h).await?, before);
        replace(&h).await?;
        let WorldResetPass::Retired(receipt) = pass(&h,&permit,&authority)
            .await
            .map_err(debug)?
        else {
            return Err("retirement required".into());
        };
        assert_eq!(receipt.item_instance_id, ground);
        assert_eq!(h.item_state(ground).await?, (0, 2));
        assert_eq!(h.item_state(backpack).await?, (1, 1));
        assert_eq!(h.item_state(key).await?,(1,1));
        assert_eq!(sqlx::query_scalar::<_,i32>("SELECT key_number FROM game_item_keys WHERE item_instance_id=encode($1,'hex')::uuid").bind(key.as_slice()).fetch_one(&h.pool).await?,7);
        let wire: Vec<u8> = sqlx::query_scalar(
            "SELECT envelope FROM game_item_audit_outbox WHERE event_id=encode($1,'hex')::uuid",
        )
        .bind(receipt.event_id.as_slice())
        .fetch_one(&h.pool)
        .await?;
        let (_, event) = audit::decode_decay_retire_envelope(&wire).map_err(debug)?;
        assert_eq!(
            audit::check_decay_retire(&event).map_err(debug)?,
            DecayRetireShape::WorldResetGround
        );
        assert_eq!(event.before.as_ref().ok_or("before")?.quantity, 3);
        assert_eq!(event.after.as_ref().ok_or("after")?.quantity, 0);
        assert_eq!(
            h.root
                .read_world_reset_retirement(&authority, ground)
                .await
                .map_err(debug)?,
            Some(receipt.clone())
        );
        assert!(matches!(
            pass(&h,&permit,&authority)
                .await
                .map_err(debug)?,
            WorldResetPass::ChannelEmpty
        ));
        let counts = footprint(&h).await?;
        drop(authority);
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            h.root
                .read_world_reset_retirement(&authority, ground)
                .await
                .map_err(debug)?,
            Some(receipt)
        );
        assert_eq!(footprint(&h).await?, counts);
        drop(authority);
        drop(seal);
        drop(permit);
        h.cleanup().await
    })
}

#[test]
fn reset_native_corpse_entries_precede_parent_and_stale_decay_cannot_commit() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move{
  let h=Harness::create(admin,"resetcorpse").await?;let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
  let corpse=crate::corpse_transfer_postgres_cases::mint_corpse(&h,&authority,400,id(crate::item_transfer_postgres_cases::CHARACTER)).await?;
  let legacy_before=crate::corpse_transfer_postgres_cases::put_loot(&h,corpse,400,1,80).await?;
  let legacy_after=crate::corpse_transfer_postgres_cases::put_loot(&h,corpse,400,2,81).await?;
  let reset_entry=crate::corpse_transfer_postgres_cases::put_loot(&h,corpse,400,3,82).await?;
  // Reach the actual native DB-clock deadline; never disable guards or rewrite a receipt.
  tokio::time::sleep(std::time::Duration::from_millis(60_100)).await;
  let old=CorpseDecayFence{world_id:world()?,channel_id:channel()?,scope_ownership_generation:ScopeOwnershipGeneration::new(1).map_err(debug)?};
  let permit=SpellLanePermit::of_fresh_lane(world()?,channel()?).await;
  let mut before=h.root.freeze_decay_retire(&authority,&h.node,old,DecayRetireStep::entry(corpse,legacy_before)).await.map_err(debug)?;
  h.root.commit_decay_retire(&permit,&authority,&h.node,old,&mut before).await.map_err(debug)?;
  let mut frozen=h.root.freeze_decay_retire(&authority,&h.node,old,DecayRetireStep::entry(corpse,reset_entry)).await.map_err(debug)?;
  let legacy_wire=sqlx::query_scalar::<_,Vec<u8>>("SELECT a.envelope FROM game_item_audit_outbox a JOIN game_item_decay_retire_receipts r USING(event_id) WHERE r.item_instance_id=encode($1,'hex')::uuid").bind(legacy_before.as_slice()).fetch_one(&h.pool).await?;
  let (_,legacy_message)=audit::decode_decay_retire_envelope(&legacy_wire).map_err(debug)?;
  assert_eq!(audit::check_decay_retire(&legacy_message).map_err(debug)?,DecayRetireShape::Entry);
  assert!(legacy_message.world_reset.is_none() && legacy_message.ground_container_entry.is_none());
  install(&h).await?;
  assert_eq!(sqlx::query_scalar::<_,Vec<u8>>("SELECT a.envelope FROM game_item_audit_outbox a JOIN game_item_decay_retire_receipts r USING(event_id) WHERE r.item_instance_id=encode($1,'hex')::uuid").bind(legacy_before.as_slice()).fetch_one(&h.pool).await?,legacy_wire);
  let mut after=h.root.freeze_decay_retire(&authority,&h.node,old,DecayRetireStep::entry(corpse,legacy_after)).await.map_err(debug)?;
  h.root.commit_decay_retire(&permit,&authority,&h.node,old,&mut after).await.map_err(debug)?;
  admit(&h).await?;replace(&h).await?;
  assert!(matches!(h.root.commit_decay_retire(&permit,&authority,&h.node,old,&mut frozen).await,Err(DecayRetireError::AuthorityRejected)));
  let mut retired=Vec::new();for _ in 0..2{
   let WorldResetPass::Retired(r)=pass(&h,&permit,&authority).await? else{return Err("retirement required".into())};retired.push(r.item_instance_id);
  }assert_eq!(retired,vec![reset_entry,corpse]);
  assert!(matches!(h.root.freeze_decay_retire(&authority,&h.node,old,DecayRetireStep::entry(corpse,reset_entry)).await,Err(DecayRetireError::ConflictingCause)));
  let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_decay_retire_receipts").fetch_one(&h.pool).await?;assert_eq!(count,4);
  let reservations:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_decay_retire_reservations WHERE item_instance_id=encode($1,'hex')::uuid").bind(reset_entry.as_slice()).fetch_one(&h.pool).await?;assert_eq!(reservations,2);
  drop(authority);drop(seal);drop(permit);h.cleanup().await
 })
}

#[test]
fn reset_native_deferred_guard_failure_rolls_back_every_line_and_retry_completes() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move{
  let h=Harness::create(admin,"resetrollback").await?;let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
  let item=h.mint(&authority,"fixture:b3.coin",11).await?;install(&h).await?;admit(&h).await?;replace(&h).await?;
  // A real extra failure at the audit boundary; all native guards remain installed/enabled.
  sqlx::raw_sql("CREATE FUNCTION reset_test_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected outbox failure' USING ERRCODE='23514'; END $$; CREATE CONSTRAINT TRIGGER reset_test_fail AFTER INSERT ON game_item_audit_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reset_test_fail();").execute(&h.pool).await?;
  let permit=SpellLanePermit::of_fresh_lane(world()?,channel()?).await;
  let candidate=h.root.freeze_next_world_reset_item(&permit,&authority,&h.node,world()?,channel()?).await.map_err(debug)?.ok_or("candidate")?;
  let before=footprint(&h).await?;
  let revision=sqlx::query_scalar::<_,String>("SELECT revision::text FROM game_world_reset_jobs").fetch_one(&h.pool).await?;
  assert!(h.root.commit_world_reset_item(&permit,&authority,&h.node,&candidate).await.is_err());assert_eq!(footprint(&h).await?,before);assert_eq!(h.item_state(item).await?,(11,1));
  assert_eq!(sqlx::query_scalar::<_,String>("SELECT revision::text FROM game_world_reset_jobs").fetch_one(&h.pool).await?,revision);
  assert_eq!(sqlx::query_scalar::<_,i16>("SELECT work_units_used FROM game_item_decay_retire_reservations").fetch_one(&h.pool).await?,1);
  sqlx::query("DROP TRIGGER reset_test_fail ON game_item_audit_outbox").execute(&h.pool).await?;
  let result=h.root.commit_world_reset_item(&permit,&authority,&h.node,&candidate).await.map_err(debug)?;
  assert_eq!(result,h.root.commit_world_reset_item(&permit,&authority,&h.node,&candidate).await.map_err(debug)?);
  assert_eq!(h.item_state(item).await?,(0,2));
  assert_eq!(sqlx::query_scalar::<_,String>("SELECT revision::text FROM game_world_reset_jobs").fetch_one(&h.pool).await?,"2");
  assert_eq!(sqlx::query_scalar::<_,i16>("SELECT work_units_used FROM game_item_decay_retire_reservations").fetch_one(&h.pool).await?,2);
  drop(authority);drop(seal);drop(permit);h.cleanup().await
 })
}

#[test]
fn reset_native_unprivileged_job_publication_and_unproven_delete_are_refused() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move{
  let h=Harness::create(admin,"resetproof").await?;let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
  let item=h.mint(&authority,"fixture:b3.stone",1).await?;install(&h).await?;
  let before=footprint(&h).await?;
  let mut tx=h.pool.begin().await?;
  assert!(sqlx::query("TRUNCATE game_world_reset_scopes").execute(&mut *tx).await.is_err());tx.rollback().await?;
  let mut tx=h.pool.begin().await?;sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
  assert!(sqlx::query("INSERT INTO game_world_reset_jobs VALUES(encode($1,'hex')::uuid,0,$2,'map-1','RETIRING',1)").bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice()).execute(&mut *tx).await.is_err());tx.rollback().await?;
  // Codex P1 4307187667: control publishes no RETIRING job directly either.
  let mut tx=h.pool.begin().await?;sqlx::query("SET LOCAL ROLE oteryn_game_control").execute(&mut *tx).await?;
  let refused=sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,active_map_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,'map-1','RETIRING')").bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice()).execute(&mut *tx).await;
  assert!(matches!(refused,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("42501")));tx.rollback().await?;
  let mut tx=h.pool.begin().await?;sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).execute(&mut *tx).await?;
  assert!(tx.commit().await.is_err());assert_eq!(footprint(&h).await?,before);
  // The job revision is no runtime or control capability: only a WORLD_RESET receipt advances it.
  admit(&h).await?;
  for set_role in ["SET LOCAL ROLE oteryn_game_runtime","SET LOCAL ROLE oteryn_game_control"]{
   let mut tx=h.pool.begin().await?;sqlx::query(set_role).execute(&mut *tx).await?;
   assert!(sqlx::query("UPDATE game_world_reset_jobs SET revision=revision+1 WHERE world_id=encode($1,'hex')::uuid").bind(id(WORLD).as_slice()).execute(&mut *tx).await.is_err());tx.rollback().await?;
  }
  assert_eq!(sqlx::query_scalar::<_,String>("SELECT revision::text FROM game_world_reset_jobs").fetch_one(&h.pool).await?,"1");
  drop(authority);drop(seal);h.cleanup().await
 })
}

#[test]
fn reset_native_retry_maximum_persists_frozen_identity_across_authority_reload() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
  let h=Harness::create(admin,"resetbudget").await?;let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
  let item=h.mint(&authority,"fixture:b3.coin",9).await?;install(&h).await?;admit(&h).await?;replace(&h).await?;
  let permit=SpellLanePermit::of_fresh_lane(world()?,channel()?).await;
  let candidate=h.root.freeze_next_world_reset_item(&permit,&authority,&h.node,world()?,channel()?).await.map_err(debug)?.ok_or("candidate")?;
  let identity=sqlx::query_scalar::<_,Vec<u8>>("SELECT envelope FROM game_item_decay_retire_reservations").fetch_one(&h.pool).await?;
  sqlx::raw_sql("CREATE FUNCTION reset_test_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'persistent failure' USING ERRCODE='23514'; END $$; CREATE CONSTRAINT TRIGGER reset_test_fail AFTER INSERT ON game_item_audit_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reset_test_fail();").execute(&h.pool).await?;
  let before=footprint(&h).await?;
  for used in 1_i16..=3 {
   assert!(h.root.commit_world_reset_item(&permit,&authority,&h.node,&candidate).await.is_err());
   assert_eq!(footprint(&h).await?,before);assert_eq!(h.item_state(item).await?,(9,1));
   assert_eq!(sqlx::query_scalar::<_,i16>("SELECT work_units_used FROM game_item_decay_retire_reservations").fetch_one(&h.pool).await?,used);
   assert_eq!(sqlx::query_scalar::<_,Vec<u8>>("SELECT envelope FROM game_item_decay_retire_reservations").fetch_one(&h.pool).await?,identity);
  }
  drop(authority);let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
  let resumed=h.root.freeze_next_world_reset_item(&permit,&authority,&h.node,world()?,channel()?).await.map_err(debug)?.ok_or("resumed")?;
  assert!(matches!(h.root.commit_world_reset_item(&permit,&authority,&h.node,&resumed).await,Err(WorldResetError::CapacityExceeded)));
  assert_eq!(sqlx::query_scalar::<_,Vec<u8>>("SELECT envelope FROM game_item_decay_retire_reservations").fetch_one(&h.pool).await?,identity);
  assert_eq!(footprint(&h).await?,before);
  drop(authority);drop(seal);drop(permit);h.cleanup().await
 })
}

#[test]
fn reset_native_concurrent_same_candidate_replays_one_receipt_without_new_ids() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "resetrace").await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let item = h.mint(&authority, "fixture:b3.stone", 1).await?;
        install(&h).await?;
        admit(&h).await?;
        replace(&h).await?;
        let permit = SpellLanePermit::of_fresh_lane(world()?, channel()?).await;
        let candidate = h
            .root
            .freeze_next_world_reset_item(&permit, &authority, &h.node, world()?, channel()?)
            .await
            .map_err(debug)?
            .ok_or("candidate")?;
        // Each genuine native Root retains its original one-connection admission bound.
        // A separate Root/authority tests DB serialization rather than local pool refusal.
        let other = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let other_authority = other.open_character_authority(&seal).await.map_err(debug)?;
        let (one, two) = crate::item_transfer_postgres_cases::join_two(
            h.root
                .commit_world_reset_item(&permit, &authority, &h.node, &candidate),
            other.commit_world_reset_item(&permit, &other_authority, &h.node, &candidate),
        )
        .await;
        assert_eq!(one.map_err(debug)?, two.map_err(debug)?);
        assert_eq!(h.item_state(item).await?, (0, 2));
        assert_eq!(h.count("game_item_decay_retire_receipts").await?, 1);
        assert_eq!(h.count("game_item_decay_retire_reservations").await?, 1);
        drop(other_authority);
        drop(other);
        drop(authority);
        drop(seal);
        drop(permit);
        h.cleanup().await
    })
}

#[test]
fn reset_native_foreign_channel_holder_is_refused_but_house_assignment_is_not_a_channel()
-> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
  let h=Harness::create(admin,"resetcohort").await?;let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
  let item=h.mint(&authority,"fixture:b3.stone",1).await?;
  let foreign=crate::item_transfer_postgres_cases::register(&h.root,2).await?;
  use crate::durability::house_scope_handoff::{HouseId,HouseScopeAssignmentRequest,HouseScopeAssignmentCommand,HouseScopeAssignmentOutcome};
  let house_key="oteryn:content.house.thais_1";
  sqlx::query("INSERT INTO game_control_house_scope_grants(control_role,world_id,house_key,operation) VALUES(session_user,encode($1,'hex')::uuid,$2,1)").bind(id(WORLD).as_slice()).bind(house_key).execute(&h.pool).await?;
  let house=h.root.assign_house_scope(HouseScopeAssignmentRequest{operation_key:OperationKey::from_bytes([100;32]),actor:ControlActor::new("oteryn_test_admin").map_err(debug)?,actor_role:"oteryn_test_admin".into(),command:HouseScopeAssignmentCommand::Assign{house:HouseId::new(world()?,house_key).ok_or("house key")?,target:foreign.fact()}}).await.map_err(debug)?;
  assert!(matches!(house,HouseScopeAssignmentOutcome::Committed(_)));
  install(&h).await?;
  let permit=SpellLanePermit::of_fresh_lane(world()?,channel()?).await;
  let before=footprint(&h).await?;
  assert!(matches!(h.root.freeze_next_world_reset_item(&permit,&authority,&h.node,world()?,channel()?).await,Err(WorldResetError::NotCurrent)));
  assert_eq!(footprint(&h).await?,before);
  admit(&h).await?;replace_with(&h,&foreign,101).await?;
  let mut census_tx=h.pool.begin().await?;
  assert!(matches!(census_retiring_world_reset(&mut census_tx,&h.node,world()?).await,Err(WorldResetError::NotCurrent)));
  census_tx.rollback().await?;
  assert!(matches!(h.root.freeze_next_world_reset_item(&permit,&authority,&h.node,world()?,channel()?).await,Err(WorldResetError::NotCurrent)));
  assert_eq!(footprint(&h).await?,before);
  replace_with(&h,&h.node,102).await?;
  let mut census_tx=h.pool.begin().await?;
  let census=census_retiring_world_reset(&mut census_tx,&h.node,world()?).await.map_err(debug)?.ok_or("retiring census")?;
  assert_eq!((census.reset_epoch,census.target_digest,census.channel_count,census.ground_root_count,census.ground_corpse_entry_count),(0,[7;32],1,1,0));
  census_tx.rollback().await?;
  assert!(matches!(pass(&h,&permit,&authority).await?,WorldResetPass::Retired(_)));
  let mut census_tx=h.pool.begin().await?;
  let census=census_retiring_world_reset(&mut census_tx,&h.node,world()?).await.map_err(debug)?.ok_or("empty retiring census")?;
  assert_eq!((census.channel_count,census.ground_root_count,census.ground_corpse_entry_count),(1,0,0));
  census_tx.rollback().await?;
  assert_eq!(h.item_state(item).await?,(0,2));
  assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_runtime_scope_assignments WHERE scope_kind=2 AND holder_node_id=encode($1,'hex')::uuid").bind(foreign.fact().node_id().as_bytes().as_slice()).fetch_one(&h.pool).await?,1);
  drop(authority);drop(seal);drop(permit);h.cleanup().await
 })
}

#[test]
fn reset_native_frozen_v1_survives_real_activation_and_new_work_uses_v2() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
  let h=Harness::create(admin,"resetactivation").await?;let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
  h.mint(&authority,"fixture:b3.stone",1).await?;h.mint(&authority,"fixture:b3.coin",2).await?;
  install(&h).await?;admit(&h).await?;replace(&h).await?;
  let permit=SpellLanePermit::of_fresh_lane(world()?,channel()?).await;
  let candidate=h.root.freeze_next_world_reset_item(&permit,&authority,&h.node,world()?,channel()?).await.map_err(debug)?.ok_or("candidate")?;
  let bytes=sqlx::query_scalar::<_,Vec<u8>>("SELECT envelope FROM game_item_decay_retire_reservations").fetch_one(&h.pool).await?;
  assert_eq!(sqlx::query_scalar::<_,i16>("SELECT type2_schema_revision FROM game_item_decay_retire_reservations").fetch_one(&h.pool).await?,1);
  sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())").execute(&h.pool).await?;
  let resumed=h.root.freeze_next_world_reset_item(&permit,&authority,&h.node,world()?,channel()?).await.map_err(debug)?.ok_or("resumed")?;
  let result=h.root.commit_world_reset_item(&permit,&authority,&h.node,&resumed).await.map_err(debug)?;
  assert_eq!(result,h.root.commit_world_reset_item(&permit,&authority,&h.node,&candidate).await.map_err(debug)?);
  assert_eq!(sqlx::query_scalar::<_,Vec<u8>>("SELECT envelope FROM game_item_audit_outbox WHERE event_id=encode($1,'hex')::uuid").bind(result.event_id.as_slice()).fetch_one(&h.pool).await?,bytes);
  assert_eq!(sqlx::query_scalar::<_,i64>("SELECT schema_revision FROM game_item_audit_outbox WHERE event_id=encode($1,'hex')::uuid").bind(result.event_id.as_slice()).fetch_one(&h.pool).await?,1);
  let WorldResetPass::Retired(second)=pass(&h,&permit,&authority).await? else{return Err("second required".into())};
  assert_eq!(sqlx::query_scalar::<_,i64>("SELECT schema_revision FROM game_item_audit_outbox WHERE event_id=encode($1,'hex')::uuid").bind(second.event_id.as_slice()).fetch_one(&h.pool).await?,2);
  drop(authority);drop(seal);drop(permit);h.cleanup().await
 })
}

#[test]
fn reset_issuance_core_is_private_and_control_wrapper_still_records() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "resetissuancecore").await?;
        install(&h).await?;
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
        let previous: Option<String> = sqlx::query_scalar("SELECT max(activation_sequence)::text FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).fetch_one(&h.pool).await?;
        let previous = previous.map(|value| value.parse::<u64>()).transpose()?;
        let next = previous.unwrap_or(0).checked_add(1).ok_or("sequence exhausted")?;
        // A genuinely granted control call proves the extracted native issuer still writes.
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5,$6,$7)")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice())
            .bind(next.to_string()).bind(previous.map(|value|value.to_string()))
            .bind([31_u8;32].as_slice()).bind([32_u8;32].as_slice()).bind([33_u8;32].as_slice())
            .execute(&h.pool).await?;
        let recorded: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND activation_sequence=$3::text::numeric AND server_artifact_digest=$4 AND client_artifact_digest=$5 AND frame_binding_digest=$6)")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).bind(next.to_string())
            .bind([31_u8;32].as_slice()).bind([32_u8;32].as_slice()).bind([33_u8;32].as_slice()).fetch_one(&h.pool).await?;
        assert!(recorded);
        let mut tx = h.pool.begin().await?;
        let exists: bool = sqlx::query_scalar("SELECT to_regprocedure('game_content_record_activation_core(uuid,uuid,numeric,numeric,bytea,bytea,bytea)') IS NOT NULL AND has_schema_privilege('oteryn_game_runtime',current_schema(),'USAGE')")
            .fetch_one(&mut *tx).await?;
        assert!(exists);
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
        let may_execute: bool = sqlx::query_scalar("SELECT has_function_privilege(current_user,'game_content_record_activation_core(uuid,uuid,numeric,numeric,bytea,bytea,bytea)','EXECUTE')")
            .fetch_one(&mut *tx).await?;
        assert!(!may_execute);
        // The exact already-recorded replay is valid data; only EXECUTE authority changes.
        let denied = sqlx::query("SELECT game_content_record_activation_core(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5,$6,$7)")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice())
            .bind(next.to_string()).bind(previous.map(|value|value.to_string()))
            .bind([31_u8;32].as_slice()).bind([32_u8;32].as_slice()).bind([33_u8;32].as_slice())
            .execute(&mut *tx).await;
        assert!(matches!(denied,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("42501")));
        tx.rollback().await?;
        h.cleanup().await
    })
}

#[test]
fn reset_calendar_uses_berlin_server_save_month_and_upstream_dst() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "resetcalendar").await?;
        install(&h).await?;
        for (instant, expected) in [
            ("2026-03-01 08:59:59+00", "2026-02"),
            ("2026-03-01 09:00:00+00", "2026-03"),
            ("2026-03-29 00:59:59+00", "2026-03"),
            ("2026-03-29 01:00:00+00", "2026-03"),
            ("2026-04-01 07:59:59+00", "2026-03"),
            ("2026-04-01 08:00:00+00", "2026-04"),
            ("2026-10-25 00:59:59+00", "2026-10"),
            ("2026-10-25 01:00:00+00", "2026-10"),
            ("2026-11-01 08:59:59+00", "2026-10"),
            ("2026-11-01 09:00:00+00", "2026-11"),
        ] {
            let month: String = sqlx::query_scalar(
                "SELECT game_world_reset_month_key($1::text::timestamptz,$2,1)",
            )
            .bind(instant)
            .bind(
                crate::durability::world_reset::ResetCalendarPolicy::TibiaServerSaveBerlin10
                    .as_str(),
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(month, expected);
        }
        let unsupported: Option<String> =
            sqlx::query_scalar("SELECT game_world_reset_month_key(clock_timestamp(),'UNKNOWN',1)")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(unsupported, None);
        h.cleanup().await
    })
}

#[test]
fn reset_epoch_requires_explicit_control_initialization_and_preserves_exact_history() -> TestResult
{
    use crate::durability::world_reset::{
        read_active_world_revision, read_locked_world_reset_target,
        read_reset_month_key_in_transaction, read_world_reset_history,
    };
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "resetinitialize").await?;
        install(&h).await?;
        let mut tx=h.pool.begin().await?;
        assert!(matches!(read_active_world_revision(&mut tx,world()?).await,Err(WorldResetError::NotInitialized)));
        assert!(matches!(read_world_reset_history(&mut tx,world()?,0).await,Err(WorldResetError::NotInitialized)));
        assert!(matches!(read_reset_month_key_in_transaction(&mut tx,world()?).await,Err(WorldResetError::NotInitialized)));
        tx.rollback().await?;
        let mut tx=h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_control").execute(&mut *tx).await?;
        let denied=sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,active_map_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,'map-1','RETIRING')")
            .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice()).execute(&mut *tx).await;
        assert!(matches!(denied,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("42501")));
        tx.rollback().await?;
        // Even a privileged writer cannot publish a job before the epoch is initialized.
        let missing=sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,active_map_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,'map-1','RETIRING')")
            .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await;
        assert!(matches!(missing,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("OTR02")));
        // Existing native fixture entrypoint uses the real Control initializer, no SQL seed.
        admit(&h).await?;
        let mut tx=h.pool.begin().await?;
        let original=read_active_world_revision(&mut tx,world()?).await.map_err(debug)?;
        assert_eq!(original.epoch,0);
        assert_eq!(original.map_revision,"map-1");
        assert_eq!(original.calendar.revision,1);
        assert_eq!(read_world_reset_history(&mut tx,world()?,0).await.map_err(debug)?,Some(original.clone()));
        assert!(matches!(read_locked_world_reset_target(&mut tx,world()?).await,Err(WorldResetError::UnqualifiedTarget)));
        let observed=read_reset_month_key_in_transaction(&mut tx,world()?).await.map_err(debug)?;
        assert_eq!(observed.calendar,original.calendar);
        let recomputed: String=sqlx::query_scalar("SELECT game_world_reset_month_key(to_timestamp($1::text::double precision/1000),$2,$3::text::numeric)")
            .bind(observed.observed_at_utc_ms.to_string()).bind(observed.calendar.policy.as_str()).bind(observed.calendar.revision.to_string()).fetch_one(&mut *tx).await?;
        assert_eq!(observed.month_key,recomputed);
        tx.rollback().await?;
        admit_initializer_replay(&h,"map-1",1,[1,2,3]).await?;
        let mut tx=h.pool.begin().await?;
        assert_eq!(read_active_world_revision(&mut tx,world()?).await.map_err(debug)?,original);
        tx.rollback().await?;
        for (map,revision) in [("other-map",1),("map-1",2)] {
            let conflict=admit_initializer_replay(&h,map,revision,[1,2,3]).await;
            assert!(matches!(conflict,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("OTR01")));
        }
        for digests in [[9,2,3],[1,9,3],[1,2,9]] {
            let conflict=admit_initializer_replay(&h,"map-1",1,digests).await;
            assert!(matches!(conflict,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("OTR01")));
        }
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_world_reset_epoch_history").fetch_one(&h.pool).await?;
        assert_eq!(count,1);
        let rewritten=sqlx::query("UPDATE game_world_reset_epoch_history SET map_revision='tampered' WHERE world_id=encode($1,'hex')::uuid")
            .bind(id(WORLD).as_slice()).execute(&h.pool).await;
        assert!(matches!(rewritten,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("23514")));
        let mut tx=h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
        let denied=sqlx::query("SELECT game_world_reset_initialize_epoch(encode($1,'hex')::uuid,'map-1',$2,$3,$4,$5,1)")
            .bind(id(WORLD).as_slice()).bind([1_u8;32].as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice())
            .bind(original.calendar.policy.as_str()).execute(&mut *tx).await;
        assert!(matches!(denied,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("42501")));
        tx.rollback().await?;
        h.cleanup().await
    })
}
async fn admit_initializer_replay(
    h: &Harness,
    map: &str,
    revision: u64,
    digests: [u8; 3],
) -> Result<(), sqlx::Error> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_control")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT game_world_reset_initialize_epoch(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7::text::numeric)")
        .bind(id(WORLD).as_slice()).bind(map).bind([digests[0];32].as_slice()).bind([digests[1];32].as_slice()).bind([digests[2];32].as_slice())
        .bind(crate::durability::world_reset::ResetCalendarPolicy::TibiaServerSaveBerlin10.as_str()).bind(revision.to_string()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

// PRIVATE PROPOSAL, NOT_RUN. Real native fixture; no drain/House proof is fabricated.
#[test]
fn reset_activation_refuses_legacy_target_without_any_issuance_or_epoch_write() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"resetlegacyactivation").await?;
        install(&h).await?;
        admit(&h).await?;
        let before: (i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_content_activations),(SELECT count(*) FROM game_world_reset_epoch_history)")
            .fetch_one(&h.pool).await?;
        let fact=h.node.fact();
        let mut tx=h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_world_reset").execute(&mut *tx).await?;
        let rejected=sqlx::query("SELECT game_world_reset_activate(encode($1,'hex')::uuid,0,1,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8)")
            .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice())
            .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
            .bind([0_u8;32].as_slice()).bind(Vec::<String>::new()).bind(Vec::<Vec<u8>>::new()).bind(Vec::<String>::new())
            .execute(&mut *tx).await;
        assert!(matches!(rejected,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("OTR03")));
        tx.rollback().await?;
        let after: (i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_content_activations),(SELECT count(*) FROM game_world_reset_epoch_history)")
            .fetch_one(&h.pool).await?;
        assert_eq!(before,after);
        let epoch: String=sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_reset_active_epochs WHERE world_id=encode($1,'hex')::uuid")
            .bind(id(WORLD).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(epoch,"0");
        let phase: String=sqlx::query_scalar("SELECT phase FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0")
            .bind(id(WORLD).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(phase,"RETIRING");
        h.cleanup().await
    })
}

// Direct SQL must take the native admission fence before inspecting a legacy job.
// A competing admission writer holds that fence; lock_timeout is the observed refusal,
// not a scheduling sleep or a manufactured drain/current-authority certificate.
#[test]
fn reset_direct_sql_waits_for_admission_fence_and_control_cannot_insert_activated() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"resetactivationfence").await?;
        install(&h).await?;
        admit(&h).await?;
        let before: (i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_content_activations),(SELECT count(*) FROM game_world_reset_epoch_history),(SELECT count(*) FROM game_world_reset_jobs)")
            .fetch_one(&h.pool).await?;
        let mut admission=h.pool.begin().await?;
        sqlx::query("LOCK TABLE game_durability_admission_account_guards IN EXCLUSIVE MODE")
            .execute(&mut *admission).await?;
        let mut contender=h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_world_reset").execute(&mut *contender).await?;
        sqlx::query("SET LOCAL lock_timeout='100ms'").execute(&mut *contender).await?;
        let fact=h.node.fact();
        let blocked=sqlx::query("SELECT game_world_reset_activate(encode($1,'hex')::uuid,0,1,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8)")
            .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice())
            .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
            .bind([0_u8;32].as_slice()).bind(Vec::<String>::new()).bind(Vec::<Vec<u8>>::new()).bind(Vec::<String>::new())
            .execute(&mut *contender).await;
        assert!(matches!(blocked,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("55P03")));
        contender.rollback().await?;
        // A missing node or revision is invalid data, not SQL UNKNOWN authority.
        // Refuse it before waiting on the held fence; the old missing guard timed out.
        let node_id = *fact.node_id().as_bytes();
        for missing_node in [true, false] {
            let mut invalid = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_world_reset").execute(&mut *invalid).await?;
            sqlx::query("SET LOCAL lock_timeout='100ms'").execute(&mut *invalid).await?;
            let refused = sqlx::query("SELECT game_world_reset_activate(encode($1,'hex')::uuid,0,$2::text::numeric,$3,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8,$9)")
                .bind(id(WORLD).as_slice())
                .bind(if missing_node { Some("1".to_owned()) } else { None })
                .bind([7_u8;32].as_slice())
                .bind(if missing_node { None } else { Some(node_id.as_slice()) })
                .bind(fact.registration_revision().to_string())
                .bind([0_u8;32].as_slice()).bind(Vec::<String>::new()).bind(Vec::<Vec<u8>>::new()).bind(Vec::<String>::new())
                .execute(&mut *invalid).await;
            assert!(matches!(refused,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("OTR03")));
            invalid.rollback().await?;
        }
        admission.rollback().await?;
        let mut control=h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_control").execute(&mut *control).await?;
        // Codex P1 4307187667: control holds no job INSERT at all.
        let denied=sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,active_map_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,'map-1','ACTIVATED')")
            .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice()).execute(&mut *control).await;
        assert!(matches!(denied,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("42501")));
        control.rollback().await?;
        // The BEFORE INSERT rule must refuse the phase before PK/current-epoch checks.
        let refused=sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,active_map_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,'map-1','ACTIVATED')")
            .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await;
        assert!(matches!(refused,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("23514")));
        let after: (i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_content_activations),(SELECT count(*) FROM game_world_reset_epoch_history),(SELECT count(*) FROM game_world_reset_jobs)")
            .fetch_one(&h.pool).await?;
        assert_eq!(before,after);
        h.cleanup().await
    })
}

// Codex P1 4300751311 and 4305018171 regression. A generic runtime or control login can
// derive the digest and stable keys from readable durable rows, so it is refused EXECUTE
// even with the exact evidence. The owner capability still rechecks: the SQL owner
// recomputes the drain cohort digest and requires House evidence. Each refusal changes
// exactly one input; only the owner capability with the exact evidence activates.
#[test]
fn reset_direct_activation_rechecks_drain_digest_and_house_evidence() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        use crate::durability::world_reset_port::{DrainedChannelBinding, world_drain_cohort_digest};
        let h=Harness::create(admin,"resetactivationevidence").await?;
        install(&h).await?;
        admit_qualified(&h).await?;
        replace(&h).await?;
        let house_key = "oteryn:content.house.thais_1";
        sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
            .bind(id(WORLD).as_slice()).bind(house_key).bind(id(crate::item_transfer_postgres_cases::CHARACTER).as_slice()).bind(id(90).as_slice())
            .execute(&h.pool).await?;
        let generation: String = sqlx::query_scalar("SELECT ownership_generation::text FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1 AND channel_id=encode($2,'hex')::uuid")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).fetch_one(&h.pool).await?;
        let fact = h.node.fact();
        let drained = |generation: u64| -> TestResult<[u8;32]> {
            world_drain_cohort_digest(world()?, fact, &[DrainedChannelBinding{scope: crate::foundation::RuntimeScopeRefV1::channel(world()?, channel()?), generation}])
                .map_err(|e| debug(e).into())
        };
        let generation = generation.parse::<u64>()?;
        let exact = drained(generation)?;
        let mut flipped = exact;
        flipped[31] ^= 1;
        let activation_state = || async {
            sqlx::query_as::<_,(i64,i64,String,String)>("SELECT (SELECT count(*) FROM game_content_activations),(SELECT count(*) FROM game_world_reset_epoch_history),(SELECT reset_epoch::text FROM game_world_reset_active_epochs WHERE world_id=encode($1,'hex')::uuid),(SELECT phase FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0)")
                .bind(id(WORLD).as_slice()).fetch_one(&h.pool).await
        };
        let before = activation_state().await?;
        assert_eq!((before.2.as_str(), before.3.as_str()), ("0", "RETIRING"));
        let stable = vec![house_key.to_owned()];
        for (role, set_role) in [
            ("oteryn_game_runtime", "SET LOCAL ROLE oteryn_game_runtime"),
            ("oteryn_game_control", "SET LOCAL ROLE oteryn_game_control"),
        ] {
            let mut tx = h.pool.begin().await?;
            sqlx::query(set_role).execute(&mut *tx).await?;
            let may_execute: bool = sqlx::query_scalar("SELECT has_function_privilege(current_user,'game_world_reset_activate(uuid,numeric,numeric,bytea,uuid,numeric,bytea,text[],bytea[],text[])','EXECUTE')")
                .fetch_one(&mut *tx).await?;
            assert!(!may_execute, "{role}");
            let denied = sqlx::query("SELECT game_world_reset_activate(encode($1,'hex')::uuid,0,1,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8)")
                .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice())
                .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
                .bind(exact.as_slice()).bind(Vec::<String>::new()).bind(Vec::<Vec<u8>>::new()).bind(stable.clone())
                .execute(&mut *tx).await;
            assert!(matches!(denied,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("42501")), "{role}");
            tx.rollback().await?;
            assert_eq!(activation_state().await?, before);
        }
        for (digest, stable_keys, activates) in [
            (flipped, stable.clone(), false),
            (drained(generation + 1)?, stable.clone(), false),
            (exact, Vec::new(), false),
            (exact, vec!["oteryn:content.house.other".to_owned()], false),
            (exact, stable, true),
        ] {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_world_reset").execute(&mut *tx).await?;
            let result = sqlx::query("SELECT game_world_reset_activate(encode($1,'hex')::uuid,0,1,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8)")
                .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice())
                .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
                .bind(digest.as_slice()).bind(Vec::<String>::new()).bind(Vec::<Vec<u8>>::new()).bind(stable_keys)
                .execute(&mut *tx).await;
            if activates {
                result?;
                sqlx::query("RESET ROLE").execute(&mut *tx).await?;
                let phase: String = sqlx::query_scalar("SELECT phase FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0")
                    .bind(id(WORLD).as_slice()).fetch_one(&mut *tx).await?;
                assert_eq!(phase, "ACTIVATED");
            } else {
                assert!(matches!(result,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("OTR03")));
            }
            tx.rollback().await?;
            assert_eq!(activation_state().await?, before);
        }
        h.cleanup().await
    })
}

/// The qualified target of [`admit_qualified`]: map-2 with server/client/frame 4/5/6.
fn qualified_target() -> TestResult<crate::durability::world_reset::WorldResetTarget> {
    use crate::durability::world_reset::{
        ResetCalendar, ResetCalendarPolicy, WorldResetArtifactDigests, WorldResetTarget,
    };
    Ok(WorldResetTarget {
        world: world()?,
        reset_epoch: 0,
        map_revision: "map-2".to_owned(),
        digests: WorldResetArtifactDigests {
            server: [4; 32],
            client: [5; 32],
            frame: [6; 32],
        },
        calendar: ResetCalendar {
            policy: ResetCalendarPolicy::TibiaServerSaveBerlin10,
            revision: 1,
        },
    })
}

/// [`admit`] with a qualified target, published by the only RETIRING publisher
/// (Codex P1 4307187667): its House preflight, the job and the Channel cohort commit together.
async fn admit_qualified(h: &Harness) -> TestResult {
    initialize_epoch(h).await?;
    record_predecessor_activation(h).await?;
    h.root
        .publish_world_reset(&qualified_target()?, [7; 32], &ResetRunHouses)
        .await
        .map_err(debug)?;
    Ok(())
}

/// The predecessor Channel content activation, equal to the epoch-0 history digests. A
/// RETIRING reset fences `game_content_record_activation` (Codex local P2), so it is recorded
/// before publication. Returns its activation sequence.
async fn record_predecessor_activation(h: &Harness) -> TestResult<u64> {
    record_channel_activation(h, [[1; 32], [2; 32], [3; 32]]).await
}

/// One ordinary content activation of the Channel issuing `digests` (server, client, frame).
async fn record_channel_activation(h: &Harness, digests: [[u8; 32]; 3]) -> TestResult<u64> {
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
    let previous: Option<String> = sqlx::query_scalar("SELECT max(activation_sequence)::text FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).fetch_one(&h.pool).await?;
    let previous = previous.map(|value| value.parse::<u64>()).transpose()?;
    let next = previous
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("sequence exhausted")?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5,$6,$7)")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice())
        .bind(next.to_string()).bind(previous.map(|value| value.to_string()))
        .bind(digests[0].as_slice()).bind(digests[1].as_slice()).bind(digests[2].as_slice())
        .execute(&h.pool).await?;
    Ok(next)
}

// HOUSE-RUNTIME-1a (#2163) real House custody: the reset census is Channel Ground only, so
// an owner-committed HouseInterior placement and its provenance survive a full Channel pass.
mod house_custody {
    use crate::durability::house_interior::{
        HouseCharacterSource, HouseItemAction, HouseItemOutcome, HouseItemRequest,
    };
    use crate::durability::house_scope_handoff::{
        HouseCommitOutcome, HouseEntryCommit, HouseEntryRequest, HouseHandoffDirection, HouseId,
        HousePrepareOutcome, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome,
        HouseScopeAssignmentRequest,
    };
    use crate::durability::item_transfer::{
        CurrentCharacterItemFence, ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome,
    };
    use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
    use crate::foundation::{CharacterId, CommandId, CommandRef, GameSessionId, WorldId};
    use crate::house_test_fixture;
    use crate::item_transfer_postgres_cases::{self as native, Harness, TestResult, debug, id};
    use sqlx::{Connection, PgConnection};

    pub(super) const HOUSE: &str = "oteryn:content.house.thais_1";

    fn house() -> TestResult<HouseId> {
        Ok(
            HouseId::new(WorldId::decode(&id(native::WORLD)).map_err(debug)?, HOUSE)
                .ok_or("house")?,
        )
    }
    fn house_fence() -> TestResult<CurrentCharacterItemFence> {
        Ok(CurrentCharacterItemFence {
            game_session_id: GameSessionId::decode(&id(61)).map_err(debug)?,
            character_lease_generation: 2,
            runtime_scope: house()?.runtime_scope()?,
            ..native::fence()?
        })
    }

    /// Native inventory funding through the item-transfer owner; returns (backpack, item).
    pub(super) async fn funded(h: &Harness) -> TestResult<([u8; 16], [u8; 16])> {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let backpack = h.mint(&a, native::BACKPACK, 1).await?;
        let to_slot = native::to_slot(native::command(1)?, backpack, native::backpack_facts());
        assert!(matches!(
            h.transfer(&a, native::fence()?, to_slot)
                .await
                .map_err(debug)?,
            ItemTransferOutcome::Committed(_)
        ));
        let item = h.mint(&a, "fixture:house.item", 1).await?;
        let facts = ItemDefinitionFacts {
            weight: Some(1),
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "fixture:house.item".into(),
                revision_ref: "definition-r1".into(),
            },
            stack: ItemStackClass::NonStackable,
            container_capacity: None,
            container_slot_equip_pattern: false,
        };
        let to_backpack = native::to_backpack(native::command(2)?, item, facts);
        assert!(matches!(
            h.transfer(&a, native::fence()?, to_backpack)
                .await
                .map_err(debug)?,
            ItemTransferOutcome::Committed(_)
        ));
        Ok((backpack, item))
    }

    /// Independent §10 House test DATA plus the native House assignment and entry handoff.
    pub(super) async fn seed(h: &Harness) -> TestResult {
        let mut c = PgConnection::connect(&h.database.url).await?;
        house_test_fixture::install(&mut c).await?;
        let world = id(native::WORLD);
        sqlx::query("INSERT INTO test_house_properties VALUES(encode($1,'hex')::uuid,$2,1,false)")
            .bind(world.as_slice())
            .bind(HOUSE)
            .execute(&mut c)
            .await?;
        sqlx::query("INSERT INTO test_house_grants VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,'OWNER',NULL)").bind(world.as_slice()).bind(HOUSE).bind(id(native::CHARACTER).as_slice()).execute(&mut c).await?;
        sqlx::query("INSERT INTO test_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3)")
            .bind(world.as_slice())
            .bind(HOUSE)
            .bind(vec![1u8, 2, 3])
            .execute(&mut c)
            .await?;
        sqlx::query("INSERT INTO game_durability_session_use_ledgers VALUES(encode($1,'hex')::uuid,1,true,1,1)")
            .bind(id(native::CHARACTER).as_slice())
            .execute(&mut c)
            .await?;
        sqlx::query("INSERT INTO game_durability_session_use_memberships VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL)").bind(id(native::SESSION).as_slice()).bind(id(native::CHARACTER).as_slice()).execute(&mut c).await?;
        sqlx::query("INSERT INTO game_control_house_scope_grants SELECT session_user,encode($1,'hex')::uuid,$2,op FROM unnest(ARRAY[1,2,3]::SMALLINT[]) op").bind(world.as_slice()).bind(HOUSE).execute(&mut c).await?;
        let assigned = h
            .root
            .assign_house_scope(HouseScopeAssignmentRequest {
                operation_key: OperationKey::from_bytes([8; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                actor_role: "oteryn_test_admin".into(),
                command: HouseScopeAssignmentCommand::Assign {
                    house: house()?,
                    target: h.node.fact(),
                },
            })
            .await
            .map_err(debug)?;
        assert!(matches!(
            assigned,
            HouseScopeAssignmentOutcome::Committed(_)
        ));
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let f = native::fence()?;
        let entry = HouseEntryRequest {
            direction: HouseHandoffDirection::Entry,
            handoff_id: id(60),
            character_id: CharacterId::decode(f.character_id.as_bytes()).map_err(debug)?,
            source_game_session_id: f.game_session_id,
            source_connection_generation: 1,
            source_character_lease_generation: 1,
            source_scope: f.runtime_scope,
            source_scope_ownership_generation: 1,
            house: house()?,
            house_scope_ownership_generation: 1,
            acl_revision: 1,
            guild_revisions: None,
            reserved_tile: vec![1, 2, 3],
            prepared_at: 100,
        };
        let prepared = h
            .root
            .prepare_house_entry(&a, &h.node, entry)
            .await
            .map_err(debug)?;
        assert!(matches!(prepared, HousePrepareOutcome::Prepared(_)));
        let commit = HouseEntryCommit {
            handoff_id: id(60),
            destination_game_session_id: house_fence()?.game_session_id,
            committed_at: 200,
        };
        let committed = h
            .root
            .commit_house_entry(&a, &h.node, commit)
            .await
            .map_err(debug)?;
        assert!(matches!(committed, HouseCommitOutcome::Committed(_)));
        Ok(())
    }

    /// Owner-committed Place of the inventory item onto the House tile.
    pub(super) async fn place(h: &Harness, backpack: [u8; 16], item: [u8; 16]) -> TestResult {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = HouseItemRequest {
            command: CommandRef::new(
                house_fence()?.game_session_id,
                CommandId::new(100).map_err(debug)?,
            ),
            house: house()?,
            item_instance_id: item,
            action: HouseItemAction::Place {
                source: HouseCharacterSource::Inventory {
                    parent: backpack,
                    ordinal: 1,
                },
                position: vec![1, 2, 3],
            },
            expected_acl_revision: 1,
            expected_guild_revisions: None,
            backpack: None,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        };
        let frozen = h
            .root
            .freeze_house_item(&a, &h.node, house_fence()?, request)
            .await
            .map_err(debug)?;
        let outcome = h
            .root
            .commit_house_item(&a, &h.node, house_fence()?, &frozen)
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, HouseItemOutcome::Committed(_)));
        Ok(())
    }
}

/// Exact HouseInterior custody snapshot: location, item state and every provenance row.
async fn house_snapshot(h: &Harness, item: [u8; 16]) -> TestResult<Vec<String>> {
    let mut rows: Vec<String> = sqlx::query_scalar(
        "SELECT row_to_json(l)::text FROM game_item_house_interior_locations l ORDER BY 1",
    )
    .fetch_all(&h.pool)
    .await?;
    for table in [
        "game_item_house_reclaim_provenance",
        "game_item_house_placement_transactions",
        "game_item_house_shape_proofs",
    ] {
        let sql = format!("SELECT row_to_json(t)::text FROM {table} t ORDER BY 1");
        rows.extend(
            sqlx::query_scalar::<_, String>(sqlx::AssertSqlSafe(sql))
                .fetch_all(&h.pool)
                .await?,
        );
    }
    rows.push(format!("{:?}", h.item_state(item).await?));
    Ok(rows)
}

#[test]
fn reset_native_channel_pass_leaves_real_house_interior_custody_untouched() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        // The accepted House fixture only installs into an `it_house_interior_*` database.
        let h = Harness::create(admin, "house_interior_reset").await?;
        install(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let ground = h.mint(&authority, "fixture:b3.stone", 2).await?;
        drop(authority);
        let (backpack, item) = house_custody::funded(&h).await?;
        house_custody::seed(&h).await?;
        house_custody::place(&h, backpack, item).await?;
        assert_eq!(h.count("game_item_house_interior_locations").await?, 1);
        let house_before = house_snapshot(&h, item).await?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        admit(&h).await?;
        replace(&h).await?;
        let permit = SpellLanePermit::of_fresh_lane(world()?, channel()?).await;
        // The House entry's destination session is still attached: no reset may start.
        assert!(matches!(
            h.root
                .freeze_next_world_reset_item(&permit, &authority, &h.node, world()?, channel()?)
                .await,
            Err(WorldResetError::NotCurrent)
        ));
        // Drain it like `replace` drains the Channel session, after all player writes.
        let drained = sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET session_state=3 \
              WHERE game_session_id=encode($1,'hex')::uuid AND session_state IN(1,2)",
        )
        .bind(id(61).as_slice())
        .execute(&h.pool)
        .await?
        .rows_affected();
        assert_eq!(drained, 1);
        let WorldResetPass::Retired(receipt) = pass(&h, &permit, &authority).await? else {
            return Err("Ground retirement required".into());
        };
        assert_eq!(receipt.item_instance_id, ground);
        assert_eq!(h.item_state(ground).await?, (0, 2));
        assert!(matches!(
            pass(&h, &permit, &authority).await?,
            WorldResetPass::ChannelEmpty
        ));
        assert_eq!(house_snapshot(&h, item).await?, house_before);
        assert_eq!(
            h.root
                .read_world_reset_retirement(&authority, item)
                .await
                .map_err(debug)?,
            None
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_decay_retire_receipts WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, 0);
        drop(authority);
        drop(seal);
        drop(permit);
        h.cleanup().await
    })
}

/// The qualified House catalogue pair of the [`admit_qualified`] job: one unchanged House
/// definition whose single cell is the real HouseInterior placement tile.
struct ResetRunHouses;
impl crate::durability::world_reset_port::WorldResetHouseSource for ResetRunHouses {
    fn reset_houses(
        &self,
        world: WorldId,
        side: crate::durability::world_reset_port::CatalogueSide,
    ) -> Result<
        crate::durability::world_reset_port::QualifiedHouseResetCatalogue,
        crate::durability::world_reset_port::WorldResetSourceRefused,
    > {
        VariantHouses {
            target_map: "map-2",
            target_definition: [9; 32],
        }
        .reset_houses(world, side)
    }
}

/// [`ResetRunHouses`] with one Target fact changed: a target map revision other than the
/// published one, or a changed definition of the owned House.
struct VariantHouses {
    target_map: &'static str,
    target_definition: [u8; 32],
}
impl crate::durability::world_reset_port::WorldResetHouseSource for VariantHouses {
    fn reset_houses(
        &self,
        world: WorldId,
        side: crate::durability::world_reset_port::CatalogueSide,
    ) -> Result<
        crate::durability::world_reset_port::QualifiedHouseResetCatalogue,
        crate::durability::world_reset_port::WorldResetSourceRefused,
    > {
        use crate::durability::world_reset_port::{
            CatalogueSide, HouseResetFact, QualifiedHouseResetCatalogue,
        };
        let (map_revision, server_artifact_digest, definition_digest) = match side {
            CatalogueSide::Active => ("map-1", [1_u8; 32], [9_u8; 32]),
            CatalogueSide::Target => (self.target_map, [4_u8; 32], self.target_definition),
        };
        Ok(QualifiedHouseResetCatalogue {
            world,
            map_revision: map_revision.to_owned(),
            server_artifact_digest,
            facts: vec![HouseResetFact {
                house_key: house_custody::HOUSE.to_owned(),
                definition_digest,
                spatial_position: vec![1, 2, 3],
            }],
        })
    }
}

/// D1451: one source-map placement of the active room, initialized through the real
/// native map owner. The same placement before and after a reset: only the epoch differs.
struct ResetSourceMap {
    binding: crate::durability::native_map_items_abi::NativeMapOwnerBinding,
    placements: Vec<crate::durability::native_map_items_abi::NativeMapItemPlacement>,
}
impl crate::durability::native_map_items_abi::map_initializer_seal::Sealed for ResetSourceMap {}
impl crate::durability::native_map_items_abi::NativeMapInitializationProof for ResetSourceMap {
    fn current_binding(
        &self,
    ) -> Result<crate::durability::native_map_items_abi::NativeMapOwnerBinding, &'static str> {
        Ok(self.binding.clone())
    }
    fn placements(&self) -> &[crate::durability::native_map_items_abi::NativeMapItemPlacement] {
        &self.placements
    }
}
fn reset_source_map(scope_generation: u64) -> TestResult<ResetSourceMap> {
    use crate::durability::item_mint::{GroundPlacement, TypedDefinitionRef};
    use crate::durability::native_map_items_abi::{NativeMapItemPlacement, NativeMapOwnerBinding};
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let (content, map, frame) = ([1_u8; 32], [2_u8; 32], [3_u8; 32]);
    Ok(ResetSourceMap {
        binding: NativeMapOwnerBinding {
            world: world()?,
            channel: channel()?,
            scope_generation,
            content_digest: content,
            map_digest: map,
            frame_digest: frame,
        },
        placements: vec![NativeMapItemPlacement {
            placement_key: "map:source-stone".to_owned(),
            definition: TypedDefinitionRef {
                family: "Item".to_owned(),
                production_key: "fixture:b3.stone".to_owned(),
                revision_ref: "definition-r1".to_owned(),
            },
            quantity: 1,
            ground: GroundPlacement {
                spatial_position: vec![0x51, 0, 0x52, 0, 0x53, 0, 7, 0, 0, 0],
                corpse_ref: b"native-source-map-init-r21".to_vec(),
                map_revision: format!("sha256:{}", hex(&map)),
                content_revision: format!("sha256:{}", hex(&content)),
                native_room_placement_context: frame.to_vec(),
            },
            source_binding: vec![0x5b; 32],
            source_attributes: serde_json::json!({}),
            blocks_movement: false,
            blocks_projectile: false,
            immovable_block_solid: false,
            owner_kind: None,
        }],
    })
}
async fn initialize_source_map(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    scope_generation: u64,
) -> TestResult<Vec<crate::durability::native_map_items_abi::MaterializedNativeMapItem>> {
    initialize_source_map_as(h, &h.root, authority, scope_generation).await
}
/// [`initialize_source_map`] through `root`, whose recovery `authority` it must be (the
/// node login's own Root after its boot reset run).
async fn initialize_source_map_as(
    h: &Harness,
    root: &crate::durability::DurabilityRoot,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    scope_generation: u64,
) -> TestResult<Vec<crate::durability::native_map_items_abi::MaterializedNativeMapItem>> {
    let permit = SpellLanePermit::of_fresh_lane(world()?, channel()?).await;
    root.initialize_native_map_current_owner(
        &permit,
        authority,
        &h.node,
        &reset_source_map(scope_generation)?,
    )
    .await
    .map_err(|error| debug(error).into())
}
/// `(reset_epoch, item)` of every source-map receipt, oldest epoch first.
async fn source_map_receipts(h: &Harness) -> TestResult<Vec<(String, Vec<u8>)>> {
    Ok(sqlx::query_as("SELECT reset_epoch::text,uuid_send(item_instance_id) FROM game_native_map_item_receipts ORDER BY reset_epoch")
        .fetch_all(&h.pool).await?)
}

struct ResetRuntimeReadiness(
    crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationChangeV1,
);
impl crate::foundation::fnd04_verifier::fresh_source_sealed::Sealed for ResetRuntimeReadiness {}
impl crate::foundation::admission_authority_publication::AdmissionAuthorityOwningPublisherV1
    for ResetRuntimeReadiness
{
    fn resolve_publication(
        &self,
        _now: i64,
    ) -> Result<
        Vec<crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationChangeV1>,
        crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationErrorV1,
    >{
        Ok(vec![self.0.clone()])
    }
}
/// The boot's readiness publication of `ownership_generation` after the reset: the successor
/// of the guard's current (not-ready) publication.
async fn publish_ready(h: &Harness, ownership_generation: u64) -> TestResult {
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
        AdmissionAuthorityPublicationChangeV1, AdmissionAuthorityPublicationV1,
        AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
        AdmissionPublicationSourceV1,
    };
    let (revision, source): (String, String) = sqlx::query_as("SELECT g.publication_revision::text,g.source_revision::text FROM game_durability_admission_runtime_guards g JOIN game_runtime_scope_assignments s USING(scope_key) WHERE s.world_id=encode($1,'hex')::uuid AND s.channel_id=encode($2,'hex')::uuid")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).fetch_one(&h.pool).await?;
    let revision: u64 = revision.parse()?;
    let source: u64 = source.parse()?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let change = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Runtime(scope()?),
        source: AdmissionPublicationSourceV1 {
            authority: "game-runtime-publisher".into(),
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision: source + 1,
            decision_identity: format!("world-reset-runtime-ready-{ownership_generation}"),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
        },
        precondition: AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: revision,
        },
        publication_revision: revision + 1,
        state: AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation,
            ready: true,
            route_revision: "route-1".into(),
            runtime_observation_revision: "runtime-1".into(),
            protocol_major: 1,
            transport_profile: 1,
            ruleset_revision: "ruleset-1".into(),
            content_revision: "content-1".into(),
            // The production readiness namespace: a bare bundle digest, never the content map
            // revision the epoch carries.
            map_revision: "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff".into(),
            world_policy_revision: "world-policy-1".into(),
            offer_revision: "offer-1".into(),
        },
    };
    let publication = AdmissionAuthorityPublicationV1::prepare(&ResetRuntimeReadiness(change), now)
        .map_err(debug)?;
    let disposition = h
        .root
        .publish_runtime_readiness(&h.node, &publication)
        .await
        .map_err(debug)?;
    if disposition
        != crate::durability::admission_authority_guards::GuardPublicationDisposition::Applied
    {
        return Err(format!("unexpected readiness outcome: {disposition:?}").into());
    }
    Ok(())
}

/// The boot witness shape: the durable cohort binding, held where no Channel runtime exists.
struct ResetRunDrain(crate::durability::world_reset_port::WorldDrainBinding);
impl crate::durability::world_reset_port::WorldDrainWitness for ResetRunDrain {
    fn drained(&self) -> &crate::durability::world_reset_port::WorldDrainBinding {
        &self.0
    }
}

/// MAP-OVERLAY-1c: the node boot path `run_world_reset` completes a RETIRING reset left by a
/// crash after one frozen candidate, a content entry of a Ground container: it resumes that
/// exact identity, retires every content before its container and every Ground item,
/// leaves the real HouseInterior custody untouched, activates epoch N+1 atomically within
/// `MAP01-RESET-RETIRE-MS`, and a second boot finds no RETIRING reset. The map reset epoch
/// boot hands its map consumers is 0 while the reset is RETIRING, then the activated record's
/// N+1 on this boot and on the next.
#[test]
fn reset_run_resumes_a_crashed_reset_retires_ground_keeps_houses_and_activates() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        use crate::durability::world_reset::WorldResetActivationDisposition;
        // The accepted House fixture only installs into an `it_house_interior_*` database.
        let h = Harness::create(admin, "house_interior_reset_run").await?;
        install(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // A Ground container holding two items; minted first, so its contents are the first
        // candidates of the run.
        let corpse = crate::corpse_transfer_postgres_cases::mint_corpse(
            &h,
            &authority,
            400,
            id(crate::item_transfer_postgres_cases::CHARACTER),
        )
        .await?;
        let first_content = crate::corpse_transfer_postgres_cases::put_loot(&h, corpse, 400, 1, 80).await?;
        let second_content = crate::corpse_transfer_postgres_cases::put_loot(&h, corpse, 400, 2, 81).await?;
        let stone = h.mint(&authority, "fixture:b3.stone", 2).await?;
        let coin = h.mint(&authority, "fixture:b3.coin", 3).await?;
        // D1451: one source-map Item, minted in epoch 0 by the real native map owner.
        let source = initialize_source_map(&h, &authority, 1).await?;
        assert!(!source[0].replayed);
        let source_item = source[0].item_instance_id;
        assert_eq!(source_map_receipts(&h).await?, vec![("0".to_owned(), source_item.to_vec())]);
        drop(authority);
        // A character container entry can never sit under a Ground container: its parent must
        // be a backpack slot item (FK), so the corpse family is the only Ground content.
        let refused = sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,9,game_character_uuid_v7())")
            .bind(coin.as_slice()).bind(id(WORLD).as_slice()).bind(id(crate::item_transfer_postgres_cases::CHARACTER).as_slice()).bind(corpse.as_slice())
            .execute(&h.pool).await;
        let code = match &refused { Err(sqlx::Error::Database(error)) => error.code().map(|code| code.into_owned()), _ => None };
        assert_eq!(code.as_deref(), Some("23503"), "{refused:?}");
        let (backpack, item) = house_custody::funded(&h).await?;
        house_custody::seed(&h).await?;
        house_custody::place(&h, backpack, item).await?;
        let house_before = house_snapshot(&h, item).await?;
        admit_qualified(&h).await?;
        // Admission is closed but the operator has not yet replaced the generation: the boot
        // run waits (NotCurrent) and changes nothing.
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = footprint(&h).await?;
        let early = match h.root.read_world_drain_binding(&h.node, world()?).await {
            Ok(binding) => {
                h.root
                    .run_world_reset(&authority, &h.node, world()?, &ResetRunHouses, &ResetRunDrain(binding))
                    .await
            }
            Err(error) => Err(error),
        };
        assert!(matches!(early, Err(WorldResetError::NotCurrent)), "{early:?}");
        assert_eq!(footprint(&h).await?, before);
        replace(&h).await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=encode($1,'hex')::uuid AND session_state IN(1,2)")
            .bind(id(61).as_slice()).execute(&h.pool).await?;
        // A crash after the first frozen candidate: its lane and process are gone.
        let frozen = {
            let permit = SpellLanePermit::of_fresh_lane(world()?, channel()?).await;
            h.root
                .freeze_next_world_reset_item(&permit, &authority, &h.node, world()?, channel()?)
                .await
                .map_err(debug)?
                .ok_or("frozen candidate")?
        };
        assert_eq!(frozen.identity().item_instance_id, first_content);
        drop(authority);
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(h.root.read_retiring_world_reset(world()?).await.map_err(debug)?, Some(0));
        assert_eq!(h.root.read_map_reset_epoch(world()?).await.map_err(debug)?, 0);
        let binding = h
            .root
            .read_world_drain_binding(&h.node, world()?)
            .await
            .map_err(debug)?;
        let run = h
            .root
            .run_world_reset(&authority, &h.node, world()?, &ResetRunHouses, &ResetRunDrain(binding))
            .await
            .map_err(debug)?
            .ok_or("RETIRING reset")?;
        eprintln!(
            "MAP01-RESET-RETIRE-MS fixture run: retired={} elapsed_ms={}",
            run.retired,
            run.elapsed.as_millis()
        );
        let WorldResetActivationDisposition::NewEpoch(activated) = &run.disposition else {
            return Err("NewEpoch disposition".into());
        };
        assert_eq!((run.reset_epoch, activated.epoch, run.retired), (0, 1, 6));
        // Boot hands the map consumers the activated epoch N+1, not epoch 0.
        assert_eq!(
            h.root.read_map_reset_epoch(world()?).await.map_err(debug)?,
            activated.epoch
        );
        assert!(run.within_budget());
        for retired in [first_content, second_content, corpse, stone, coin, source_item] {
            assert_eq!(h.item_state(retired).await?, (0, 2));
        }
        assert_eq!(h.count("game_item_corpse_container_entries").await?, 0);
        // Each content's receipt names its container and ordinal; the container's names itself.
        let receipts: Vec<(Vec<u8>, Option<String>)> = sqlx::query_as("SELECT uuid_send(reset_parent_item),placement_ordinal::text FROM game_item_decay_retire_receipts WHERE cause_kind='WORLD_RESET' AND item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid) ORDER BY placement_ordinal NULLS LAST")
            .bind(first_content.as_slice()).bind(second_content.as_slice()).bind(corpse.as_slice())
            .fetch_all(&h.pool).await?;
        let container = corpse.to_vec();
        assert_eq!(
            receipts,
            vec![
                (container.clone(), Some("1".to_owned())),
                (container.clone(), Some("2".to_owned())),
                (container, None),
            ]
        );
        assert_eq!(
            h.root
                .read_world_reset_retirement(&authority, frozen.identity().item_instance_id)
                .await
                .map_err(debug)?,
            Some(frozen.identity().clone())
        );
        assert_eq!(h.count("game_item_ground_locations").await?, 0);
        assert_eq!(house_snapshot(&h, item).await?, house_before);
        let state: (String, String) = sqlx::query_as("SELECT (SELECT phase FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0),(SELECT reset_epoch::text FROM game_world_reset_active_epochs WHERE world_id=encode($1,'hex')::uuid)")
            .bind(id(WORLD).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!((state.0.as_str(), state.1.as_str()), ("ACTIVATED", "1"));
        // D1451: the reset is complete only when the source Items it retired come back. The
        // activated epoch's first initialization mints the same placement once, under N+1;
        // the retired epoch-0 receipt stays immutable history.
        publish_ready(&h, 2).await?;
        let restored = initialize_source_map(&h, &authority, 2).await?;
        assert_eq!(restored.len(), 1);
        assert!(!restored[0].replayed);
        let restored_item = restored[0].item_instance_id;
        assert_ne!(restored_item, source_item);
        assert_ne!(restored[0].transaction_id, source[0].transaction_id);
        assert_eq!(
            source_map_receipts(&h).await?,
            vec![
                ("0".to_owned(), source_item.to_vec()),
                ("1".to_owned(), restored_item.to_vec()),
            ]
        );
        assert_eq!(h.item_state(restored_item).await?, (1, 1));
        assert!(h.on_ground(restored_item).await?);
        assert_eq!(h.item_state(source_item).await?, (0, 2));
        assert_eq!(h.count("game_item_ground_locations").await?, 1);
        // The next boot finds no RETIRING reset and changes nothing.
        let after = footprint(&h).await?;
        assert_eq!(h.root.read_retiring_world_reset(world()?).await.map_err(debug)?, None);
        let binding = h
            .root
            .read_world_drain_binding(&h.node, world()?)
            .await
            .map_err(debug)?;
        assert!(h
            .root
            .run_world_reset(&authority, &h.node, world()?, &ResetRunHouses, &ResetRunDrain(binding))
            .await
            .map_err(debug)?
            .is_none());
        assert_eq!(footprint(&h).await?, after);
        // The next boot's map consumers stay on N+1.
        assert_eq!(h.root.read_map_reset_epoch(world()?).await.map_err(debug)?, 1);
        // Its source initialization replays epoch 1's receipt: no second Item.
        let replayed = initialize_source_map(&h, &authority, 2).await?;
        assert_eq!(replayed.len(), 1);
        assert!(replayed[0].replayed);
        assert_eq!(
            (replayed[0].transaction_id, replayed[0].item_instance_id),
            (restored[0].transaction_id, restored_item)
        );
        assert_eq!(source_map_receipts(&h).await?.len(), 2);
        assert_eq!(h.count("game_item_ground_locations").await?, 1);
        assert_eq!(footprint(&h).await?, after);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

/// One `oteryn-game-ops` argument vector after `--config`, split into command words and
/// `--name value` options as the ops tool splits it, then parsed and run by the same
/// `world-reset` dispatch. The fixture bundles stand in for the qualified rooms: the active
/// one is the epoch-0 history's (map-1, 1/2/3), the target the qualified one (map-2, 4/5/6).
async fn operator(
    root: &crate::durability::DurabilityRoot,
    argv: &[&str],
) -> Result<String, crate::durability::world_reset_command::WorldResetCommandFailure> {
    use crate::durability::world_reset::WorldResetArtifactDigests;
    use crate::durability::world_reset_command::{WorldResetCommand, execute_world_reset_command};
    let mut words = Vec::new();
    let mut options = std::collections::BTreeMap::new();
    let mut argv = argv.iter();
    while let Some(argument) = argv.next() {
        match argument.strip_prefix("--") {
            Some(name) => {
                let value = argv
                    .next()
                    .map_or_else(String::new, |value| (*value).to_owned());
                assert!(options.insert(name.to_owned(), value).is_none());
            }
            None => words.push((*argument).to_owned()),
        }
    }
    let command = WorldResetCommand::parse(&words, &mut options)?;
    execute_world_reset_command(
        root,
        &command,
        WorldResetArtifactDigests {
            server: [1; 32],
            client: [2; 32],
            frame: [3; 32],
        },
        WorldResetArtifactDigests {
            server: [4; 32],
            client: [5; 32],
            frame: [6; 32],
        },
        &ResetRunHouses,
    )
    .await
}

/// Codex P1 4307633900: the SQLSTATEs of a new Game session and a new reconnect attempt of
/// the World. A committed RETIRING job refuses both before any column or key check (OTR03);
/// without one the same inserts reach the NOT NULL checks (23502).
async fn admission_codes(h: &Harness) -> TestResult<[Option<String>; 2]> {
    let mut codes = [None, None];
    let statements = [
        "INSERT INTO game_durability_reconnect_sessions(world_id) VALUES(encode($1,'hex')::uuid)",
        "INSERT INTO game_durability_reconnect_attempts(world_id) VALUES(encode($1,'hex')::uuid)",
    ];
    for (code, statement) in codes.iter_mut().zip(statements) {
        let inserted = sqlx::query(statement)
            .bind(id(WORLD).as_slice())
            .execute(&h.pool)
            .await;
        *code = match inserted {
            Err(sqlx::Error::Database(error)) => error.code().map(|code| code.into_owned()),
            other => Some(format!("{other:?}")),
        };
    }
    Ok(codes)
}
fn admission(code: &str) -> [Option<String>; 2] {
    [Some(code.to_owned()), Some(code.to_owned())]
}

/// Codex P1 4307818778: the SQLSTATE of completing a reconnect prepared before publication.
/// In one rolled-back TX every nonterminal session of the World closes (the drain's own
/// transition, open while RETIRING), the harness session is prepared for reconnect (state 1,
/// no transport, a prepared attempt) and then completed by the exact `complete_reconnect`
/// statement. A committed RETIRING job refuses that completion (OTR03).
async fn readmission_code(h: &Harness) -> TestResult<String> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE world_id=encode($1,'hex')::uuid AND session_state IN(1,2)")
        .bind(id(WORLD).as_slice())
        .execute(&mut *tx)
        .await?;
    let prepared = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1,current_generation=1,current_transport_ref=NULL,prepared_attempt_ref=$2 WHERE game_session_id=encode($1,'hex')::uuid AND control_loss_epoch IS NOT NULL")
        .bind(id(SESSION).as_slice())
        .bind([3_u8; 8].as_slice())
        .execute(&mut *tx)
        .await?;
    assert_eq!(prepared.rows_affected(), 1);
    let completed = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 2, current_generation=$2::text::numeric, current_transport_ref=$3, prepared_attempt_ref=NULL WHERE game_session_id=encode($1,'hex')::uuid AND session_state = 1 AND current_transport_ref IS NULL AND prepared_attempt_ref = $4 AND current_generation=$5::text::numeric")
        .bind(id(SESSION).as_slice())
        .bind("2")
        .bind([8_u8; 16].as_slice())
        .bind([3_u8; 8].as_slice())
        .bind("1")
        .execute(&mut *tx)
        .await;
    let code = match completed {
        Ok(done) => {
            assert_eq!(done.rows_affected(), 1);
            "ok".to_owned()
        }
        Err(sqlx::Error::Database(error)) => error
            .code()
            .map_or_else(|| "none".to_owned(), |code| code.into_owned()),
        Err(other) => return Err(debug(other).into()),
    };
    tx.rollback().await?;
    Ok(code)
}

/// One direct SQL owner call with House evidence for the placed House: `activate` selects
/// the activation owner, else the publisher. Returns the SQLSTATE, `None` on success.
async fn direct_owner_call(conn: &mut sqlx::PgConnection, activate: bool) -> Option<String> {
    let houses = vec![house_custody::HOUSE.to_owned()];
    let called = if activate {
        sqlx::query("SELECT game_world_reset_activate(encode($1,'hex')::uuid,0,1,$2,encode($1,'hex')::uuid,1,$2,$3,$4,$5)")
            .bind(id(WORLD).as_slice())
            .bind([7_u8; 32].as_slice())
            .bind(houses.clone())
            .bind(vec![vec![1_u8, 2, 3]])
            .bind(houses)
            .execute(&mut *conn)
            .await
    } else {
        sqlx::query("SELECT game_world_reset_publish(encode($1,'hex')::uuid,0,$2,'map-2',$3,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_10',1,$6,$7,$8)")
            .bind(id(WORLD).as_slice())
            .bind([7_u8; 32].as_slice())
            .bind([4_u8; 32].as_slice())
            .bind([5_u8; 32].as_slice())
            .bind([6_u8; 32].as_slice())
            .bind(houses.clone())
            .bind(vec![vec![1_u8, 2, 3]])
            .bind(houses)
            .execute(&mut *conn)
            .await
    };
    match called {
        Ok(_) => None,
        Err(sqlx::Error::Database(error)) => error.code().map(|code| code.into_owned()),
        Err(other) => Some(format!("{other:?}")),
    }
}

/// A real LOGIN role that is a member of `roles` (as deployment creates one per capability),
/// with its connection URL. A superuser `SET ROLE` is no membership and keeps the superuser
/// session, so only a login exercises the holder walk and the login's own privileges.
struct Login {
    role: String,
    url: String,
}

async fn create_login(h: &Harness, tag: &str, roles: &str) -> TestResult<Login> {
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&h.pool)
        .await?;
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let role = format!("world_reset_{tag}_{suffix}");
    let password = format!("{role}-secret");
    for statement in [
        format!("CREATE ROLE {role} LOGIN PASSWORD '{password}' IN ROLE {roles}"),
        format!("GRANT CONNECT ON DATABASE {database} TO {role}"),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .execute(&h.pool)
            .await?;
    }
    let (_, address) = h
        .database
        .url
        .split_once('@')
        .ok_or("database URL has no authority separator")?;
    let url = format!("postgresql://{role}:{password}@{address}");
    Ok(Login { role, url })
}

/// A ready Root connected as `login` only, as the ops tool or the node connects its login.
async fn login_root(login: &Login) -> TestResult<crate::durability::DurabilityRoot> {
    let root =
        crate::durability::DurabilityRoot::connect_test_runtime(&login.url).map_err(debug)?;
    assert!(root.maintain_ready_once().await.map_err(debug)?);
    Ok(root)
}

async fn drop_login(h: &Harness, login: Login) -> TestResult {
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP OWNED BY {}", login.role)))
        .execute(&h.pool)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP ROLE {}", login.role)))
        .execute(&h.pool)
        .await?;
    Ok(())
}

/// Codex P1 4307818772: the (activate, publish) SQLSTATEs of a real login that is a member of
/// `roles`.
async fn login_codes(
    h: &Harness,
    tag: &str,
    roles: &str,
) -> TestResult<(Option<String>, Option<String>)> {
    use sqlx::Connection;
    let login = create_login(h, tag, roles).await?;
    let outcome = async {
        let mut conn = sqlx::PgConnection::connect(&login.url).await?;
        let mut tx = conn.begin().await?;
        let activate = direct_owner_call(&mut tx, true).await;
        tx.rollback().await?;
        let mut tx = conn.begin().await?;
        let publish = direct_owner_call(&mut tx, false).await;
        tx.rollback().await?;
        conn.close().await?;
        TestResult::Ok((activate, publish))
    }
    .await;
    drop_login(h, login).await?;
    outcome
}

/// MAP-OVERLAY-1c D1451 operator path, end to end through the `world-reset` dispatch of
/// `oteryn-game-ops`: `initialize` records epoch 0, `publish` closes admission with a
/// RETIRING job of the qualified target (an identical re-run replays it, another cause is
/// refused) and also closes reconnect completion, the node's boot run retires the epoch-0
/// source Item and activates N+1, and the source map comes back once under epoch 1. A publication of the retired epoch is not current.
#[test]
fn reset_operator_command_publishes_retiring_and_the_boot_run_remints_under_n_plus_one()
-> TestResult {
    use crate::durability::world_reset_command::{WorldResetCommandFailure, retirement_binding};
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        use crate::durability::world_reset::WorldResetActivationDisposition;
        let h = Harness::create(admin, "house_interior_reset_ops").await?;
        install(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let source = initialize_source_map(&h, &authority, 1).await?;
        let source_item = source[0].item_instance_id;
        assert_eq!(source_map_receipts(&h).await?, vec![("0".to_owned(), source_item.to_vec())]);
        drop(authority);
        let hex: String = id(WORLD).iter().map(|byte| format!("{byte:02x}")).collect();
        let world_text = format!(
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        );
        let publish = |revision: &'static str| {
            [
                "world-reset",
                "publish",
                "--world",
                world_text.as_str(),
                "--epoch",
                "0",
                "--calendar-revision",
                revision,
            ]
        };
        let jobs = || async {
            let jobs: Vec<(String, String, Vec<u8>)> = sqlx::query_as("SELECT reset_epoch::text,phase,target_digest FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid ORDER BY reset_epoch")
                .bind(id(WORLD).as_slice()).fetch_all(&h.pool).await?;
            Ok::<_, Box<dyn std::error::Error>>(jobs)
        };
        // Codex P1/P2: `publish` runs as a publisher-only login, exactly as `[world_reset]`
        // connects it, so its fences and reads must hold for that login's own privileges.
        let publisher = create_login(&h, "ops", "oteryn_game_world_reset_publisher").await?;
        let publisher_root = login_root(&publisher).await?;
        // Before Control initializes the World there is no active epoch to publish from.
        let early = operator(&publisher_root, &publish("1")).await;
        assert!(matches!(early, Err(WorldResetCommandFailure::Rejected(_))), "{early:?}");
        assert!(jobs().await?.is_empty());
        assert_eq!(admission_codes(&h).await?, admission("23502"));
        let initialize = [
            "world-reset",
            "initialize",
            "--world",
            world_text.as_str(),
            "--calendar-revision",
            "1",
        ];
        let line = operator(&h.root, &initialize).await.map_err(debug)?;
        assert!(line.starts_with("world_reset=initialized reset_epoch=0 map_revision=map-1 "), "{line}");
        // The initializer is exact: the identical re-run replays it.
        assert_eq!(operator(&h.root, &initialize).await.map_err(debug)?, line);
        assert_eq!(h.root.read_retiring_world_reset(world()?).await.map_err(debug)?, None);
        // The predecessor Channel activation, recorded while no reset is RETIRING.
        let next = record_predecessor_activation(&h).await?;
        // A prepared reconnect still completes while no job is published.
        assert_eq!(readmission_code(&h).await?, "ok");
        // Publication: the job of exactly the qualified target and its binding; admission closes.
        let line = operator(&publisher_root, &publish("1")).await.map_err(debug)?;
        let binding = retirement_binding(&qualified_target()?);
        let binding_hex: String = binding.iter().map(|byte| format!("{byte:02x}")).collect();
        assert!(line.starts_with("world_reset=published phase=RETIRING reset_epoch=0 map_revision=map-2 "), "{line}");
        assert!(line.ends_with(&format!("retirement_binding={binding_hex} replayed=false")), "{line}");
        assert_eq!(jobs().await?, vec![("0".to_owned(), "RETIRING".to_owned(), binding.to_vec())]);
        assert_eq!(h.root.read_retiring_world_reset(world()?).await.map_err(debug)?, Some(0));
        assert_eq!(admission_codes(&h).await?, admission("OTR03"));
        // Codex local P2: no ordinary content activation while the reset is RETIRING.
        let refused = sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5,$6,$7)")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice())
            .bind((next + 1).to_string()).bind(next.to_string())
            .bind([4_u8; 32].as_slice()).bind([5_u8; 32].as_slice()).bind([6_u8; 32].as_slice())
            .execute(&h.pool).await;
        assert!(matches!(&refused, Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("OTR03")), "{refused:?}");
        // Codex P1 4307818778: a reconnect prepared before publication cannot complete now.
        assert_eq!(readmission_code(&h).await?, "OTR03");
        let mut tx = h.pool.begin().await?;
        let job = crate::durability::world_reset::read_locked_world_reset_target(&mut tx, world()?)
            .await
            .map_err(debug)?;
        assert_eq!(job, qualified_target()?);
        tx.rollback().await?;
        // The identical re-run (an unknown outcome's retry) replays; another cause is refused.
        let replayed = operator(&publisher_root, &publish("1")).await.map_err(debug)?;
        assert_eq!(replayed, line.replace("replayed=false", "replayed=true"));
        let other = operator(&publisher_root, &publish("2")).await;
        assert!(matches!(other, Err(WorldResetCommandFailure::Rejected(_))), "{other:?}");
        assert_eq!(jobs().await?, vec![("0".to_owned(), "RETIRING".to_owned(), binding.to_vec())]);
        // The node's boot run completes the published reset (as `recover_world_reset`).
        replace(&h).await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=encode($1,'hex')::uuid AND session_state IN(1,2)")
            .bind(id(61).as_slice()).execute(&h.pool).await?;
        // The boot run as the node's own login: runtime plus the activation capability only.
        let node_login =
            create_login(&h, "node", "oteryn_game_runtime,oteryn_game_world_reset").await?;
        let node_root = login_root(&node_login).await?;
        let authority = node_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let binding = node_root
            .read_world_drain_binding(&h.node, world()?)
            .await
            .map_err(debug)?;
        let run = node_root
            .run_world_reset(&authority, &h.node, world()?, &ResetRunHouses, &ResetRunDrain(binding))
            .await
            .map_err(debug)?
            .ok_or("published RETIRING reset")?;
        let WorldResetActivationDisposition::NewEpoch(activated) = &run.disposition else {
            return Err("NewEpoch disposition".into());
        };
        assert_eq!((run.reset_epoch, activated.epoch, run.retired), (0, 1, 1));
        assert_eq!(activated.map_revision, "map-2");
        assert_eq!(h.item_state(source_item).await?, (0, 2));
        assert_eq!(h.root.read_map_reset_epoch(world()?).await.map_err(debug)?, 1);
        assert_eq!(h.root.read_retiring_world_reset(world()?).await.map_err(debug)?, None);
        // Activation reopens admission, reconnect completion included.
        assert_eq!(admission_codes(&h).await?, admission("23502"));
        assert_eq!(readmission_code(&h).await?, "ok");
        // N+1: the source map comes back once, as a new Item under epoch 1.
        // The node remints it with its own login's Root, whose recovery authority it holds.
        publish_ready(&h, 2).await?;
        let restored = initialize_source_map_as(&h, &node_root, &authority, 2).await?;
        assert_eq!(restored.len(), 1);
        assert!(!restored[0].replayed);
        let restored_item = restored[0].item_instance_id;
        assert_ne!(restored_item, source_item);
        assert_eq!(
            source_map_receipts(&h).await?,
            vec![
                ("0".to_owned(), source_item.to_vec()),
                ("1".to_owned(), restored_item.to_vec()),
            ]
        );
        assert_eq!(h.item_state(restored_item).await?, (1, 1));
        assert!(h.on_ground(restored_item).await?);
        // The retired epoch can no longer be published.
        let stale = operator(&publisher_root, &publish("1")).await;
        assert!(matches!(stale, Err(WorldResetCommandFailure::Rejected(_))), "{stale:?}");
        assert_eq!(jobs().await?.len(), 1);
        drop(authority);
        drop(seal);
        drop(node_root);
        drop(publisher_root);
        drop_login(&h, node_login).await?;
        drop_login(&h, publisher).await?;
        h.cleanup().await
    })
}

/// Codex P1 4307187667: a RETIRING job closes admission, so it is published only through the
/// reset owner, after the House preflight of the proposed target and in the same TX. Runtime
/// and control hold no publication path; an incompatible House catalogue, a direct owner call
/// without House evidence or a non-active epoch creates no job, closes no admission and
/// leaves the boot reset run nothing to resume.
#[test]
fn reset_publication_runs_the_house_preflight_and_closes_admission_atomically() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        // The accepted House fixture only installs into an `it_house_interior_*` database.
        let h = Harness::create(admin, "house_interior_reset_publish").await?;
        install(&h).await?;
        initialize_epoch(&h).await?;
        let (backpack, item) = house_custody::funded(&h).await?;
        house_custody::seed(&h).await?;
        house_custody::place(&h, backpack, item).await?;
        // The placed House is owned, so its Active and Target definitions must agree.
        sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
            .bind(id(WORLD).as_slice()).bind(house_custody::HOUSE).bind(id(crate::item_transfer_postgres_cases::CHARACTER).as_slice()).bind(id(90).as_slice())
            .execute(&h.pool).await?;
        let house_before = house_snapshot(&h, item).await?;
        let published = || async {
            let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_reset_jobs")
                .fetch_one(&h.pool)
                .await?;
            let scopes: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_reset_scopes")
                .fetch_one(&h.pool)
                .await?;
            let retiring = h.root.read_retiring_world_reset(world()?).await.map_err(debug)?;
            Ok::<_, Box<dyn std::error::Error>>((jobs, scopes, retiring))
        };
        let target = qualified_target()?;
        // Runtime and control hold no EXECUTE on the publisher, even with exact evidence.
        for role in ["SET LOCAL ROLE oteryn_game_runtime", "SET LOCAL ROLE oteryn_game_control"] {
            let mut tx = h.pool.begin().await?;
            sqlx::query(role).execute(&mut *tx).await?;
            let denied = sqlx::query("SELECT game_world_reset_publish(encode($1,'hex')::uuid,0,$2,'map-2',$3,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_10',1,$6,$7,$8)")
                .bind(id(WORLD).as_slice()).bind([7_u8; 32].as_slice())
                .bind([4_u8; 32].as_slice()).bind([5_u8; 32].as_slice()).bind([6_u8; 32].as_slice())
                .bind(vec![house_custody::HOUSE.to_owned()]).bind(vec![vec![1_u8, 2, 3]]).bind(vec![house_custody::HOUSE.to_owned()])
                .execute(&mut *tx).await;
            assert!(matches!(denied, Err(sqlx::Error::Database(ref e)) if e.code().as_deref() == Some("42501")), "{role}: {denied:?}");
            tx.rollback().await?;
        }
        assert_eq!(published().await?, (0, 0, None));
        // A Target catalogue bound to another map revision, or one that changes the owned
        // House's definition, refuses before any job exists.
        let wrong_map = h
            .root
            .publish_world_reset(&target, [7; 32], &VariantHouses { target_map: "map-3", target_definition: [9; 32] })
            .await;
        assert!(matches!(wrong_map, Err(WorldResetError::HouseSourceRefused)), "{wrong_map:?}");
        let changed = h
            .root
            .publish_world_reset(&target, [7; 32], &VariantHouses { target_map: "map-2", target_definition: [8; 32] })
            .await;
        assert!(
            matches!(changed, Err(WorldResetError::HouseCatalogueChanged { ref house_key }) if house_key == house_custody::HOUSE),
            "{changed:?}"
        );
        // A target of a non-active epoch is not current.
        let stale = h
            .root
            .publish_world_reset(&crate::durability::world_reset::WorldResetTarget { reset_epoch: 1, ..target.clone() }, [7; 32], &ResetRunHouses)
            .await;
        assert!(matches!(stale, Err(WorldResetError::NotCurrent)), "{stale:?}");
        assert_eq!(published().await?, (0, 0, None));
        // The SQL owner rechecks: the publisher capability without House evidence for the owned
        // House and its interior cell is refused, with no job.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_world_reset_publisher").execute(&mut *tx).await?;
        let bare = sqlx::query("SELECT game_world_reset_publish(encode($1,'hex')::uuid,0,$2,'map-2',$3,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_10',1,$6,$7,$8)")
            .bind(id(WORLD).as_slice()).bind([7_u8; 32].as_slice())
            .bind([4_u8; 32].as_slice()).bind([5_u8; 32].as_slice()).bind([6_u8; 32].as_slice())
            .bind(Vec::<String>::new()).bind(Vec::<Vec<u8>>::new()).bind(Vec::<String>::new())
            .execute(&mut *tx).await;
        assert!(matches!(bare, Err(sqlx::Error::Database(ref e)) if e.code().as_deref() == Some("OTR03")), "{bare:?}");
        tx.rollback().await?;
        assert_eq!(published().await?, (0, 0, None));
        // Codex local P2: a Channel issuing no content, or content other than the active
        // epoch's, refuses publication with no job; the predecessor issuance then admits it.
        let unissued = h.root.publish_world_reset(&target, [7; 32], &ResetRunHouses).await;
        assert!(matches!(unissued, Err(WorldResetError::Unavailable(_))), "{unissued:?}");
        assert_eq!(published().await?, (0, 0, None));
        record_channel_activation(&h, [[4; 32], [5; 32], [6; 32]]).await?;
        let other = h.root.publish_world_reset(&target, [7; 32], &ResetRunHouses).await;
        assert!(matches!(other, Err(WorldResetError::Unavailable(_))), "{other:?}");
        assert_eq!(published().await?, (0, 0, None));
        record_predecessor_activation(&h).await?;
        // Codex P1 4307818772: each capability reaches only its own owner, and a login holding
        // both is refused by both owners; a single-capability login passes its own check.
        for (role, activate) in [
            ("oteryn_game_world_reset", false),
            ("oteryn_game_world_reset_publisher", true),
        ] {
            let mut tx = h.pool.begin().await?;
            sqlx::query(sqlx::AssertSqlSafe(format!("SET LOCAL ROLE {role}")))
                .execute(&mut *tx)
                .await?;
            let code = direct_owner_call(&mut tx, activate).await;
            assert_eq!(code.as_deref(), Some("42501"), "{role}");
            tx.rollback().await?;
        }
        let both = login_codes(
            &h,
            "both",
            "oteryn_game_world_reset, oteryn_game_world_reset_publisher",
        )
        .await?;
        assert_eq!(
            (both.0.as_deref(), both.1.as_deref()),
            (Some("42501"), Some("42501"))
        );
        // The publisher-only login passes every check and publishes (its TX rolls back).
        let publisher = login_codes(&h, "publisher", "oteryn_game_world_reset_publisher").await?;
        assert_eq!(
            (publisher.0.as_deref(), publisher.1.as_deref()),
            (Some("42501"), None)
        );
        let activator = login_codes(&h, "activator", "oteryn_game_world_reset").await?;
        assert!(
            matches!(activator.0.as_deref(), Some(code) if code != "42501"),
            "{activator:?}"
        );
        assert_eq!(activator.1.as_deref(), Some("42501"));
        assert_eq!(published().await?, (0, 0, None));
        assert_eq!(house_snapshot(&h, item).await?, house_before);
        // Every refusal above left admission open.
        assert_eq!(admission_codes(&h).await?, admission("23502"));
        // The checked publisher commits the job and its Channel cohort together, and that
        // committed job closes admission to the World (Codex P1 4307633900).
        h.root
            .publish_world_reset(&target, [7; 32], &ResetRunHouses)
            .await
            .map_err(debug)?;
        let channels: i64 = sqlx::query_scalar("SELECT count(*) FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1")
            .bind(id(WORLD).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(channels, 1);
        assert_eq!(published().await?, (1, channels, Some(0)));
        assert_eq!(admission_codes(&h).await?, admission("OTR03"));
        let cohort: Vec<(Vec<u8>, String)> = sqlx::query_as("SELECT uuid_send(channel_id),predecessor_generation::text FROM game_world_reset_scopes WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0")
            .bind(id(WORLD).as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(cohort, vec![(id(CHANNEL).to_vec(), "1".to_owned())]);
        // Codex local P2: the committed job closes the Channel cohort, so a new Channel of the
        // World is refused (OTR03); the boot run's remint of an existing scope stays open.
        // The session holds the control claim grant for the new Channel, so the control effect
        // guard admits the insert and the refusal is the reset cohort trigger's own.
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,$2::text::uuid,1)")
            .bind(id(WORLD).as_slice())
            .bind("018f0000-0000-7000-8000-0000000000aa")
            .execute(&mut *tx)
            .await?;
        let joined = sqlx::query("INSERT INTO game_runtime_scope_assignments(scope_key,world_id,channel_id,ownership_generation,state,holder_node_id,holder_registration_revision,source_revision,decision_identity,operation_key,decided_at) SELECT '\\x01'::bytea||uuid_send(a.world_id)||uuid_send($2::text::uuid),a.world_id,$2::text::uuid,1,1,a.holder_node_id,a.holder_registration_revision,9000000001,'reset-cohort-closed',$3,0 FROM game_runtime_scope_assignments a WHERE a.world_id=encode($1,'hex')::uuid AND a.scope_kind=1")
            .bind(id(WORLD).as_slice())
            .bind("018f0000-0000-7000-8000-0000000000aa")
            .bind([9_u8; 32].as_slice())
            .execute(&mut *tx)
            .await;
        assert!(
            matches!(joined, Err(sqlx::Error::Database(ref e)) if e.code().as_deref() == Some("OTR03") && e.message().contains("the Channel cohort is closed")),
            "{joined:?}"
        );
        tx.rollback().await?;
        let mut tx = h.pool.begin().await?;
        let job = crate::durability::world_reset::read_locked_world_reset_target(&mut tx, world()?)
            .await
            .map_err(debug)?;
        assert_eq!(job, target);
        tx.rollback().await?;
        // One RETIRING job per World: a second publication is refused and changes nothing.
        let again = h.root.publish_world_reset(&target, [7; 32], &ResetRunHouses).await;
        assert!(again.is_err(), "{again:?}");
        assert_eq!(published().await?, (1, channels, Some(0)));
        assert_eq!(house_snapshot(&h, item).await?, house_before);
        h.cleanup().await
    })
}
