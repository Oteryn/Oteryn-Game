use super::fixture::Harness;

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

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
    let actors = super::classic_quests_fixtures::install(
        owner,
        admitted,
        ["oteryn:npc.rhargu", "oteryn:npc.domizian"],
    )
    .await?;
    assert_ne!(actors[0], actors[1], "separate canonical NPC actors");
    let progress = super::questpc7_ferumbras_fixtures::value(
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
