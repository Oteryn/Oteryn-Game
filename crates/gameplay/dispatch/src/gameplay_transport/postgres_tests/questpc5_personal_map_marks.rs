//! M16: real Sterling dialogue commits a personal fact, then the read adapter re-proves its controller.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, reward_callbacks};
use oteryn_protocol_oteryn::{
    npc_service::NpcIntentDisposition, personal_map_marks::PersonalMapMarks,
};
use std::sync::Arc;
const KEY: &str = "oteryn:quest-reward/targuna/sterling-mark-treasure";
const PREFIX: &str = "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/long_lost_treasure/";
#[test]
fn questpc5_personal_map_query_reproves_controller_and_reads_postgres_facts() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-questpc5-personal-map-query")
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M16_QUERY_POSTGRES_EXECUTED server={version}");
    let callback = reward_callbacks::load()
        .map_err(debug)?
        .into_iter()
        .find(|row| row.key == KEY)
        .ok_or("production Sterling callback")?;
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    // A stage-ready Character fixture, not a weaker write to globally protected tracks.
    // The production callback and its exact bound transition are unchanged.
    let tracks = [("mission", 3), ("morla_done", 1), ("treasure_marked", 0)]
        .into_iter()
        .map(|(name, initial)| {
            let mut track = canonical
                .track(&format!("{PREFIX}{name}"))
                .cloned()
                .ok_or("track")?;
            track.initial = initial;
            Ok(track)
        })
        .collect::<Result<Vec<_>, &str>>()?;
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            canonical.content_revision(),
            tracks,
            vec![reward_callbacks::primary_transition(&callback).map_err(debug)?],
        )
        .map_err(debug)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("current Character controller")?;
    let fence = current.item_fence();
    admitted.item_fence = Some(fence);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    assert!(
        crate::gameplay_transport::battle_instinct_status::actor_is_current(
            &*owner.runtime.lock().await,
            *admitted,
        )
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.retain(|id| *id != 449);
    let offered = owner.offered_capabilities();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(offered, &selected)
            .ok_or("initial selection")?;
    assert!(owner.personal_map_marks_query(*admitted).await.is_none());
    selected.extend([3, 4, 6, 18, 449]);
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(offered, &selected)
            .ok_or("selection")?;
    assert!(
        offered.iter().any(|entry| entry.id == 449),
        "World offers personal marks"
    );
    assert!(
        admitted.continuity.selected_capabilities.contains(449),
        "the full World map dependency closure was negotiated"
    );
    assert!(
        owner.personal_map_marks_available(),
        "composed capability hook is installed"
    );
    {
        let permit = owner
            .spell_lane_permit()
            .await
            .ok_or("map query lane unavailable")?;
        drop(permit);
    }
    let read_fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("map query current fence")?;
    assert_eq!(Some(read_fence.item_fence()), admitted.item_fence);
    let root_view = owner
        .root
        .read_character_quest_map_marks(owner.character, owner.holder, read_fence)
        .await
        .map_err(debug)?;
    assert!(root_view.marks.is_empty());
    let empty = owner
        .personal_map_marks_query(*admitted)
        .await
        .ok_or("empty current view")?;
    assert_eq!(
        PersonalMapMarks::decode(&empty),
        Some(PersonalMapMarks::default())
    );
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.sterling"]).await?;
    for word in ["hi", "mission", "yes"] {
        let command = admitted.continuity.next_command_id;
        admitted.continuity.next_command_id = command.checked_add(1).ok_or("command overflow")?;
        assert_eq!(
            super::qengine_rewards::talk(owner, admitted, npc, command, word).await?,
            NpcIntentDisposition::Ok,
            "Sterling {word}"
        );
    }
    let before = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    let state = owner
        .root
        .read_character_quest_state(owner.character, before.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(&format!("{PREFIX}mission")), Some(&4));
    assert_eq!(
        state.tracks().get(&format!("{PREFIX}treasure_marked")),
        Some(&1)
    );
    let payload = owner
        .personal_map_marks_query(*admitted)
        .await
        .ok_or("current marked view")?;
    let marks = PersonalMapMarks::decode(&payload).ok_or("wire")?;
    assert_eq!(marks.0.len(), 1);
    assert_eq!(
        (
            marks.0[0].key.as_str(),
            marks.0[0].x,
            marks.0[0].y,
            marks.0[0].floor,
            marks.0[0].text.as_str()
        ),
        (KEY, 31948, 31924, -7, "Treasure")
    );
    assert_eq!(
        owner.personal_map_marks_query(*admitted).await,
        Some(payload)
    );
    let after = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    assert_eq!(
        before.expected_character_revision, after.expected_character_revision,
        "display reads never advance Character revision"
    );
    let mut stale = *admitted;
    stale.continuity.connection_generation += 1;
    assert!(owner.personal_map_marks_query(stale).await.is_none());
    let mut stale = *admitted;
    stale
        .item_fence
        .as_mut()
        .ok_or("item fence")?
        .character_lease_generation += 1;
    assert!(owner.personal_map_marks_query(stale).await.is_none());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_map_marks")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 1);
    eprintln!("M16_STERLING_DIALOGUE_PROVEN mission=4 marker=Treasure personal_rows=1");
    Ok(())
}
