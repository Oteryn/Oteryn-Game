//! Generic binding fixture only; this is not source authoring for the packet quest.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const PHASE: &str = "oteryn:quest-progress/authored-kills/phase";
const COUNT: &str = "oteryn:quest-progress/authored-kills/count";
const STEP: &str = "oteryn:quest-transition/native-kills/authored-engine-rat";

#[test]
fn authored_bindings_native_kill_threshold_completion_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-authored-kills")
}

#[test]
fn authored_bindings_queued_kill_burst_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-authored-kills-burst")
}

async fn value(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2),0)").bind(QUEST).bind(track).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted:&mut AdmittedSession,h:&Harness,burst:bool) -> TestResult {
    let bindings=Bindings::parse(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../quests/src/quest/authored_bindings/fixtures/kill.json")))?;
    let start=bindings.reward_callbacks().next().cloned().ok_or("start binding")?;
    let base=QuestStateCatalogue::empty(owner.quest_catalogue.as_ref().ok_or("catalogue")?.content_revision()).ok_or("empty catalogue")?;
    owner.quest_catalogue=Some(Arc::new(bindings.attach(&base)?));
    let mut selected=admitted.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3,4,6,11,15,18,25]);selected.sort_unstable();selected.dedup();
    let offered=crate::gameplay_transport::appearance_change::offer::offered(owner.offered_capabilities(),owner.appearance_available());
    admitted.continuity.selected_capabilities=crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered,&selected).ok_or("capabilities")?;
    let [npc]=super::classic_quests::install(owner,admitted,["oteryn:npc.cledwyn"]).await?;
    owner.active_generation.and_then(|generation|generation.native_gameplay()).ok_or("native policies")?.install_companion_policies(&mut *owner.runtime.lock().await).map_err(debug)?;
    super::qengine_kill_stories::death(owner,admitted,"oteryn:creature.rat",88000).await?;
    assert_eq!(value(h,COUNT).await?,0,"inactive gate earns no credit");
    assert_eq!(super::qengine_rewards::wire_reward(owner,admitted,npc,&start,false).await?.0,NpcIntentDisposition::Ok);
    assert_eq!(value(h,PHASE).await?,4,"arbitrary declared gate value");
    if burst {
        let mut targets = Vec::new();
        for command in 88101..=88103 {
            targets.push(super::qengine_kill_stories::queue_death(owner, admitted, "oteryn:creature.rat", command).await?);
        }
        assert_eq!(value(h,COUNT).await?,0,"capture alone cannot credit a kill");
        super::qengine_kill_stories::drain_deaths(owner,admitted,&targets).await?;
        assert_eq!(value(h,COUNT).await?,3,"all deaths captured from one committed snapshot must settle");
        assert_eq!(value(h,PHASE).await?,5,"only final queued kill advances the gate");
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND transition_key=$2").bind(QUEST).bind(STEP).fetch_one(&h.pool).await?;
        assert_eq!(receipts,3,"each queued death credits once despite duplicate projections and drains");
        let completed:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes").bind(QUEST).fetch_one(&h.pool).await?;
        assert_eq!(completed,1,"only the threshold occurrence completes");
        return Ok(());
    }
    for expected in 1..=3 {
        super::qengine_kill_stories::death(owner,admitted,"oteryn:creature.rat",88000+expected).await?;
        assert_eq!(value(h,COUNT).await?,i64::try_from(expected)?);
        assert_eq!(value(h,PHASE).await?,if expected==3 {5} else {4},"advance only at the goal using the locked original count");
        let completed:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes").bind(QUEST).fetch_one(&h.pool).await?;
        assert_eq!(completed,i64::from(expected==3),"completion only at threshold");
        if expected==1 {
            super::qengine_kill_stories::death(owner,admitted,"oteryn:creature.dragon",88999).await?;
            assert_eq!(value(h,COUNT).await?,1,"wrong species earns no credit");
        }
    }
    super::qengine_kill_stories::death(owner,admitted,"oteryn:creature.rat",89000).await?;
    assert_eq!(value(h,COUNT).await?,3,"saturated and completed counter never grows");
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND transition_key=$2").bind(QUEST).bind(STEP).fetch_one(&h.pool).await?;
    assert_eq!(receipts,3,"duplicate lethal projections and drains credit exactly once");
    Ok(())
}
