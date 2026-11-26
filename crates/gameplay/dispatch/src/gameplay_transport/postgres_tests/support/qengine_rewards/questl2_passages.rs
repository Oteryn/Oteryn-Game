//! L2 delivery rewards must be followed by the real entitled player journey.
use super::*;

pub(in super::super) fn probe(case: &'static str, required: &[&str]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_quest_reward(admin, "l2_passage_binding", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool).await?;
        eprintln!("POSTGRES_CASE_STARTED {case}-binding server_version={version}");
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let present = required.iter().all(|key| callbacks.iter().any(|c| c.key.ends_with(key)));
        h.cleanup().await?;
        eprintln!("POSTGRES_BINDING_PROBE {case} duration={:?}", started.elapsed());
        assert!(present, "complete passage hand-in bindings absent: {required:?}");
        Ok(())
    })?;
    // A registry row alone never qualifies a quest: execute its native journey.
    scenario(case)
}



pub(in super::super) async fn white_raven(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::questl2_journeys::{quantity, use_at, use_at_ordinal};
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let mut selected = admitted.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable(); selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(), owner.appearance_available());
    admitted.continuity.selected_capabilities = crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
        .ok_or("passage capabilities")?;
    let [dalbrect, costello] = super::super::questl2_fixture::install_at(owner, admitted,
        ["oteryn:npc.dalbrect", "oteryn:npc.costello"],
        [Position {x:32206,y:31756,floor:-6}, Position {x:32177,y:31936,floor:-7}]).await?;
    let callbacks=oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    assert!(owner.states.chat.npc_talk().ok_or("NPC book")?.get("oteryn:npc.dalbrect").ok_or("Dalbrect")?
        .travel_routes.iter().any(|route|route.keyword=="passage" && route.route.price==10 && !route.held));
    let find=|key:&str|callbacks.iter().find(|c|c.key.ends_with(key)).ok_or("White Raven callback");
    let brooch=find("/l2-white-raven-brooch")?;
    let ghostlands=Position{x:32261,y:31861,floor:-11};
    let corpse=Position{x:32262,y:31861,floor:-11};
    relocate(owner,admitted,ghostlands).await?;
    use_at_ordinal(owner,admitted,corpse,2,UseDisposition::Rejected).await?;
    assert_eq!(quantity(h,3212).await?,0);
    let dalbrect_contact=Position{x:32206,y:31757,floor:-6};
    relocate(owner,admitted,dalbrect_contact).await?;
    assert_eq!(wire_reward(owner,admitted,dalbrect,brooch,false).await?.0,NpcIntentDisposition::Rejected);
    passage(owner,admitted,dalbrect,false).await?;
    assert_eq!(travel_count(h).await?,0,"friendship is required before passage");
    assert_eq!(at(owner,admitted).await?,dalbrect_contact);
    relocate(owner,admitted,Position{x:32247,y:31866,floor:-8}).await?;
    let source=Position{x:32248,y:31866,floor:-8};
    use_at(owner,admitted,source,UseDisposition::Committed).await?;
    assert_eq!(quantity(h,3205).await?,1,"real source chest issues the Family Brooch");
    use_at(owner,admitted,source,UseDisposition::NothingToUse).await?;
    assert_eq!(quantity(h,3205).await?,1,"once-claim refusal preserves the one actual Brooch");
    relocate(owner,admitted,dalbrect_contact).await?;
    assert_eq!(wire_reward(owner,admitted,dalbrect,brooch,false).await?.0,NpcIntentDisposition::Ok);
    assert_eq!(quantity(h,3205).await?,0);
    assert_eq!(white_track(h,"passage").await?,2);
    assert_eq!(white_track(h,"quest_log").await?,1);
    passage(owner,admitted,dalbrect,false).await?;
    assert_eq!(travel_count(h).await?,0,"entitlement does not waive the ten-gold fee");
    provision(owner,admitted,3031,20,19001,dalbrect_contact).await?;
    passage(owner,admitted,dalbrect,true).await?;
    assert_eq!(travel_count(h).await?,1);
    assert_eq!(quantity(h,3031).await?,10,"ten gold is charged exactly once");
    assert_eq!(at(owner,admitted).await?,Position{x:32190,y:31957,floor:-6});
    let door=Position{x:32171,y:31936,floor:-7};
    let door_contact=Position{x:32172,y:31936,floor:-7};
    relocate(owner,admitted,door_contact).await?;
    use_at(owner,admitted,door,UseDisposition::Locked).await?;
    let costello_contact=Position{x:32177,y:31937,floor:-7};
    relocate(owner,admitted,costello_contact).await?;
    assert_eq!(wire_reward(owner,admitted,costello,find("/l2-white-raven-catacombs")?,false).await?.0,NpcIntentDisposition::Ok);
    assert_eq!(white_track(h,"door").await?,1);
    assert_eq!(white_track(h,"diary").await?,1);
    let diary=find("/l2-white-raven-diary")?;
    assert_eq!(wire_reward(owner,admitted,costello,diary,false).await?.0,NpcIntentDisposition::Rejected);
    assert_eq!(quantity(h,3214).await?,0);
    relocate(owner,admitted,door_contact).await?;
    use_at(owner,admitted,door,UseDisposition::Committed).await?;
    // Simulate an unfinished save pinned by main before the diary callback was shipped.
    let previous = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quests/tests/fixtures/white_raven_catacombs_predecessor.txt"
    ));
    let mut restored_save = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *restored_save)
        .await?;
    let rewritten = sqlx::query(
        "UPDATE game_character_quest_states SET definition_hash=decode($1, 'hex')
         WHERE quest_key=$2 AND completed_character_revision IS NULL",
    )
    .bind(previous.trim())
    .bind(&diary.quest)
    .execute(&mut *restored_save)
    .await?;
    assert_eq!(rewritten.rows_affected(), 1);
    restored_save.commit().await?;
    relocate(owner,admitted,ghostlands).await?;
    use_at_ordinal(owner,admitted,corpse,2,UseDisposition::Committed).await?;
    assert_eq!(quantity(h,3212).await?,1,"a live use of the Ghostlands corpse issues the diary");
    relocate(owner,admitted,costello_contact).await?;
    let (disposition,_,command)=wire_reward(owner,admitted,costello,diary,false).await?;
    assert_eq!(disposition,NpcIntentDisposition::Ok);
    assert_eq!(quantity(h,3212).await?,0);
    assert_eq!(quantity(h,3214).await?,1);
    assert_eq!(white_track(h,"diary").await?,2);
    let completions: Vec<String> = sqlx::query_scalar(
        "SELECT transition_key FROM game_character_quest_receipts \
         WHERE quest_key='oteryn:quest.the_white_raven_monastery_quest' AND completes",
    )
    .fetch_all(&h.pool).await?;
    assert!(
        matches!(completions.as_slice(), [key] if matches!(key.as_str(),
            "oteryn:quest-reward/l2-white-raven-diary"
            | "oteryn:quest-npc/the_white_raven_monastery_quest/costello/diary-complete")),
        "expected one Costello completion receipt, got {completions:?}"
    );
    let completed: bool = sqlx::query_scalar(
        "SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states \
         WHERE quest_key='oteryn:quest.the_white_raven_monastery_quest'",
    )
        .fetch_one(&h.pool).await?;
    assert!(completed);
    let before=counts(h).await?;
    let payload=oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent{npc_actor:costello,text:"yes".into()}).map_err(debug)?;
    let replay=super::super::quest_world_callbacks_fixtures::wire(owner,admitted,command,7,payload).await?;
    assert_eq!(oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,NpcIntentDisposition::Ok);
    assert_eq!(counts(h).await?,before);
    assert_eq!(quantity(h,3214).await?,1);
    Ok(())
}

