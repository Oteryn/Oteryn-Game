//! The approved on-demand Kingsday recipe through literal movement and NPC commands.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::log::questpc_authored::{QUEST, TRACK};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use std::sync::Arc;

#[test]
fn questpc_kingsday_visit_greet_journal_once_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-questpc-kingsday")
}

async fn phase(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    )
    .bind(TRACK)
    .fetch_one(&h.pool)
    .await?)
}
async fn travel(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: Position,
) -> TestResult {
    // Use the existing sealed relocation proof for travel setup, never quest writes.
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let map = owner.bundle_map().ok_or("map")?;
    let closed = crate::map::floor::BundleDoors(map.facts.doors());
    let mut runtime = owner.runtime.lock().await;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        owner.movement_cells,
        actor,
        admitted.game_session_id,
        expected,
        super::quest_world_callbacks::local(at),
        false,
        &closed,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    println!("POSTGRES_CASE_STARTED questpc_kingsday server_version={version}");
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let journal = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)
        .map_err(debug)?;
    let (catalogue, journal) = oteryn_gameplay_quests::quest::log::questpc_authored::extend(
        owner.quest_catalogue.as_ref().ok_or("catalogue")?,
        &journal.content,
    )?;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load_for(&catalogue)?;
    let greet = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/questpc-kingsday-greet")
        .ok_or("Kingsday greeting binding missing")?
        .clone();
    let finish = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/questpc-kingsday-finish")
        .ok_or("Kingsday journal binding missing")?
        .clone();
    owner.quest_catalogue = Some(Arc::new(catalogue));
    owner.quest_log_content = Some(
        crate::gameplay_transport::quest_log::QuestLogLines::new(journal.clone())
            .ok_or("journal lines")?,
    );
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
            .ok_or("Kingsday capabilities")?;
    let king_adjacent = Position {
        x: 32311,
        y: 32171,
        floor: -6,
    };
    travel(owner, admitted, king_adjacent).await?;
    let npc_catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = npc_catalogue.service_model_for_project().map_err(debug)?;
    assert!(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model)
            .get("oteryn:npc.king_tibianus")
            .is_some(),
        "production promotion service already admits royal dialogue"
    );
    let [king] =
        super::classic_quests::install(owner, admitted, ["oteryn:npc.king_tibianus"]).await?;
    assert_ne!(
        super::qengine_rewards::wire_reward(owner, admitted, king, &greet, false)
            .await?
            .0,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    assert_eq!(
        phase(h).await?,
        0,
        "a greeting cannot replace the courtyard visit"
    );
    assert_ne!(
        super::qengine_rewards::wire_reward(owner, admitted, king, &finish, false)
            .await?
            .0,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    assert_eq!(phase(h).await?, 0, "completion cannot skip either stage");
    travel(
        owner,
        admitted,
        Position {
            x: 32312,
            y: 32182,
            floor: -7,
        },
    )
    .await?;
    use oteryn_protocol_oteryn::world_spatial::*;
    let next = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        next,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        phase(h).await?,
        1,
        "literal courtyard entry earns exploration"
    );
    travel(owner, admitted, king_adjacent).await?;
    assert_eq!(
        super::qengine_rewards::wire_reward(owner, admitted, king, &greet, false)
            .await?
            .0,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    assert_eq!(phase(h).await?, 2);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let (_, _, finish_command) =
        super::qengine_rewards::wire_reward(owner, admitted, king, &finish, false).await?;
    assert_eq!(phase(h).await?, 3);
    let completed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL").bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(completed, 1);
    let achievements: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/loyal_subject'").fetch_one(&h.pool).await?;
    assert_eq!(achievements, 1);
    let before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(QUEST)
            .fetch_one(&h.pool)
            .await?;
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
        &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
            npc_actor: king,
            text: "kingsday".into(),
        },
    )
    .map_err(debug)?;
    super::quest_world_callbacks::wire(owner, admitted, finish_command, 7, payload).await?;
    super::qengine_rewards::wire_reward(owner, admitted, king, &finish, false).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(phase(h).await?, 3);
    let observation = owner
        .quest_log_observation(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            None,
        )
        .await;
    let crate::gameplay_transport::quest_log::QuestLogObservation::Changed(source) = observation
    else {
        return Err("reloaded Kingsday journal unavailable".into());
    };
    let index = journal
        .active()
        .quests()
        .iter()
        .position(|q| q.key == QUEST)
        .ok_or("Kingsday journal entry")?;
    let line = journal
        .line(source.copy.as_ref(), u32::try_from(index)?)
        .ok_or("visible Kingsday journal")?;
    assert!(line.completed);
    assert_eq!(line.missions.len(), 1);
    assert!(line.missions[0].done);
    assert!(line.missions[0].text.contains("Loyal Subject"));
    let after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(QUEST)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(after, before, "replay and fresh repeat create no receipts");
    let achievements: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/loyal_subject'").fetch_one(&h.pool).await?;
    assert_eq!(achievements, 1);
    Ok(())
}
