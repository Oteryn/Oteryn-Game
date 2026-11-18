//! Crystal 201189f8 data-global/npc/iskan.lua:53-127 through fenced NPC dispatch.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, classic_quests};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, npc_rules::NpcRuleCatalog, world_callbacks::Position,
};
use serde_json::{Value, json};

pub(super) const NAME: &str = "qcx62-iskan-musher-report";
const NPC: &str = "oteryn:npc.iskan";
const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
const BARBARIAN: &str = "oteryn:quest-progress/quest/u8_0/barbarian_test/questline";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission01";
const LANDING: Position = Position {
    x: 32325,
    y: 31049,
    floor: -7,
};

#[test]
fn iskan_requires_barbarian_completion_and_musher_report_before_nibelor_passage() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(
            NAME,
            Position {
                x: 32322,
                ..LANDING
            },
            false,
        )
    })
}

pub(super) fn catalogue(native: &QuestStateCatalogue) -> TestResult<QuestStateCatalogue> {
    let source =
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/iskan.lua:53-127";
    let guard = |track, op, value| json!({"kind":"storage", "track":track, "op":op, "value":value});
    let set = |track, value| json!({"kind":"set", "track":track, "value":value});
    let topic = |value| json!({"kind":"set_topic", "value":value});
    let rule = |key,
                words: &[&str],
                topics: Option<&[i32]>,
                guards: Vec<Value>,
                reply: &[&str],
                effects: Vec<Value>| {
        json!({
            "key": format!("oteryn:quest-npc-rule.ice-islands-iskan.{key}"),
            "quest": QUEST, "npc": NPC, "trigger":"say", "keywords": words, "topics": topics,
            "guards": guards, "reply": reply, "effects": effects, "sources": [source]
        })
    };
    let tracks = [QUESTLINE, MISSION]
        .into_iter()
        .map(|key| {
            let track = native.track(key).ok_or("Iskan progress track")?;
            Ok(json!({
                "key": key, "quest": QUEST, "initial": track.initial, "min": track.min,
                "max": track.max.max(3), "sources": [source]
            }))
        })
        .collect::<TestResult<Vec<_>>>()?;
    let admitted = || guard(BARBARIAN, "ge", 8);
    let program = json!({
        "schema":1, "quest":QUEST, "program_key":"oteryn:quest-npc-program.ice-islands-iskan",
        "tracks":tracks,
        "source_files":[{
            "path":"data-global/npc/iskan.lua",
            "revision":"201189f871d23597b82caf88fd611454a2ddefb8",
            "sha256":"d9169860bba1568a08460744b49630a94e4ca00a83072544cca9eed863a6f5ad"
        }],
        "rules":[
            rule("passage-ready", &["passage"], None, vec![admitted(), guard(QUESTLINE, "ge", 3)],
                &["So, do you want a passage to Nibelor my friend?"], vec![topic(2)]),
            rule("passage-before-report", &["passage"], None, vec![admitted(), guard(QUESTLINE, "lt", 3)],
                &["Do you want to Nibelor?"], vec![topic(2)]),
            rule("mission", &["mission"], None, vec![admitted(), guard(QUESTLINE, "eq", 1)],
                &["Well, one of the boys has run away. I think he got the scent of some beast. He's young and inexperienced so I can't blame the cub ...",
                  "I would like you to see after him. He should be somewhere north west of the town. He is probably marking his territory so you should be able to find his trace. Are you willing to do that?"],
                vec![topic(1)]),
            rule("report", &["mission"], None, vec![admitted(), guard(QUESTLINE, "eq", 2)],
                &["You are a friend of mine and the boys now. I tell you something. If you ever need to go to the isle of Nibelor, just ask me for a {passage}."],
                vec![set(QUESTLINE, 3), set(MISSION, 3), topic(0)]),
            rule("mission-confirm", &["yes"], Some(&[1]), vec![admitted(), guard(QUESTLINE, "eq", 1)],
                &["That's surprising. Take a piece of meat. If you find the boy, feed it to him. That will give him enough strength and incentive to return to his pack ...",
                  "Talk to him by calling his name 'Sniffler' and tell him you got meat for him. After he has eaten the meat, return here to talk to me about your mission."],
                vec![set(QUESTLINE, 1), set(MISSION, 1), topic(0)]),
            rule("passage-confirm", &["yes"], Some(&[2]), vec![admitted(), guard(QUESTLINE, "ge", 3)], &[],
                vec![json!({"kind":"teleport", "position": LANDING}),
                    json!({"kind":"magic_effect", "effect":"CONST_ME_TELEPORT"}), topic(0)]),
            rule("passage-refused", &["yes"], Some(&[2]), vec![admitted(), guard(QUESTLINE, "lt", 3)],
                &["Sorry, first time you have to do a mission for me."], vec![topic(0)])
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
) -> TestResult {
    let [iskan] = classic_quests::install(owner, admitted, [NPC]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    import_track(h, fence.character_id.as_bytes().as_slice(), QUESTLINE, 1).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;

    classic_quests::talk(owner, admitted, iskan, 19200, "hi").await?;
    classic_quests::talk(owner, admitted, iskan, 19201, "mission").await?;
    assert_eq!(
        classic_quests::progress(owner, admitted, authority, QUESTLINE).await?,
        1
    );

    import_track(h, fence.character_id.as_bytes().as_slice(), BARBARIAN, 8).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;

    classic_quests::talk(owner, admitted, iskan, 19202, "passage").await?;
    let refused = classic_quests::talk(owner, admitted, iskan, 19203, "yes").await?;
    assert!(refused.iter().any(|line| line.contains("first time")));
    let receipts = "SELECT count(*) FROM game_quest_npc_teleport_receipts";
    assert_eq!(
        sqlx::query_scalar::<_, i64>(receipts)
            .fetch_one(&h.pool)
            .await?,
        0
    );

    let request = classic_quests::talk(owner, admitted, iskan, 19204, "mission").await?;
    assert!(request.iter().any(|line| line.contains("run away")));
    let instructions = classic_quests::talk(owner, admitted, iskan, 19205, "yes").await?;
    assert!(instructions.iter().any(|line| line.contains("Sniffler")));
    assert_eq!(
        classic_quests::progress(owner, admitted, authority, MISSION).await?,
        1
    );

    // Sniffler's successful report was earned outside this conversation.
    import_track(h, fence.character_id.as_bytes().as_slice(), QUESTLINE, 2).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let report = classic_quests::talk(owner, admitted, iskan, 19206, "mission").await?;
    assert!(report.iter().any(|line| line.contains("friend of mine")));
    assert_eq!(
        classic_quests::progress(owner, admitted, authority, QUESTLINE).await?,
        3
    );
    assert_eq!(
        classic_quests::progress(owner, admitted, authority, MISSION).await?,
        3
    );
    classic_quests::talk(owner, admitted, iskan, 19207, "passage").await?;
    classic_quests::talk(owner, admitted, iskan, 19208, "yes").await?;

    let actor = admitted.runtime_actor.ok_or("actor")?;
    let placed = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        placed.position(),
        super::quest_world_callbacks::local(LANDING)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(receipts)
            .fetch_one(&h.pool)
            .await?,
        1
    );
    Ok(())
}

async fn import_track(h: &Harness, character: &[u8], track: &str, value: i64) -> TestResult {
    // Import source progress earned before the dialogue under test.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let inserted = sqlx::query(
        "INSERT INTO game_character_quest_tracks
         (character_id,track_key,quest_key,value,committed_character_revision)
         SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots
         WHERE character_id=encode($1,'hex')::uuid
         ON CONFLICT(character_id,track_key) DO UPDATE SET
         value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision",
    )
    .bind(character)
    .bind(track)
    .bind(QUEST)
    .bind(value)
    .execute(&mut *tx)
    .await?;
    assert_eq!(inserted.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}
