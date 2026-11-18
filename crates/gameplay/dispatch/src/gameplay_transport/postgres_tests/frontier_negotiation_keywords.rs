//! Crystal 201189f8, data-global/npc/ongulf.lua:181-195.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition, npc_rules::NpcRuleCatalog, world_callbacks::Position,
};
use serde_json::json;
use std::sync::Arc;

pub(super) const NAME: &str = "frontier-negotiation-keywords";
const QUEST: &str = "oteryn:quest.the_new_frontier_quest";
const ONGULF: &str = "oteryn:npc.ongulf";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/ongulf.lua:181-195";
const STAGE: &str = "oteryn:quest-progress/frontier-fixture/questline";
const KEYWORDS: [(&str, i16); 5] = [
    ("leeland", 2), ("angus", 2), ("wyrdin", 4), ("telas", 2), ("humgolf", 2),
];

#[test]
fn negotiation_acceptance_assigns_five_character_keywords_once() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, Position { x: 33294, y: 31677, floor: -15 }, false)
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let keywords = KEYWORDS.map(|(name, max)| (format!("oteryn:quest-progress/frontier-fixture/{name}"), max));
    let mut tracks = vec![QuestTrack { key: STAGE.into(), quest: QUEST.into(), initial: 13, min: 13, max: 14 }];
    tracks.extend(keywords.iter().map(|(key, max)| QuestTrack {
        key: key.clone(), quest: QUEST.into(), initial: 0, min: 0, max: i64::from(*max),
    }));
    let definitions: Vec<_> = tracks.iter().map(|t| json!({
        "key":t.key,"quest":t.quest,"initial":t.initial,"min":t.min,"max":t.max,"sources":[SOURCE],
    })).collect();
    let mut effects = vec![json!({"kind":"set","track":STAGE,"value":14})];
    effects.extend(keywords.iter().map(|(track, maximum)| json!({
        "kind":"set_random","track":track,"minimum":1,"maximum":maximum,
    })));
    let program = json!({"schema":1,"quest":QUEST,"tracks":definitions,"rules":[{
        "key":"oteryn:quest-npc/frontier-fixture/negotiate","quest":QUEST,"npc":ONGULF,
        "trigger":"say","keywords":["mission"],"guards":[{"kind":"storage","track":STAGE,"op":"eq","value":13}],
        "effects":effects,"reply":["Negotiate for support."],"sources":[SOURCE]
    },{
        "key":"oteryn:quest-npc/frontier-fixture/active","quest":QUEST,"npc":ONGULF,
        "trigger":"say","keywords":["mission"],"guards":[{"kind":"storage","track":STAGE,"op":"eq","value":14}],
        "effects":[],"reply":["Negotiations are active."],"sources":[SOURCE]
    }]});
    let source = NpcRuleCatalog::from_json_documents(&[&program.to_string()]).map_err(debug)?;
    let digest: String = source.programs()[0].digest.iter().map(|b| format!("{b:02x}")).collect();
    let catalogue = QuestStateCatalogue::new("content-1", tracks, vec![QuestTransition {
        key: format!("oteryn:quest-npc-pin/{digest}"), quest: QUEST.into(),
        effects: vec![QuestEffect { track: STAGE.into(), from: QuestComparison::Any, effect: QuestEffectKind::Computed }],
        completes: false, experience: None, world_change: None,
    }]).map_err(debug)?.with_fixture_npc_rules(source);
    owner.quest_catalogue = Some(Arc::new(catalogue));
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let [ongulf] = super::classic_quests::install(owner, admitted, [ONGULF]).await?;
    super::classic_quests::talk(owner, admitted, ongulf, 9540, "hi").await?;
    assert_eq!(super::classic_quests::talk(owner, admitted, ongulf, 9541, "mission").await?, ["Negotiate for support."]);
    let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "quest fence")?.ok_or("quest fence")?;
    let state = owner.root.read_character_quest_state(owner.character, fence.character_id).await.map_err(debug)?;
    assert_eq!(state.tracks().get(STAGE).copied(), Some(14));
    for (key, maximum) in keywords {
        let track = state.tracks().get(&key).ok_or("unassigned keyword")?;
        assert!((1..=i64::from(maximum)).contains(track));
    }
    assert_eq!(super::classic_quests::talk(owner, admitted, ongulf, 9542, "mission").await?, ["Negotiations are active."]);
    assert_eq!(owner.root.read_character_quest_state(owner.character, fence.character_id).await.map_err(debug)?, state);
    Ok(())
}