async fn relocate(owner:&ComposedFreshAdmission<'_, '_, '_>,admitted:&AdmittedSession,position:Position)->TestResult {
    let mut runtime=owner.runtime.lock().await;
    let actor=admitted.runtime_actor.ok_or("actor")?;
    let before=runtime.read_actor_position(actor).map_err(debug)?;
    runtime.relocate_arena_player(actor,admitted.game_session_id,before,
        super::super::quest_world_callbacks_fixtures::local(position)).map_err(debug)?;
    Ok(())
}

pub(in super::super) async fn provision(owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:&AdmittedSession,item:u32,count:u16,id:u64,position:Position)->TestResult {
    let fence=owner.current_quest_fence(admitted.game_session_id).await.map_err(|_|"input fence")?.ok_or("input fence")?;
    super::super::quest_world_callbacks_fixtures::input_item(owner,fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering{position,item:format!("oteryn:item.tibia.i{item}"),count},id,true).await
}

pub(in super::super) async fn passage(owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:&mut AdmittedSession,npc:EntityRef,replay:bool)->TestResult {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let first=admitted.continuity.next_command_id;
    let mut commands=Vec::new();
    for (offset,text) in ["hi","passage","yes"].into_iter().enumerate() {
        let payload=oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent{npc_actor:npc,text:text.into()}).map_err(debug)?;
        commands.push((first+u64::try_from(offset)?,7,payload));
    }
    if replay {commands.push(commands[2].clone());}
    let results=super::super::questl2_fixture::wire_sequence(owner,admitted,commands).await?;
    if replay {assert_eq!(results[2],results[3],"literal passage replay");}
    Ok(())
}

pub(in super::super) async fn white_track(h:&Harness,track:&str)->TestResult<i64>{
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
        .bind(format!("oteryn:quest-progress/quest/u7_24/the_white_raven_monastery/{track}")).fetch_one(&h.pool).await?)
}
pub(in super::super) async fn travel_count(h:&Harness)->TestResult<i64>{
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_newhaven_travel_receipts").fetch_one(&h.pool).await?)
}

pub(in super::super) async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<Position> {
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    Ok(Position {
        x: u16::try_from(at.x)?,
        y: u16::try_from(at.y)?,
        floor: -i8::try_from(at.floor)?,
    })
}
