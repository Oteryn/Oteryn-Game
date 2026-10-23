//! Ground retirement uses the already mounted Item/Corpse PostgreSQL fixture.
use crate::corpse_transfer_postgres_cases::{mint_corpse, put_loot};
use crate::durability::item_decay_retire_audit::{DecayRetireShape, decode_decay_retire_envelope};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
    RuntimeScopeAssignmentWriter,
};
use crate::foundation::WorldId;
use crate::item_transfer_postgres_cases::{
    CHANNEL, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, id, runtime, scope,
};
use oteryn_gameplay_durability::durability::world_reset::test_support as retiring_fixture;

#[test]
fn world_reset_ground_retires_entries_before_roots_and_resumes_each_item() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "reset_ground").await.map_err(|error| format!("reset fixture: {error}"))?;
        let guards: Vec<(String,bool)> = sqlx::query_as("SELECT proname::text,prosecdef FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND proname=ANY($1::text[]) ORDER BY proname LIMIT 5")
            .bind(vec!["game_item_decay_retire_consistency_guard","game_item_instance_change_proven","game_item_ground_removal_proven","game_item_corpse_entry_removal_proven"])
            .fetch_all(&harness.pool).await?;
        assert_eq!(guards.len(),4);
        assert!(guards.iter().all(|(_,definer)|*definer));
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness.root.open_character_authority(&seal).await.map_err(|error| format!("reset authority: {error:?}"))?;
        let ordinary = harness.mint(&authority,"fixture:reset.ground",3).await.map_err(|error| format!("reset ordinary MINT: {error}"))?;
        let corpse = mint_corpse(&harness,&authority,1000,id(CHARACTER)).await.map_err(|error| format!("reset corpse MINT: {error}"))?;
        let child = put_loot(&harness,corpse,1000,1,100).await?;
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        retiring_fixture::seed_retiring(&harness.pool, world).await?;
        let reset = harness.root.retiring_world_reset(world).await?.ok_or("reset")?;
        assert!(!harness.root.retire_world_reset_ground(&authority,&harness.node,&reset).await.map_err(debug)?);
        assert_eq!(harness.count("game_item_decay_retire_receipts").await?,0);
        // This Item fixture's raw admission row models the already drained
        // terminal state. The real healthy-session release is separately driven
        // through dispatch's planned_world_reset PostgreSQL scenario.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3")
            .execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2)")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&harness.pool).await?;
        let writer = RuntimeScopeAssignmentWriter::open(harness.root.clone(),"item-transfer-writer").await.map_err(debug)?;
        let predecessor = harness.root.read_runtime_scope_predecessor(scope()?).await.map_err(debug)?.ok_or("predecessor")?;
        let result = writer.submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([9;32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Replace {scope:scope()?,predecessor,target:harness.node.fact()},
        }).await.map_err(debug)?;
        assert!(matches!(result,AssignmentOutcome::Committed(_)));
        // Omit the second outbox insert: the deferred reset guard must reject
        // that whole retirement, then resume from the first committed item.
        sqlx::query("CREATE FUNCTION stop_reset_after_one() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF (SELECT count(*) FROM game_item_decay_retire_receipts)>1 THEN RETURN NULL; END IF; RETURN NEW; END $$")
            .execute(&harness.pool).await?;
        sqlx::query("CREATE TRIGGER stop_reset_after_one BEFORE INSERT ON game_item_audit_outbox FOR EACH ROW EXECUTE FUNCTION stop_reset_after_one()")
            .execute(&harness.pool).await?;
        assert!(harness.root.retire_world_reset_ground(&authority,&harness.node,&reset).await.is_err());
        assert_eq!(harness.count("game_item_decay_retire_receipts").await?,1);
        sqlx::query("DROP TRIGGER stop_reset_after_one ON game_item_audit_outbox").execute(&harness.pool).await?;
        assert!(harness.root.retire_world_reset_ground(&authority,&harness.node,&reset).await.map_err(debug)?);
        assert!(harness.root.retire_world_reset_ground(&authority,&harness.node,&reset).await.map_err(debug)?);
        assert_eq!(harness.count("game_item_decay_retire_receipts").await?,3);
        assert_eq!(harness.count("game_item_ground_locations").await?,0);
        assert_eq!(harness.count("game_item_corpse_container_entries").await?,0);
        let rows: Vec<(String,Vec<u8>)> = sqlx::query_as("SELECT r.item_instance_id::text,o.envelope FROM game_item_decay_retire_receipts r JOIN game_item_audit_outbox o USING(event_id) ORDER BY r.occurred_at,r.transaction_id LIMIT 4")
            .fetch_all(&harness.pool).await?;
        let mut retired_child = false;
        for (item,envelope) in rows {
            let (_,message) = decode_decay_retire_envelope(&envelope).map_err(debug)?;
            let item = crate::item_transfer_postgres_cases::uuid_text(id(100))==item;
            let shape=crate::durability::item_decay_retire_audit::check_decay_retire(&message).map_err(debug)?;
            if item { assert_eq!(shape,DecayRetireShape::GroundContainerEntry); retired_child=true; }
            if message.before.as_ref().ok_or("before")?.item_instance_id==corpse {
                assert!(retired_child,"child commits before parent");
            }
            assert_eq!(message.world_reset.ok_or("reset cause")?.reset_epoch,0);
        }
        for item in [ordinary,corpse,child] {
            let state:(i16,i64)=sqlx::query_as("SELECT lifecycle,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(item.as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(state,(2,0));
        }
        let revision:String=sqlx::query_scalar("SELECT revision::text FROM game_world_reset_jobs LIMIT 1").fetch_one(&harness.pool).await?;
        assert_eq!(revision,"4");
        drop(authority);drop(seal);harness.cleanup().await
    })
}
