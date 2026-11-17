//! SRV6: only Kesar's actual post-siege report grants Hero of Bounac and Lionheart.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_progression::ExperienceCommitOutcome;
use crate::foundation::WorldId;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use std::sync::Arc;

const NPC: &str = "oteryn:npc.kesar_the_younger_day";
const QUEST: &str = "oteryn:quest.the_order_of_the_lion_quest";
const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_40/the_order_of_the_lion/kesar_mission";
const DOOR: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_40/the_order_of_the_lion/augustin_door";

#[test]
fn srv6_hero_title_real_npc_final_report_atomic_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(|| {
        run(async |admin| {
            let began=std::time::Instant::now();
            let h=Harness::create_appearance(admin,"srv6_hero_title").await?;
            prepare_current_session(&h).await?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
            let world=WorldId::decode(&id(42)).map_err(debug)?;
            let (generation,pin)=super::death_ml_loss_composed::production_generation_with_pin(world,"content-1")?;
            let active=generation.active().ok_or("active")?;
            let room=pin.qualified_room().clone();
            let (channel_pin,_,_,_)=pin.into_channel_parts();
            let deps=Dependencies::with_content(channel_pin,room,active.native_gameplay().ok_or("native")?.spell_book().clone())?;
            deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
            let mut owner=deps.owner(&h,&authority);
            owner.active_generation=Some(active);
            let canonical=oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?.catalogue().clone();
            owner.quest_catalogue=Some(Arc::new(canonical));
            deps.initialize(&owner).await;
            let mut admitted=deps.admitted;
            admitted.first_entry=FirstEntryOutcome::Positioned;
            let fence=owner.current_quest_fence(admitted.game_session_id).await.map_err(|_|"fence")?.ok_or("fence")?;
            admitted.item_fence=Some(fence.item_fence());
            owner.admit_quest_session(&admitted).await;
            let [kesar]=install(&owner,&admitted,[NPC]).await?;
            talk(&mut owner,&admitted,kesar,1000,"hi").await?;
            // The same word below the source's final stage cannot award a title.
            let _=talk(&mut owner,&admitted,kesar,1001,"siege").await?;
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=90").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(count,0);
            // Advance real Character history through the fixture's existing XP owner.
            // Only the completed siege prerequisite is seeded administratively; a
            // generic quest command cannot write Fugue's protected mission track.
            let before=owner.current_quest_fence(admitted.game_session_id).await.map_err(|_|"XP fence")?.ok_or("XP fence")?;
            let award=h.root.commit_character_experience(&authority,&h.node,before,super::fixture::xp_request(91)?).await.map_err(debug)?;
            let ExperienceCommitOutcome::Committed(award)=award else {return Err("fixture XP history was not committed".into());};
            assert_eq!(award.committed_character_revision.get(),before.expected_character_revision.get()+1);
            assert_eq!(award.level_after,award.level_before);
            let mut tx=h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,5,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).bind(MISSION).bind(QUEST).execute(&mut *tx).await?;
            tx.commit().await?;
            owner.refresh_quest_session(admitted.game_session_id,false).await;
            assert_eq!(progress(&owner,&admitted,&authority,MISSION).await?,5);
            assert_eq!(progress(&owner,&admitted,&authority,DOOR).await?,0);
            // A title insert failure occurs after the achievement has been staged:
            // all three source writes must roll back together.
            sqlx::query("CREATE FUNCTION srv6_hero_refuse_title() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.title=90 THEN RAISE EXCEPTION 'srv6 injected title failure'; END IF; RETURN NEW; END $$").execute(&h.pool).await?;
            sqlx::query("CREATE TRIGGER srv6_hero_refuse_title BEFORE INSERT ON game_character_titles FOR EACH ROW EXECUTE FUNCTION srv6_hero_refuse_title()").execute(&h.pool).await?;
            let current=owner.current_quest_fence(admitted.game_session_id).await.map_err(|_|"fence")?.ok_or("fence")?;
            let failed_command=UseCommand{game_session_id:admitted.game_session_id,command_id:2001,item_fence:Some(current.item_fence())};
            owner.clock_origin-=std::time::Duration::from_secs(2);
            let failed=owner.npc_talk(admitted.runtime_actor.ok_or("actor")?,failed_command,NpcTalkIntent{npc_actor:kesar,text:"siege".into()}).await;
            assert_eq!(failed.disposition,NpcIntentDisposition::Rejected);
            let rolled_back:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=90),(SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/lionheart')").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(rolled_back,(0,0));
            assert_eq!(progress(&owner,&admitted,&authority,DOOR).await?,0);
            sqlx::query("DROP TRIGGER srv6_hero_refuse_title ON game_character_titles").execute(&h.pool).await?;
            sqlx::query("DROP FUNCTION srv6_hero_refuse_title()").execute(&h.pool).await?;
            if failed.unknown {
                let reconciled=owner.resume_npc_bank(admitted.runtime_actor.ok_or("actor")?,failed_command).await.ok_or("failed report reconciliation")?;
                assert!(!reconciled.unknown);
                assert_eq!(reconciled.disposition,NpcIntentDisposition::Rejected);
            }
            let report=talk(&mut owner,&admitted,kesar,2010,"siege").await?;
            assert!(report.iter().any(|line|line.contains("Hero of Bounac")),"source final report");
            assert_eq!(progress(&owner,&admitted,&authority,DOOR).await?,1,"real NPC route must persist the report");
            let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=90),(SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/lionheart')").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(counts,(1,1));
            let _=talk(&mut owner,&admitted,kesar,2010,"siege").await?;
            let replay:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=90),(SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/lionheart')").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(replay,counts);
            // A stale source report cannot enter the durable owner.
            let mut stale=owner.current_quest_fence(admitted.game_session_id).await.map_err(|_|"fence")?.ok_or("fence")?.item_fence();
            stale.connection_generation=crate::foundation::ConnectionGeneration::new(stale.connection_generation.get()+1).map_err(debug)?;
            owner.clock_origin-=std::time::Duration::from_secs(2);
            let refused=owner.npc_talk(admitted.runtime_actor.ok_or("actor")?,UseCommand{game_session_id:admitted.game_session_id,command_id:2011,item_fence:Some(stale)},NpcTalkIntent{npc_actor:kesar,text:"siege".into()}).await;
            assert_eq!(refused.disposition,NpcIntentDisposition::Rejected);
            eprintln!("SRV6_HERO_NPC_POSTGRES_COMPLETED elapsed={:?}",began.elapsed());
            drop(owner);drop(authority);drop(seal);h.cleanup().await
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"Hero NPC scenario panicked")?.map_err(Into::into)
}
