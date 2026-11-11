//! Tereban's complete eight-material hand-in programme through NPC dispatch.
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const PREFIX: &str = "oteryn:quest-progress/quest/u8_6/afathers_burden/";
const MATERIALS: [(&str, &str, u32); 8] = [
    ("strong sinew", "sinew", 11548),
    ("exquisite wood", "wood", 11547),
    ("spectral cloth", "cloth", 11546),
    ("exquisite silk", "silk", 11545),
    ("magic crystal", "crystal", 11552),
    ("mystic root", "root", 11551),
    ("old iron", "iron", 11549),
    ("flexible dragon scale", "scale", 11550),
];

#[test]
fn fathers_burden_eight_handins_cape_experience_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("fathers-burden-handins", || {
        super::quest_world_callbacks::scenario(
            "fathers-burden-handins",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED fathers_burden_eight_handins_cape_experience_and_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    tereban: EntityRef,
) -> TestResult {
    let initial_xp: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    for (index, (keyword, track, source)) in MATERIALS.into_iter().enumerate() {
        let command = 2700 + u64::try_from(index)? * 10;
        super::classic_quests::talk(owner, admitted, tereban, command, keyword).await?;
        let missing =
            super::classic_quests::talk(owner, admitted, tereban, command + 1, "yes").await?;
        assert!(!missing.is_empty(), "source missing-material reply");
        assert_eq!(
            super::classic_quests::progress(
                owner,
                admitted,
                authority,
                &format!("{PREFIX}{track}")
            )
            .await?,
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
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: format!("oteryn:item.tibia.i{source}"),
                count: 1,
            },
            300 + u64::try_from(index)?,
            true,
        )
        .await?;
        super::classic_quests::talk(owner, admitted, tereban, command + 2, "no").await?;
        super::classic_quests::talk(owner, admitted, tereban, command + 3, keyword).await?;
        super::classic_quests::talk(owner, admitted, tereban, command + 4, "yes").await?;
        assert_eq!(
            super::classic_quests::progress(
                owner,
                admitted,
                authority,
                &format!("{PREFIX}{track}")
            )
            .await?,
            2
        );
        assert_eq!(
            super::classic_quests::progress(
                owner,
                admitted,
                authority,
                &format!("{PREFIX}progress")
            )
            .await?,
            i64::try_from(index + 1)?
        );
        let xp: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(xp, initial_xp + 2500 * i64::try_from(index + 1)?);
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
            .bind(format!("oteryn:item.tibia.i{source}")).fetch_one(&h.pool).await?;
        assert_eq!(remaining, 0, "material must be consumed atomically");
        super::classic_quests::talk(owner, admitted, tereban, command + 5, keyword).await?;
        assert_eq!(
            super::classic_quests::progress(
                owner,
                admitted,
                authority,
                &format!("{PREFIX}progress")
            )
            .await?,
            i64::try_from(index + 1)?
        );
    }
    super::classic_quests::talk(owner, admitted, tereban, 2800, "mission").await?;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, &format!("{PREFIX}status"))
            .await?,
        2
    );
    let expected = initial_xp + 28_000;
    let actual: (i64, i64) = sqlx::query_as("SELECT (SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i11701')")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(actual, (expected, 1));
    super::classic_quests::talk(owner, admitted, tereban, 2801, "mission").await?;
    let again: (i64, i64) = sqlx::query_as("SELECT (SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i11701')")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(again, actual, "completed quest must not grant twice");
    Ok(())
}
