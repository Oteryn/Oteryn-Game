//! M10 real NPC wire commands compose fee/input custody, quest progress and visual receipts.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::reward_callbacks::{RewardCallback, load};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::presentation::{CAPABILITY_PRESENTATION_V1, PresentationEventKind};

#[test]
fn qnpcfx_m10_oldrak_repeatable_payment_items_and_npc_magic() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-qnpcfx-m10-oldrak")
}
#[test]
fn qnpcfx_m10_inquisition_five_real_interviews_progress_and_player_magic() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-qnpcfx-m10-inquisition")
}

type CueAt = (u32, (i64, i64, i64));
async fn cues(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    after: u64,
) -> TestResult<(u64, Vec<CueAt>)> {
    let (cursor, events) = owner
        .observe_sounds(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            after,
        )
        .await
        .ok_or("presentation journal")?;
    Ok((
        cursor,
        events
            .iter()
            .filter_map(|event| match &event.kind {
                PresentationEventKind::MagicEffect {
                    effect_id,
                    position,
                    ..
                } => Some((
                    *effect_id,
                    (
                        i64::from(position.x),
                        i64::from(position.y),
                        i64::from(position.floor),
                    ),
                )),
                _ => None,
            })
            .collect(),
    ))
}
fn callback<'a>(all: &'a [RewardCallback], suffix: &str) -> TestResult<&'a RewardCallback> {
    all.iter()
        .find(|c| c.key == format!("oteryn:quest-reward/qnpcfx-{suffix}"))
        .ok_or_else(|| format!("callback {suffix}").into())
}
async fn wire(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    c: &RewardCallback,
) -> TestResult<NpcIntentDisposition> {
    let npc = {
        let runtime = owner.runtime.lock().await;
        runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find_map(|npc| {
                (runtime.npc_key(npc.actor) == Some(c.npc.as_str())).then_some(
                    oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                        identity: npc.actor.placement_identity(),
                        generation: npc.generation,
                    },
                )
            })
            .ok_or("placed M10 NPC")?
    };
    let (result, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, c, false).await?;
    Ok(result)
}
async fn item(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
    count: u16,
    n: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: key.into(),
            count,
        },
        n,
        true,
    )
    .await
}
async fn count(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(key).fetch_one(&h.pool).await?)
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
    selected.extend([3, 6, 25, CAPABILITY_PRESENTATION_V1]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    if name.ends_with("oldrak") {
        super::classic_quests::install(owner, admitted, ["oteryn:npc.oldrak"]).await?;
    } else {
        super::classic_quests::install_with_offsets(
            owner,
            admitted,
            [
                "oteryn:npc.henricus",
                "oteryn:npc.kulag_the_guard",
                "oteryn:npc.grof_the_guard",
                "oteryn:npc.tim_the_guard",
                "oteryn:npc.walter_the_guard",
                "oteryn:npc.miles_the_guard",
            ],
            Some([(-1, -1), (0, -1), (1, -1), (-1, 1), (0, 1), (1, 1)]),
        )
        .await?;
    }
    let all = load().map_err(debug)?;
    let (mut cursor, _) = cues(owner, admitted, 0).await?;
    if name.ends_with("oldrak") {
        assert_eq!(
            wire(owner, admitted, callback(&all, "oldrak-begin")?).await?,
            NpcIntentDisposition::Ok
        );
        let paid = callback(&all, "oldrak-hallowed-axe")?;
        assert_eq!(
            wire(owner, admitted, paid).await?,
            NpcIntentDisposition::Rejected,
            "missing axe and funds"
        );
        assert!(cues(owner, admitted, cursor).await?.1.is_empty());
        item(owner, admitted, "oteryn:item.tibia.i3274", 1, 61000).await?;
        assert_eq!(
            wire(owner, admitted, paid).await?,
            NpcIntentDisposition::Rejected,
            "no funds cannot consume axe"
        );
        assert_eq!(count(h, "oteryn:item.tibia.i3274").await?, 1);
        assert!(cues(owner, admitted, cursor).await?.1.is_empty());
        item(owner, admitted, "oteryn:item.tibia.i3035", 20, 61001).await?;
        for turn in 0..2 {
            if turn == 1 {
                item(owner, admitted, "oteryn:item.tibia.i3274", 1, 61002).await?;
            }
            assert_eq!(wire(owner, admitted, paid).await?, NpcIntentDisposition::Ok);
            let (next, events) = cues(owner, admitted, cursor).await?;
            cursor = next;
            let position = {
                let runtime = owner.runtime.lock().await;
                let npc = runtime
                    .visible_entities()
                    .npcs
                    .into_iter()
                    .find(|n| runtime.npc_key(n.actor) == Some(paid.npc.as_str()))
                    .ok_or("Oldrak")?;
                (
                    i64::from(npc.position.x),
                    i64::from(npc.position.y),
                    i64::from(npc.position.floor),
                )
            };
            assert_eq!(events, vec![(49, position)], "committed cue at NPC tile");
            assert_eq!(
                count(h, "oteryn:item.tibia.i919").await?,
                i64::from(turn + 1)
            );
            assert_eq!(count(h, "oteryn:item.tibia.i3274").await?, 0);
        }
        assert_eq!(count(h, "oteryn:item.tibia.i3035").await?, 0);
        let fees:(i64,i64)=sqlx::query_as("SELECT count(*),coalesce(sum(fee_gold_units),0)::bigint FROM game_item_fee_burns WHERE cause_kind=18").fetch_one(&h.pool).await?;
        assert_eq!(fees, (2, 2000));
        let plans:Vec<serde_json::Value>=sqlx::query_scalar("SELECT plan FROM game_quest_reward_sources WHERE transition_key=$1 ORDER BY cause_ordinal LIMIT 3").bind(&paid.key).fetch_all(&h.pool).await?;
        assert_eq!(plans.len(), 2);
        for plan in plans {
            assert_eq!(plan["magic_effects"][0]["effect"], "CONST_ME_YELLOWENERGY");
            assert_eq!(plan["magic_effects"][0]["target"], "npc");
        }
    } else {
        let p = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/";
        assert_eq!(
            wire(owner, admitted, callback(&all, "inquisition-join")?).await?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(
            wire(owner, admitted, callback(&all, "inquisition-start")?).await?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(
            wire(owner, admitted, callback(&all, "inquisition-report")?).await?,
            NpcIntentDisposition::Rejected
        );
        assert!(cues(owner, admitted, cursor).await?.1.is_empty());
        for (i, guard) in ["kulag", "grof", "tim", "walter", "miles"]
            .into_iter()
            .enumerate()
        {
            let c = callback(&all, &format!("inquisition-{guard}_guard"))?;
            assert_eq!(
                wire(owner, admitted, c).await?,
                NpcIntentDisposition::Ok,
                "{guard}"
            );
            let (next, events) = cues(owner, admitted, cursor).await?;
            cursor = next;
            let at = {
                let runtime = owner.runtime.lock().await;
                let pos = runtime
                    .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                    .map_err(debug)?
                    .position();
                (i64::from(pos.x), i64::from(pos.y), i64::from(pos.floor))
            };
            assert_eq!(
                events,
                vec![(50, at)],
                "exact one committed player cue: {guard}"
            );
            assert_eq!(
                super::classic_quests::progress(
                    owner,
                    admitted,
                    authority,
                    &format!("{p}{guard}_guard")
                )
                .await?,
                1
            );
            assert_eq!(
                super::classic_quests::progress(
                    owner,
                    admitted,
                    authority,
                    &format!("{p}mission01")
                )
                .await?,
                i64::try_from(i)? + 2
            );
            let plan: serde_json::Value = sqlx::query_scalar(
                "SELECT plan FROM game_quest_reward_sources WHERE transition_key=$1 LIMIT 1",
            )
            .bind(&c.key)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(plan["magic_effects"][0]["effect"], "CONST_ME_HOLYAREA");
        }
        assert_eq!(
            wire(owner, admitted, callback(&all, "inquisition-report")?).await?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, &format!("{p}mission01"))
                .await?,
            7
        );
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, &format!("{p}questline"))
                .await?,
            3
        );
        assert!(
            cues(owner, admitted, cursor).await?.1.is_empty(),
            "report has no invented cue"
        );
        let completed: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE completes",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(completed, 0, "interrogation is not whole-quest completion");
    }
    Ok(())
}
