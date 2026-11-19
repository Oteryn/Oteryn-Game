use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use super::fixture::{Harness, debug, id};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const TRACK: &str = "oteryn:quest-progress/questpc7/tokens/programme";

async fn progress(owner:&ComposedFreshAdmission<'_, '_, '_>, admitted:&AdmittedSession)->TestResult<i64>{
    let f=owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
    let state=owner.root.read_character_quest_state(owner.character,f.character_id).await.map_err(debug)?;
    Ok(state.tracks().get(TRACK).copied().unwrap_or(-1))
}

async fn count(h:&Harness,item:u32)->TestResult<i64>{
    super::questpc7_outfit_items_fixtures::quantity(h,item).await
}

async fn mint(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&AdmittedSession,quantity:u16,nonce:u64)->TestResult{
    let fence=owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(owner,fence.item_fence(),&Offering{
        position:Position{x:33293,y:31677,floor:-15},item:"oteryn:item.tibia.i19083".into(),count:quantity
    },nonce,true).await
}

pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,h:&Harness)->TestResult{
    let version:String=sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_TOKENS POSTGRES_STARTED version={version}");
    let mut selected=admitted.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3,6,25]);
    selected.sort_unstable();
    selected.dedup();
    let offered=crate::gameplay_transport::appearance_change::offer::offered(owner.offered_capabilities(),owner.appearance_available());
    admitted.continuity.selected_capabilities=crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered,&selected).ok_or("capabilities")?;
    let [jorge]=super::classic_quests_fixtures::install(owner,admitted,["oteryn:npc.jorge"]).await?;
    assert_eq!(progress(owner,admitted).await?,-1,"no subject quest progress seeds");
    let callbacks=oteryn_gameplay_quests::quest::reward_callbacks::load().map_err(debug)?;
    let find=|name:&str|callbacks.iter().find(|c|c.key==format!("oteryn:quest-reward/questpc7-tokens-{name}")).ok_or_else(||format!("missing selected token callback {name}"));
    let intro=find("replica-rewards")?;
    let exchange=find("abacus-replica")?;
    let complete=find("finish-the-commission")?;
    assert_eq!(super::qengine_rewards_fixtures::wire_grant(owner,admitted,jorge,intro).await?.0,NpcIntentDisposition::Ok);
    assert_eq!(progress(owner,admitted).await?,0);
    super::qengine_rewards_fixtures::wire_grant(owner,admitted,jorge,complete).await?;
    assert_eq!(progress(owner,admitted).await?,0,"completion cannot bypass the exchange");
    mint(owner,admitted,19,6000).await?;
    assert_eq!(super::qengine_rewards_fixtures::wire_grant(owner,admitted,jorge,exchange).await?.0,NpcIntentDisposition::Rejected);
    assert_eq!(count(h,19083).await?,19);
    assert_eq!(count(h,19151).await?,0);
    assert_eq!(progress(owner,admitted).await?,0);
    mint(owner,admitted,5,6001).await?;
    assert_eq!(count(h,19083).await?,24);
    assert_eq!(super::qengine_rewards_fixtures::wire_grant(owner,admitted,jorge,exchange).await?.0,NpcIntentDisposition::Ok);
    assert_eq!(count(h,19083).await?,4,"consume exactly twenty across carried stacks");
    assert_eq!(count(h,19151).await?,1);
    assert_eq!(progress(owner,admitted).await?,1);
    super::qengine_rewards_fixtures::wire_grant(owner,admitted,jorge,exchange).await?;
    assert_eq!(count(h,19083).await?,4,"a duplicate request must not charge again");
    assert_eq!(count(h,19151).await?,1);
    assert_eq!(super::qengine_rewards_fixtures::wire_grant(owner,admitted,jorge,complete).await?.0,NpcIntentDisposition::Ok);
    assert_eq!(progress(owner,admitted).await?,2);
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.hunting_for_tokens'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    super::qengine_rewards_fixtures::wire_grant(owner,admitted,jorge,complete).await?;
    assert_eq!(count(h,19083).await?,4);
    assert_eq!(count(h,19151).await?,1);
    let grants:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE character_id=encode($1,'hex')::uuid AND transition_key='oteryn:quest-reward/questpc7-tokens-abacus-replica'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(grants,1);
    Ok(())
}
