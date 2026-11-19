use super::classic_quests_fixtures::{install, progress, talk};

use super::fixture::Harness;

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::durability::character_authority::ReconciledCharacterAuthority;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC2_HERO_POSTGRES_SERVER_VERSION={version}");
    let npcs = install(
        owner,
        admitted,
        [
            "oteryn:npc.strange_pipe",
            "oteryn:npc.strange_pipe_1",
            "oteryn:npc.strange_pipe_2",
            "oteryn:npc.strange_pipe_3",
        ],
    )
    .await?;
    let tracks = [
        "access_door",
        "access_teleport1",
        "access_teleport2",
        "access_teleport3",
    ];
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    for (index, npc) in npcs.into_iter().enumerate() {
        let command = 28100 + u64::try_from(index)? * 10;
        talk(owner, admitted, npc, command, "hi").await?;
        for (other, track) in tracks.iter().enumerate() {
            assert_eq!(
                progress(
                    owner,
                    admitted,
                    authority,
                    &format!("oteryn:quest-progress/hero_rathleton/{track}")
                )
                .await?,
                i64::from(other <= index),
                "pipe {index}, track {track}"
            );
        }
        // Neither a repeated greeting nor source exposition earns another grant.
        talk(owner, admitted, npc, command + 1, "hi").await?;
        let lines = talk(owner, admitted, npc, command + 2, "madman").await?;
        assert!(lines.iter().any(|line| line.contains("laboratory")));
        talk(owner, admitted, npc, command + 3, "bye").await?;
    }
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        after - before,
        4,
        "exactly one durable grant per source pipe"
    );
    Ok(())
}
