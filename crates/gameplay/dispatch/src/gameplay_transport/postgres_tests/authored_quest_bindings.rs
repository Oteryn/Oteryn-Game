//! Generic binding execution fixtures; these are not source authoring for the named quest.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::authored_bindings::{
    Bindings, BoundQuest, Stage, Track, Trigger,
};
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const TRACK: &str = "oteryn:quest-progress/authored-engine/phase";

#[test]
fn authored_bindings_npc_delivery_and_carried_use_dispatch_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-authored-bindings")
}

fn callback(carried: bool) -> TestResult<RewardCallback> {
    let item = "oteryn:item.tibia.i3031";
    Ok(serde_json::from_value(serde_json::json!({
        "key": if carried {"oteryn:quest-reward/authored-engine-use"} else {"oteryn:quest-reward/authored-engine-talk"},
        "quest": QUEST, "npc":"oteryn:npc.cledwyn", "keywords":["enginebinding"],
        "topic":0, "next_topic":0,
        "carried_item": if carried {Some(item)} else {None},
        "carried_exchange": carried,
        "storage":[{"track":TRACK,"minimum":i64::from(carried),"maximum":i64::from(carried),"set":if carried {2} else {1}}],
        "take":if carried {serde_json::json!([{"item":item,"count":1},{"item":"oteryn:item.tibia.i5901","count":1}])} else {serde_json::json!([])},
        "give":serde_json::json!({"item":item,"count":if carried {5} else {1},"contents":[]}),
        "outfit":null,"achievement":null,"reply":["Bound engine stage."],
        "sources":["engine fixture: explicit native identities; no quest source claim"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let talk = callback(false)?;
    let use_ = callback(true)?;
    let bindings = Bindings {
        schema: 1,
        quests: vec![BoundQuest {
            quest: QUEST.into(),
            recipe_sha256: "1".repeat(64),
            occurrence: None,
            admission: None,
            tracks: vec![Track {
                key: TRACK.into(),
                initial: 0,
                minimum: 0,
                maximum: 2,
            }],
            stages: vec![
                Stage {
                    key: "npc-delivery".into(),
                    completes: false,
                    trigger: Trigger::NpcDialogue {
                        callback: Box::new(talk.clone()),
                        alternatives: Vec::new(),
                    },
                },
                Stage {
                    key: "carried-use".into(),
                    completes: true,
                    trigger: Trigger::CarriedUse {
                        callback: Box::new(use_),
                    },
                },
            ],
            sources: vec!["engine fixture: no production quest qualification".into()],
        }],
    };
    let base = QuestStateCatalogue::empty(
        owner
            .quest_catalogue
            .as_ref()
            .ok_or("catalogue")?
            .content_revision(),
    )
    .ok_or("empty catalogue")?;
    let bytes = serde_json::to_vec(&bindings)?;
    let bindings = Bindings::parse(&bytes)?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
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
    owner.states.chat.admit(
        admitted.game_session_id,
        "Binding witness",
        owner.owner_now(),
    );
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.cledwyn"]).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let before = super::qengine_voucher_stories::quantity(h, 3031).await?;
    let (result, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, &talk, false).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, 3031).await?,
        before + 1,
        "NPC delivers exactly the authored native item"
    );
    let wood_before = super::qengine_voucher_stories::quantity(h, 5901).await?;
    assert_eq!(
        wood_before, 0,
        "fixture starts without the additional input"
    );
    super::qengine_voucher_stories::use_native(owner, admitted, 3031, UseDisposition::Rejected)
        .await?;
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, 3031).await?,
        before + 1,
        "missing ingredient rolls back selected input and reward"
    );
    let mut ordinal = 99200;
    super::qengine_voucher_stories::mint(owner, admitted, 5901, 1, &mut ordinal).await?;
    super::qengine_voucher_stories::use_native(owner, admitted, 3031, UseDisposition::Committed)
        .await?;
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, 3031).await?,
        before + 5,
        "carried USE consumes one item and delivers five, including literal replay"
    );
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, 5901).await?,
        0,
        "the additional ingredient is consumed atomically"
    );
    let phase: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
    )
    .bind(QUEST)
    .bind(TRACK)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(phase, 2);
    let completions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        completions, 1,
        "completion and item consumption share the durable transition"
    );
    let (result, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, &talk, false).await?;
    let _ = result;
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, 3031).await?,
        before + 5
    );
    Ok(())
}
