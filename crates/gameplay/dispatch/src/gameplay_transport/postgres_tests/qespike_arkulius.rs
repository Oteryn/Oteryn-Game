//! Arkulius' branches share one bounded custody snapshot (Crystal 201189f8,
//! data-global/npc/arkulius.lua:104-165, 189-197).
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
    npc_rules::NpcRuleCatalog,
    world_callbacks::{Offering, Position},
};
use serde_json::json;
use std::sync::Arc;

pub(super) const NAME: &str = "qespike-arkulius";
const NPC: &str = "oteryn:npc.arkulius";
const QUEST: &str = "oteryn:quest.elemental_spheres_quest";
const TRACK: &str = "oteryn:quest-progress/elemental_spheres/quest_line";
const PURE_ENERGY: &str = "oteryn:item.tibia.i948";
const START: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};

#[test]
fn arkulius_dialogue_reads_carried_input_with_eleven_program_item_identities() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, START, false)
    })
}

fn catalogue(revision: &str) -> TestResult<QuestStateCatalogue> {
    let witness = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/arkulius.lua:104-197";
    // Only the mission keyword is exercised. Alternate fixture branches make
    // the reader scan the entire NPC program without emitting their effects.
    let mut rules = vec![json!({
        "key":"oteryn:quest-npc/qespike/arkulius/mission", "npc":NPC, "quest":QUEST, "trigger":"say",
        "keywords":["mission"], "topics":[0],
        "guards":[{"kind":"storage","track":TRACK,"op":"eq","value":1},
            {"kind":"item","item":PURE_ENERGY,"count":1,"op":"ge"}],
        "effects":[{"kind":"set","track":TRACK,"value":2}],
        "reply":["Impressive!! Let me take a look.......Ahh, PURE ENERGY!"], "sources":[witness]
    })];
    for item in [940, 941, 942, 944, 945, 946, 947, 954, 8039, 8041] {
        let key = format!("oteryn:item.tibia.i{item}");
        rules.push(json!({
            "key":format!("oteryn:quest-npc/qespike/arkulius/check-{item}"),
            "npc":NPC, "quest":QUEST, "trigger":"say",
            "keywords":[format!("check {item}")], "topics":[0],
            "guards":[{"kind":"item","item":key,"count":1,"op":"ge"}],
            "effects":[{"kind":"take_item","item":key,"count":1}],
            "reply":["Let me take a look."], "sources":[witness]
        }));
    }
    let source = NpcRuleCatalog::from_json_documents(&[&json!({
        "schema":1, "quest":QUEST,
        "tracks":[{"key":TRACK,"quest":QUEST,"initial":1,"min":-1,"max":3,"sources":[witness]}],
        "rules":rules
    }).to_string()]).map_err(debug)?;
    let digest = source.programs()[0].digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    Ok(QuestStateCatalogue::new(
        revision,
        vec![QuestTrack { key: TRACK.into(), quest: QUEST.into(), initial: 1, min: -1, max: 3 }],
        vec![QuestTransition {
            key: format!("oteryn:quest-npc-pin/{digest}"),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: TRACK.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Computed,
            }],
            completes: false,
            experience: None,
            world_change: None,
        }],
    ).map_err(debug)?.with_npc_program_fixture(source))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    _harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let revision = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?.content_revision();
    owner.quest_catalogue = Some(Arc::new(catalogue(revision)?));
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let [arkulius] = super::classic_quests::install(owner, admitted, [NPC]).await?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "quest fence")?.ok_or("quest fence")?;
    super::quest_world_callbacks::input_item(owner, fence.item_fence(), &Offering {
        position: START, item: PURE_ENERGY.into(), count: 1,
    }, 5931, true).await?;
    super::classic_quests::talk(owner, admitted, arkulius, 5932, "hi").await?;
    let reply = super::classic_quests::talk(owner, admitted, arkulius, 5933, "mission").await?;
    assert!(reply.iter().any(|line| line.contains("PURE ENERGY")));
    assert_eq!(super::classic_quests::progress(owner, admitted, authority, TRACK).await?, 2);
    Ok(())
}
