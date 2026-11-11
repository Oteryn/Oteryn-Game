//! Native contact admission for the selected Grimvale programme.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_grimvale_native_contacts() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-grimvale-contacts", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-grimvale-contacts",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_GRIMVALE_CONTACTS POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn contacts(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_GRIMVALE_CONTACTS POSTGRES_STARTED version={version}");
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/world/npc-placements");
    for key in ["oteryn:npc.rhargu", "oteryn:npc.domizian"] {
        let mut count = 0;
        for entry in std::fs::read_dir(&directory)? {
            let path = entry?.path();
            if !path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("placements-"))
            {
                continue;
            }
            let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
            for record in doc["records"].as_array().ok_or("placement records")? {
                if record["declaration"]["npc"].as_str() == Some(key) {
                    count += 1;
                }
            }
        }
        assert_eq!(
            count, 1,
            "exactly one chosen native contact placement for {key}"
        );
    }
    let actors = super::classic_quests::install(
        owner,
        admitted,
        ["oteryn:npc.rhargu", "oteryn:npc.domizian"],
    )
    .await?;
    assert_ne!(actors[0], actors[1], "separate canonical NPC actors");
    let progress = super::questpc7_ferumbras::value(
        owner,
        admitted,
        "oteryn:quest-progress/questpc7/grimvale/programme",
    )
    .await?;
    assert_eq!(
        progress, -1,
        "contact admission cannot seed subject progress"
    );
    Ok(())
}
