use super::classic_quests_fixtures::{progress, talk};
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const QUEST: &str = "oteryn:quest.dangerous_depths_quest";
const ORDNANCE: &str = "oteryn:quest-progress/quest/u11_50/dangerous_depths/gnomes/ordnance";
const DEADLINE: &str =
    "oteryn:quest-progress/quest/u11_50/dangerous_depths/gnomes/time_task_ordnance";

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    npc: EntityRef,
) -> TestResult {
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;

    // Escort completion has no runtime owner. Seed the completed task with a
    // 20-hour deadline on either side of the current time without sleeping.
    for (deadline, command, reply_len, expected) in [
        (
            now + 60,
            34101,
            1,
            "I don't need your help for now. Come back later.",
        ),
        (now - 60, 34102, 5, "Are you willing to help?"),
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for (track, value) in [(ORDNANCE, 3), (DEADLINE, deadline)] {
            sqlx::query(
                "INSERT INTO game_character_quest_tracks
                    (character_id, track_key, quest_key, value, committed_character_revision)
                 SELECT character_id, $2, $3, $4, character_revision
                 FROM game_character_roots WHERE character_id = encode($1, 'hex')::uuid
                 ON CONFLICT (character_id, track_key) DO UPDATE SET value = EXCLUDED.value",
            )
            .bind(id(41).as_slice())
            .bind(track)
            .bind(QUEST)
            .bind(value)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        owner.refresh_quest_session(admitted.game_session_id, false).await;

        let reply = talk(owner, admitted, npc, command, "ordnance").await?;
        assert_eq!(reply.len(), reply_len, "{reply:?}");
        assert_eq!(reply.last().map(String::as_str), Some(expected));
    }
    talk(owner, admitted, npc, 34103, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, ORDNANCE).await?, 1);
    Ok(())
}
