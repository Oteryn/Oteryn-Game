//! M09: source-authored independent outputs through the production NPC wire dispatcher.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const TRACK: &str = "oteryn:quest-progress/quest/u8_1/tower_defence_quest/questline";
const REWARD: &str = "oteryn:quest-npc/tower-defence/reward";
const NPC: &str = "oteryn:npc.bunny_bonecrusher";

#[test]
fn m09_tower_defence_four_rewards_once_and_replay() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m09-tower-once")
}

#[test]
fn m09_tower_defence_three_free_slots_reject_whole_reward() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m09-tower-full")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
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
        "oteryn:quest-transition/quest/u8_1/tower_defence_quest/questline/npc_4",
        9000,
    )
    .await?;
    let full = name.ends_with("-full");
    if full {
        for n in 0..17 {
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                    position: oteryn_gameplay_quests::quest::world_callbacks::Position {
                        x: 33293,
                        y: 31677,
                        floor: -15,
                    },
                    item: "oteryn:item.tibia.i3054".into(),
                    count: 1,
                },
                100 + n,
                true,
            )
            .await?;
        }
        let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(entries, 17, "only three direct entries remain free");
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = super::classic_quests::install(owner, admitted, [NPC]).await?;
    // The callback value is only the existing wire driver's word sequence.
    // The runtime selects the real native source programme, not this fixture.
    let mut conversation = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .next()
        .ok_or("wire driver")?;
    conversation.key = "oteryn:quest-reward/m09-tower".into();
    conversation.npc = NPC.into();
    conversation.keywords = vec!["password".into()];
    let (result, _, command) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, &conversation, false).await?;
    let received:Vec<(String,i64)>=sqlx::query_as("SELECT definition_production_key,sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key IN ('oteryn:item.tibia.i762','oteryn:item.tibia.i774','oteryn:item.tibia.i763','oteryn:item.tibia.i761') AND lifecycle=1 GROUP BY definition_production_key ORDER BY definition_production_key")
        .fetch_all(&h.pool).await?;
    if full {
        assert_ne!(
            result,
            NpcIntentDisposition::Ok,
            "the whole four-output grant is refused"
        );
        assert!(received.is_empty());
        let sources: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_quest_npc_item_sources WHERE transition_key=$1",
        )
        .bind(REWARD)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(sources, 0);
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
            4
        );
    } else {
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(
            received,
            vec![
                ("oteryn:item.tibia.i761".into(), 50),
                ("oteryn:item.tibia.i762".into(), 50),
                ("oteryn:item.tibia.i763".into(), 50),
                ("oteryn:item.tibia.i774".into(), 50)
            ]
        );
        let siblings:i64=sqlx::query_scalar("SELECT jsonb_array_length(r.additional_plan)::bigint FROM game_reward_claim_mint_receipts r JOIN game_quest_npc_item_sources s ON s.mint_transaction_id=r.transaction_id WHERE s.transition_key=$1").bind(REWARD).fetch_one(&h.pool).await?;
        assert_eq!(siblings, 3);
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
            5
        );
        let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
            .fetch_one(&h.pool)
            .await?;
        let replay = super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            7,
            oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
                &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                    npc_actor: npc,
                    text: "password".into(),
                },
            )
            .map_err(debug)?,
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay)
                .map_err(debug)?,
            NpcIntentDisposition::Ok
        );
        // Even restored source eligibility cannot bypass the original once ledger.
        super::qengine_outfit_stories::commit_fixture(
            owner,
            admitted,
            "oteryn:quest-transition/quest/u8_1/tower_defence_quest/questline/npc_4",
            9001,
        )
        .await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        let (second, _, _) =
            super::qengine_rewards::wire_reward(owner, admitted, npc, &conversation, false).await?;
        assert_eq!(second, NpcIntentDisposition::Rejected);
        let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(after, before, "a second literal claim cannot mint again");
        let sources: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_quest_npc_item_sources WHERE transition_key=$1",
        )
        .bind(REWARD)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(sources, 1);
    }
    Ok(())
}
