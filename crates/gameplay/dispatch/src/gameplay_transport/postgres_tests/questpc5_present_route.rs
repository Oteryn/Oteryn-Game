//! Actual Present chest custody -> Seymour's source exchange -> durable once reward.
use super::TestResult;

pub(super) const CLAIM: &str = "oteryn:reward-claim.kv.present_box";
pub(super) const CALLBACK: &str = "oteryn:quest-reward/present-box";
pub(super) const TRACK: &str = "oteryn:quest-progress/native-reward/present/seymour";

#[test]
fn questpc5_present_complete_source_route() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_present_route",
            false,
            false,
        )
        .await?;
        eprintln!(
            "POSTGRES_CASE_STARTED present_source_exchange {:?}",
            started.elapsed()
        );
        assert!(
            oteryn_gameplay_quests::quest::reward_callbacks::load()?
                .iter()
                .any(|c| c.key == CALLBACK),
            "Seymour's source Present exchange has no admitted native callback"
        );
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED present_source_exchange_registration {:?}",
            started.elapsed()
        );
        Ok(())
    })?;
    super::quest_catalogue_chests::present_route_cohort()
}

use super::fixture::debug;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

type Child = (String, String, i64, i32, Option<i32>);
async fn children(h: &super::fixture::Harness, root: &[u8; 16]) -> TestResult<Vec<Child>> {
    Ok(sqlx::query_as("SELECT i.item_instance_id::text,i.definition_production_key,i.quantity,n.placement_ordinal::integer,f.fluid_subtype::integer FROM game_item_nested_entries n JOIN game_item_instances i USING(item_instance_id) LEFT JOIN game_item_fluid_states f USING(item_instance_id) WHERE n.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY n.placement_ordinal")
        .bind(root.as_slice()).fetch_all(&h.pool).await?)
}
async fn counts(h: &super::fixture::Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_reward_sources),(SELECT count(*) FROM game_quest_callback_consumptions),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_account_appearance_unlock_receipts)").fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &super::fixture::Harness,
    actor: ExactActorRef,
) -> TestResult {
    let started = std::time::Instant::now();
    let fence = admitted.item_fence.ok_or("item fence")?;
    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    assert_eq!(bag.entries.len(), 1);
    assert_eq!(
        bag.entries[0].item.definition.production_key,
        "oteryn:item.tibia.i2854"
    );
    let root = bag.entries[0].item.item_instance_id;
    let before = children(h, &root).await?;
    assert_eq!(before.len(), 4);
    let present: Vec<_> = before
        .iter()
        .filter(|r| r.1 == "oteryn:item.tibia.i2856")
        .collect();
    assert_eq!(present.len(), 1);
    assert_eq!(present[0].2, 1);
    let present_id = present[0].0.clone();
    assert_eq!(
        before.iter().filter(|r| r.4 == Some(1)).count(),
        2,
        "both exact source water children exist"
    );
    assert_eq!(counts(h).await?, (0, 0, 1, 0));
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_ne!(state.tracks().get(TRACK), Some(&1));
    let source: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/npc-placements/placements-00000-00033.json"
    )))?;
    let seymour: Vec<_> = source["records"]
        .as_array()
        .ok_or("NPC placements")?
        .iter()
        .filter(|r| r["declaration"]["npc"] == "oteryn:npc.seymour")
        .collect();
    assert_eq!(seymour.len(), 1);
    assert_eq!(
        seymour[0]["declaration"]["cell"],
        serde_json::json!({"x":32104,"y":32189,"floor":7})
    );
    super::quest_catalogue_chests::relocate_source_actor(
        owner,
        admitted,
        actor,
        Position {
            x: 32104,
            y: 32188,
            floor: -7,
        },
    )
    .await?;
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.seymour"]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("NPC capabilities")?;
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key == CALLBACK)
        .ok_or("Present callback")?;
    let (disposition, _, command) =
        super::qengine_rewards::wire_grant(owner, admitted, npc, &callback).await?;
    assert_eq!(disposition, NpcIntentDisposition::Ok);
    let after = counts(h).await?;
    assert_eq!(after, (1, 1, 2, 0));
    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    let mut keys: Vec<_> = bag
        .entries
        .iter()
        .map(|e| e.item.definition.production_key.as_str())
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["oteryn:item.tibia.i2854", "oteryn:item.tibia.i3374"]);
    assert_eq!(
        children(h, &root).await?,
        before
            .into_iter()
            .filter(|r| r.0 != present_id)
            .collect::<Vec<_>>(),
        "only the nested Present is consumed; original water/torch identities and state survive"
    );
    let consumed:(i32,String) = sqlx::query_as("SELECT quantity::integer,location_kind FROM game_quest_callback_consumptions WHERE item_instance_id=$1::uuid").bind(&present_id).fetch_one(&h.pool).await?;
    assert_eq!(consumed, (1, "nested".into()));
    let active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances WHERE item_instance_id=$1::uuid AND lifecycle=1",
    )
    .bind(&present_id)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(active, 0);
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(TRACK), Some(&1));
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        7,
        oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc,
            text: "yes".into(),
        })
        .map_err(debug)?,
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&reply).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        counts(h).await?,
        after,
        "literal confirmation replay cannot duplicate input/output"
    );
    let (again, _, _) = super::qengine_rewards::wire_grant(owner, admitted, npc, &callback).await?;
    assert_eq!(again, NpcIntentDisposition::Ok);
    assert_eq!(
        counts(h).await?,
        after,
        "selected once recipe cannot exchange twice"
    );
    assert_eq!(
        owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?,
        state
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED present_actual_claim_nested_exchange_replay_once {:?}",
        started.elapsed()
    );
    Ok(())
}
