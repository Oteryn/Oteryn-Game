//! Crystal 201189f8 data-global/npc/emiliana.lua:72-84,209-242.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition, reward_callbacks::RewardCallback,
};
use serde_json::json;

const QUEST: &str = "oteryn:quest.targuna_quest";
const NPC: &str = "oteryn:npc.emiliana";
const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/burning_heart/mission";
const ADVANCE: &str = "oteryn:quest-reward/targuna/emiliana/crimson-court-briefing";
const BRIEFING: &str = "Your task is to do exactly that, find out where the Herald resides and defeat him to end his reign of terror once and for all.";
const SURFACE: &str = "Go upstairs, open the door and walk through the portal. I'll be waiting for you on the other side.";

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    owner.quest_catalogue = Some(std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: MISSION.into(),
                quest: QUEST.into(),
                min: 1,
                max: 4,
                initial: 1,
            }],
            vec![QuestTransition {
                key: ADVANCE.into(),
                quest: QUEST.into(),
                effects: vec![QuestEffect {
                    track: MISSION.into(),
                    from: QuestComparison::Eq(1),
                    effect: QuestEffectKind::Set(2),
                }],
                completes: false,
                experience: None,
                world_change: None,
            }],
        )
        .map_err(debug)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    owner
        .states
        .npc_conversations
        .fixture_reward_callbacks(vec![serde_json::from_value::<RewardCallback>(json!({
            "key": ADVANCE, "quest": QUEST, "npc": NPC,
            "keywords": ["mission"], "topic": 0, "next_topic": 0,
            "storage": [{"track": MISSION, "minimum": 1, "maximum": 1, "set": 2}],
            "placement": {"npc_floor_minimum": -15, "npc_floor_maximum": -8,
                "player_area": {
                    "minimum": {"x": 32409, "y": 32683, "floor": -12},
                    "maximum": {"x": 32420, "y": 32693, "floor": -12}
                }},
            "take": [], "give": null, "outfit": null, "achievement": null, "reply": [BRIEFING],
            "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/emiliana.lua:209-242"]
        }))?])?;
    owner.states.npc_conversations.fixture_reward_offers(
        &json!({
            "schema": 1, "offers": [{
                "npc": NPC, "keywords": ["mission"], "topic": 0, "next_topic": 0, "storage": [],
                "placement": {"npc_floor_minimum": -7, "npc_floor_maximum": 0}, "reply": [SURFACE]
            }]
        })
        .to_string(),
    )?;
    let [npc] = super::classic_quests_fixtures::install(owner, admitted, [NPC]).await?;
    let first = admitted.continuity.next_command_id;
    super::classic_quests_fixtures::talk(owner, admitted, npc, first, "hi").await?;
    let response =
        super::classic_quests_fixtures::talk(owner, admitted, npc, first + 1, "mission").await?;
    let progress: Option<i64> =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(MISSION)
            .fetch_optional(&h.pool)
            .await?;
    match name {
        "qewrath-emiliana-inside" => {
            assert_eq!(response, [BRIEFING]);
            assert_eq!(progress, Some(2));
        }
        "qewrath-emiliana-outside" => {
            assert!(
                !response
                    .iter()
                    .any(|line| line == BRIEFING || line == SURFACE)
            );
            assert_eq!(progress, None);
        }
        "qewrath-emiliana-surface" => {
            assert_eq!(response, [SURFACE]);
            assert_eq!(progress, None);
        }
        _ => return Err("unknown Emiliana fixture".into()),
    }
    Ok(())
}
