//! Crystal 201189f data-global/npc/myzzi.lua:67-102 and
//! scripts/quests/the_dream_courts/movements_courts_entrance.lua:2-7,128-152.
use super::classic_quests::{install, progress, talk};
use super::fixture::debug;
use super::quest_world_callbacks::{local, scenario, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, npc_rules::NpcRuleCatalog, world_callbacks::{self, Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) const NAME: &str = "myzzi-court-entry";
const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const NPC: &str = "oteryn:npc.myzzi";
const PROGRAM: &str = "oteryn:quest-npc-program.dream-courts-myzzi";
const TRACK: &str = "oteryn:quest-progress/quest/u12_00/the_dream_courts/main/questline";
const PIN: &str = "201189f871d23597b82caf88fd611454a2ddefb8";
const ENTRY: Position = Position { x: 33675, y: 32147, floor: -7 };
const DESTINATION: Position = Position { x: 32354, y: 31249, floor: -3 };

pub(super) fn binding() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/dream-courts/winter-entrance",
        "quest":QUEST, "trigger":"step_in", "selector":{"position":ENTRY,"action":23100},
        "teleport":DESTINATION, "reversible":false, "return_on_failure":true,
        "passages":[], "offerings":[], "effects":{"teleport_effect":11},
        "storage":[{"track":TRACK,"minimum":1,"maximum":1,"set":1,"read_only":true}],
        "sources":[format!("crystal:{PIN}:data-global/scripts/quests/the_dream_courts/movements_courts_entrance.lua:2-7")]
    }))?)
}

fn rule(label: &str, trigger: &str, line: &str, reply: &[&str]) -> Value {
    json!({
        "key":format!("oteryn:quest-npc/dream-courts/myzzi/{label}"),
        "quest":QUEST, "npc":NPC, "trigger":trigger, "reply":reply,
        "sources":[format!("crystal:{PIN}:data-global/npc/myzzi.lua:{line}")]
    })
}

fn program() -> Value {
    let mut rules = vec![
        rule("greet", "greet", "100", &["Hello adventurer. It is {good} to see you."]),
        rule("farewell", "farewell", "101", &["Well, bye then."]),
        rule("walkaway", "walkaway", "102", &["Well, bye then."]),
    ];
    let guard = json!([{"kind":"storage","track":TRACK,"op":"lt","value":1}]);
    for (keyword, topic, next, line, reply) in [
        ("good", None, 2, "68-70", &["I'm just a mere messenger and I'm here to find brave adventurers that might {help} my friends in this time of need."][..]),
        ("help", Some(2), 3, "71-73", &["Lady Alivar of the Summer Court and Lord Cadion of the Winter Court are in need of brave adventurers to avert a great {threat} for the whole world."][..]),
        ("threat", Some(3), 4, "74-79", &["I know only little and I forget so much. So many things going around my mind! ...", "However, I can grant you access to the {Courts} of Summer and Winter if you promise to help! There you can meet with Undal or Vanys, the servants of Lord Cadion and Lady Alivar. They will be able to tell you more about the issue."][..]),
        ("courts", Some(4), 5, "80-82", &["There are {entrances} to the hidden Courts of Summer and Winter in secluded places. You can find the portal to the Winter Court high in the mountains of Tyrsung and the portal to the Summer Court in the meadows of Feyrist. ...", "With my magic you will be able to enter the Courts. Find Undal or Vanys and talk to them."][..]),
        ("entrances", Some(5), 6, "83-85", &["So, are you willing to help in this time of need?"][..]),
        ("yes", Some(6), 0, "86-90", &["You are a true hero! Here, take my enchantment and you will be able to pass the portals. Now hurry, my friends are waiting."][..]),
    ] {
        let mut row = rule(keyword, "say", line, reply);
        row["keywords"] = json!([keyword]);
        row["topics"] = json!(topic.map(|topic| [topic]));
        row["guards"] = guard.clone();
        row["effects"] = if keyword == "yes" {
            json!([{"kind":"set","track":TRACK,"value":1},{"kind":"set_topic","value":next}])
        } else {
            json!([{"kind":"set_topic","value":next}])
        };
        rules.push(row);
    }
    let mut fallback = rule("refusal", "say", "91-92", &["What?!"]);
    fallback["any_message"] = json!(true);
    fallback["topics"] = json!([6]);
    fallback["guards"] = guard;
    rules.push(fallback);
    json!({
        "schema":1, "program_key":PROGRAM, "quest":QUEST,
        "tracks":[{"key":TRACK,"quest":QUEST,"initial":0,"min":0,"max":1,
            "sources":[format!("crystal:{PIN}:data-global/npc/myzzi.lua:67-90")]}],
        "rules":rules,
        "source_files":[{"path":"data-global/npc/myzzi.lua","revision":PIN,
            "sha256":"8a362f3651f4a15643cce9aad258195050d75ad6a3c3b79b8869d548bec27db1"}]
    })
}

pub(super) fn catalogue(base: &QuestStateCatalogue, callback: &Callback) -> TestResult<Arc<QuestStateCatalogue>> {
    let native = NpcRuleCatalog::with_fixture_document(base, &program().to_string()).map_err(debug)?;
    let world = json!({"schema":1,"callbacks":[callback]}).to_string();
    Ok(Arc::new(world_callbacks::with_fixture_world_document(&native, &world)?))
}

#[test]
fn myzzi_enchantment_admits_the_winter_court_after_the_full_dialogue() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, ENTRY, false))
}

async fn step(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession, id: u64, destination: Position) -> TestResult {
    let reply = wire(owner, admitted, id, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, encode_step_intent(StepDirection::East)).await?;
    assert_eq!(decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
    let position = owner.runtime.lock().await.read_actor_position(admitted.runtime_actor.ok_or("actor")?).map_err(debug)?;
    assert_eq!(position.position(), local(destination));
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let origin = Position { x: ENTRY.x - 1, ..ENTRY };
    step(owner, admitted, 1, origin).await?;
    let [myzzi] = install(owner, admitted, [NPC]).await?;
    talk(owner, admitted, myzzi, 10, "hi").await?;
    talk(owner, admitted, myzzi, 11, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 0);
    for (id, word) in (12..).zip(["good", "help", "threat", "courts", "entrances"]) {
        assert!(!talk(owner, admitted, myzzi, id, word).await?.is_empty());
        assert_eq!(progress(owner, admitted, authority, TRACK).await?, 0);
    }
    assert_eq!(talk(owner, admitted, myzzi, 17, "no").await?, ["What?!"]);
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 0);
    talk(owner, admitted, myzzi, 18, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 1);
    step(owner, admitted, 19, DESTINATION).await?;
    Ok(())
}
