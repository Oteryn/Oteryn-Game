//! M51 deciding PostgreSQL clock, fenced reset and immutable native replay.
use super::*;
use crate::bestiary_postgres_harness as h;
use crate::foundation::ExactActorRef;

#[test]
fn m51_postgres_timeout_uses_commit_clock_and_replays_without_new_mutation() -> h::TestResult {
    let Some(admin) = h::configured_admin() else {
        return Ok(());
    };
    h::runtime()?.block_on(async move {
        let started=std::time::Instant::now();
        let fixture=h::Harness::create(admin,"srvenc_m51_timeout",true).await?;
        let result:h::TestResult=async {
            let seal=fixture.recovery.seal_current().map_err(h::debug)?;
            let authority=fixture.root.open_character_authority(&seal).await.map_err(h::debug)?;
            let fence=h::fence(1)?;
            let crate::foundation::RuntimeScopeRefV1::Channel {world_id,channel_id}=fence.runtime_scope else {return Err("channel fixture".into());};
            let actor=ExactActorRef::transport_fixture(world_id,channel_id);
            let mut tx=fixture.pool.begin().await?;
            let clock=now(&mut tx).await?;
            // Short timer is fixture state; production still installs donor600s.
            let state=MatchState {rank:1,pit:1,phase:1,run:h::id(230),deadline_ms:clock+5000,entry_revision:1,
                session:*fence.game_session_id.as_bytes(),scope_generation:fence.scope_ownership_generation.get(),..MatchState::default()};
            let seed=CommandRef::new(fence.game_session_id,crate::foundation::CommandId::new(91).map_err(h::debug)?);
            let payload=oteryn_protocol_oteryn::arena::encode_match_request(oteryn_protocol_oteryn::arena::ArenaAction::Status).map_err(h::debug)?;
            let initial=oteryn_protocol_oteryn::arena::encode_match_result(&ArenaResult {disposition:ArenaDisposition::Applied,state:state.wire(clock,[0;3])}).map_err(h::debug)?;
            save_origin(&mut tx,fence,seed,&payload,&state,&state,&initial,None).await?;
            assert!(pit_lease::claim(&mut tx,fence,&state,clock).await?);
            tx.commit().await?;
            let origin=NativeArenaOrigin::Timeout(NativeArenaTimeout::new(actor,fence.game_session_id,1,&state).ok_or("timeout")?);
            let request=|| MatchTransition {native_origin:Some(origin.clone()),
 cause: (origin.command()).into(),payload:payload.clone(),
                before:state.clone(),after:state.idle(),disposition:ArenaDisposition::TimedOut,victory:None,change:None,achievement:None};
            assert!(fixture.root.commit_arena_match(&authority,&fixture.node,fence,request()).await.is_err(),"native timeout cannot cancel a run before the deciding clock reaches its deadline");
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_arena_match_receipts").fetch_one(&fixture.pool).await?;
            assert_eq!(count,1,"early timeout writes no receipt");
            sqlx::query("SELECT pg_sleep(GREATEST(0,($1-floor(extract(epoch FROM clock_timestamp())*1000))/1000.0)+0.01)").bind(state.deadline_ms).execute(&fixture.pool).await?;
            let bytes=fixture.root.commit_arena_match(&authority,&fixture.node,fence,request()).await.map_err(h::debug)?;
            let reply=oteryn_protocol_oteryn::arena::decode_match_result(&bytes).map_err(h::debug)?;
            assert_eq!(reply.disposition,ArenaDisposition::TimedOut);
            assert_eq!((reply.state.phase,reply.state.pit),(0,0));
            let leases:i64=sqlx::query_scalar("SELECT count(*) FROM game_svargrond_pit_leases").fetch_one(&fixture.pool).await?;
            assert_eq!(leases,0);
            assert_eq!(fixture.root.commit_arena_match(&authority,&fixture.node,fence,request()).await.map_err(h::debug)?,bytes);
            assert!(fixture.root.read_arena_match(&authority,&fixture.node,fence,origin.command(),&payload).await.is_err(),"native timeout is not a client Status replay");
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_arena_match_receipts").fetch_one(&fixture.pool).await?;
            assert_eq!(count,2);
            Ok(())
        }.await;
        fixture.cleanup().await?;result?;
        eprintln!("M51 REAL POSTGRES deciding timeout clock duration={:?}",started.elapsed());
        assert!(started.elapsed().as_secs_f64()>0.01);
        Ok(())
    })
}
