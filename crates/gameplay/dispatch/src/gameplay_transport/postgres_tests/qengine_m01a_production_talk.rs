//! M01a production wiring: the fixed Torch mark starts through Smaralda's live NPC actor.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::authored_bindings::Trigger;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const QUEST: &str = "oteryn:quest.authored.a_pirate_s_death_to_me";

#[test]
fn m01a_pirates_death_production_talk_dispatch_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m01a-production-pirate-talk")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M01A_PRODUCTION_TALK_POSTGRES {version}");
    let catalogue = owner
        .quest_catalogue
        .as_ref()
        .ok_or("production catalogue")?;
    let quest = catalogue
        .authored_bindings()
        .ok_or("production binding registry")?
        .quests
        .iter()
        .find(|q| q.quest == QUEST)
        .ok_or("production Pirate binding")?;
    let stage = quest
        .stages
        .iter()
        .find(|s| s.key == "s1")
        .ok_or("production TALK stage")?;
    let Trigger::NpcDialogue { callback, .. } = &stage.trigger else {
        return Err("TALK trigger".into());
    };
    let callback = callback.as_ref().clone();
    assert!(!stage.completes);
    assert_eq!(callback.npc, "oteryn:npc.smaralda");
    let track = callback.storage[0].track.clone();
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.smaralda"]).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let (result, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, &callback, false).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    let progress: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
    )
    .bind(QUEST)
    .bind(&track)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(progress, 1);
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(QUEST)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        receipts, 1,
        "the transport replay cannot duplicate the Torch mark"
    );
    let completes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        completes, 0,
        "TALK wiring does not certify the remaining quest stages"
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let persisted: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
    )
    .bind(QUEST)
    .bind(track)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(persisted, 1);
    Ok(())
}
