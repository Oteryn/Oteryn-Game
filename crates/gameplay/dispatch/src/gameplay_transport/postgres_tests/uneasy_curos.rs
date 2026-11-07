//! Curos source placement, native NPC commands, and atomic two-track quest acceptance.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition, world_callbacks::Position,
};
const NPC: &str = "oteryn:npc.curos";
const DOOR: &str = "oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/quest_door";
const LINE: &str = "oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/questline";
const FRONTIER: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/";
#[test]
fn uneasy_curos_requires_completed_frontier_mission() -> TestResult {
    scenario("missing")
}
#[test]
fn uneasy_curos_accepts_two_flags_atomically_and_replays() -> TestResult {
    scenario("start")
}
#[test]
fn uneasy_curos_preserves_preceding_frontier_branch() -> TestResult {
    scenario("precedence")
}
fn scenario(case: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(case, move || {
        super::quest_world_callbacks::scenario(
            &format!("uneasy-curos-{case}"),
            Position {
                x: 33175,
                y: 31393,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "UNEASY_CUROS POSTGRES_CASE_{} case={case} duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}
pub(super) fn source_placement() -> TestResult {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/npc-placements"
    ));
    for entry in std::fs::read_dir(path)? {
        let path = entry?.path();
        if !path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("placements-"))
        {
            continue;
        }
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        for row in value["records"].as_array().ok_or("placement records")? {
            let d = &row["declaration"];
            if d["npc"].as_str() == Some(NPC) {
                assert_eq!(d["cell"]["x"], 33174);
                assert_eq!(d["cell"]["y"], 31394);
                assert_eq!(d["cell"]["floor"], 7);
                return Ok(());
            }
        }
    }
    Err("native Curos placement missing from committed world source table".into())
}
/// Controlled prerequisite only: this does not qualify The New Frontier itself.
/// The normal fenced writer seeds both foreign guard values; target NPC code uses
/// its unchanged production catalogue and no fixture rule is installed there.
pub(super) async fn prerequisite(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    precedence: bool,
) -> TestResult {
    let quest = "oteryn:quest.the_new_frontier_quest";
    let key = "oteryn:quest-transition/fixture/uneasy-prerequisite";
    let values = [
        (format!("{FRONTIER}mission06"), 5, 5),
        (
            format!("{FRONTIER}questline"),
            if precedence { 16 } else { 19 },
            27,
        ),
    ];
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            values
                .iter()
                .map(|(key, _, max)| QuestTrack {
                    key: key.clone(),
                    quest: quest.into(),
                    initial: 0,
                    min: 0,
                    max: *max,
                })
                .collect(),
            vec![QuestTransition {
                world_change: None,
                key: key.into(),
                quest: quest.into(),
                effects: values
                    .iter()
                    .map(|(track, value, _)| QuestEffect {
                        track: track.clone(),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::Set(*value),
                    })
                    .collect(),
                completes: false,
                experience: None,
            }],
        )
        .map_err(debug)?,
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(9100).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "prerequisite fixture: {outcome:?}"
    );
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    assert!(version.starts_with("PostgreSQL"));
    eprintln!("UNEASY_CUROS actual_postgresql=true {version}");
    source_placement()?;
    if !name.ends_with("missing") {
        prerequisite(owner, admitted, name.ends_with("precedence")).await?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    if !name.ends_with("missing") {
        let copied = {
            let sessions = owner
                .states
                .quest_sessions
                .lock()
                .map_err(|_| "quest copy lock")?;
            sessions
                .get(&admitted.game_session_id)
                .and_then(|entry| entry.copy.as_ref())
                .and_then(|copy| copy.tracks().get(&format!("{FRONTIER}mission06")))
                .copied()
        };
        assert_eq!(
            copied,
            Some(5),
            "controlled prerequisite must reach the real NPC read copy"
        );
    }
    let [npc] = super::classic_quests::install(owner, admitted, [NPC]).await?;
    // The common NPC helper places at player x,y+1: this is exactly Curos's
    // committed source coordinate, rather than an arbitrary test-only anchor.
    let greet = super::classic_quests::talk(owner, admitted, npc, 9200, "hi").await?;
    assert!(
        greet.iter().any(|line| line == "Be greeted blank skin."),
        "literal Curos source greeting: {greet:?}"
    );
    let reply = super::classic_quests::talk(owner, admitted, npc, 9201, "mission").await?;
    let rows:Vec<(String,i64)>=sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key").bind(admitted.item_fence.ok_or("fence")?.character_id.as_bytes().as_slice()).bind(vec![DOOR,LINE]).fetch_all(&h.pool).await?;
    if name.ends_with("start") {
        assert!(
            reply
                .iter()
                .any(|s| s.contains("seek out this rebel commander")),
            "Curos source start reply: {reply:?}"
        );
        assert_eq!(rows, vec![(DOOR.into(), 0), (LINE.into(), 1)]);
        let receipt:Vec<(Vec<String>,Vec<i64>)>=sqlx::query_as("SELECT track_keys,values_after FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.an_uneasy_alliance_quest'").bind(admitted.item_fence.ok_or("fence")?.character_id.as_bytes().as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(
            receipt,
            vec![(vec![DOOR.into(), LINE.into()], vec![0, 1])],
            "one receipt owns both flag writes"
        );
        super::classic_quests::talk(owner, admitted, npc, 9201, "mission").await?;
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.an_uneasy_alliance_quest'").bind(admitted.item_fence.ok_or("fence")?.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(count, 1);
    } else {
        assert!(
            rows.is_empty(),
            "source prerequisites must not grant quest flags: {rows:?}"
        );
    }
    Ok(())
}
