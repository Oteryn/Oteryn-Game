//! SRV11 real PostgreSQL persistence, audit atomicity and account scope.
use super::{
    TestResult,
    fixture::{Harness, run},
};
use crate::durability::cast_audit::CastAuditRecord;
use oteryn_gameplay_social::{
    cast::{CastOwner, Settings},
    spectate::spectate_test_support::facts,
};
struct Preferences;
impl oteryn_gameplay_social::cast::preferences::PreferencesStore for Preferences {
    async fn load(
        &self,
        _: [u8; 16],
    ) -> Result<
        oteryn_gameplay_social::cast::preferences::Preferences,
        oteryn_gameplay_social::cast::Error,
    > {
        Ok(oteryn_gameplay_social::cast::preferences::Preferences {
            settings: Settings::default(),
            description: "A cast".into(),
        })
    }
}
#[test]
fn s17_persistent_ban_survives_cast_restart_and_is_owner_scoped() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(64<<20).spawn(|| -> Result<(),String> {
        run(async |admin| {
            let started=std::time::Instant::now();
            let h=Harness::create(admin,"s17_bans",true).await?;
            let result=async {
                let f=facts(1,1,1);
                let mut cast=CastOwner::new(50)?;
                let state=cast.start(f,Settings::default(),0)?;
                let record=CastAuditRecord {ordinal:1,kind:12,target_session:f.authority.current_game_session_id().as_bytes().to_vec(),cast_id:state.cast_id.to_vec(),revision:state.revision,epoch:state.epoch,subject_account:Some(vec![2;16]),..Default::default()};
                assert!(!h.root.cast_account_banned(&h.node,[1;16],[2;16]).await?);
                h.root.ban_cast_account(&h.node,[1;16],[2;16],record.clone()).await?;
                h.root.ban_cast_account(&h.node,[1;16],[2;16],record.clone()).await?;
                drop(cast);
                let restarted=CastOwner::new(50)?;
                assert!(restarted.state(f.authority.current_game_session_id()).is_none());
                assert!(h.root.cast_account_banned(&h.node,[1;16],[2;16]).await?);
                assert!(!h.root.cast_account_banned(&h.node,[3;16],[2;16]).await?);
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_cast_audit_outbox").fetch_one(&h.pool).await?;
                assert_eq!(count,1,"idempotent ban has exactly one durable audit");
                sqlx::query("ALTER TABLE game_cast_audit_outbox ADD CONSTRAINT s17_reject CHECK (ordinal < 0) NOT VALID").execute(&h.pool).await?;
                let denied=CastAuditRecord {subject_account:Some(vec![9;16]),..record};
                assert!(h.root.ban_cast_account(&h.node,[1;16],[9;16],denied).await.is_err());
                sqlx::query("ALTER TABLE game_cast_audit_outbox DROP CONSTRAINT s17_reject").execute(&h.pool).await?;
                assert!(!h.root.cast_account_banned(&h.node,[1;16],[9;16]).await?,"failed audit rolls back ban");

                use oteryn_gameplay_social::cast::{service::{apply_durable,Operation,Reply},test_support::admission,Error};
                let mut cast=CastOwner::new(50)?;
                let cast_clock=std::time::Instant::now();
                let now=||u64::try_from(cast_clock.elapsed().as_millis());
                let mut accounts=std::collections::HashMap::new();
                accounts.insert(f.authority.current_game_session_id(),[1;16]);
                let Reply::State(live)=apply_durable(&mut cast,&h.root,&h.node,&Preferences,&accounts,Operation::Start(f,Settings::default(),[1;16]),now()?,0).await? else {return Err("start reply".into());};
                cast.persist_audit(&h.root,&h.node).await?;
                let Reply::Observer(first)=apply_durable(&mut cast,&h.root,&h.node,&Preferences,&accounts,Operation::Join(admission(live,[4;16],[4;16],1)),now()?,0).await? else {return Err("join reply".into());};
                let Reply::Observer(second)=apply_durable(&mut cast,&h.root,&h.node,&Preferences,&accounts,Operation::Join(admission(live,[4;16],[5;16],1)),now()?,0).await? else {return Err("second join reply".into());};
                cast.persist_audit(&h.root,&h.node).await?;
                assert!(apply_durable(&mut cast,&h.root,&h.node,&Preferences,&accounts,Operation::Ban(f,live.revision+1,first),now()?,0).await.is_err());
                apply_durable(&mut cast,&h.root,&h.node,&Preferences,&accounts,Operation::Ban(f,live.revision,first),now()?,0).await?;
                cast.persist_audit(&h.root,&h.node).await?;
                assert!(cast.leave(first).is_err());assert!(cast.leave(second).is_err());
                assert_eq!(cast.take_releases().len(),2);
                assert!(matches!(apply_durable(&mut cast,&h.root,&h.node,&Preferences,&accounts,Operation::Join(admission(live,[4;16],[6;16],1)),now()?,0).await,Err(Error::Unavailable)));
                cast.target_gone(f.authority.current_game_session_id());
                let fresh=cast.start(f,Settings::default(),now()?)?;
                assert!(matches!(apply_durable(&mut cast,&h.root,&h.node,&Preferences,&accounts,Operation::Join(admission(fresh,[4;16],[7;16],1)),now()?,0).await,Err(Error::Unavailable)));

                let grants:bool=sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_runtime','game_cast_account_bans','SELECT,INSERT') AND NOT has_table_privilege('oteryn_game_runtime','game_cast_account_bans','DELETE') AND has_table_privilege('oteryn_game_control','game_cast_account_bans','SELECT')").fetch_one(&h.pool).await?;
                assert!(grants);
                h.root.revoke_node_registration(h.node.fact()).await.map_err(|e|format!("{e:?}"))?;
                assert!(h.root.cast_account_banned(&h.node,[1;16],[2;16]).await.is_err(),"revoked node cannot consume bans");

                eprintln!("S17 executed real PostgreSQL persistent ban case: {:.3}s",started.elapsed().as_secs_f64());
                Ok::<(),Box<dyn std::error::Error>>(())
            }.await;
            h.cleanup().await?;
            result
        }).map_err(|e|e.to_string())
    })?.join().map_err(|_|"S17 thread panicked")?.map_err(Into::into)
}
