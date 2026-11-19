//! Exercise the authored reward owner through live Rottin TALK and PostgreSQL.
use super::classic_quests_fixtures::{install, talk};
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::{ItemKey, SessionItemView, inventory_from_backpack};
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.rottin_wood_and_the_married_men_quest";
const TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/task01";
const COOLDOWN: &str = "oteryn:quest-progress/crystalserver/kv/lucky-loop-task";
pub(super) const START_MISSION: &str =
    "oteryn:quest-transition/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/mission01/npc_1";
pub(super) const START_TASK: &str =
    "oteryn:quest-transition/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/task01/npc_1";
const ACHIEVEMENT: &str = "oteryn:achievement/bunny_slipped";
const PAYOUT: &str = "oteryn:quest-reward/rottin-wood/lucky-loop";
const ROTTIN: &str = "oteryn:npc.rottin_wood";
const PLATINUM: &str = "oteryn:item.tibia.i3035";
const RING: &str = "oteryn:item.tibia.i3053";
const RIBBON: &str = "oteryn:item.tibia.i12171";
const FOOT: &str = "oteryn:item.tibia.i12172";
const CHARM: &str = "oteryn:item.tibia.i12173";
const DONOR: &str =
    "201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/rottin_wood.lua:238-255";

fn payout_key(repeat: bool) -> &'static str {
    if repeat {
        "oteryn:quest-reward/rottin-wood/lucky-loop-repeat"
    } else {
        PAYOUT
    }
}

pub(super) fn catalogue(repeat: bool) -> TestResult<QuestStateCatalogue> {
    // This transaction fixture exercises the two gold bundles with admitted
    // output definitions. The quest unit case also checks the horn interval.
    let digest = Sha256::digest(DONOR.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    // First completion grants the account achievement. Later completions can
    // reuse the item-only occurrence owner after the quest's daily cooldown.
    let (prior_stamp, cooldown, achievement) = if repeat {
        (
            1,
            json!({"kind": "elapsed_at_least", "seconds": 72001}),
            None,
        )
    } else {
        (
            0,
            json!({"kind": "between", "minimum": 0, "maximum": 0}),
            Some(ACHIEVEMENT),
        )
    };
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST,
            "recipe_sha256": digest,
            "sources": [DONOR],
            "tracks": [
                {"key": TRACK, "initial": 0, "minimum": 0, "maximum": 2},
                {"key": COOLDOWN, "initial": prior_stamp, "minimum": 0, "maximum": i64::MAX}
            ],
            "stages": [{
                "key": "lucky-loop",
                "completes": false,
                "trigger": {
                    "kind": "npc_dialogue",
                    "callback": {
                        "key": payout_key(repeat), "quest": QUEST, "npc": ROTTIN,
                        "keywords": ["yes"], "topic": 0, "next_topic": 0,
                        "storage": [{"track": TRACK, "minimum": 1, "maximum": 1, "set": 2}],
                        "state_ops": [{
                            "track": COOLDOWN,
                            "from": cooldown,
                            "effect": {"kind": "set_now"}
                        }],
                        "inventory": [{"item": CHARM, "minimum": 7, "maximum": u32::MAX}],
                        "take": [{"item": CHARM, "count": 7}],
                        "give": {"item": PLATINUM, "count": 5},
                        "weighted_rewards": [{"chance": 9, "reward": {
                            "item": PLATINUM, "count": 3,
                            "additional": [{"item": RING, "count": 1}]
                        }}],
                        "outfit": null, "achievement": achievement, "experience": 100,
                        "per_occurrence_mint": repeat,
                        "reply": ["Good hunt."],
                        "failure_reply": ["Bring seven lucky charms."], "sources": [DONOR]
                    }
                }
            }]
        }]
    }))?)?;
    let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let track = source.catalogue().track(TRACK).ok_or("Lucky Loop track")?;
    let start = source
        .catalogue()
        .transition(START_TASK)
        .ok_or("Lucky Loop start transition")?;
    let base = QuestStateCatalogue::new("content-1", vec![track.clone()], vec![start.clone()])
        .map_err(debug)?;
    Ok(bindings.attach(&base)?)
}

