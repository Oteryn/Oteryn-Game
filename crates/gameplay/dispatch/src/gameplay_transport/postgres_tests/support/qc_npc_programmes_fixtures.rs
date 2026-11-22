use super::classic_quests_fixtures::{install, talk};
use super::fixture::Harness;
use super::qengine_rewards_fixtures;
use super::questpc7_friends_fixtures::{mint, select_npc_capabilities};
use super::questpc7_outfit_items_fixtures::quantity;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const SHIELD: u32 = 3416;
const STEEL: u32 = 5889;
const PLATINUM: u32 = 3035;
const FRAGMENTS: [u32; 4] = [7528, 7529, 7530, 7531];
const REWARD: &str = "oteryn:quest-reward/qc-obsidian-draconian-steel";

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    case: &str,
) -> TestResult {
    select_npc_capabilities(owner, admitted)?;
    let [cyclops] = install(owner, admitted, ["oteryn:npc.a_sweaty_cyclops"]).await?;
    talk(owner, admitted, cyclops, 5000, "hi").await?;
    talk(owner, admitted, cyclops, 5001, "forging friendship").await?;
    for nonce in 7000..7003 {
        mint(owner, admitted, 3560, 1, nonce).await?;
    }
    talk(owner, admitted, cyclops, 5002, "deliver skirts").await?;

    match case {
        "replaces-amulet" => {
            for item in FRAGMENTS {
                mint(owner, admitted, item, 1, u64::from(item)).await?;
            }
            mint(owner, admitted, PLATINUM, 50, 7003).await?;
            mint(owner, admitted, SHIELD, 1, 7004).await?;
            let quote = talk(owner, admitted, cyclops, 5003, "amulet").await?;
            assert_eq!(
                quote,
                ["Me can do unbroken but Big Ben want gold 5000 and all four pieces. Yes?"]
            );

            let offer =
                qengine_rewards_fixtures::talk(owner, admitted, cyclops, 5004, "uth'lokr").await?;
            talk(owner, admitted, cyclops, 5005, "yes").await?;

            assert_eq!(quantity(h, SHIELD).await?, 0, "yes must spend the shield");
            assert_eq!(quantity(h, STEEL).await?, 1);
            for item in FRAGMENTS {
                assert_eq!(quantity(h, item).await?, 1, "amulet fragment {item}");
            }
            assert_eq!(quantity(h, PLATINUM).await?, 50);
            assert_eq!(offer, NpcIntentDisposition::Ok);
        }
        "repeat" => {
            for use_number in 1..=2 {
                mint(owner, admitted, SHIELD, 1, 7100 + use_number).await?;
                let command = 5100 + 2 * use_number;
                talk(owner, admitted, cyclops, command, "uth'lokr").await?;
                assert_eq!(quantity(h, SHIELD).await?, 1, "the quote spends no shield");
                assert_eq!(quantity(h, STEEL).await?, i64::try_from(use_number)? - 1);
                talk(owner, admitted, cyclops, command + 1, "yes").await?;
                assert_eq!(quantity(h, SHIELD).await?, 0);
                assert_eq!(quantity(h, STEEL).await?, i64::try_from(use_number)?);
            }
            let exchanges: Vec<(i32, i64, bool)> = sqlx::query_as(
                "SELECT c.quantity, m.quantity, c.created_xact_id=s.created_xact_id
                         AND m.created_xact_id=s.created_xact_id
                 FROM game_quest_reward_sources s
                 JOIN game_quest_callback_consumptions c USING(character_id,cause_id,cause_ordinal,transition_key)
                 JOIN game_reward_claim_mint_receipts m ON m.transaction_id=s.mint_transaction_id
                 JOIN game_item_instances i ON i.item_instance_id=m.item_instance_id
                 WHERE s.transition_key=$1
                   AND c.before_item->>'definition_production_key'='oteryn:item.tibia.i3416'
                   AND i.definition_production_key='oteryn:item.tibia.i5889'",
            )
            .bind(REWARD)
            .fetch_all(&h.pool)
            .await?;
            assert_eq!(exchanges, [(1, 1, true); 2]);
        }
        "no-shield" => {
            talk(owner, admitted, cyclops, 5003, "uth'lokr").await?;
            let before = snapshot(h).await?;
            assert_eq!(
                qengine_rewards_fixtures::talk(owner, admitted, cyclops, 5004, "yes").await?,
                NpcIntentDisposition::Rejected
            );
            assert_eq!(snapshot(h).await?, before);
        }
        _ => return Err(format!("unknown Cyclops exchange case {case}").into()),
    }
    Ok(())
}

async fn snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_array(
             (SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),
             (SELECT jsonb_agg(to_jsonb(t) ORDER BY character_id,track_key) FROM game_character_quest_tracks t),
             (SELECT count(*) FROM game_character_quest_receipts),
             (SELECT count(*) FROM game_quest_reward_sources),
             (SELECT count(*) FROM game_item_fee_burns)
         )::text",
    )
    .fetch_one(&h.pool)
    .await?)
}
