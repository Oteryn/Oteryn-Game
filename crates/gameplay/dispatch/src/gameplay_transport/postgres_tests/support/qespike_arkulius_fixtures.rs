//! data-global/npc/arkulius.lua:104-165, 189-197).
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition,
    npc_rules::NpcRuleCatalog,
    world_callbacks::{Offering, Position},
};
use serde_json::json;
use std::sync::Arc;

pub(super) const NAME: &str = "qespike-arkulius";
pub(super) const PAYOUT: &str = "qespike-arkulius-payout";
const NPC: &str = "oteryn:npc.arkulius";
const QUEST: &str = "oteryn:quest.elemental_spheres_quest";
const TRACK: &str = "oteryn:quest-progress/elemental_spheres/quest_line";
// Crystal arkulius.lua:190; this source shop item already has native custody.
const ENERGY_SOIL: &str = "oteryn:item.tibia.i945";
const NATIVE_REWARD_FIXTURE: &str = "oteryn:item.tibia.i3304";
pub(super) const START: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};

fn catalogue(revision: &str, payout: Option<&str>) -> TestResult<QuestStateCatalogue> {
    let witness =
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/arkulius.lua:104-197";
    // Only the mission keyword is exercised. Alternate fixture branches make
    // the reader scan the entire NPC program without emitting their effects.
    let mut rules = vec![json!({
        "key":"oteryn:quest-npc/qespike/arkulius/mission", "npc":NPC, "quest":QUEST, "trigger":"say",
        "keywords":["mission"], "topics":[0],
        "guards":[{"kind":"storage","track":TRACK,"op":"eq","value":1},
            {"kind":"item","item":ENERGY_SOIL,"count":1,"op":"ge"}],
        "effects":[{"kind":"set","track":TRACK,"value":2}],
        "reply":["Impressive!! Let me take a look.......Ahh, ENERGY SOIL!"], "sources":[witness]
    })];
    if let Some(reward) = payout {
        rules.push(json!({
            "key":"oteryn:quest-npc/qespike/arkulius/payout", "npc":NPC, "quest":QUEST, "trigger":"say",
            "keywords":["mission"],"topics":[0],
            "guards":[{"kind":"storage","track":TRACK,"op":"eq","value":2},
                {"kind":"item","item":ENERGY_SOIL,"count":1,"op":"ge"}],
            "effects":[{"kind":"take_item","item":ENERGY_SOIL,"count":1},
                {"kind":"give_item","item":reward,"count":1},
                {"kind":"set","track":TRACK,"value":3}],
            "reply":["AMAZING!! I'm going to start immediately with the research."],
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/arkulius.lua:162-165"]
        }));
    }
    for item in [940, 941, 942, 944, 946, 947, 948, 954, 8039, 8041] {
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
    let initial = if payout.is_some() { 2 } else { 1 };
    let source = NpcRuleCatalog::from_json_documents(&[&json!({
        "schema":1, "quest":QUEST,
        "tracks":[{"key":TRACK,"quest":QUEST,"initial":initial,"min":-1,"max":3,"sources":[witness]}],
        "rules":rules
    })
    .to_string()])
    .map_err(debug)?;
    let digest = source.programs()[0]
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(QuestStateCatalogue::new(
        revision,
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial,
            min: -1,
            max: 3,
        }],
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
    )
    .map_err(debug)?
    .with_npc_program_fixture(source))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    _harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let revision = owner
        .quest_catalogue
        .as_ref()
        .ok_or("quest catalogue")?
        .content_revision();
    owner.quest_catalogue = Some(Arc::new(catalogue(revision, None)?));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [arkulius] = super::classic_quests_fixtures::install(owner, admitted, [NPC]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: START,
            item: ENERGY_SOIL.into(),
            count: 1,
        },
        5931,
        true,
    )
    .await?;
    super::classic_quests_fixtures::talk(owner, admitted, arkulius, 5932, "hi").await?;
    let reply =
        super::classic_quests_fixtures::talk(owner, admitted, arkulius, 5933, "mission").await?;
    assert!(reply.iter().any(|line| line.contains("ENERGY SOIL")));
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?,
        2
    );
    Ok(())
}

pub(super) async fn payout(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let revision = owner
        .quest_catalogue
        .as_ref()
        .ok_or("quest catalogue")?
        .content_revision()
        .to_owned();
    super::item_look_fixtures::seed_main_backpack(harness, &[(ENERGY_SOIL, 1)]).await?;
    let [arkulius] = super::classic_quests_fixtures::install(owner, admitted, [NPC]).await?;
    // The ordinary MINT rejects a Ground-only definition and accepts a native
    // backpack definition. The source's exact reward needs Content admission.
    for (index, reward, expected) in [
        (0, "oteryn:item.tibia.i5969", (1, 0, 2)),
        (1, NATIVE_REWARD_FIXTURE, (0, 1, 3)),
    ] {
        owner.quest_catalogue = Some(Arc::new(catalogue(&revision, Some(reward))?));
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        super::classic_quests_fixtures::talk(owner, admitted, arkulius, 5950 + index * 2, "hi")
            .await?;
        super::classic_quests_fixtures::talk(
            owner,
            admitted,
            arkulius,
            5951 + index * 2,
            "mission",
        )
        .await?;
        let observed: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1),
             (SELECT count(*) FROM game_item_instances WHERE world_id=encode($2,'hex')::uuid AND definition_production_key=$3 AND lifecycle=1),
             coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$4),2)",
        ).bind(id(71).as_slice()).bind(id(42).as_slice()).bind(reward).bind(TRACK)
        .fetch_one(&harness.pool).await?;
        assert_eq!(observed, expected);
    }
    Ok(())
}
