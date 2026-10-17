//! Exact native Root/PG cases. No fake Root, disabled trigger or invented current owner.
use crate::durability::item_decay_retire::{CorpseDecayFence, DecayRetireError, DecayRetireStep};
use crate::durability::item_decay_retire_audit::{self as audit, DecayRetireShape};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
    RuntimeScopeAssignmentWriter,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::world_reset::{
    WorldResetError, WorldResetRetirement, candidate_schema_sql, census_retiring_world_reset,
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
async fn install(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(candidate_schema_sql()?.to_owned()))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
async fn admit(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_control")
        .execute(&mut *tx)
        .await?;
    // Explicit Control bootstrap via the same operation; never seed epoch0 rows directly.
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
        return Err("routed PostgreSQL required".into());
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
        let request=RewardClaimMintRequest{command:crate::item_transfer_postgres_cases::command(20)?,claim:crate::durability::item_mint::TypedDefinitionRef{family:"RewardClaim".into(),production_key:"fixture:chest.reset-key".into(),revision_ref:"definition-r1".into()},source_placement:"fixture:placement.chest".into(),item:ItemDefinitionFacts{definition,stack:ItemStackClass::NonStackable,container_capacity:None,container_slot_equip_pattern:false},quantity:1,backpack:crate::item_transfer_postgres_cases::backpack_facts(),content_revision:"content-1".into(),ruleset_revision:"ruleset-1".into(),sim_revision:"sim-1".into(),achievement:None,quest_transition:None,key_number:Some(7)};
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
        return Err("routed PostgreSQL required".into());
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
        return Err("routed PostgreSQL required".into());
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
        return Err("routed PostgreSQL required".into());
    };
    runtime()?.block_on(async move{
  let h=Harness::create(admin,"resetproof").await?;let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
  let item=h.mint(&authority,"fixture:b3.stone",1).await?;install(&h).await?;
  let before=footprint(&h).await?;
  let mut tx=h.pool.begin().await?;
  assert!(sqlx::query("TRUNCATE game_world_reset_scopes").execute(&mut *tx).await.is_err());tx.rollback().await?;
  let mut tx=h.pool.begin().await?;sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
  assert!(sqlx::query("INSERT INTO game_world_reset_jobs VALUES(encode($1,'hex')::uuid,0,$2,'map-1','RETIRING',1)").bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice()).execute(&mut *tx).await.is_err());tx.rollback().await?;
  let mut tx=h.pool.begin().await?;sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).execute(&mut *tx).await?;
  assert!(tx.commit().await.is_err());assert_eq!(footprint(&h).await?,before);
  drop(authority);drop(seal);h.cleanup().await
 })
}

#[test]
fn reset_native_retry_maximum_persists_frozen_identity_across_authority_reload() -> TestResult {
    let admin = configured_admin().ok_or("routed PostgreSQL required")?;
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
    let admin = configured_admin().ok_or("routed PostgreSQL required")?;
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
    let admin = configured_admin().ok_or("routed PostgreSQL required")?;
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
    let admin = configured_admin().ok_or("routed PostgreSQL required")?;
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
    let admin = configured_admin().ok_or("routed PostgreSQL required")?;
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
    let admin = configured_admin().ok_or("routed PostgreSQL required")?;
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
    let admin = configured_admin().ok_or("routed PostgreSQL required")?;
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
        let missing=sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,active_map_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,'map-1','RETIRING')")
            .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice()).execute(&mut *tx).await;
        assert!(matches!(missing,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("OTR02")));
        tx.rollback().await?;
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
    let admin = configured_admin().ok_or("routed PostgreSQL required")?;
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"resetlegacyactivation").await?;
        install(&h).await?;
        admit(&h).await?;
        let before: (i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_content_activations),(SELECT count(*) FROM game_world_reset_epoch_history)")
            .fetch_one(&h.pool).await?;
        let fact=h.node.fact();
        let mut tx=h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
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
    let admin = configured_admin().ok_or("routed PostgreSQL required")?;
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
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *contender).await?;
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
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *invalid).await?;
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
        // The BEFORE INSERT rule must refuse the phase before PK/current-epoch checks.
        let refused=sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,active_map_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,'map-1','ACTIVATED')")
            .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice()).execute(&mut *control).await;
        assert!(matches!(refused,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("23514")));
        control.rollback().await?;
        let after: (i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_content_activations),(SELECT count(*) FROM game_world_reset_epoch_history),(SELECT count(*) FROM game_world_reset_jobs)")
            .fetch_one(&h.pool).await?;
        assert_eq!(before,after);
        h.cleanup().await
    })
}

// Codex P1 4300751311 regression, PRIVATE PROPOSAL, NOT_RUN. With every other durable
// prerequisite current, direct runtime SQL that skips the Rust drain/House checks must
// still refuse: the SQL owner recomputes the drain cohort digest and requires House
// evidence. Each refusal changes exactly one input; the exact evidence activates.
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
        // Predecessor Channel activation equal to the epoch-0 history digests.
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
        let previous: Option<String> = sqlx::query_scalar("SELECT max(activation_sequence)::text FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).fetch_one(&h.pool).await?;
        let previous = previous.map(|value| value.parse::<u64>()).transpose()?;
        let next = previous.unwrap_or(0).checked_add(1).ok_or("sequence exhausted")?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5,$6,$7)")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice())
            .bind(next.to_string()).bind(previous.map(|value|value.to_string()))
            .bind([1_u8;32].as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice())
            .execute(&h.pool).await?;
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
        for (digest, stable_keys, activates) in [
            (flipped, stable.clone(), false),
            (drained(generation + 1)?, stable.clone(), false),
            (exact, Vec::new(), false),
            (exact, vec!["oteryn:content.house.other".to_owned()], false),
            (exact, stable, true),
        ] {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
            let result = sqlx::query("SELECT game_world_reset_activate(encode($1,'hex')::uuid,0,1,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8)")
                .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice())
                .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
                .bind(digest.as_slice()).bind(Vec::<String>::new()).bind(Vec::<Vec<u8>>::new()).bind(stable_keys)
                .execute(&mut *tx).await;
            if activates {
                result?;
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

/// [`admit`] with a qualified target: the separate target facts final activation requires.
async fn admit_qualified(h: &Harness) -> TestResult {
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
    sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,active_map_revision,target_map_revision,target_server_digest,target_client_digest,target_frame_digest,target_calendar_policy,target_calendar_revision,target_calendar_parity,phase) VALUES(encode($1,'hex')::uuid,0,$2,'map-1','map-2',$3,$4,$5,$6,1,'PARITY_PENDING','RETIRING')")
        .bind(id(WORLD).as_slice()).bind([7_u8;32].as_slice())
        .bind([4_u8;32].as_slice()).bind([5_u8;32].as_slice()).bind([6_u8;32].as_slice())
        .bind(crate::durability::world_reset::ResetCalendarPolicy::TibiaServerSaveBerlin10.as_str())
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_reset_scopes SELECT world_id,0,channel_id,ownership_generation FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1").bind(id(WORLD).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
