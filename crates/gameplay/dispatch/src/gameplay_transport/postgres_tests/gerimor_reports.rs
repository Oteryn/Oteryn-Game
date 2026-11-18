use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, npc_rules::NpcRuleCatalog};
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) const NAME: &str = "qengine-rewards-gerimor-reports";
const QUEST: &str = "oteryn:quest.cults_of_tibia_quest";
const PREFIX: &str = "oteryn:quest-progress/quest/u11_40/cults_of_tibia/";
const MISSION: &str = "final_boss/mission";
const ACCESS: &str = "final_boss/access_door";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gerimor.lua:256-447";
const REPORTS: [(&str, &str, i64, i64); 7] = [
    ("minotaurs", "minotaurs/mission", 5, 25_000),
    ("prosperity", "mot_a/mission", 14, 50_000),
    ("barkless", "barkless/mission", 6, 50_000),
    ("orcs", "orcs/mission", 2, 25_000),
    ("life", "life/mission", 9, 50_000),
    ("misguided", "misguided/mission", 4, 50_000),
    ("humans", "humans/mission", 2, 25_000),
];

#[test]
fn gerimor_reports_preserve_prior_assignment_pin_and_settle_finale_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(NAME)
}

fn guard(track: &str, value: i64) -> Value {
    json!({"kind":"storage","track":format!("{PREFIX}{track}"),"op":"eq","value":value})
}

fn set(track: &str, value: i64) -> Value {
    json!({"kind":"set","track":format!("{PREFIX}{track}"),"value":value})
}

fn rule(name: &str, word: &str, topic: Option<i32>, guards: Vec<Value>, effects: Vec<Value>) -> Value {
    json!({
        "key":format!("oteryn:quest-npc/cults_of_tibia/gerimor-fixture/{name}"),
        "quest":QUEST, "npc":"oteryn:npc.gerimor", "trigger":"say",
        "keywords":[word], "keyword_mode":"exact", "topics":topic.map(|topic| vec![topic]),
        "guards":guards, "effects":effects, "sources":[SOURCE]
    })
}

fn source(base: &QuestStateCatalogue) -> TestResult<NpcRuleCatalog> {
    // Crystal 201189f8 gerimor.lua:256-339,356-447. Use fixture rules to exercise
    // the actual XP, inventory, programme predecessor and achievement owners.
    let mut rules: Vec<_> = REPORTS.into_iter().map(|(word, track, stage, xp)| {
        rule(&format!("report-{word}"), word, Some(2), vec![guard(track, stage)], vec![
            set(track, stage + 1),
            json!({"kind":"experience","amount":xp}),
            json!({"kind":"set_topic","value":0}),
        ])
    }).collect();
    let mut requirements: Vec<_> = REPORTS.into_iter()
        .map(|(_, track, stage, _)| guard(track, stage + 1)).collect();
    requirements.push(guard(MISSION, 0));
    rules.push(rule("finale-access", "mission", None, requirements, vec![
        set(MISSION, 1), set(ACCESS, 1), json!({"kind":"set_topic","value":0}),
    ]));
    rules.push(rule("finale-offer", "mission", None, vec![guard(MISSION, 2)], vec![
        json!({"kind":"set_topic","value":4}),
    ]));
    rules.push(rule("finale-reward", "yes", Some(4), vec![
        guard(MISSION, 2), json!({"kind":"vocation","value":"oteryn:vocation.monk"}),
    ], vec![
        set(MISSION, 3),
        json!({"kind":"give_item","item":"oteryn:item.tibia.i50313","count":1}),
        json!({"kind":"give_item","item":"oteryn:item.tibia.i26186","count":1}),
        json!({"kind":"experience","amount":50000}),
        json!({"kind":"achievement","name":"Corruption Contained"}),
        json!({"kind":"set_topic","value":0}),
    ]));
    let tracks = REPORTS.into_iter().map(|(_, track, stage, _)| (track, stage + 1))
        .chain([(MISSION, 3), (ACCESS, 1)])
        .map(|(suffix, max)| {
            let key = format!("{PREFIX}{suffix}");
            let track = base.track(&key).ok_or_else(|| format!("track {key}"))?;
            Ok::<_, String>(json!({
                "key":key,"quest":QUEST,"initial":track.initial,"min":track.min,"max":track.max.max(max),"sources":[SOURCE]
            }))
        }).collect::<Result<Vec<_>, _>>()?;
    let document = json!({
        "schema":1,"quest":QUEST,"program_key":"oteryn:quest-npc-program.gerimor-reports",
        "source_files":[{"path":"data-global/npc/gerimor.lua","revision":"201189f871d23597b82caf88fd611454a2ddefb8","sha256":"bf0fba86d07bb8a70d6242c2cc748fbb137cc7b583b47f0886530b6b5c584b2f"}],
        "tracks":tracks,"rules":rules
    });
    Ok(NpcRuleCatalog::from_json_documents(&[&document.to_string()])?)
}

