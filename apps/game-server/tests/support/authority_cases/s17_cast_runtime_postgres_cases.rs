//! Real Game owner -> current-node-fenced outbox. Platform password KDF and
//! cross-node quota tests require the Platform runner; these do not fake them.
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, runtime};
use crate::durability::cast_audit::CastAuditRecord;
use oteryn_gameplay_social::{
    cast::{CastOwner, Error, Settings, test_support::admission},
    spectate::spectate_test_support::facts,
};
use prost::Message;

fn run(case: u8) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64<<20).spawn(move ||->Result<(),String>{
        runtime().map_err(|e|e.to_string())?.block_on(async {
            let started=std::time::Instant::now();
            let h=Harness::create(admin,&format!("s17_runtime_{case}"),true).await?;
            let result=async {
                let mut owner=CastOwner::new(100)?;
                let target=facts(1,1,1);
                let state=owner.start(target,Settings::default(),0)?;
                owner.persist_audit(&h.root,&h.node).await?;
                let first=owner.join(&admission(state,[1;16],[1;16],1),0)?;
                owner.persist_audit(&h.root,&h.node).await?;
                if case==4 {
                    let raw:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(incarnation) FROM game_cast_audit_streams LIMIT 1").fetch_one(&h.pool).await?;
                    let incarnation:[u8;16]=raw.try_into().map_err(|_|"invalid incarnation")?;
                    let extra=CastAuditRecord {ordinal:3,kind:10,target_session:target.authority.current_game_session_id().as_bytes().to_vec(),cast_id:state.cast_id.to_vec(),revision:state.revision,epoch:state.epoch,..Default::default()};
                    h.root.commit_cast_audit(&h.node,incarnation,&[extra]).await?;
                    assert!(owner.persist_audit(&h.root,&h.node).await.is_err());
                    assert!(owner.state(target.authority.current_game_session_id()).is_none());
                    assert_eq!(owner.take_releases().len(),1);
                    eprintln!("S17 real PostgreSQL receipt-mismatch case: {:.3}s",started.elapsed().as_secs_f64());
                    return Ok(());
                }
                match case {
                    1=> {
                        owner.join(&admission(state,[1;16],[2;16],1),0)?;
                        assert_eq!(owner.join(&admission(state,[1;16],[3;16],1),0),Err(Error::Full));
                        for i in 2..=49 {owner.join(&admission(state,[i;16],[i+2;16],1),0)?;}
                        assert_eq!(owner.join(&admission(state,[99;16],[99;16],1),0),Err(Error::Full));
                        owner.persist_audit(&h.root,&h.node).await?;
                        assert_eq!(owner.state(target.authority.current_game_session_id()).map(|s|s.viewers),Some(50));
                    }
                    2=> {
                        let next=owner.update(target,state.revision,Settings {password_required:true,access_revision:2,..Settings::default()},0)?;
                        owner.persist_audit(&h.root,&h.node).await?;
                        assert!(owner.leave(first).is_err());
                        assert_eq!(owner.join(&admission(next,[2;16],[2;16],1),0),Err(Error::Unavailable));
                        owner.join(&admission(next,[2;16],[3;16],2),0)?;
                    }
                    3=> {
                        owner.mute(target,state.revision,first,true)?;
                        owner.persist_audit(&h.root,&h.node).await?;
                        assert_eq!(owner.chat(first,"blocked",0),Err(Error::Muted));
                        owner.kick(target,state.revision,first)?;
                        owner.persist_audit(&h.root,&h.node).await?;
                        assert_eq!(owner.chat(first,"after kick",0),Err(Error::Unavailable));
                        assert_eq!(owner.take_releases().len(),1);
                    }
                    _=>return Err("unknown S17 test case".into()),
                }
                owner.persist_audit(&h.root,&h.node).await?;
                let rows:i64=sqlx::query_scalar("SELECT count(*) FROM game_cast_audit_outbox").fetch_one(&h.pool).await?;
                assert_eq!(rows,if case==1 {51}else {5});
                let roots:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_roots").fetch_one(&h.pool).await?;
                assert_eq!(roots,1,"viewers must not create characters");
                let envelopes:Vec<Vec<u8>>=sqlx::query_scalar("SELECT envelope FROM game_cast_audit_outbox ORDER BY ordinal LIMIT 64").fetch_all(&h.pool).await?;
                for bytes in envelopes {
                    let envelope=crate::durability::item_mint_audit::EventEnvelopeV1::decode(bytes.as_slice())?;
                    assert_eq!(envelope.event_type_id,12);
                }
                let payloads:Vec<Vec<u8>>=sqlx::query_scalar("SELECT payload FROM game_cast_audit_outbox ORDER BY ordinal LIMIT 64").fetch_all(&h.pool).await?;
                let records:Vec<_>=payloads.iter().map(|p|CastAuditRecord::decode(p.as_slice())).collect::<Result<_,_>>()?;
                let raw:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(incarnation) FROM game_cast_audit_streams LIMIT 1").fetch_one(&h.pool).await?;
                let incarnation:[u8;16]=raw.try_into().map_err(|_|"invalid audit incarnation")?;
                h.root.commit_cast_audit(&h.node,incarnation,&records).await?;
                let mut conflict=records[0].clone();conflict.kind=11;
                assert!(h.root.commit_cast_audit(&h.node,incarnation,&[conflict]).await.is_err());
                let mut gap=records.last().ok_or("empty audit")?.clone();gap.ordinal+=2;
                assert!(h.root.commit_cast_audit(&h.node,incarnation,&[gap]).await.is_err());
                // UUIDv7 identity, exact P30D retention, digest and no duplicate
                // identities are SQL constraints exercised by the real writer.
                let valid:bool=sqlx::query_scalar("SELECT bool_and(game_character_is_uuid_v7(event_id) AND expires_at=occurred_at+2592000000 AND envelope_sha256=sha256(envelope)) FROM game_cast_audit_outbox").fetch_one(&h.pool).await?;
                assert!(valid);
                owner.target_gone(target.authority.current_game_session_id());
                owner.persist_audit(&h.root,&h.node).await?;
                eprintln!("S17 real PostgreSQL case {case}: {:.3}s",started.elapsed().as_secs_f64());
                Ok::<(),Box<dyn std::error::Error>>(())
            }.await;
            h.cleanup().await?;result
        }).map_err(|e|e.to_string())
    })?.join().map_err(|_|"S17 runtime thread panicked")?.map_err(Into::into)
}
#[test]
fn s17_cast_postgres_admission_limits() -> TestResult {
    run(1)
}
#[test]
fn s17_cast_postgres_password_policy_rotation() -> TestResult {
    run(2)
}
#[test]
fn s17_cast_postgres_mute_and_kick() -> TestResult {
    run(3)
}

#[test]
fn s17_cast_postgres_receipt_mismatch_closes_cast() -> TestResult {
    run(4)
}
