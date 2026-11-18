//! Crystal 201189f8, data-global/npc/albinius.lua:172-192.
use super::classic_quests::{install, progress, talk};
use super::questpc2_albinius_portals::{carried, give};
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug,
};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    definition_predecessors, npc_rules::NpcRuleCatalog, world_callbacks::Position,
};
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) const CASE: &str = "questpc2-albinius-ice-predecessor";
const QUEST: &str = "oteryn:quest.forgotten_knowledge_quest";
const TOMES: &str = "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/tomes";
const ICE: &str = "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/access_ice";
const FORMORGAR: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/formorgar_mines_door";
const FISH: &str = "oteryn:item.tibia.i3578";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/albinius.lua:172-192";

#[test]
fn albinius_ice_offering_requires_formorgar_and_charges_fifty_fish_on_existing_tome_pin()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CASE, || {
        super::quest_world_callbacks::scenario(
            CASE,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

fn guard(track: &str, op: &str) -> Value {
    json!({"kind":"storage","track":track,"op":op,"value":1})
}

fn row(
    name: &str,
    keyword: &str,
    topic: Option<i32>,
    guards: Vec<Value>,
    reply: &str,
    effects: Vec<Value>,
) -> Value {
    let mut rule = json!({
        "key":format!("oteryn:quest-npc/forgotten_knowledge/albinius/{name}"),
        "quest":QUEST,"npc":"oteryn:npc.albinius","trigger":"say","keywords":[keyword],
        "guards":guards,"reply":[reply],"effects":effects,
        "sources":[SOURCE],"continue_matching":true
    });
    if let Some(topic) = topic {
        rule["topics"] = json!([topic]);
    }
    rule
}

fn successor(base: &QuestStateCatalogue) -> TestResult<QuestStateCatalogue> {
    let mut program: Value = serde_json::from_str(include_str!(
        "../../../../../../content/dialogues/quest-rules-supplemental/questpc2-albinius.json"
    ))?;
    program["source_files"]
        .as_array_mut()
        .ok_or("source files")?
        .push(json!({
            "path":"data-global/npc/albinius.lua",
            "revision":"201189f871d23597b82caf88fd611454a2ddefb8",
            "sha256":"1a476ee079849acee80dd99fb107ae5fd674c2d3c3343beed8002840d09cea18"
        }));
    program["tracks"]
        .as_array_mut()
        .ok_or("tracks")?
        .push(json!({
            "key":ICE,"quest":QUEST,"initial":0,"min":0,"max":1,"sources":[SOURCE]
        }));
    let prerequisite =
        "Sorry, you first need to bring my Heavy Old Tomes or do the quest before continuing.";
    let insufficient =
        "I'm sorry, you don't have enough fish. Return if you can offer fifty of them.";
    program["rules"].as_array_mut().ok_or("rules")?.extend([
        row("ice-offer","ice portal",None,vec![guard(TOMES,"eq"),guard(FORMORGAR,"eq")],
            "You may pass this portal if you have 50 fish as offering. Do you have the fish with you?",
            vec![json!({"kind":"set_topic","value":2})]),
        row("ice-needs-tomes","ice portal",None,vec![guard(TOMES,"ne")],prerequisite,vec![]),
        row("ice-needs-formorgar","ice portal",None,vec![guard(TOMES,"eq"),guard(FORMORGAR,"ne")],
            prerequisite,vec![]),
        row("ice-already","yes",Some(2),vec![guard(ICE,"ge")],insufficient,vec![]),
        row("ice-missing","yes",Some(2),vec![guard(ICE,"lt"),
            json!({"kind":"item","item":FISH,"count":50,"op":"lt"})],insufficient,vec![]),
        row("ice-accept","yes",Some(2),vec![guard(ICE,"lt"),
            json!({"kind":"item","item":FISH,"count":50,"op":"ge"})],
            "Thank you for your offering. You may pass the Portal to the Powers of Ice now.",
            vec![json!({"kind":"take_item","item":FISH,"count":50}),json!({"kind":"set","track":ICE,"value":1})]),
        row("ice-decline","no",Some(2),vec![],
            "In this case I'm sorry, you may not pass this portal.",vec![]),
    ]);
    let source = NpcRuleCatalog::from_json_documents(&[&program.to_string()]).map_err(debug)?;
    let mut catalogue = source.attach_fixture(base)?;
    let hash: String = base
        .definition_hash(QUEST)
        .ok_or("original Albinius definition")?
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let binding =
        json!({"schema":1,"predecessors":[{"quest":QUEST,"definition_sha256":hash}]}).to_string();
    definition_predecessors::register(&mut catalogue, &[("forgotten-knowledge.json", &binding)])?;
    Ok(catalogue)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [npc] = install(owner, admitted, ["oteryn:npc.albinius"]).await?;
    give(owner, admitted, "oteryn:item.tibia.i23986", 5, 9300).await?;
    give(owner, admitted, FISH, 49, 9301).await?;
    talk(owner, admitted, npc, 35100, "hi").await?;
    talk(owner, admitted, npc, 35101, "temple yes").await?;
    assert_eq!(progress(owner, admitted, owner.character, TOMES).await?, 1);
    let base = owner.quest_catalogue.as_deref().ok_or("catalogue")?;
    let formorgar_quest = base
        .track(FORMORGAR)
        .ok_or("Formorgar track")?
        .quest
        .clone();
    let mut successor = successor(base)?;
    let key = "oteryn:quest-transition/fixture/formorgar-mines-access";
    successor = successor
        .with_fixture_transition(QuestTransition {
            key: key.into(),
            quest: formorgar_quest,
            effects: vec![QuestEffect {
                track: FORMORGAR.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(1),
            }],
            world_change: None,
            completes: false,
            experience: None,
        })
        .map_err(debug)?;
    owner.quest_catalogue = Some(Arc::new(successor));
    let refused = talk(owner, admitted, npc, 35102, "ice portal").await?;
    assert!(refused.iter().any(|line| line.contains("Heavy Old Tomes")));
    assert_eq!(carried(h, FISH).await?, 49);
    super::qengine_outfit_stories::commit_fixture(owner, admitted, key, 35200).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let offered = talk(owner, admitted, npc, 35103, "ice portal").await?;
    assert!(offered.iter().any(|line| line.contains("50 fish")));
    let refused = talk(owner, admitted, npc, 35104, "yes").await?;
    assert!(refused.iter().any(|line| line.contains("enough fish")));
    assert_eq!(carried(h, FISH).await?, 49);
    give(owner, admitted, FISH, 1, 9302).await?;
    let accepted = talk(owner, admitted, npc, 35105, "yes").await?;
    assert!(
        accepted
            .iter()
            .any(|line| line.contains("Portal to the Powers of Ice"))
    );
    assert_eq!(progress(owner, admitted, owner.character, ICE).await?, 1);
    assert_eq!(carried(h, FISH).await?, 0);
    assert_eq!(progress(owner, admitted, owner.character, TOMES).await?, 1);
    Ok(())
}