async fn seed_stage(h: &Harness, suffix: &str, stage: i64) -> TestResult {
    sqlx::query(
        "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
         SELECT character_id,$2,quest_key,$3,committed_character_revision FROM game_character_quest_states
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$4
         ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value",
    )
    .bind(id(41).as_slice()).bind(format!("{PREFIX}{suffix}")).bind(stage).bind(QUEST)
    .execute(&h.pool).await?;
    Ok(())
}

async fn xp(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT total_experience + COALESCE((
            SELECT SUM(amount)::bigint FROM game_character_quest_xp_obligations o
            WHERE o.character_id=p.character_id),0)
         FROM game_character_progression_state p WHERE character_id=encode($1,'hex')::uuid",
    ).bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [npc] = install(owner, admitted, ["oteryn:npc.gerimor"]).await?;
    talk(owner, admitted, npc, 32000, "hi").await?;
    talk(owner, admitted, npc, 32001, "mission").await?;
    talk(owner, admitted, npc, 32002, "minotaurs").await?;
    talk(owner, admitted, npc, 32003, "yes").await?;
    seed_stage(h, "minotaurs/mission", 5).await?;
    let base = owner.quest_catalogue.as_ref().ok_or("base quest catalogue")?;
    owner.quest_catalogue = Some(Arc::new(source(base)?.attach_fixture(base)?));
    let before = xp(h).await?;

    talk(owner, admitted, npc, 32004, "mission").await?;
    talk(owner, admitted, npc, 32005, "minotaurs").await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}minotaurs/mission")).await?, 6);
    assert_eq!(xp(h).await? - before, 25_000);

    talk(owner, admitted, npc, 32006, "mission").await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}{ACCESS}")).await?, 0);
    for (_, track, stage, _) in REPORTS.into_iter().skip(1) {
        seed_stage(h, track, stage + 1).await?;
    }
    talk(owner, admitted, npc, 32007, "mission").await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}{ACCESS}")).await?, 1);
    seed_stage(h, MISSION, 2).await?;
    talk(owner, admitted, npc, 32008, "mission").await?;
    talk(owner, admitted, npc, 32009, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}{MISSION}")).await?, 3);
    assert_eq!(xp(h).await? - before, 75_000);
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    let inventory = h.root.read_item_use_inventory(authority, &h.node, fence.item_fence()).await?;
    let backpack = inventory.backpack.as_ref().ok_or("starter backpack")?;
    for item in ["oteryn:item.tibia.i50313", "oteryn:item.tibia.i26186"] {
        assert_eq!(backpack.entries.iter().filter(|entry| entry.item.definition.production_key == item).count(), 1);
    }
    talk(owner, admitted, npc, 32010, "yes").await?;
    assert_eq!(xp(h).await? - before, 75_000);
    let achievements: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements a
         JOIN game_character_roots c USING (account_id)
         WHERE c.character_id=encode($1,'hex')::uuid
           AND a.achievement_key='oteryn:achievement/corruption_contained'",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(achievements, 1);
    Ok(())
}
