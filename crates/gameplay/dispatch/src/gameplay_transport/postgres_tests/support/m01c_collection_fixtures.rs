use super::TestResult;

use super::fixture::{Harness, debug};

use oteryn_gameplay_quests::quest::QuestStateCatalogue;

use oteryn_gameplay_quests::quest::authored_bindings::Bindings;

pub(super) const BINDING: &str = r#"{
  "schema":1,"quests":[{
    "quest":"oteryn:quest.black_knight_quest",
    "recipe_sha256":"187d512b68cabfe55e1d114362d97fe5ac410fd256af3adc68e9b4b413c4ca53",
    "sources":["M01c engine fixture; not production quest qualification"],
    "tracks":[{"key":"oteryn:quest-progress/m01c/collected","initial":0,"minimum":0,"maximum":1}],
    "stages":[{"key":"s1","completes":false,"trigger":{"kind":"collect","collection":{
      "key":"oteryn:quest-collection/m01c/black-knight-key",
      "items":[{"item":"oteryn:item.tibia.i2969","count":1,"key_number":5010}],
      "storage":[{"track":"oteryn:quest-progress/m01c/collected","minimum":0,"maximum":0,"set":1}],
      "sources":["fixture: exact key number, not interchangeable with another key"]
    }}}]
  }]
}"#;

pub(super) async fn exercise(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
    use std::sync::Arc;
    let mut document: serde_json::Value = serde_json::from_str(BINDING)?;
    document["quests"][0]["stages"][0]["trigger"]["collection"]["items"] =
        serde_json::json!([{"item":"oteryn:item.tibia.i3035","count":2}]);
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?).map_err(debug)?;
    // This isolated engine fixture must not qualify a canonical quest whose source
    // route still has data holds. Production catalogues retain recipe admission.
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base).map_err(debug)?));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let offer = Offering {
        position: Position {
            x: 33294,
            y: 31677,
            floor: -15,
        },
        item: "oteryn:item.tibia.i3035".into(),
        count: 1,
    };
    let fence = admitted.item_fence.ok_or("item fence")?;
    super::quest_world_callbacks_fixtures::input_item(owner, fence, &offer, 7001, true).await?;
    async fn commit(
        owner: &super::ComposedFreshAdmission<'_, '_, '_>,
        session: crate::foundation::GameSessionId,
        ordinal: u64,
    ) -> TestResult<QuestTransitionOutcome> {
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "quest fence read")?
            .ok_or("quest fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        slot.commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: "oteryn:quest-collection/m01c/black-knight-key".into(),
                cause: QuestCause::Command(CommandRef::new(
                    session,
                    CommandId::new(ordinal).map_err(debug)?,
                )),
            },
            Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
        )
        .await
        .map_err(debug)
    }
    assert!(
        matches!(
            commit(owner, admitted.game_session_id, 71010).await?,
            QuestTransitionOutcome::Refused(
                oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch
            )
        ),
        "one coin cannot satisfy two units"
    );
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-collection/m01c/black-knight-key'").fetch_one(&h.pool).await?;
    assert_eq!(receipts, 0, "a short collection writes no receipt");
    super::quest_world_callbacks_fixtures::input_item(owner, fence, &offer, 7002, true).await?;
    let before: serde_json::Value = sqlx::query_scalar("SELECT jsonb_agg(jsonb_build_array(i.item_instance_id,i.quantity) ORDER BY i.item_instance_id) FROM game_item_instances i WHERE i.definition_production_key='oteryn:item.tibia.i3035' AND i.lifecycle=1").fetch_one(&h.pool).await?;
    crate::gameplay_transport::quest_collections::after_move(
        owner,
        admitted.runtime_actor.ok_or("runtime actor")?,
        crate::gameplay_transport::connection::UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 71020,
            item_fence: admitted.item_fence,
        },
    )
    .await;
    let collected: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/m01c/collected'").fetch_one(&h.pool).await?;
    assert_eq!(
        collected, 1,
        "the native move consumer advances the collected stage"
    );
    assert!(
        matches!(
            commit(owner, admitted.game_session_id, 71020).await?,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ),
        "same committed move replays the original collection result"
    );
    let after: serde_json::Value = sqlx::query_scalar("SELECT jsonb_agg(jsonb_build_array(i.item_instance_id,i.quantity) ORDER BY i.item_instance_id) FROM game_item_instances i WHERE i.definition_production_key='oteryn:item.tibia.i3035' AND i.lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(
        before, after,
        "COLLECT never consumes, replaces or duplicates carried Items"
    );
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-collection/m01c/black-knight-key'").fetch_one(&h.pool).await?;
    assert_eq!(receipts, 1, "exactly one durable collection receipt");
    Ok(())
}
