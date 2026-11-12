//! PUBPROJ-B: native Guild writer + checked-catalogue durable producer proofs.
use crate::bestiary_postgres_harness::{
    CHARACTER, CHANNEL, WORLD, Harness, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::guild::GuildId;
use crate::durability::guild::{GuildCommand, GuildEditRanksRequest, GuildOperationIdentity, GuildRequest};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::public_projections::house_catalogue::{PublicHouseCatalogue, PublicHouseMetadata};
use crate::foundation::{ChannelId, CommandId, CommandRef, WorldId};
use crate::native_admission_source::public_projections::v2::{Body, Family, HouseState};
use sqlx::{Executor, Row};

fn item_fence() -> TestResult<CurrentCharacterItemFence> {
    let f=fence(1)?;
    Ok(CurrentCharacterItemFence { character_id:f.character_id,game_session_id:f.game_session_id,
        connection_generation:f.connection_generation,character_lease_generation:f.character_lease_generation,
        runtime_scope:f.runtime_scope,scope_ownership_generation:f.scope_ownership_generation })
}
fn run_case(tag:&'static str,case:impl for<'a> FnOnce(&'a Harness)->std::pin::Pin<Box<dyn std::future::Future<Output=TestResult>+'a>> + Send+'static)->TestResult {
    let Some(admin)=configured_admin() else { return Ok(()); };
    std::thread::Builder::new().stack_size(64<<20).spawn(move || -> Result<(),String> {
        runtime().map_err(debug)?.block_on(async move {
            let started=std::time::Instant::now();
            let h=Harness::create(admin,tag,false).await?;
            let result=case(&h).await;
            h.cleanup().await?;
            eprintln!("PUBPROJ_GUILD_HOUSE_PG_EXECUTED case={tag} elapsed_ms={}",started.elapsed().as_millis());
            result
        }).map_err(debug)
    })?.join().map_err(|_|"PUBPROJ thread failed")?.map_err(Into::into)
}
#[test]
fn pubproj_guild_native_rank_write_revision_outbox_ack_race_and_resync()->TestResult {
    run_case("pubproj_guild",|h|Box::pin(async move {
        let guild=GuildId::from_bytes(id(110)).map_err(debug)?;
        // Fixture-only source baseline; all assertions below drive the unmodified
        // native writer and its enabled production source triggers.
        let mut tx=h.pool.begin().await?;
        tx.execute("SET LOCAL session_replication_role=replica").await?;
        sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Public Rose',1,9999999999999,1,1789000000000)")
            .bind(guild.as_bytes().as_slice()).bind(id(WORLD).as_slice()).execute(&mut *tx).await?;
        for (rank,name) in [(1_i16,"Leader"),(2,"Vice Leader"),(3,"Member")] {
            sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3)")
                .bind(guild.as_bytes().as_slice()).bind(rank).bind(name).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,'',1789000000000)")
            .bind(id(CHARACTER).as_slice()).bind(guild.as_bytes().as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_guild_leadership(account_id,guild_id,character_id) SELECT account_id,encode($1,'hex')::uuid,character_id FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
            .bind(guild.as_bytes().as_slice()).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&h.pool).await?;
        let request=|command,expected,names:Vec<String>| -> TestResult<GuildEditRanksRequest> {
            let seed=u8::try_from(command).map_err(debug)?;
            Ok(GuildEditRanksRequest::new(GuildRequest {
                identity:GuildOperationIdentity {wire_digest:None,occurrence:id(111+seed),transaction_id:id(121+seed),event_id:id(131+seed),occurred_at_unix_ms:now},
                command:CommandRef::new(item_fence()?.game_session_id,CommandId::new(command).map_err(debug)?),guild_id:guild,operation:GuildCommand::EditRanks,
            },expected,names).map_err(debug)?)
        };
        h.root.commit_guild_edit_ranks(&authority,&h.node,item_fence()?,request(1,1,vec!["Captain".into(),"Vice Leader".into(),"Member".into()])?).await.map_err(debug)?;
        let first=h.root.next_public_v2(&authority,Family::Guild,&[]).await.map_err(debug)?.ok_or("Guild outbox")?;
        let Body::Guild(records)=&first.body else { return Err("Guild body".into()); };
        assert_eq!(records[0].ranks[0].name,"Captain");
        assert_eq!(records[0].members[0].character_id,"29020304-0506-7008-800a-0b0c0d0e0f29");
        h.root.commit_guild_edit_ranks(&authority,&h.node,item_fence()?,request(2,2,vec!["Commander".into(),"Vice Leader".into(),"Member".into()])?).await.map_err(debug)?;
        h.root.ack_public_v2(&authority,&first).await.map_err(debug)?;
        let next=h.root.next_public_v2(&authority,Family::Guild,&[]).await.map_err(debug)?.ok_or("newer Guild revision survives old ACK")?;
        assert!(next.projection_revision>first.projection_revision);
        let Body::Guild(records)=&next.body else { return Err("Guild body".into()); };
        assert_eq!(records[0].ranks[0].name,"Commander");
        h.root.ack_public_v2(&authority,&next).await.map_err(debug)?;
        sqlx::query("SELECT game_public_projection_resync('guild',false)").execute(&h.pool).await?;
        assert!(h.root.next_public_v2(&authority,Family::Guild,&[]).await.map_err(debug)?.is_some());
        Ok(())
    }))
}

fn catalogue(houses:Option<Vec<PublicHouseMetadata>>)->TestResult<PublicHouseCatalogue> {
    Ok(PublicHouseCatalogue {world:WorldId::decode(&id(WORLD)).map_err(debug)?,channel:ChannelId::decode(&id(CHANNEL)).map_err(debug)?,generation:1,reset_epoch:0,map_revision:"pubproj-map-r1".into(),houses})
}
fn house(name:&str)->PublicHouseMetadata {
    PublicHouseMetadata {house_key:"oteryn:content.house.public_test".into(),name:name.into(),kind:"private_house".into(),town:"oteryn:content.area.city.thais".into(),size_sqm:24,beds:2,rent_gold:5000,content_digest:"01".repeat(32)}
}
#[test]
fn pubproj_house_catalogue_source_revision_withdrawal_pending_freshness_and_resync()->TestResult {
    run_case("pubproj_house",|h|Box::pin(async move {
        let mut tx=h.pool.begin().await?;
        tx.execute("SET LOCAL session_replication_role=replica").await?;
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,decode(repeat('01',32),'hex'),'pubproj-map-r1',decode(repeat('02',32),'hex'),decode(repeat('03',32),'hex'),decode(repeat('04',32),'hex'),'TIBIA_SERVER_SAVE_BERLIN_06',1,statement_timestamp(),'2026-11') ON CONFLICT DO NOTHING")
            .bind(id(WORLD).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0) ON CONFLICT DO NOTHING").bind(id(WORLD).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest) VALUES(encode($1,'hex')::uuid,'oteryn:content.house.public_test','private_house',1,5000,decode(repeat('01',32),'hex'))")
            .bind(id(WORLD).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        h.root.sync_public_house_catalogue(&authority,&h.node,catalogue(None)?).await.map_err(debug)?;
        let pending:Option<i64>=sqlx::query_scalar("SELECT game_public_v2_source_pending('house')").fetch_one(&h.pool).await?;
        assert!(pending.is_some(),"unknown catalogue cannot advance freshness");
        h.root.sync_public_house_catalogue(&authority,&h.node,catalogue(Some(vec![house("Public House")]))?).await.map_err(debug)?;
        let first=h.root.next_public_v2(&authority,Family::House,&[]).await.map_err(debug)?.ok_or("house outbox")?;
        let Body::House(rows)=&first.body else { return Err("house body".into()); };
        assert_eq!(rows[0].name,"Public House");
        assert!(matches!(&rows[0].state,HouseState::Vacant));
        h.root.sync_public_house_catalogue(&authority,&h.node,catalogue(Some(vec![house("Renamed House")]))?).await.map_err(debug)?;
        h.root.ack_public_v2(&authority,&first).await.map_err(debug)?;
        let second=h.root.next_public_v2(&authority,Family::House,&[]).await.map_err(debug)?.ok_or("concurrent house source revision survives ACK")?;
        assert!(second.projection_revision>first.projection_revision);
        h.root.ack_public_v2(&authority,&second).await.map_err(debug)?;
        sqlx::query("SELECT game_public_projection_resync('house',false)").execute(&h.pool).await?;
        assert!(h.root.next_public_v2(&authority,Family::House,&[]).await.map_err(debug)?.is_some());
        h.root.sync_public_house_catalogue(&authority,&h.node,catalogue(Some(Vec::new()))?).await.map_err(debug)?;
        let withdrawn=h.root.next_public_v2(&authority,Family::House,&[]).await.map_err(debug)?.ok_or("explicit catalogue withdrawal")?;
        assert!(matches!(&withdrawn.body,Body::House(rows) if rows.is_empty()));
        let row=sqlx::query("SELECT (SELECT count(*) FROM game_public_house_catalogue) AS retained,game_public_v2_source_pending('house') AS pending").fetch_one(&h.pool).await?;
        assert_eq!(row.try_get::<i64,_>("retained")?,0);
        assert_eq!(row.try_get::<Option<i64>,_>("pending")?,None);
        Ok(())
    }))
}
