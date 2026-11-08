//! Mainland Citizen: literal NPC programmes, exact costs, clock gate and account addons.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::reward_callbacks::{ItemAmount, RewardCallback};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;
const PREFIX: &str = "oteryn:quest-progress/quest/u7_8/citizen_outfits/";
const CLOCK: &str = "oteryn:quest-transition/test/questpc-citizen-clock";

#[test]
fn questpc_citizen_male_backpack_then_hat_source_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-questpc-citizen-male-story")
}
#[test]
fn questpc_citizen_female_hat_then_backpack_source_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-questpc-citizen-female-story")
}
async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_reward_sources),(SELECT count(*) FROM game_quest_callback_consumptions),(SELECT count(*) FROM game_account_appearance_unlock_receipts)").fetch_one(&h.pool).await?)
}
async fn quantity(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(item).fetch_one(&h.pool).await?)
}
async fn replay(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    npc: EntityRef,
    callback: &RewardCallback,
    command: u64,
) -> TestResult {
    let before = counts(h).await?;
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: callback.keywords[0].clone(),
    })
    .map_err(debug)?;
    let bytes = super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&bytes).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        counts(h).await?,
        before,
        "literal command replay cannot consume or grant twice"
    );
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    println!("POSTGRES_CASE_STARTED {name} server_version={version}");
    let all = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let find = |suffix: &str| {
        all.iter()
            .find(|c| c.key == format!("oteryn:quest-reward/questpc-citizen-{suffix}"))
            .cloned()
            .ok_or_else(|| format!("missing Citizen source callback {suffix}"))
    };
    let materials = find("backpack-materials")?;
    let backpack = find("backpack")?;
    let hat = find("hat")?;
    // Preserve every active programme track and production compatibility history.
    // Only the clock is arranged; outfit stages, items and entitlements are earned.
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    owner.quest_catalogue = Some(Arc::new(
        oteryn_gameplay_quests::quest::questpc_clock_fixture::with_citizen_clock(
            canonical,
            now - 7202,
        )?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
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
            .ok_or("Citizen source capabilities")?;
    let [lubo, hanna] =
        super::classic_quests::install(owner, admitted, ["oteryn:npc.lubo", "oteryn:npc.hanna"])
            .await?;
    let mut bags = Vec::new();
    let mut occurrence = 700;
    let hat_first = name.contains("-female-");
    let order = if hat_first {
        [true, false]
    } else {
        [false, true]
    };
    let mut mask = 0;
    for do_hat in order {
        let (callback, npc, track) = if do_hat {
            (&hat, hanna, "addon_hat")
        } else {
            (&backpack, lubo, "addon_backpack")
        };
        let start = if do_hat { &hat } else { &materials };
        let (result, _, _) =
            super::qengine_rewards::wire_reward(owner, admitted, npc, start, false).await?;
        assert_eq!(
            result,
            NpcIntentDisposition::Ok,
            "source introduction and acceptance"
        );
        assert_eq!(
            super::classic_quests::progress(
                owner,
                admitted,
                authority,
                &format!("{PREFIX}{track}")
            )
            .await?,
            1
        );
        let before = counts(h).await?;
        if do_hat {
            // One honeycomb short exercises complete rollback with two other
            // recipe types already owned. Both source addons are independent.
            for (item, count) in [(3374, 1), (5890, 100), (5902, 49)] {
                super::qengine_outfit_stories::supply(
                    owner,
                    admitted,
                    h,
                    &ItemAmount {
                        item: format!("oteryn:item.tibia.i{item}"),
                        // The shared outfit helper deliberately mints one surplus
                        // for multi-item recipes; arrange the exact source amount.
                        count: if count > 2 { count - 1 } else { count },

                        fluid_subtype: None,
                    },
                    &mut bags,
                    &mut occurrence,
                )
                .await?;
            }
            assert_eq!(quantity(h, "oteryn:item.tibia.i3374").await?, 1);
            assert_eq!(quantity(h, "oteryn:item.tibia.i5890").await?, 100);
            assert_eq!(quantity(h, "oteryn:item.tibia.i5902").await?, 49);
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, &hat, false).await?;
            assert_eq!(result, NpcIntentDisposition::Rejected);
            assert_eq!(quantity(h, "oteryn:item.tibia.i3374").await?, 1);
            assert_eq!(quantity(h, "oteryn:item.tibia.i5890").await?, 100);
            assert_eq!(quantity(h, "oteryn:item.tibia.i5902").await?, 49);
            assert_eq!(counts(h).await?, before);
            super::qengine_outfit_stories::supply(
                owner,
                admitted,
                h,
                &ItemAmount {
                    item: "oteryn:item.tibia.i5902".into(),
                    count: 1,

                    fluid_subtype: None,
                },
                &mut bags,
                &mut occurrence,
            )
            .await?;
        } else {
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, &materials, false)
                    .await?;
            assert_eq!(
                result,
                NpcIntentDisposition::Rejected,
                "100 real leathers required"
            );
            assert_eq!(counts(h).await?, before);
            super::qengine_outfit_stories::supply(
                owner,
                admitted,
                h,
                &ItemAmount {
                    item: "oteryn:item.tibia.i5878".into(),
                    // Shared helper adds one surplus to multi-item requests.
                    count: 99,

                    fluid_subtype: None,
                },
                &mut bags,
                &mut occurrence,
            )
            .await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, &materials, false)
                    .await?;
            assert_eq!(result, NpcIntentDisposition::Ok);
            assert_eq!(quantity(h, "oteryn:item.tibia.i5878").await?, 0);
            assert_eq!(
                super::classic_quests::progress(
                    owner,
                    admitted,
                    authority,
                    &format!("{PREFIX}addon_backpack")
                )
                .await?,
                2
            );
            let before = counts(h).await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, &backpack, false).await?;
            assert_eq!(
                result,
                NpcIntentDisposition::Rejected,
                "strict source two-hour wait"
            );
            assert_eq!(counts(h).await?, before);
            let stages:Vec<(String,i64)>=sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE track_key<>$1 ORDER BY track_key").bind(format!("{PREFIX}addon_backpack_timer")).fetch_all(&h.pool).await?;
            super::qengine_outfit_stories::commit_fixture(owner, admitted, CLOCK, 9800).await?;
            let after:Vec<(String,i64)>=sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE track_key<>$1 ORDER BY track_key").bind(format!("{PREFIX}addon_backpack_timer")).fetch_all(&h.pool).await?;
            assert_eq!(after, stages, "only clock arranged; no source stage seeded");
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
        }
        let (result, catalogues, command) =
            super::qengine_rewards::wire_reward(owner, admitted, npc, callback, true).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        mask |= if do_hat { 2 } else { 1 };
        let look = if name.contains("-male-") { 128 } else { 136 };
        assert!(catalogues.iter().any(|c| {
            c.outfits.iter().any(|o| {
                o.key == "oteryn:outfit.citizen"
                    && o.permitted_addons == mask
                    && o.look_type == look
            })
        }));
        assert_eq!(
            super::classic_quests::progress(
                owner,
                admitted,
                authority,
                &format!("{PREFIX}{track}")
            )
            .await?,
            if do_hat { 2 } else { 3 }
        );
        replay(owner, admitted, h, npc, callback, command).await?;
    }
    let facts:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key LIKE 'oteryn:outfit.citizen/quest-grant/%'").fetch_one(&h.pool).await?;
    assert_eq!(facts, 3, "one base and two immutable addon facts");
    assert_eq!(mask, 3);
    for item in [3374, 5890, 5902, 5878] {
        assert_eq!(quantity(h, &format!("oteryn:item.tibia.i{item}")).await?, 0);
    }
    let before = counts(h).await?;
    let (result, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, lubo, &backpack, false).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(
        counts(h).await?,
        before,
        "completed source branch never grants again"
    );
    Ok(())
}
