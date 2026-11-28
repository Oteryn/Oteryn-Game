use super::fixture::{Harness, debug};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::durability::character_authority::ReconciledCharacterAuthority;

use oteryn_protocol_oteryn::world_object::UseDisposition;

use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const STATE: &str = "oteryn:quest-progress/qd/steal_from_thieves/state";
const RECOVERED: &str = "oteryn:quest-progress/qd/steal_from_thieves/bag-recovered";

const START: &str = "oteryn:quest-reward/qd-steal-from-thieves-start";

const RETURN: &str = "oteryn:quest-reward/qd-steal-from-thieves-return";

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
    let [npc] = super::classic_quests_fixtures::install(owner, admitted, [start.npc.as_str()]).await?;
    let plain = admitted.continuity.selected_capabilities;
    super::qd_world_fixtures::select_talk_capabilities(owner, admitted, &[3, 6])?;
    let talk = super::qengine_rewards_fixtures::wire_conversation;
    // A bare confirmation before the offer is ordinary dialogue and cannot grant the reward.
    let (result, _, _) = talk(owner, admitted, npc, finish, false, vec!["hi", "yes"]).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(super::classic_quests_fixtures::progress(owner, admitted, authority, STATE).await?, 0);
    assert_eq!(count(h, "oteryn:item.tibia.i3725").await?, 0);
    assert_eq!(count(h, "oteryn:item.tibia.i3031").await?, 0);
    let (result, _, _) = talk(owner, admitted, npc, start, false, vec!["hi", "mission", "yes"]).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(super::classic_quests_fixtures::progress(owner, admitted, authority, STATE).await?, 1);
    // Without Ahmet's bag the hand-in is refused and nothing is granted.
    let (result, _, _) = talk(owner, admitted, npc, finish, false, vec!["hi", "quest", "yes"]).await?;
    assert_eq!(result, NpcIntentDisposition::Rejected);
    assert_eq!(count(h, "oteryn:item.tibia.i3725").await?, 0);
    let chest = super::qd_world_fixtures::OASIS_BOX;
    assert_eq!(
        super::qd_world_fixtures::use_at(owner, admitted, chest, plain).await?,
        UseDisposition::Committed
    );
    assert_eq!(count(h, "oteryn:item.tibia.i235").await?, 1);
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, RECOVERED).await?,
        1
    );
    assert_ne!(
        super::qd_world_fixtures::use_at(owner, admitted, chest, plain).await?,
        UseDisposition::Committed
    );
    assert_eq!(count(h, "oteryn:item.tibia.i235").await?, 1);
    let (result, _, _) = talk(owner, admitted, npc, finish, false, vec!["hi", "quest", "yes"]).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(super::classic_quests_fixtures::progress(owner, admitted, authority, STATE).await?, 2);
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