pub(super) async fn craft_charms(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    count: i64,
    start_transition: &str,
) -> TestResult {
    super::item_look_fixtures::seed_main_backpack(h, &[(RIBBON, count), (FOOT, count)]).await?;
    let quest_fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let fence = quest_fence.item_fence();
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence)
        .await
        .map_err(debug)?;
    let index = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("production item index")?;
    let mut view = SessionItemView::resume(Default::default());
    view.snapshot(
        inventory_from_backpack(inventory.backpack.as_ref(), |definition| {
            index.definition_ref(
                &definition.family,
                &definition.production_key,
                &definition.revision_ref,
            )
        })
        .ok_or("recipe inventory")?,
        None,
    )
    .map_err(debug)?;
    let backpack = inventory.backpack.as_ref().ok_or("recipe backpack")?;
    let operand = |key: &str| -> TestResult<_> {
        let item = backpack
            .entries
            .iter()
            .find(|entry| entry.item.definition.production_key == key)
            .ok_or("recipe operand")?;
        let instance = item.item.item_instance_id;
        let handle = view
            .table()
            .handle(&ItemKey::Instance(instance))
            .ok_or("recipe operand handle")?;
        Ok((instance, handle))
    };
    let (ribbon_id, ribbon) = operand(RIBBON)?;
    let (foot_id, foot) = operand(FOOT)?;
    let actor = admitted.runtime_actor.ok_or("recipe actor")?;
    let use_pair = async |command_id, source, target, view: &SessionItemView| {
        crate::gameplay_transport::mining::dispatch_map_use(
            owner,
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence),
            },
            UseIntent {
                target: UseTarget::Item(source),
                use_with: None,
                use_with_map_item: Some(MapUseTarget::Item(target)),
                use_with_position: None,
            },
            Some(view),
        )
        .await
    };
    assert_eq!(
        use_pair(1599, ribbon, foot, &view).await.ok_or("inactive recipe")?.disposition,
        UseDisposition::NothingToUse
    );
    // Use the source-lowered start transition. Dialogue coverage lives in the NPC lane.
    let started = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            quest_fence,
            QuestTransitionRequest {
                transition_key: start_transition.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(1600).map_err(debug)?,
                )),
            },
            Arc::clone(owner.quest_catalogue.as_ref().ok_or("quest catalogue")?),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(started, QuestTransitionOutcome::Committed(_)), "{started:?}");
    assert_eq!(
        use_pair(1601, foot, ribbon, &view).await.ok_or("reversed recipe")?.disposition,
        UseDisposition::NothingToUse
    );
    for command_id in 1602..1602 + u64::try_from(count)? {
        assert_eq!(
            use_pair(command_id, ribbon, foot, &view).await.ok_or("ribbon on foot")?.disposition,
            UseDisposition::Committed
        );
    }
    // A resumed session no longer resolves either consumed handle.
    let resumed_view = SessionItemView::resume(Default::default());
    assert_eq!(
        use_pair(1602, ribbon, foot, &resumed_view).await.ok_or("recipe replay")?.disposition,
        UseDisposition::Committed
    );
    let quantities: Vec<(String, i64)> = sqlx::query_as(
        "SELECT definition_production_key, sum(quantity)::bigint
         FROM game_item_instances JOIN game_item_container_entries USING(item_instance_id)
         WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1
           AND definition_production_key IN ($2,$3,$4)
         GROUP BY definition_production_key",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(RIBBON)
    .bind(FOOT)
    .bind(CHARM)
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(quantities, [(CHARM.to_owned(), count)]);

    let receipts: (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE recipe=$2 AND consume_source
           AND source_item_instance_id=encode($3,'hex')::uuid
           AND (before_item->>'quantity')::bigint=$6::bigint-(command_id-1602)
           AND (ingredient_before_item->>'quantity')::bigint=$6::bigint-(command_id-1602)
           AND ingredient_item_instance_id=encode($4,'hex')::uuid
           AND ingredient_consumed_quantity=1
           AND jsonb_array_length(outputs)=1
           AND outputs #>> '{0,key}'=$5
           AND outputs #>> '{0,count}'='1')
         FROM game_item_craft_records WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(admitted.game_session_id.as_bytes().as_slice())
    .bind("oteryn:quest-craft/rottin-wood/lucky-charm")
    .bind(ribbon_id.as_slice())
    .bind(foot_id.as_slice())
    .bind(CHARM)
    .bind(count)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, (count, count));
    Ok(())
}

