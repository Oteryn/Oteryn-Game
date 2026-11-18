//! Crystal 201189f8 data-global/npc/sven.lua:70-150 through composed NPC TALK.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, classic_quests};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    npc_rules::NpcRuleCatalog,
    reward_callbacks,
    world_callbacks::{Offering, Position},
};
use serde_json::{Value, json};

pub(super) const NAME: &str = "qcx62-sven-source-dialogue";
const NPC: &str = "oteryn:npc.sven";
const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
const BASE: &str = "oteryn:quest-progress/quest/u8_0/barbarian_test/";
const BARBARIAN: &str = "oteryn:quest-progress/quest/u8_0/barbarian_test/questline";
const ICE: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline";
const LEGACY: &str = "oteryn:quest-reward/native-talk/barbarian_test/s1";
const HONEY: &str = "oteryn:item.tibia.i5902";
const HORN: &str = "oteryn:item.tibia.i7140";

#[test]
fn sven_source_confirmation_and_reports_take_precedence_over_the_retained_starter() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(
            NAME,
            Position {
                x: 32243,
                y: 31056,
                floor: -7,
            },
            false,
        )
    })
}

pub(super) fn catalogue(native: &QuestStateCatalogue) -> TestResult<QuestStateCatalogue> {
    let source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/sven.lua:70-150";
    let stage = |op, value| json!({"kind":"storage", "track":BARBARIAN, "op":op, "value":value});
    let topic = |value| json!({"kind":"set_topic","value":value});
    let set = |track: &str, value| json!({"kind":"set","track":track,"value":value});
    let test_track = |name, value| set(&format!("{BASE}{name}"), value);
    let rule = |key,
                word,
                topics: Option<&[i32]>,
                guards: Vec<Value>,
                reply: &[&str],
                effects: Vec<Value>| {
        json!({
            "key": format!("oteryn:quest-npc-rule.barbarian-test-sven.{key}"),
            "quest": QUEST, "npc": NPC, "trigger":"say", "keywords": [word], "topics": topics,
            "guards": guards, "reply": reply, "effects": effects, "sources": [source]
        })
    };
    let tracks = [
        "questline",
        "mission01",
        "mission02",
        "mission03",
        "mead_total_sips",
    ]
    .into_iter()
    .map(|key| format!("{BASE}{key}"))
    .chain([ICE.to_owned()])
    .map(|key| {
        let track = native.track(&key).ok_or("Sven progress track")?;
        Ok(json!({
            "key": key, "quest": QUEST, "initial": track.initial, "min": track.min,
            "max": track.max, "sources": [source]
        }))
    })
    .collect::<TestResult<Vec<_>>>()?;
    let program = json!({
        "schema": 1, "quest": QUEST,
        "program_key": "oteryn:quest-npc-program.barbarian-test-sven",
        "supersedes_reward_callbacks": [LEGACY],
        "tracks": tracks,
        "source_files":[{
            "path":"data-global/npc/sven.lua",
            "revision":"201189f871d23597b82caf88fd611454a2ddefb8",
            "sha256":"885076bfc128330b5ab9c41599bc6e604e710927b75367f9945d5cd3034bbe95"
        }],
        "rules": [
            rule("barbarian", "barbarian", None, vec![stage("lt", 1)],
                &["A true barbarian is something special among our people. Everyone who wants to become a barbarian will have to pass the barbarian {test}."],
                vec![topic(1)]),
            rule("test", "test", None, vec![stage("lt", 1)],
                &["All of our juveniles have to take the barbarian test to become a true member of our community. Foreigners who manage to master the test are granted the title of an honorary barbarian and the respect of our people ...",
                  "Are you willing to take the barbarian test?"], vec![topic(2)]),
            rule("describe-mead", "yes", Some(&[2]), vec![stage("lt", 1)],
                &["That's the spirit! The barbarian test consists of a few tasks you will have to fulfill. All are rather simple - for a barbarian that is...",
                  "Your first task is to drink some barbarian mead. But be warned, it's a strong brew that could even knock out a bear. You need to make at least ten sips of mead in a row without passing out to pass the test ...",
                  "Do you think you can do this?"], vec![topic(3)]),
            rule("start", "yes", Some(&[3]), vec![stage("lt", 1)],
                &["Good, but to make barbarian mead we need some honey which is rare here. I'd hate to waste mead just to learn you're not worth it ...",
                  "Therefore, you have to get your own honey. You'll probably need more than one try so better get some extra honeycombs. Then talk to me again about barbarian {mead}."],
                vec![test_track("questline", 1), set(ICE, 1), test_track("mission01", 1), topic(0)]),
            rule("ask-for-honey", "mead", None, vec![stage("eq", 1)],
                &["Do you have some honey with you?"], vec![topic(4)]),
            rule("take-honey", "yes", Some(&[4]),
                vec![stage("eq", 1), json!({"kind":"item","item":HONEY,"count":1})],
                &["Good, for this honeycomb I allow you 20 sips from the mead bucket over there. Talk to me again about barbarian mead if you have passed the test."],
                vec![json!({"kind":"take_item","item":HONEY,"count":1}),
                    test_track("questline", 2), test_track("mission01", 2),
                    test_track("mead_total_sips", 0), topic(0)]),
            rule("mead-report", "mead", None, vec![stage("eq", 3)],
                &["An impressive start. Here, take your own mead horn to fill it at the mead bucket as often as you like ...",
                  "But there is much left to be done. Your next test will be to hug a bear ...",
                  "You will find one in a cave north of the town. If you are lucky, it's still sleeping. If not ... well that might hurt ...",
                  "Unless you feel that you hugged the bear, the test is not passed. Once you are done, talk to me about the bear hugging."],
                vec![test_track("questline", 4), test_track("mission02", 1),
                    json!({"kind":"give_item","item":HORN,"count":1}), topic(0)]),
            rule("bear-report", "hug", None, vec![stage("eq", 5)],
                &["Amazing. That was as clever and brave as a barbarian is supposed to be. But a barbarian also has to be strong and fearless. To prove that you will have to knock over a mammoth ...",
                  "Did your face just turn into the color of fresh snow? However, you will find a lonely mammoth north west of the town in the wilderness. Knock it over to prove to be a true barbarian ...",
                  "Return to me and talk about the {mammoth} pushing when you are done."],
                vec![test_track("questline", 6), test_track("mission03", 1), topic(0)]),
            rule("mammoth-report", "mammoth", None, vec![stage("eq", 7)],
                &["As you have passed all three tests, I welcome you in our town as an honorary barbarian. You can now become a citizen. Don't forget to talk to the people here. Some of them might need some help ...",
                  "We usually solve our problems on our own but some of the people might have a mission for you. Old Iskan, on the ice in the northern part of the town had some trouble with his dogs lately."],
                vec![test_track("questline", 8),
                    json!({"kind":"achievement","name":"Honorary Barbarian"}), topic(0)])
        ]
    });
    NpcRuleCatalog::from_json_documents(&[&program.to_string()])
        .map_err(debug)?
        .attach_fixture(native.clone())
        .map_err(Into::into)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    at: Position,
) -> TestResult {
    assert!(
        reward_callbacks::offered_for(owner.quest_catalogue.as_deref().ok_or("Sven catalogue")?)?
            .iter()
            .any(|callback| callback.key == LEGACY),
        "the retained starter must be eligible for source precedence"
    );
    let [sven] = classic_quests::install(owner, admitted, [NPC]).await?;
    classic_quests::talk(owner, admitted, sven, 19300, "hi").await?;
    let first = classic_quests::talk(owner, admitted, sven, 19301, "barbarian test").await?;
    assert!(first.iter().any(|line| line.contains("A true barbarian")));
    classic_quests::talk(owner, admitted, sven, 19302, "yes").await?;
    assert_eq!(
        classic_quests::progress(owner, admitted, authority, BARBARIAN).await?,
        0
    );

    let offer = classic_quests::talk(owner, admitted, sven, 19303, "test").await?;
    assert!(offer.iter().any(|line| line.contains("willing")));
    classic_quests::talk(owner, admitted, sven, 19304, "yes").await?;
    classic_quests::talk(owner, admitted, sven, 19305, "yes").await?;
    assert_eq!(
        classic_quests::progress(owner, admitted, authority, BARBARIAN).await?,
        1
    );
    assert_eq!(
        classic_quests::progress(owner, admitted, authority, ICE).await?,
        1
    );

    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            item: HONEY.into(),
            count: 1,
            position: at,
        },
        981,
        true,
    )
    .await?;
    classic_quests::talk(owner, admitted, sven, 19306, "mead").await?;
    classic_quests::talk(owner, admitted, sven, 19307, "yes").await?;
    assert_eq!(
        classic_quests::progress(owner, admitted, authority, BARBARIAN).await?,
        2
    );
    assert_eq!(active_items(h, HONEY).await?, 0);

    for (command, completed_test, keyword, next) in [
        (19308, 3, "mead", 4),
        (19309, 5, "hug", 6),
        (19310, 7, "mammoth", 8),
    ] {
        // Fixture input represents successful mead, bear and mammoth actions.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let updated = sqlx::query("UPDATE game_character_quest_tracks SET value=$3 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
            .bind(fence.character_id.as_bytes().as_slice()).bind(BARBARIAN).bind(completed_test)
            .execute(&mut *tx).await?;
        assert_eq!(updated.rows_affected(), 1);
        tx.commit().await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        let report = classic_quests::talk(owner, admitted, sven, command, keyword).await?;
        assert!(!report.is_empty());
        assert_eq!(
            classic_quests::progress(owner, admitted, authority, BARBARIAN).await?,
            next
        );
    }
    assert_eq!(active_items(h, HORN).await?, 1);
    let achievements: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/honorary_barbarian'")
        .fetch_one(&h.pool).await?;
    assert_eq!(achievements, 1);
    Ok(())
}

async fn active_items(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(item).fetch_one(&h.pool).await?)
}
