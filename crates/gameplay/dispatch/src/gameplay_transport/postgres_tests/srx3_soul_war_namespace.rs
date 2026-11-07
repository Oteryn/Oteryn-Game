//! Namespace gate only: the copied production CHECK grants no quest transition.
use super::fixture::{Harness, run};
use super::TestResult;

#[test]
fn srx3_soul_war_tagged_instance_death_receipt_namespace_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {return Ok(());}
    let started=std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-soul-war-namespace",||run(async |admin| {
        let h=Harness::create_with_admission_seed(admin,"srx3_soul_war_namespace",false,false).await?;
        let mut tx=h.pool.begin().await?;
        sqlx::query("CREATE TEMP TABLE srx3_soul_war_key_gate (LIKE game_character_soul_war_receipts INCLUDING DEFAULTS INCLUDING CONSTRAINTS) ON COMMIT DROP")
            .execute(&mut *tx).await?;
        // These are DATA encodings, never a captured lethal occurrence or a
        // current Character/Instance grant. Real source/receipt guards remain
        // on the actual table; this isolates the pre-existing 52-byte CHECK.
        let channel=vec![9u8;52];
        insert(&mut tx,&channel,1).await?;
        let mut instance=vec![2];instance.extend_from_slice(&channel);
        let result=insert(&mut tx,&instance,2).await;
        if let Err(error)=result {
            tx.rollback().await?;h.cleanup().await?;
            return Err(format!("production Soul War CHECK rejects canonical tag2+52-byte Instance encoding: {error}").into());
        }
        sqlx::query("SAVEPOINT wrong_tag").execute(&mut *tx).await?;
        instance[0]=1;
        let refused=insert(&mut tx,&instance,3).await;
        assert!(refused.as_ref().err().and_then(sqlx::Error::as_database_error)
            .is_some_and(|e|e.code().as_deref()==Some("23514")),"53 bytes must carry the Instance tag2");
        sqlx::query("ROLLBACK TO SAVEPOINT wrong_tag").execute(&mut *tx).await?;
        tx.rollback().await?;h.cleanup().await
    }))?;
    eprintln!("SRX3_SOUL_WAR_NAMESPACE_POSTGRES_CASE_EXECUTED duration={:?}",started.elapsed());
    Ok(())
}
async fn insert(tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,death:&[u8],occurrence:u8)->Result<(),sqlx::Error> {
    sqlx::query("INSERT INTO srx3_soul_war_key_gate(character_id,occurrence,request_binding,result,source_death_key,boss_key,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed,original_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES('00000000-0000-7000-8000-000000000001',$1,$2,$2,$3,'greed',false,0,NULL,false,false,1,1,1,0,0,'p','r','c','s','e','d','q','v')")
        .bind(vec![occurrence;32]).bind(vec![1u8]).bind(death).execute(&mut **tx).await?;
    Ok(())
}
