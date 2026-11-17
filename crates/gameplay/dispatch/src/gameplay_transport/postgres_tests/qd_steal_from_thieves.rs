//! Steal From Thieves (TibiaWiki Steal_From_Thieves_Quest): Ahmet's mission, the
//! Darama oasis bag hand-in and the global reward of 100 brown mushrooms and 100 gold.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const STATE: &str = "oteryn:quest-progress/qd/steal_from_thieves/state";
const START: &str = "oteryn:quest-reward/qd-steal-from-thieves-start";
const RETURN: &str = "oteryn:quest-reward/qd-steal-from-thieves-return";

#[test]
fn qd_steal_from_thieves_mission_bag_return_and_global_reward() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-qd-steal-from-thieves")
}

async fn count(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(item).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load().map_err(debug)?;
    let start = callbacks.iter().find(|c| c.key == START).ok_or("start")?;
    let finish = callbacks.iter().find(|c| c.key == RETURN).ok_or("return")?;
    let [npc] = super::classic_quests::install(owner, admitted, [start.npc.as_str()]).await?;
    let talk = super::qengine_rewards::wire_conversation;
    // The return route is closed before Ahmet hands out the mission.
    let (result, _, _) = talk(owner, admitted, npc, finish, false, vec!["hi", "quest", "yes"]).await?;
    assert_eq!(result, NpcIntentDisposition::Rejected);
    assert_eq!(super::classic_quests::progress(owner, admitted, authority, STATE).await?, 0);
    let (result, _, _) = talk(owner, admitted, npc, start, false, vec!["hi", "mission", "yes"]).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(super::classic_quests::progress(owner, admitted, authority, STATE).await?, 1);
    // Without Ahmet's bag the hand-in is refused and nothing is granted.
    let (result, _, _) = talk(owner, admitted, npc, finish, false, vec!["hi", "quest", "yes"]).await?;
    assert_eq!(result, NpcIntentDisposition::Rejected);
    assert_eq!(count(h, "oteryn:item.tibia.i3725").await?, 0);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position { x: 33293, y: 31677, floor: -15 },
            item: "oteryn:item.tibia.i235".into(),
            count: 1,
        },
        52000,
        true,
    )
    .await?;
    let (result, _, _) = talk(owner, admitted, npc, finish, false, vec!["hi", "quest", "yes"]).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(super::classic_quests::progress(owner, admitted, authority, STATE).await?, 2);
    assert_eq!(count(h, "oteryn:item.tibia.i235").await?, 0, "the bag is returned");
    assert_eq!(count(h, "oteryn:item.tibia.i3725").await?, 100);
    assert_eq!(count(h, "oteryn:item.tibia.i3031").await?, 100);
    let completed: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key='oteryn:quest.steal_from_thieves_quest'")
        .fetch_one(&h.pool).await?;
    assert!(completed, "the hand-in completes the canonical quest");
    // The reward is granted once.
    let (result, _, _) = talk(owner, admitted, npc, finish, false, vec!["hi", "quest", "yes"]).await?;
    assert_eq!(result, NpcIntentDisposition::Rejected);
    assert_eq!(count(h, "oteryn:item.tibia.i3725").await?, 100);
    Ok(())
}
