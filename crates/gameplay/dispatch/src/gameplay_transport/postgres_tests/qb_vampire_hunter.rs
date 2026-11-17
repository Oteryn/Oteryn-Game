//! QB-VAMPIRE: Storkus takes vampire lord tokens in six ranks (1, 4, 5, 10, 30 and 50; 100 in
//! all), sharing experience each time; the last rank adds a vampire doll and opens the Vengoth
//! door to the blood skull chest. The token talk waits for The Inquisition's vampire mission.
//!
//! Reference: TibiaWiki `Vampire_Hunter_Quest`, `Vampire_Lord_Token`, `Vampire_Doll` and
//! `Blood_Skull`; Crystal `data-global/npc/storkus.lua:100-196`.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::reward_callbacks::{ItemAmount, RewardCallback};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

const ROOT: &str = "oteryn:quest-progress/quest/u8_2/vampire_hunter_quest/";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
const TOKEN: &str = "oteryn:item.tibia.i8192";
const DOLL: &str = "oteryn:item.tibia.i8191";
/// Global rank thresholds and the experience Storkus shares at each rank.
const RANKS: [(u16, i64); 6] = [
    (1, 1000),
    (4, 5000),
    (5, 10000),
    (10, 20000),
    (30, 50000),
    (50, 100_000),
];

#[test]
fn qb_vampire_hunter_storkus_is_placed_and_ranks_complete() -> TestResult {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../"));
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/npc-placements/index.json"),
    )?)?;
    let mut placed = false;
    for shard in index["shards"].as_array().ok_or("shards")? {
        let data: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join(shard.as_str().ok_or("shard")?))?)?;
        placed |= data["records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .any(|record| record["declaration"]["npc"] == "oteryn:npc.storkus");
    }
    assert!(placed, "Storkus stands in the native placement index");
    super::qengine_rewards::scenario("qengine-rewards-qb-vampire-hunter")
}

fn callbacks() -> TestResult<Vec<RewardCallback>> {
    let all = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    (1..=RANKS.len())
        .map(|rank| {
            let key = format!("oteryn:quest-reward/qb-vampire-hunter-rank-{rank}");
            all.iter()
                .find(|callback| callback.key == key)
                .cloned()
                .ok_or_else(|| format!("missing {key}").into())
        })
        .collect()
}

async fn stage(h: &Harness, track: &str) -> TestResult<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(track)
            .fetch_optional(&h.pool)
            .await?,
    )
}

async fn quantity(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(item).fetch_one(&h.pool).await?)
}

async fn xp(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?,
    )
}

