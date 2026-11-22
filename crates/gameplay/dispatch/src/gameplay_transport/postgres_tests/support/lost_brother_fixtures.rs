use super::classic_quests_fixtures::{install, progress, talk};
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

const TRACK: &str = "oteryn:quest-progress/quest/u10_80/the_lost_brother_quest";
const GUILD: &str = "oteryn:quest-progress/quest/u9_80/adventurers_guild/quest_line";

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [tarun] = install(owner, admitted, ["oteryn:npc.tarun"]).await?;
    talk(owner, admitted, tarun, 10, "hi").await?;
    assert!(
        !talk(owner, admitted, tarun, 11, "mission")
            .await?
            .is_empty()
    );
    talk(owner, admitted, tarun, 12, "no").await?;
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 0);
    talk(owner, admitted, tarun, 13, "mission").await?;
    talk(owner, admitted, tarun, 14, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 1);
    assert_eq!(progress(owner, admitted, authority, GUILD).await?, 1);

    let (step, _) = owner
        .paced_step(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            StepDirection::East,
        )
        .await;
    assert_eq!(step.disposition, StepDisposition::Moved);
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 2);

    let reward = || async {
        let result: (i64, i64) = sqlx::query_as(
            "SELECT total_experience, (SELECT count(*) FROM game_item_instances \
             WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3039') \
             FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        Ok::<_, sqlx::Error>(result)
    };
    let before = reward().await?;
    assert!(
        !talk(owner, admitted, tarun, 16, "mission")
            .await?
            .is_empty()
    );
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 3);
    assert_eq!(reward().await?, (before.0 + 3000, before.1 + 1));
    talk(owner, admitted, tarun, 17, "mission").await?;
    assert_eq!(reward().await?, (before.0 + 3000, before.1 + 1));
    Ok(())
}
