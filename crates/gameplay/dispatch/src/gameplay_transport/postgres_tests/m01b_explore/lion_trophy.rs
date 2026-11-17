//! Own-character prerequisite completion and Current Premium are independent gates.
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.lion_trophy_quest";
const PREREQUISITE: &str = "oteryn:quest.the_ice_islands_quest";
const TRACK: &str = "oteryn:quest-progress/chosen-source/lion_trophy_quest/s1";
const START: Position = Position {
    x: 32147,
    y: 31120,
    floor: -9,
};

#[test]
fn m01b_lion_trophy_requires_own_completed_ice_islands_and_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run("m01b-lion-trophy", || {
        super::super::quest_world_callbacks::scenario(
            "m01b/lion-trophy/s1/entry",
            Position {
                x: 32146,
                y: 31120,
                floor: -9,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_lion_trophy duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    native_scene(
        native,
        vec![
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z09-x125-y121.b3"
                )),
                region: (9, 125, 121),
                bounds: (32146, 31119, 32149, 31121),
                start: (START.x, START.y),
                expected: vec![
                    ExpectedTile {
                        position: (32146, 31120),
                        ids: vec![6685],
                        counted_entry: None,
                    },
                    ExpectedTile {
                        position: (START.x, START.y),
                        ids: vec![6683],
                        counted_entry: None,
                    },
                ],
            },
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x127-y124.b3"
                )),
                region: (7, 127, 124),
                bounds: (32662, 31969, 32666, 31972),
                start: (32664, 31970),
                expected: vec![ExpectedTile {
                    position: (32663, 31970),
                    ids: vec![4525],
                    counted_entry: None,
                }],
            },
        ],
        &[],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("Lion Trophy actor")?;
    h.seed_pvp_target()
        .await
        .map_err(|e| format!("Lion other-character fixture: {e}"))?;
    seed_prerequisite(h, 101, true).await?;
    // Establish a legitimate revision with the existing native writer before
    // arranging prerequisite records; never advance the live root with SQL.
    koshei::relocate(
        owner,
        actor,
        admitted.game_session_id,
        Position {
            x: 32664,
            y: 31970,
            floor: -7,
        },
    )
    .await?;
    koshei::step(owner, admitted, 1, StepDirection::West).await?;
    let initialized:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/chosen-source/longsword_quest/s1'")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        initialized, 1,
        "revision is established by a real native quest visit"
    );
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "the unrelated visit grants no Lion progress"
    );
    koshei::relocate(owner, actor, admitted.game_session_id, START).await?;
    koshei::step(owner, admitted, 2, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "another character's completion grants no credit"
    );
    assert!(
        !owner.has_pending_quest_callback(admitted.game_session_id),
        "prerequisite refusal is definite, never an unknown callback"
    );
    seed_prerequisite(h, 41, false).await?;
    koshei::relocate(owner, actor, admitted.game_session_id, START).await?;
    koshei::step(owner, admitted, 3, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "starting the prerequisite is not completion"
    );
    assert!(
        !owner.has_pending_quest_callback(admitted.game_session_id),
        "prerequisite refusal is definite, never an unknown callback"
    );
    seed_prerequisite(h, 41, true).await?;
    koshei::step(owner, admitted, 3, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "completion cannot retroactively credit the refused visit"
    );
    let premium = owner
        .premium_coordinator
        .take()
        .ok_or("Current Premium fixture")?;
    koshei::relocate(owner, actor, admitted.game_session_id, START).await?;
    koshei::step(owner, admitted, 4, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "completed prerequisite cannot replace Current Premium"
    );
    owner.premium_coordinator = Some(premium);
    koshei::step(owner, admitted, 4, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "Premium cannot retroactively credit a refused visit"
    );
    koshei::relocate(owner, actor, admitted.game_session_id, START).await?;
    koshei::step(owner, admitted, 5, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    koshei::step(owner, admitted, 5, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    koshei::step(owner, admitted, 6, StepDirection::East).await?;
    koshei::step(owner, admitted, 7, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "revisits do not finish the whole quest"
    );
    Ok(())
}

// Arrange only the prerequisite record, never Lion Trophy progress. The actual
// completion owner supplies these records in production; replica scope ends here.
async fn seed_prerequisite(h: &Harness, character: u8, completed: bool) -> TestResult {
    let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let hash = catalogue
        .catalogue()
        .definition_hash(PREREQUISITE)
        .ok_or("canonical prerequisite declaration")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    if character != 41 {
        sqlx::query("UPDATE game_character_roots SET character_revision=greatest(character_revision,2) WHERE character_id=encode($1,'hex')::uuid").bind(super::super::fixture::id(character).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,completed_character_revision,committed_character_revision) SELECT character_id,$2,'content-1',$3,character_revision,CASE WHEN $4 THEN character_revision ELSE NULL END,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,quest_key) DO UPDATE SET completed_character_revision=EXCLUDED.completed_character_revision,committed_character_revision=EXCLUDED.committed_character_revision")
        .bind(super::super::fixture::id(character).as_slice()).bind(PREREQUISITE).bind(hash.as_slice()).bind(completed).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/m01b/lion-trophy/s1/entry'),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(TRACK).bind(QUEST).fetch_one(&h.pool).await?)
}
