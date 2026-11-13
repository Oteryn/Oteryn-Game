//! The actual node decider must recover immutable receipts before fresh eligibility.
use super::*;
use native_admission_source::character_commands::{Decision, Input};

#[test]
fn character_services_v2_node_replays_receipt_when_security_peer_is_down() -> TestResult {
    let Ok(admin)=std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {return Ok(());};
    let (send,receive)=std::sync::mpsc::sync_channel(1);
    let stage=std::sync::Arc::new(std::sync::Mutex::new("fixture"));let observed=stage.clone();
    std::thread::Builder::new().name("service-node-replay".into()).stack_size(64<<20).spawn(move||{
        let result=(||->TestResult{
            tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async move {
                let database=Database::create(admin,"service_node_replay").await?;
                let result=node_replay(&database,&stage).await;
                let cleanup=database.cleanup().await;result?;cleanup
            })
        })().map_err(|error|error.to_string());
        let _=send.send(result);
    })?;
    receive.recv_timeout(std::time::Duration::from_secs(120)).map_err(|error|format!("node replay watchdog at {}: {error}",observed.lock().map(|v|*v).unwrap_or("stage lock")))?.map_err(Into::into)
}
async fn node_replay(database:&Database,stage:&std::sync::Mutex<&'static str>)->TestResult {
    let root=DurabilityRoot::connect_test_runtime(&database.url)?;assert!(root.maintain_ready_once().await?);
    let pool=sqlx::PgPool::connect(&database.url).await?;
    let retained=fence_parent().join(format!("service-node-replay-{}",database.name));std::fs::create_dir(&retained)?;
    let recovery=CharacterRecoveryStore::open(&retained,"character-primary","game-ops").map_err(|e|format!("{e:?}"))?;
    *stage.lock().map_err(|_|"stage")?="fresh recovery admission";
    let fresh=recovery.authorize_fresh_store(id(11),100).map_err(|e|format!("{e:?}"))?;
    root.admit_fresh_character_recovery(&fresh).await.map_err(|e|format!("{e:?}"))?;
    drop(fresh);
    *stage.lock().map_err(|_|"stage")?="recovery seal";
    let fence=recovery.seal_current().map_err(|e|format!("{e:?}"))?;
    let authority=root.open_character_authority(&fence).await.map_err(|e|format!("{e:?}"))?;
    let node=register(&root,1,None).await?;initialize_s2(&root,&node).await?;
    assign_world(&pool,&root,&node,90,95).await?;configure(&pool,["profile-1","ruleset-1","content-1","starter-1"]).await?;allow(&root,&node,31).await?;
    let created=root.bootstrap_character(&authority,&node,&intent(21,31,1)?).await.map_err(|e|format!("{e:?}"))?;
    let character=uuid(*created.character_id.as_bytes());let account=uuid(id(31));
    sqlx::query("INSERT INTO game_character_progression_state SELECT character_id,1,50,1000,profile_revision,ruleset_revision,content_revision,'simulation-1','evidence-1','declaration-1','policy-1','reward-1' FROM game_character_roots WHERE character_id=$1::uuid").bind(&character).execute(&pool).await?;
    let now:i64=sqlx::query_scalar("SELECT game_account_closure_now()").fetch_one(&pool).await?;
    sqlx::query("CREATE TABLE service_node_clock(value bigint NOT NULL)").execute(&pool).await?;
    sqlx::query("INSERT INTO service_node_clock VALUES($1)").bind(now).execute(&pool).await?;
    sqlx::query("GRANT SELECT ON service_node_clock TO oteryn_game_runtime").execute(&pool).await?;
    sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_now() RETURNS bigint LANGUAGE sql STABLE SET search_path=pg_catalog,public AS 'SELECT value FROM service_node_clock'").execute(&pool).await?;
    sqlx::query("CREATE OR REPLACE FUNCTION game_character_service_projection_now() RETURNS bigint LANGUAGE sql STABLE SET search_path=pg_catalog,public AS 'SELECT value FROM service_node_clock'").execute(&pool).await?;
    let operation=uuid(id(201));
    let raw=format!(r#"{{"contract_version":2,"command":"RenameCharacter","variant":"PLATFORM_USER_RENAME","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"{}","source_revision":"1","operation_id":"{}","character_id":"{}","account_id":"{}","expected_character_revision":"1","requested_name":"Redwood Vale","issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}}"#,uuid(id(202)),operation,character,account,now,now+300).into_bytes();
    let stored=root.decide_character_service(&authority,&node,"oteryn:character-authority:primary",("RenameCharacter",&operation),Input::Intent(&raw)).await;
    assert!(matches!(stored,Decision::Publish{ref result_code,..} if result_code=="CHAR_CMD_OK"),"service must commit first: {stored:?}");
    *stage.lock().map_err(|_|"stage")?="unavailable security peer";
    let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await?;let port=listener.local_addr()?.port();drop(listener);
    let ca=rcgen::generate_simple_self_signed(vec!["source.test".into()])?;
    let descriptor=native_admission_source::descriptor::ProducerDescriptor::new("platform".into(),("127.0.0.1".into(),port),"source.test".into(),"source.test".into(),vec![ca.cert.der().clone()],vec![rustls::pki_types::CertificateDer::from(include_bytes!("../spell-save-tls-fixture/client.der").to_vec())],rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("../spell-save-tls-fixture/client-key.der").to_vec())?).map_err(|e|format!("{e:?}"))?;
    let evidence=gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
    assert!(evidence.refresh_account(&root,&node,&account).await.is_err());
    *stage.lock().map_err(|_|"stage")?="production node receipt recovery";
    let replay=production_server::node::serve::decide_character_service_for_node(&root,&authority,&node,&evidence,"oteryn:character-authority:primary",("RenameCharacter",&operation),Input::Intent(&raw)).await;
    assert_eq!(replay,stored,"stored receipt must not depend on fresh security availability");
    Ok(())
}
