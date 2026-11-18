//! Both Omrabas dialogue transitions use the shared native NPC registration and writer.
use super::quest_world_callbacks::{self, tool_fixture};
use super::{TestResult, fixture::debug, foolish_npc_reactions::talk};
use oteryn_gameplay_quests::quest::{
    authored_bindings::{Bindings, BoundQuest, Stage, Track, Trigger},
    reward_callbacks::RewardCallback,
    world_callbacks::{Callback, Position},
};

const QUEST: &str = "oteryn:quest.the_gravedigger_of_drefia_quest";
const OMRABAS: &str = "oteryn:npc.omrabas";
const START: &str = "oteryn:quest-progress/quest/u10_10/the_gravedigger_of_drefia/quest_start";
const RITE: &str = "oteryn:quest-progress/quest/u10_10/the_gravedigger_of_drefia/mission75";
const REWARD: &str = "oteryn:quest-progress/quest/u10_10/the_gravedigger_of_drefia/mission76";
const NAIL_CASE: &str = "oteryn:item.tibia.i19136";
const INVITATION: &str = "Hmm. You could be of assistance, I presume. I need several body parts. I will reward you adequately. Interested?";
const FINAL_LINES: [&str; 4] = [
    "It failed! IT FAILED! WHY? What have you done! This must be your fault! ...",
    "You... did... that was a recipe for chicken soup! No wonder the scroll failed! Now... all is lost ...",
    "<sobs drily> I never want to see you again! You and your dirty gravedigger hands! Take this, you ungrateful, useless, imbecile... human! ...",
    "Now... get out! And never, ever, dare come back! You ruined EVERYTHING!",
];

#[test]
fn omrabas_starts_the_quest_and_delivers_the_nail_case_once_through_npc_talk() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("gravedigger-omrabas", move || scenario(admin))
}

fn scenario(admin: String) -> TestResult {
    let origin = Position {
        x: 33018,
        y: 32425,
        floor: -11,
    };
    let npc_at = Position {
        x: origin.x,
        y: origin.y + 1,
        ..origin
    };
    let geometry: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/fixture/omrabas-map",
        "quest": QUEST, "trigger": "use", "selector": { "position": origin, "appearance": 100 },
        "reversible": false, "passages": [], "storage": [], "offerings": [], "return_on_failure": false,
        "sources": ["PostgreSQL dispatch fixture"]
    }))?;
    let world = quest_world_callbacks::world(&geometry, origin)?;
    // Crystal 201189f8 data-global/npc/omrabas.lua:65-69,220-229.
    let callbacks: [RewardCallback; 2] = serde_json::from_value(serde_json::json!([
        { "key": "oteryn:quest-reward/fixture/omrabas-start", "quest": QUEST, "npc": OMRABAS,
          "keywords": ["mission"], "topic": 0, "next_topic": 1,
          "storage": [{ "track": START, "minimum": 0, "maximum": 0, "set": 1 }],
          "take": [], "reply": [INVITATION],
          "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/omrabas.lua:65-69"] },
        { "key": "oteryn:quest-reward/fixture/omrabas-nail-case", "quest": QUEST, "npc": OMRABAS,
          "keywords": ["mission"], "topic": 0, "next_topic": 0,
          "storage": [
              { "track": START, "minimum": 1, "maximum": 1, "set": 1, "preserve": true },
              { "track": RITE, "minimum": 1, "maximum": 1, "set": 1, "preserve": true },
              { "track": REWARD, "minimum": 0, "maximum": 0, "set": 1 }
          ],
          "take": [], "give": { "item": NAIL_CASE, "count": 1 }, "reply": FINAL_LINES,
          "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/omrabas.lua:220-229"] }
    ]))?;
    let bindings = Bindings {
        schema: 1,
        quests: vec![BoundQuest {
            quest: QUEST.into(),
            recipe_sha256: "1".repeat(64),
            occurrence: None,
            admission: None,
            tracks: [(START, 0), (RITE, 1), (REWARD, 0)]
                .into_iter()
                .map(|(key, initial)| Track {
                    key: key.into(),
                    initial,
                    minimum: 0,
                    maximum: 1,
                })
                .collect(),
            stages: ["start", "reward"]
                .into_iter()
                .zip(callbacks)
                .map(|(key, callback)| Stage {
                    key: key.into(),
                    completes: false,
                    trigger: Trigger::NpcDialogue {
                        callback: Box::new(callback),
                        alternatives: Vec::new(),
                    },
                })
                .collect(),
            sources: vec![
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/omrabas.lua"
                    .into(),
            ],
        }],
    };
    tool_fixture::run_bound_world(
        admin,
        "gravedigger_omrabas",
        world,
        origin,
        bindings,
        async |owner, admitted, h, fence| {
            let [npc] =
                super::classic_quests::install_positions(owner, &admitted, [OMRABAS], [npc_at])
                    .await?;
            let _clock = owner.freeze_owner_clock_for_test();
            talk(owner, admitted, fence, npc, 1, "hi").await?;
            let start = talk(owner, admitted, fence, npc, 2, "mission").await?;
            assert_eq!(start.lines, [INVITATION]);
            assert!(!start.inventory_changed);
            talk(owner, admitted, fence, npc, 3, "bye").await?;
            talk(owner, admitted, fence, npc, 4, "hi").await?;
            let finished = talk(owner, admitted, fence, npc, 5, "mission").await?;
            assert_eq!(finished.lines, FINAL_LINES);
            assert!(finished.inventory_changed);
            let replay = talk(owner, admitted, fence, npc, 6, "mission").await?;
            assert!(!replay.inventory_changed);
            let cases: Vec<i64> = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
                .bind(NAIL_CASE).fetch_all(&h.pool).await?;
            assert_eq!(cases, [1]);
            let snapshot = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(snapshot.tracks().get(START), Some(&1));
            assert_eq!(snapshot.tracks().get(REWARD), Some(&1));
            Ok(())
        },
    )
}
