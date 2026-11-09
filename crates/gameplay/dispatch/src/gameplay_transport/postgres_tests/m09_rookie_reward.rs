//! M09: Vascalir's actual two-item mission acceptance on the production wire.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const MISSION: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission11";
const REWARD: &str = "oteryn:quest-reward/m09-rookie-mission11";

#[test]
fn m09_rookie_guard_two_rewards_once_and_replay() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m09-rookie")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("NPC capabilities")?;
    super::qengine_outfit_stories::commit_fixture(
        owner,
        admitted,
        "oteryn:quest-transition/quest/u9_1/the_rookie_guard/mission10/npc_3",
        9000,
    )
    .await?;
    super::qengine_outfit_stories::commit_fixture(
        owner,
        admitted,
        "oteryn:quest-transition/quest/u9_1/the_rookie_guard/mission11/npc_4",
        9001,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?
            .item_fence(),
    );
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key == REWARD)
        .ok_or("Rookie callback")?;
    let [npc] = super::classic_quests::install(owner, admitted, [callback.npc.as_str()]).await?;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, MISSION).await?,
        -1
    );
    let (result, _, command) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, &callback, false).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    let expected = vec![
        ("oteryn:item.tibia.i12785".to_owned(), 1),
        ("oteryn:item.tibia.i3054".to_owned(), 1),
    ];
    assert_eq!(items(h).await?, expected);
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, MISSION).await?,
        1
    );
    let siblings: i64 = sqlx::query_scalar("SELECT jsonb_array_length(additional_plan)::bigint FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1")
        .bind(REWARD).fetch_one(&h.pool).await?;
    assert_eq!(siblings, 1);
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
        .fetch_one(&h.pool)
        .await?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        7,
        oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
            &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                npc_actor: npc,
                text: "yes".into(),
            },
        )
        .map_err(debug)?,
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&reply).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    // Restore the real source prerequisite, without replacing the full runtime
    // catalogue. A new claim must still fail the already committed once ledger.
    super::qengine_outfit_stories::commit_fixture(
        owner,
        admitted,
        "oteryn:quest-transition/quest/u9_1/the_rookie_guard/mission11/npc_4",
        9002,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let (second, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, &callback, false).await?;
    assert_eq!(second, NpcIntentDisposition::Rejected);
    assert_eq!(items(h).await?, expected);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(after, before);
    let sources: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1",
    )
    .bind(REWARD)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(sources, 1);
    Ok(())
}

async fn items(h: &Harness) -> TestResult<Vec<(String, i64)>> {
    Ok(sqlx::query_as("SELECT definition_production_key,sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i3054','oteryn:item.tibia.i12785') GROUP BY definition_production_key ORDER BY definition_production_key")
        .fetch_all(&h.pool).await?)
}