pub(super) async fn refuse_short_hand_in(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = install(owner, admitted, [ROTTIN]).await?;
    talk(owner, admitted, npc, 1701, "hi").await?;
    let before_xp = experience(h).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("dialogue clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let refusal = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("quest actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 1702,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: "yes".into(),
            },
        )
        .await;
    assert_eq!(refusal.disposition, NpcIntentDisposition::Rejected);
    assert!(!refusal.unknown);
    assert_eq!(
        refusal.conversation.ok_or("refusal reply")?.lines,
        ["Bring seven lucky charms."]
    );
    assert_eq!(experience(h).await?, before_xp);
    let actual: (i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1),
         (SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$2),
         (SELECT count(*) FROM game_character_quest_tracks
          WHERE (track_key=$3 AND value<>1) OR (track_key=$4 AND value<>0))",
    )
    .bind(CHARM)
    .bind(PAYOUT)
    .bind(TRACK)
    .bind(COOLDOWN)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(actual, (6, 0, 0));
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    repeat: bool,
) -> TestResult {
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = install(owner, admitted, [ROTTIN]).await?;
    talk(owner, admitted, npc, 1701, "hi").await?;
    let before_xp = experience(h).await?;
    let before_stamp = owner
        .quest_catalogue
        .as_ref()
        .and_then(|catalogue| catalogue.track(COOLDOWN))
        .ok_or("fixture cooldown track")?
        .initial;
    assert_eq!(
        talk(owner, admitted, npc, 1702, "yes").await?,
        ["Good hunt."]
    );
    assert_eq!(experience(h).await?, before_xp + 100);
    let actual: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1),
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$2 AND lifecycle=1),
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$3 AND lifecycle=1),
         (SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$4 AND mint_transaction_id IS NOT NULL)"
    ).bind(CHARM).bind(PLATINUM).bind(RING).bind(payout_key(repeat)).fetch_one(&h.pool).await?;
    assert!(matches!(actual, (0, 5, 0, 1) | (0, 3, 1, 1)), "{actual:?}");
    let (awarded_bundle, retained_odds, percentile): (serde_json::Value, i32, Option<i32>) =
        sqlx::query_as(
            "SELECT plan->'give', (plan #>> '{weighted_rewards,0,chance}')::integer,
                (plan->>'reward_percentile')::integer
         FROM game_quest_reward_sources WHERE transition_key=$1",
        )
        .bind(payout_key(repeat))
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(retained_odds, 9);
    let percentile = percentile.ok_or("weighted payout receipt lacks the recorded draw")?;
    assert!((0..100).contains(&percentile));
    assert_eq!(percentile >= 100 - retained_odds, actual.2 == 1);
    assert_eq!(awarded_bundle["item"], PLATINUM);
    assert_eq!(awarded_bundle["count"], actual.1);
    if actual.2 == 1 {
        assert_eq!(
            awarded_bundle["additional"],
            json!([{"item": RING, "count": 1}])
        );
    }
    let stage: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(TRACK)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(stage, 2);
    let (stamped, awarded): (bool, bool) = sqlx::query_as(
        "SELECT
         EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE track_key=$1 AND value > $3),
         EXISTS(SELECT 1 FROM game_account_achievements WHERE achievement_key=$2)",
    )
    .bind(COOLDOWN)
    .bind(ACHIEVEMENT)
    .bind(before_stamp)
    .fetch_one(&h.pool)
    .await?;
    assert!(stamped);
    assert_eq!(awarded, !repeat);
    Ok(())
}

pub(super) async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT total_experience FROM game_character_progression_state
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
