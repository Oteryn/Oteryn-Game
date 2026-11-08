//! M16: the production read adapter re-proves the admitted controller and transmits real facts.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    reward_callbacks::{self, RewardCallback},
};
use oteryn_protocol_oteryn::personal_map_marks::PersonalMapMarks;
use std::sync::Arc;
#[test]
fn questpc5_personal_map_query_reproves_controller_and_reads_postgres_facts() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-questpc5-personal-map-query")
}
struct NoItems<'a>(&'a NativeGameplayState);
impl ItemWeightLookup for NoItems<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn weight_of(&self, _: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        Err(ItemWeightRefused::Unknown)
    }
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
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.retain(|id| *id != 449);
    let mut offered = owner.offered_capabilities().to_vec();
    if !offered.iter().any(|row| row.id == 449) {
        offered.push(crate::gameplay_transport::capabilities::OfferedCapability {
            id: 449,
            requires: &[3, 18],
        });
        offered.sort_by_key(|row| row.id);
    }
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("initial selection")?;
    assert!(
        owner.personal_map_marks_query(*admitted).await.is_none(),
        "an unselected capability cannot disclose personal marks"
    );
    selected.extend([3, 6, 18, 449]);
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("selection")?;
    let empty = owner
        .personal_map_marks_query(*admitted)
        .await
        .ok_or("empty current view")?;
    assert_eq!(
        PersonalMapMarks::decode(&empty),
        Some(PersonalMapMarks::default())
    );
    const KEY: &str = "oteryn:quest-reward/questpc5-personal-query-fixture";
    const QUEST: &str = "oteryn:quest.targuna_quest";
    const TRACK: &str = "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/long_lost_treasure/treasure_marked";
    let callback: RewardCallback = serde_json::from_value(serde_json::json!({
        "key":KEY,"quest":QUEST,"npc":"oteryn:npc.sterling","keywords":["yes"],"topic":2,"next_topic":0,
        "storage":[{"track":TRACK,"minimum":0,"maximum":0,"set":1}],"take":[],"give":null,"outfit":null,"achievement":null,"reply":["Marked."],"sources":["M16 read-adapter fixture"],
        "map_mark":{"position":{"x":31948,"y":31924,"floor":-7},"kind":"cross","text":"Treasure"}
    }))?;
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            canonical.content_revision(),
            vec![canonical.track(TRACK).cloned().ok_or("track")?],
            vec![reward_callbacks::primary_transition(&callback).map_err(debug)?],
        )
        .map_err(debug)?,
    );
    owner.quest_catalogue = Some(catalogue.clone());
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let weights = NoItems(native);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    let outcome = owner
        .root
        .commit_fixture_quest_reward(
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: KEY.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(9801).map_err(debug)?,
                )),
            },
            catalogue,
            QuestRewardAdmission {
                native,
                weights: &weights,
                fee_payment: None,
                actor_conditions: None,
                carried_use: None,
                peer_use: None,
            },
            callback,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let before = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
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
    Ok(())
}
