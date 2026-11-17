//! Rathleton Quest: Marvin's rank-gated voting topics, one counted ballot per cast and
//! receipt replay, through the real NPC talk path (TibiaWiki Rathleton_Quest/Spoiler
//! oldid 1141625; Crystal 201189f8 data-global/npc/marvin.lua:78-81 spends one point).
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug, id, run};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session};
use crate::durability::character_progression::ExperienceCommitOutcome;
use crate::foundation::WorldId;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const MARVIN: &str = "oteryn:npc.marvin";
const POINTS: &str = "oteryn:quest-progress/quest/u10_50/oramond_quest/voting_points";
const RANK: &str = "oteryn:quest-progress/chosen/rathleton-quest/rank";
const VOTES: &str = "oteryn:quest-progress/chosen/rathleton-quest/votes_cast";
const QUEST: &str = "oteryn:quest.authored.rathleton_quest";

#[test]
fn rathleton_rank_gate_counts_each_vote_once_and_replay_never_recounts() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new().stack_size(256<<20).spawn(|| {
        run(async |admin| {
            let began=std::time::Instant::now();
            let h=Harness::create_appearance(admin,"rathleton_election").await?;
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
            owner.admit_premium(id(40), admitted.game_session_id);
            let [marvin]=install(&owner,&admitted,[MARVIN]).await?;
            talk(&mut owner,&admitted,marvin,3000,"hi").await?;
            // Advance real Character history through the fixture's existing XP owner,
            // then seed the earned Oramond points; their task owner is another lane.
            let before=owner.current_quest_fence(admitted.game_session_id).await.map_err(|_|"XP fence")?.ok_or("XP fence")?;
            let award=h.root.commit_character_experience(&authority,&h.node,before,super::fixture::xp_request(91)?).await.map_err(debug)?;
            let ExperienceCommitOutcome::Committed(_)=award else {return Err("fixture XP history was not committed".into());};

            // No voting point: the confirmed cast is refused and nothing is written.
            let refused=cast(&mut owner,&admitted,marvin,3010,"shortcut","catacomb").await?;
            assert_eq!(refused,NpcIntentDisposition::Rejected);
            assert_eq!(counts(&h).await?,(0,0,0));
            assert_eq!(progress(&owner,&admitted,&authority,VOTES).await?,0);

            seed(&h,&[(POINTS,"oteryn:quest.oramond_quest",3),(VOTES,QUEST,49)]).await?;
            owner.refresh_quest_session(admitted.game_session_id,false).await;
            // Below Commoner the dungeon topic is not offered and a forged chain spends nothing.
            let dungeon=talk(&mut owner,&admitted,marvin,3020,"topic").await?;
            assert!(!dungeon.iter().any(|line|line.contains("{dungeon}")),"{dungeon:?}");
            let _=say(&mut owner,&admitted,marvin,3021,"dungeon").await?;
            let _=say(&mut owner,&admitted,marvin,3022,"golem").await?;
            let _=say(&mut owner,&admitted,marvin,3023,"1").await?;
            let _=say(&mut owner,&admitted,marvin,3024,"yes").await?;
            assert_eq!(counts(&h).await?,(0,0,0),"dungeon vote below Commoner");
            assert_eq!(progress(&owner,&admitted,&authority,POINTS).await?,3);

            // One shortcut vote: one point spent, one ballot, one World tally vote.
            assert_eq!(cast(&mut owner,&admitted,marvin,3030,"shortcut","catacomb").await?,NpcIntentDisposition::Ok);
            assert_eq!(counts(&h).await?,(1,1,13));
            assert_eq!(progress(&owner,&admitted,&authority,POINTS).await?,2);
            assert_eq!(progress(&owner,&admitted,&authority,VOTES).await?,50);
            // Replaying the confirmed command returns its stored receipt and counts nothing.
            for (command,text) in [(3040,"topic"),(3041,"shortcut"),(3042,"catacomb"),(3043,"1")] {
                talk(&mut owner,&admitted,marvin,command,text).await?;
            }
            // Command ordering may refuse the reused id before the owner; either way nothing recounts.
            let _=say(&mut owner,&admitted,marvin,3034,"yes").await?;
            owner.states.quest_sessions.lock().map_err(|_|"quest cache")?.remove(&admitted.game_session_id);
            owner.admit_quest_session(&admitted).await;
            assert_eq!(counts(&h).await?,(1,1,13));
            assert_eq!(progress(&owner,&admitted,&authority,POINTS).await?,2);
            assert_eq!(progress(&owner,&admitted,&authority,VOTES).await?,50);
            let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_npc_program_receipts WHERE transition_key='oteryn:quest-npc/authored-rathleton/marvin/shortcut/catacomb/cast'").fetch_one(&h.pool).await?;
            assert_eq!(receipts,1);

            // 50 lifetime votes: promotion to Commoner with its achievement opens the dungeon topic.
            let promoted=talk(&mut owner,&admitted,marvin,3050,"promotion").await?;
            assert!(promoted.iter().any(|line|line.contains("promote your citizen rank to commoner")),"{promoted:?}");
            assert_eq!(progress(&owner,&admitted,&authority,RANK).await?,1);
            let achievement:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/rathleton_commoner'").fetch_one(&h.pool).await?;
            assert_eq!(achievement,1);
            assert_eq!(cast(&mut owner,&admitted,marvin,3060,"dungeon","golem").await?,NpcIntentDisposition::Ok);
            let golem:i64=sqlx::query_scalar("SELECT votes FROM game_rathleton_election_tallies WHERE world_id=encode($1,'hex')::uuid AND option_code=21").bind(id(42).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(golem,1);
            assert_eq!(progress(&owner,&admitted,&authority,POINTS).await?,1);
            eprintln!("POSTGRES_CASE_COMPLETED rathleton_election elapsed={:?}",began.elapsed());
            drop(owner);drop(authority);drop(seal);h.cleanup().await
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"Rathleton election scenario panicked")?.map_err(Into::into)
}

/// The full topic chain; returns the disposition of the confirming "yes".
async fn cast(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    marvin: EntityRef,
    command: u64,
    category: &str,
    option: &str,
) -> TestResult<NpcIntentDisposition> {
    for (offset, text) in [(0, "topic"), (1, category), (2, option), (3, "1")] {
        talk(owner, admitted, marvin, command + offset, text).await?;
    }
    say(owner, admitted, marvin, command + 4, "yes").await
}

async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    marvin: EntityRef,
    command_id: u64,
    text: &str,
) -> TestResult<NpcIntentDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let result = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: marvin,
                text: text.into(),
            },
        )
        .await;
    assert!(!result.unknown);
    Ok(result.disposition)
}

/// (ballots, summed tally votes, last option code) across every World and period.
async fn counts(h: &Harness) -> TestResult<(i64, i64, i16)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_rathleton_election_ballots),(SELECT coalesce(sum(votes),0)::bigint FROM game_rathleton_election_tallies),coalesce((SELECT max(option_code) FROM game_rathleton_election_ballots),0::smallint)")
        .fetch_one(&h.pool)
        .await?)
}

/// Administrative prerequisite rows, written at the current Character revision.
async fn seed(h: &Harness, tracks: &[(&str, &str, i64)]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (track, quest, value) in tracks {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(track).bind(quest).bind(value).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
