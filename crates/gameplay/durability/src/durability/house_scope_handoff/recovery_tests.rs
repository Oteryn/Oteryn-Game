use super::*;
use crate::bestiary_postgres_harness::{self as fixture, TestResult, debug, id};

const ROWS: i32 = 205;
const BATCH: i64 = 100;
const HOUSE: &str = "oteryn:content.house.recovery";

// Retained handoff/session DATA only. Node, recovery, assignments and mutations use real owners.
async fn backlog(h: &fixture::Harness, committed: bool) -> TestResult<Vec<[u8; 16]>> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE TEMP TABLE recovery_rows ON COMMIT DROP AS SELECT n,game_character_uuid_v7() handoff,game_character_uuid_v7() src,game_character_uuid_v7() dst,game_character_uuid_v7() actor,game_character_uuid_v7() account FROM generate_series(1,$1) n")
        .bind(ROWS).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_character_account_guards(account_id) SELECT account FROM recovery_rows",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) SELECT x.actor,x.account,r.world_id,1,1,r.profile_revision,r.ruleset_revision,r.content_revision,r.starter_template_revision,'Recovery '||chr(65+x.n/26)||chr(97+x.n%26),2 FROM recovery_rows x CROSS JOIN game_character_roots r WHERE r.character_id=encode($1,'hex')::uuid")
        .bind(id(fixture::CHARACTER).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_durability_reconnect_sessions OVERRIDING USER VALUE SELECT (jsonb_populate_record(NULL::game_durability_reconnect_sessions,to_jsonb(s)||jsonb_build_object('game_session_id',x.src,'character_id',x.actor,'account_id',x.account,'session_state',3))).* FROM recovery_rows x CROSS JOIN game_durability_reconnect_sessions s WHERE s.game_session_id=encode($1,'hex')::uuid")
        .bind(id(fixture::SESSION).as_slice()).execute(&mut *tx).await?;
    if committed {
        sqlx::query("INSERT INTO game_durability_reconnect_sessions OVERRIDING USER VALUE SELECT (jsonb_populate_record(NULL::game_durability_reconnect_sessions,to_jsonb(s)||jsonb_build_object('game_session_id',x.dst,'character_id',x.actor,'account_id',x.account,'session_state',3,'runtime_scope_kind',2,'runtime_scope_channel_id',NULL,'runtime_scope_house_key',$2||'_'||((x.n-1)/100)::text,'runtime_scope_instance_id',game_house_scope_instance_id(s.world_id,$2||'_'||((x.n-1)/100)::text),'origin_channel_id',s.runtime_scope_channel_id))).* FROM recovery_rows x CROSS JOIN game_durability_reconnect_sessions s WHERE s.game_session_id=encode($1,'hex')::uuid")
            .bind(id(fixture::SESSION).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_house_scope_handoffs(handoff_id,direction,character_id,account_id,world_id,origin_channel_id,source_game_session_id,source_connection_generation,source_character_lease_generation,source_scope_ownership_generation,house_key,destination_scope_ownership_generation,acl_revision,reserved_tile,state,destination_game_session_id,prepared_at,committed_at) SELECT x.handoff,1,x.actor,x.account,s.world_id,s.runtime_scope_channel_id,x.src,s.current_generation,s.character_lease_generation,1,$2||'_'||((x.n-1)/100)::text,1,1,int4send(x.n),CASE WHEN $3 THEN 2 ELSE 1 END,CASE WHEN $3 THEN x.dst END,1,CASE WHEN $3 THEN 2 END FROM recovery_rows x CROSS JOIN game_durability_reconnect_sessions s WHERE s.game_session_id=encode($1,'hex')::uuid")
        .bind(id(fixture::SESSION).as_slice()).bind(HOUSE).bind(committed).execute(&mut *tx).await?;
    let ids: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT uuid_send(handoff) FROM recovery_rows ORDER BY handoff")
            .fetch_all(&mut *tx)
            .await?;
    tx.commit().await?;
    ids.into_iter()
        .map(|id| id.try_into().map_err(|_| "handoff id".into()))
        .collect()
}

async fn observe_batches(h: &fixture::Harness) -> TestResult {
    sqlx::raw_sql("CREATE TABLE recovery_observations(seq BIGSERIAL, xid XID8 NOT NULL, handoff UUID NOT NULL); CREATE FUNCTION observe_recovery_batch() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN INSERT INTO recovery_observations(xid,handoff) VALUES(pg_current_xact_id(),OLD.handoff_id); IF (SELECT count(*) FROM recovery_observations WHERE xid=pg_current_xact_id())>100 THEN RAISE EXCEPTION 'unbounded recovery transaction'; END IF; RETURN NULL; END $$; CREATE TRIGGER observe_recovery_batch AFTER DELETE OR UPDATE ON game_house_scope_handoffs FOR EACH ROW EXECUTE FUNCTION observe_recovery_batch(); GRANT SELECT,INSERT ON recovery_observations TO oteryn_game_runtime; GRANT USAGE ON SEQUENCE recovery_observations_seq_seq TO oteryn_game_runtime;")
        .execute(&h.pool).await?;
    Ok(())
}
async fn assert_batches(h: &fixture::Harness, expected: &[[u8; 16]]) -> TestResult {
    let batches: Vec<i64> = sqlx::query_scalar(
        "SELECT count(*) FROM recovery_observations GROUP BY xid ORDER BY min(seq)",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(batches, vec![BATCH, BATCH, 5]);
    let ids: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(handoff) FROM recovery_observations ORDER BY xid,handoff",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        ids,
        expected.iter().map(|id| id.to_vec()).collect::<Vec<_>>()
    );
    Ok(())
}

fn scenario(mode: u8) -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async {
        let h=fixture::Harness::create(admin,&format!("recovery_batch_{mode}"),true).await?;
        let result:TestResult=async {
            if mode==2 {
              for group in 0..3 {
                let key=format!("{HOUSE}_{group}");
                sqlx::query("INSERT INTO game_control_house_scope_grants(control_role,world_id,house_key,operation) SELECT session_user,encode($1,'hex')::uuid,$2,op FROM unnest(ARRAY[1,3]::smallint[]) op")
                    .bind(id(fixture::WORLD).as_slice()).bind(&key).execute(&h.pool).await?;
                let house=HouseId::new(fixture::scope()?.world_id(),&key).ok_or("house")?;
                let assigned=h.root.assign_house_scope(HouseScopeAssignmentRequest { operation_key:OperationKey::from_bytes([91+2*group;32]),actor:ControlActor::new("oteryn_test_admin").map_err(debug)?,actor_role:"oteryn_test_admin".into(),command:HouseScopeAssignmentCommand::Assign {house:house.clone(),target:h.node.fact()} }).await.map_err(debug)?;
                let HouseScopeAssignmentOutcome::Committed(assigned)=assigned else {return Err(format!("assignment {assigned:?}").into())};
                let outcome=h.root.assign_house_scope(HouseScopeAssignmentRequest {operation_key:OperationKey::from_bytes([92+2*group;32]),actor:ControlActor::new("oteryn_test_admin").map_err(debug)?,actor_role:"oteryn_test_admin".into(),command:HouseScopeAssignmentCommand::Revoke {house,predecessor:assigned.predecessor()} }).await.map_err(debug)?;
                assert!(matches!(outcome,HouseScopeAssignmentOutcome::Committed(_)));
              }
            }
            // Qualify the real recovery authority before adding inactive history DATA;
            // these fixtures make no claim to be a Character creation producer.
            let seal=h.recovery.seal_current().map_err(debug)?;
            let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
            let ids=backlog(&h,mode==2).await?;
            if mode==1 {
                use crate::durability::runtime_scope_assignment::{AssignmentCommand,AssignmentRequest,AssignmentOutcome};
                let outcome=h.writer.submit(&AssignmentRequest {operation_key:OperationKey::from_bytes([93;32]),actor:ControlActor::new("oteryn_test_admin").map_err(debug)?,command:AssignmentCommand::Revoke {scope:fixture::scope()?,predecessor:crate::durability::runtime_scope_assignment::AssignmentPredecessor {ownership_generation:1,source_revision:1,runtime_guard_publication_revision:Some(1)}} }).await.map_err(debug)?;
                assert!(matches!(outcome,AssignmentOutcome::Committed(_)));
            }
            observe_batches(&h).await?;
            if mode==0 {
                let report=h.root.reconcile_house_entries(&authority,&h.node,None).await.map_err(debug)?;
                assert_eq!(report.aborted,ids);
                assert!(h.root.reconcile_house_entries(&authority,&h.node,None).await.map_err(debug)?.aborted.is_empty());
            } else {
                let report=h.root.recover_stale_house_entries(&authority,&h.node,300).await.map_err(debug)?;
                if mode==1 {assert_eq!(report.aborted,ids);assert!(report.released.is_empty());}
                else {assert_eq!(report.released,ids);assert!(report.aborted.is_empty());}
                assert_eq!(h.root.recover_stale_house_entries(&authority,&h.node,300).await.map_err(debug)?,HouseRecoveryReport::default());
            }
            assert_batches(&h,&ids).await?;
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
#[test]
fn house_recovery_commits_bounded_ordered_batches() -> TestResult {
    for mode in 0..3 {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || scenario(mode).map_err(|e| e.to_string()))?
            .join()
            .map_err(|_| "recovery batch thread")?
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }
    Ok(())
}
