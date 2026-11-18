//! Source-gated dialogue on a character whose saved quest predates the new route.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition, authored_bindings::Bindings, reward_callbacks, world_callbacks::Position,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const CHILDREN: &str = "oteryn:quest.children_of_the_revolution_quest";
const WRATH: &str = "oteryn:quest.wrath_of_the_emperor_quest";
const SEED: &str = "oteryn:quest-transition/qewrath/predecessor";

#[test]
fn zalamon_opens_phantom_army_from_the_saved_mission_four_definition() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario("qewrath-npc-handoff-zalamon")
}

#[test]
fn sleeping_dragon_commits_charm_custody_and_mission_ten_together() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario("qewrath-npc-handoff-dragon")
}

fn scenario(name: &'static str) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(name, || {
        super::quest_world_callbacks::scenario(
            name,
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
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let (quest, npc, prefix, topic, start, tracks, give, source) = if name.ends_with("-dragon") {
        // Crystal 201189f8 data-global/npc/a_sleeping_dragon.lua:203-222.
        (
            WRATH,
            "oteryn:npc.a_sleeping_dragon",
            "quest/u8_6/wrath_of_the_emperor",
            41,
            28,
            vec![("questline", 28, 29), ("mission10", -1, 1)],
            Some("oteryn:item.tibia.i10343"),
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/a_sleeping_dragon.lua:203-222",
        )
    } else {
        // Crystal 201189f8 data-global/npc/zalamon.lua:143-149,337-345.
        (
            CHILDREN,
            "oteryn:npc.zalamon",
            "quest/u8_54/children_of_the_revolution",
            13,
            18,
            vec![
                ("questline", 18, 19),
                ("teleport_access", -1, 1),
                ("mission05", -1, 1),
            ],
            None,
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/zalamon.lua:143-149,337-345",
        )
    };
    let (offer, reply) = if give.is_some() {
        (
            vec![
                "Aaaah... free at last. Hmmm. ...",
                "I assume you need to get through the gate to reach the evildoer. I can help you if you trust me, wayfarer. I will share a part of my mind with you which should enable you to step through the gate. ...",
                "This procedure may be exhausting. Are you prepared to receive my key?",
            ],
            vec![
                "SAETHELON TORILUN GARNUM. ...",
                "SLEEP. ...",
                "GAIN. ...",
                "RISE. ...",
                "The transfer was successful. ...",
                "You are now prepared to enter the realm of the evildoer. I am grateful for your help, wayfarer. Should you seek my council, use this charm I cede to you. For my spirit will guide you wherever you are. May you enjoy a sheltered future, you shall prevail.",
            ],
        )
    } else {
        (
            vec![
                "If I'm correct, zze portal in zze chamber beyond zze mechanizzm will lead you to zze great gate. It izz perfectly pozzible, however, zzat you will not be tranzzported directly into zze area. ...",
                "You will razzer be brought to a plazze clozze by your goal. Where exzzactly, I cannot zzay. ...",
                "Take all your courage and walk zze pazz zzrough zze portal. At leazzt TRY it - after all, I didn't make you my pupil for nozzing, Kohei. Are you prepared?",
            ],
            vec![
                "You did well on your quezzt zzo far. I hope you will reach zze great gate in time. If we are lucky, it will zztill be open. ...",
                "If not, it will already be overrun by enemy zzoldierzz. Direct confrontation will be inevitable in zzat cazze, palezzkin. Now clear your mind and approach zze portal.",
            ],
        )
    };
    let track = |name: &str| format!("oteryn:quest-progress/{prefix}/{name}");
    let definitions = tracks
        .iter()
        .map(|(key, _, after)| QuestTrack {
            key: track(key),
            quest: quest.into(),
            min: -1,
            initial: -1,
            max: *after,
        })
        .collect::<Vec<_>>();
    let base = QuestStateCatalogue::new(
        "content-1",
        definitions.clone(),
        vec![QuestTransition {
            key: SEED.into(),
            quest: quest.into(),
            effects: vec![QuestEffect {
                track: track("questline"),
                from: QuestComparison::Eq(-1),
                effect: QuestEffectKind::Set(start),
            }],
            completes: false,
            experience: None,
            world_change: None,
        }],
    )
    .map_err(debug)?;
    let receipt_key = format!(
        "oteryn:quest-reward/{}/mission-handoff",
        if give.is_some() { "wrath" } else { "children" }
    );
    let stage = json!({
        "key": "mission-handoff", "completes": false,
        "trigger": { "kind": "npc_dialogue", "callback": {
            "key": receipt_key, "quest": quest, "npc": npc,
            "keywords": ["yes"], "topic": topic, "next_topic": 0,
            "storage": tracks.iter().map(|(key, before, after)| json!({
                "track": track(key), "minimum": before, "maximum": before, "set": after,
            })).collect::<Vec<_>>(),
            "take": [], "give": give.map(|item| json!({"item":item,"count":1,"contents":[]})),
            "outfit": null, "achievement": null, "reply": reply,
            "sources": [source],
        }}
    });
    let digest = Sha256::digest(serde_json::to_vec(&stage)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let document = json!({"schema": 1, "quests": [{
        "quest": quest, "recipe_sha256": digest, "sources": [source],
        "tracks": definitions.iter().map(|t| json!({
            "key": t.key, "initial": t.initial, "minimum": t.min, "maximum": t.max,
        })).collect::<Vec<_>>(),
        "stages": [stage],
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    owner.quest_catalogue = Some(Arc::new(base.clone()));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    super::qengine_outfit_stories::commit_fixture(owner, admitted, SEED, 9100).await?;
    let mut next = bindings.attach(&base)?;
    reward_callbacks::register_additive_predecessor(&mut next, &base, quest)?;
    owner.quest_catalogue = Some(Arc::new(next));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;

    owner.states.npc_conversations.fixture_reward_offers(&json!({
        "schema": 1, "offers": [{
            "npc": npc, "keywords": ["mission"], "topic": 0, "next_topic": topic,
            "storage": [{"track":track("questline"),"minimum":start,"maximum":start,"set":start,"preserve":true}],
            "reply": offer,
        }]
    }).to_string())?;
    let [npc] = super::classic_quests::install(owner, admitted, [npc]).await?;
    let first = admitted.continuity.next_command_id;
    super::classic_quests::talk(owner, admitted, npc, first, "hi").await?;
    super::classic_quests::talk(owner, admitted, npc, first + 1, "mission").await?;
    for command in [first + 2, first + 3] {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "quest fence")?
            .ok_or("quest fence")?;
        let outcome = owner
            .npc_talk(
                admitted.runtime_actor.ok_or("actor")?,
                crate::gameplay_transport::connection::UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command,
                    item_fence: Some(fence.item_fence()),
                },
                oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                    npc_actor: npc,
                    text: "yes".into(),
                },
            )
            .await;
        assert!(!outcome.unknown);
        if command == first + 2 {
            assert_eq!(
                outcome.disposition,
                oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
            );
        }
    }
    for (name, _, expected) in tracks {
        let value: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(track(name))
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(value, expected);
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&receipt_key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 1);
    if let Some(item) = give {
        let quantity: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1",
        ).bind(item).fetch_one(&h.pool).await?;
        assert_eq!(quantity, 1);
    }
    Ok(())
}
