//! The real receipt validator must agree with the checked native counter evaluator.
use crate::bestiary_postgres_harness as h;
use oteryn_gameplay_quests::quest::{self, reward_callbacks::StateOperation};
use sha2::{Digest, Sha384};

#[test]
fn m07_floor_counter_postgres_matches_native_arithmetic_and_retains_bounds() -> h::TestResult {
    let Some(admin)=h::configured_admin() else { return Ok(()); };
    let started=std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED m07/floor-counter");
    let result=std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        h::runtime().map_err(|e|e.to_string())?.block_on(async {
            let harness=h::Harness::create(admin,"m07_floor_counter",true).await.map_err(|e|e.to_string())?;
            let result=exercise(&harness).await.map_err(|e|e.to_string());
            harness.cleanup().await.map_err(|e|e.to_string())?;
            result
        })
    })?.join().map_err(|_|"M07 floor counter PostgreSQL worker panicked")?;
    eprintln!("POSTGRES_CASE_COMPLETED m07/floor-counter duration={:?} success={}",started.elapsed(),result.is_ok());
    result.map_err(Into::into)
}

async fn valid(harness:&h::Harness, plan:serde_json::Value,keys:Vec<String>,before:Vec<i64>,after:Vec<i64>)->h::TestResult<bool> {
    Ok(sqlx::query_scalar("SELECT game_quest_reward_effects_valid($1::jsonb,$2::text[],$3::bigint[],$4::bigint[])")
        .bind(sqlx::types::Json(plan)).bind(keys).bind(before).bind(after).fetch_one(&harness.pool).await?)
}

async fn exercise(harness:&h::Harness)->h::TestResult {
    let version:String=sqlx::query_scalar("SHOW server_version_num").fetch_one(&harness.pool).await?;
    assert_eq!(version,crate::CANONICAL_POSTGRES_SERVER_VERSION_NUM);
    eprintln!("POSTGRES_DATABASE_READY m07/floor-counter version={version}");
    let checksum:Vec<u8>=sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version=2061 AND success")
        .fetch_one(&harness.pool).await?;
    assert_eq!(checksum,Sha384::digest(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../apps/game-server/migrations/2061_quest_counter_floor.sql"))).as_slice());
    let operation:StateOperation=serde_json::from_value(serde_json::json!({
        "track":"oteryn:quest-progress/m07/counter","from":{"kind":"any"},
        "effect":{"kind":"add_at_least","value":1,"minimum":2}
    }))?;
    let owner="oteryn:quest.m07_counter_fixture";
    let transition=quest::QuestTransition { key:"oteryn:quest-transition/m07/counter".into(),quest:owner.into(),
        effects:vec![operation.quest_effect()],completes:false,experience:None,world_change:None };
    let catalogue=quest::QuestStateCatalogue::new("m07-counter",vec![quest::QuestTrack {
        key:operation.track.clone(),quest:owner.into(),initial:0,min:i64::MIN,max:i64::MAX,
    }],vec![transition.clone()]).map_err(|e|format!("{e:?}"))?;
    let plan=serde_json::json!({"storage":[],"state_ops":[operation]});
    let keys=vec![operation.track.clone()];
    for before in [-1,0,1,2,3,100] {
        let changes=catalogue.evaluate(&transition,&std::collections::BTreeMap::from([(operation.track.clone(),before)]),0).map_err(|e|format!("{e:?}"))?;
        let after=changes.first().ok_or("native counter change")?.after;
        assert!(valid(harness,plan.clone(),keys.clone(),vec![before],vec![after]).await?);
        assert!(!valid(harness,plan.clone(),keys.clone(),vec![before],vec![after+1]).await?);
    }
    assert!(!valid(harness,plan.clone(),keys.clone(),vec![i64::MAX],vec![i64::MAX]).await?);
    let mut negative=plan.clone();negative["state_ops"][0]["effect"]["value"]=(-1).into();
    assert!(!valid(harness,negative,keys.clone(),vec![i64::MIN],vec![2]).await?,"floor cannot mask signed overflow");
    for field in ["value","minimum"] {
        for malformed in [serde_json::Value::Null,serde_json::json!("2"),serde_json::json!(2.5)] {
            let mut bad=plan.clone();bad["state_ops"][0]["effect"][field]=malformed;
            assert!(!valid(harness,bad,keys.clone(),vec![0],vec![2]).await?);
        }
    }
    let mut legacy=plan.clone();legacy["state_ops"][0]["effect"]=serde_json::json!({"kind":"add","value":1});
    assert!(valid(harness,legacy,keys,vec![2],vec![3]).await?);
    let mut many_keys=Vec::new();let mut operations=Vec::new();
    for index in 0..16 {
        let key=format!("oteryn:quest-progress/m07/counter-{index}");
        let mut op=serde_json::to_value(&operation)?;op["track"]=key.clone().into();
        many_keys.push(key);operations.push(op);
    }
    let many=serde_json::json!({"storage":[],"state_ops":operations});
    assert!(valid(harness,many.clone(),many_keys.clone(),vec![0;16],vec![2;16]).await?,"retain the established primary receipt capacity");
    many_keys.push("oteryn:quest-progress/m07/counter-16".into());
    let mut excess=many;let mut op=serde_json::to_value(&operation)?;op["track"]=many_keys.last().ok_or("last key")?.clone().into();
    excess["state_ops"].as_array_mut().ok_or("operations")?.push(op);
    assert!(!valid(harness,excess,many_keys,vec![0;17],vec![2;17]).await?);
    Ok(())
}
