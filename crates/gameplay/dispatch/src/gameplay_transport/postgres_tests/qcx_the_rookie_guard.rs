//! Mission 11's separately confirmed brass shield on the production NPC wire.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const MISSION: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission11";

#[test]
fn vascalir_grants_one_brass_shield_after_poison_hand_in() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m09-rookie")
}

pub(super) async fn shield(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    npc: EntityRef,
    acceptance: &RewardCallback,
) -> TestResult {
    // State 4 is the source's completed poison hand-in, before claiming the shield.
    super::qengine_outfit_stories::commit_fixture(
        owner,
        admitted,
        "oteryn:quest-transition/quest/u9_1/the_rookie_guard/mission11/npc_2",
        9003,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, MISSION).await?,
        4
    );
    assert_eq!(shield_count(h).await?, 0);
    let (result, _, _) = super::qengine_rewards::wire_conversation(
        owner,
        admitted,
        npc,
        acceptance,
        false,
        vec!["hi", "yes"],
    )
    .await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, MISSION).await?,
        5
    );
    assert_eq!(shield_count(h).await?, 1);
    Ok(())
}

async fn shield_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(quantity), 0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3411'",
    )
    .fetch_one(&h.pool)
    .await?)
}