fn fixture(key: &str, quest: &str, value: i64) -> QuestTransition {
    QuestTransition {
        world_change: None,
        key: format!("oteryn:quest-transition/qb-vampire-hunter/{key}"),
        quest: quest.to_owned(),
        effects: vec![QuestEffect {
            track: QUESTLINE.into(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Set(value),
        }],
        completes: false,
        experience: None,
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let callbacks = callbacks()?;
    // Independent global expectations, not the production callbacks, fix the exchange.
    for (callback, (count, experience)) in callbacks.iter().zip(RANKS) {
        assert_eq!(
            callback.take,
            vec![ItemAmount {
                fluid_subtype: None,
                item: TOKEN.into(),
                count,
            }]
        );
        assert_eq!(
            serde_json::to_value(callback)?["experience"],
            serde_json::json!(experience)
        );
        assert!(callback.outfit.is_none() && callback.outfits.is_empty());
    }
    assert_eq!(
        callbacks
            .iter()
            .map(|c| c.give.as_ref().map(|g| (g.item.as_str(), g.count)))
            .collect::<Vec<_>>(),
        [None, None, None, None, None, Some((DOLL, 1))]
    );
    // Only The Inquisition's questline is arranged; every rank runs through Storkus.
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut tracks = BTreeMap::new();
    let mut transitions = Vec::new();
    for callback in &callbacks {
        let transition = canonical
            .transition(&callback.key)
            .cloned()
            .ok_or("canonical transition")?;
        for effect in &transition.effects {
            tracks.insert(
                effect.track.clone(),
                canonical
                    .track(&effect.track)
                    .cloned()
                    .ok_or("effect track")?,
            );
        }
        transitions.push(transition);
    }
    let questline = canonical
        .track(QUESTLINE)
        .cloned()
        .ok_or("inquisition questline")?;
    let quest = questline.quest.clone();
    tracks.insert(QUESTLINE.into(), questline);
    transitions.push(fixture("vampire-mission", &quest, 10));
    transitions.push(fixture("vampire-mission-done", &quest, 11));
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            canonical.content_revision(),
            tracks.into_values().collect(),
            transitions,
        )
        .map_err(debug)?,
    ));
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
    let [storkus] = super::classic_quests::install(owner, admitted, ["oteryn:npc.storkus"]).await?;
    let mut bags = Vec::new();
    let mut occurrence = 0;
    super::qengine_outfit_stories::supply(
        owner,
        admitted,
        h,
        &callbacks[0].take[0],
        &mut bags,
        &mut occurrence,
    )
    .await?;
    // Before the vampire mission is finished Storkus will not talk about tokens.
    let rank = format!("{ROOT}rank");
    let door = format!("{ROOT}door");
    super::qengine_outfit_stories::commit_fixture(
        owner,
        admitted,
        "oteryn:quest-transition/qb-vampire-hunter/vampire-mission",
        9700,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let (result, _, _) = super::qengine_rewards::wire_reward_with_prefix_refusal(
        owner,
        admitted,
        storkus,
        &callbacks[0],
        false,
        Some(1),
    )
    .await?;
    assert_eq!(result, NpcIntentDisposition::Rejected, "questline 10 refuses");
    assert_eq!(stage(h, &rank).await?, None);
    assert_eq!(quantity(h, TOKEN).await?, 1);
    super::qengine_outfit_stories::commit_fixture(
        owner,
        admitted,
        "oteryn:quest-transition/qb-vampire-hunter/vampire-mission-done",
        9701,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    for (index, (callback, (count, experience))) in callbacks.iter().zip(RANKS).enumerate() {
        let reached = i64::try_from(index)? + 1;
        if index > 0 {
            // One token short of the rank: the whole exchange refuses and nothing is spent.
            let short = ItemAmount {
                fluid_subtype: None,
                item: TOKEN.into(),
                count: count - 1,
            };
            super::qengine_outfit_stories::supply(
                owner,
                admitted,
                h,
                &short,
                &mut bags,
                &mut occurrence,
            )
            .await?;
            let before = xp(h).await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, storkus, callback, false)
                    .await?;
            assert_eq!(result, NpcIntentDisposition::Rejected, "rank {reached} short");
            assert_eq!(quantity(h, TOKEN).await?, i64::from(count - 1));
            assert_eq!(xp(h).await?, before);
            assert_eq!(stage(h, &rank).await?, Some(reached - 1));
            let one = ItemAmount {
                fluid_subtype: None,
                item: TOKEN.into(),
                count: 1,
            };
            super::qengine_outfit_stories::supply(
                owner,
                admitted,
                h,
                &one,
                &mut bags,
                &mut occurrence,
            )
            .await?;
        }
        assert_eq!(quantity(h, TOKEN).await?, i64::from(count));
        let before = xp(h).await?;
        let (result, _, _) =
            super::qengine_rewards::wire_reward(owner, admitted, storkus, callback, false).await?;
        assert_eq!(result, NpcIntentDisposition::Ok, "rank {reached}");
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        assert_eq!(stage(h, &rank).await?, Some(reached));
        assert_eq!(quantity(h, TOKEN).await?, 0, "rank {reached} spends its tokens");
        assert_eq!(xp(h).await?, before + experience, "rank {reached} experience");
        let last = index + 1 == RANKS.len();
        assert_eq!(quantity(h, DOLL).await?, i64::from(last));
        assert_eq!(stage(h, &door).await?, last.then_some(1));
    }
    // A finished hunter has no further rank to buy.
    super::qengine_outfit_stories::supply(
        owner,
        admitted,
        h,
        &callbacks[0].take[0],
        &mut bags,
        &mut occurrence,
    )
    .await?;
    let before = xp(h).await?;
    let (result, _, _) = super::qengine_rewards::wire_reward_with_prefix_refusal(
        owner,
        admitted,
        storkus,
        &callbacks[0],
        false,
        Some(1),
    )
    .await?;
    assert_eq!(result, NpcIntentDisposition::Rejected, "no seventh rank");
    assert_eq!(quantity(h, TOKEN).await?, 1);
    assert_eq!(xp(h).await?, before);
    assert_eq!(quantity(h, DOLL).await?, 1);
    Ok(())
}
