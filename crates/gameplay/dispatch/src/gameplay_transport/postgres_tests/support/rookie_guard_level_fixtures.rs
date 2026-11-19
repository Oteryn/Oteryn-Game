//! Crystal 201189f8 data-global/npc/vascalir.lua:64-74,903-912.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    npc_rules::NpcRuleCatalog,
};

pub(super) const NAME: &str = "rookie-guard-level-warning";
const NPC: &str = "oteryn:npc.vascalir";
const QUEST: &str = "oteryn:quest.the_rookie_guard_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/questline";
const WARNING: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/level8_warning";
const START: &str = "oteryn:quest-transition/rookie-guard-active-fixture";
const GREET: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/vascalir.lua:64-74";
const CONTINUE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/vascalir.lua:903-912";

pub(super) fn run_case() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(
            NAME,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use serde_json::json;
    let program = json!({
        "schema":1,"quest":QUEST,
        "tracks":[
            {"key":QUESTLINE,"quest":QUEST,"initial":-1,"min":-1,"max":2,"sources":[GREET]},
            {"key":WARNING,"quest":QUEST,"initial":-1,"min":-1,"max":1,"sources":[CONTINUE]}
        ],
        "rules":[
            {"key":"oteryn:quest-npc/rookie-guard/vascalir/late-entry","quest":QUEST,"npc":NPC,
             "trigger":"greet","keywords":["hi","hello"],"sources":[GREET],
             "guards":[{"kind":"storage","track":QUESTLINE,"op":"eq","value":-1},
                       {"kind":"level","op":"gt","value":5}],
             "reply":["Welcome, adventurer |PLAYERNAME|. Thank you for offering your help - but you are already too experienced to start this quest. Just go on hunting monsters, you'll be better off that way."],
             "effects":[{"kind":"end_conversation","delay_ms":0}]},
            {"key":"oteryn:quest-npc/rookie-guard/vascalir/warning","quest":QUEST,"npc":NPC,
             "trigger":"greet","keywords":["hi","hello"],"sources":[GREET],
             "guards":[{"kind":"storage","track":QUESTLINE,"op":"eq","value":1},
                       {"kind":"level","op":"eq","value":8},
                       {"kind":"storage","track":WARNING,"op":"eq","value":-1}],
             "reply":["What would you like to do? {Continue} the mission or {delete} the unfinished questline from your questlog?"]},
            {"key":"oteryn:quest-npc/rookie-guard/vascalir/continue","quest":QUEST,"npc":NPC,
             "trigger":"say","keywords":["continue"],"sources":[CONTINUE],
             "guards":[{"kind":"storage","track":QUESTLINE,"op":"eq","value":1},
                       {"kind":"level","op":"eq","value":8},
                       {"kind":"storage","track":WARNING,"op":"eq","value":-1}],
             "reply":["Alright. Talk to me again to continue with your mission, but heed my words!"],
             "effects":[{"kind":"set","track":WARNING,"value":1},
                        {"kind":"end_conversation","delay_ms":0}]}
        ]
    }).to_string();
    let catalogue = QuestStateCatalogue::new(
        "content-1",
        vec![oteryn_gameplay_quests::quest::QuestTrack {
            key: QUESTLINE.into(),
            quest: QUEST.into(),
            initial: -1,
            min: -1,
            max: 2,
        }],
        vec![QuestTransition {
            key: START.into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: QUESTLINE.into(),
                from: QuestComparison::Eq(-1),
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
            world_change: None,
        }],
    )
    .map_err(debug)?;
    owner.quest_catalogue = Some(std::sync::Arc::new(
        NpcRuleCatalog::from_json_documents(&[&program])?.bind_fixture(catalogue)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = super::classic_quests_fixtures::install(owner, admitted, [NPC]).await?;
    let talk = super::classic_quests_fixtures::talk;
    let refused = talk(owner, admitted, npc, 6410, "hi").await?;
    assert!(refused.iter().any(|line| line.contains("too experienced")));
    assert_eq!(stored_warning(h).await?, None);

    super::qengine_outfit_stories_fixtures::commit_fixture(owner, admitted, START, 6409).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let warned = talk(owner, admitted, npc, 6411, "hi").await?;
    assert!(
        warned
            .iter()
            .any(|line| line.contains("unfinished questline"))
    );
    let accepted = talk(owner, admitted, npc, 6412, "continue").await?;
    assert!(accepted.iter().any(|line| line.contains("heed my words")));
    assert_eq!(stored_warning(h).await?, Some(1));
    let warned_again = talk(owner, admitted, npc, 6413, "hi").await?;
    assert!(
        !warned_again
            .iter()
            .any(|line| line.contains("unfinished questline"))
    );
    Ok(())
}

async fn stored_warning(h: &Harness) -> TestResult<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(WARNING)
            .fetch_optional(&h.pool)
            .await?,
    )
}
