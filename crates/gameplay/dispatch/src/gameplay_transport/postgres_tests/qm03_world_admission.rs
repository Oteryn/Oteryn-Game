//! M03: native Inferno STEP entrance consumes the live, fenced Premium predicate.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_spatial::*;

#[test]
fn qm03_native_inferno_step_refuses_without_live_premium() -> TestResult {
    scenario("qm03/pits")
}

#[test]
fn qm03_native_inferno_step_commits_with_live_premium() -> TestResult {
    scenario("qm03/pits-premium")
}

fn scenario(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            callback(name)?
                .selector
                .position
                .ok_or("entrance position")?,
            false,
        )
    })
}

pub(super) fn callback(_name: &str) -> TestResult<Callback> {
    oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/qm03/pits")
        .ok_or_else(|| "Inferno entrance callback".into())
}

pub(super) fn world(
    _name: &str,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z10-x128-y126.b3"
            )),
            region: (10, 128, 126),
            bounds: (32789, 32329, 32794, 32334),
            start: (32792, 32331),
            expected: vec![ExpectedTile {
                position: (32791, 32331),
                ids: vec![354],
                counted_entry: None,
            }],
        },
    )
}

async fn state(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_character_quest_tracks),(SELECT count(*) FROM game_character_quest_receipts)")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    name: &str,
) -> TestResult {
    // Arrange the level only; commands use the runtime role and production catalogue.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=80 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let before = state(h).await?;
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::West),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?;
    if name.ends_with("-premium") {
        assert_eq!(
            position.position(),
            super::quest_world_callbacks::local(callback.selector.position.ok_or("entrance")?)
        );
        let progress: i64 = sqlx::query_scalar(
            "SELECT value::bigint FROM game_character_quest_tracks WHERE track_key=$1",
        )
        .bind(&callback.storage[0].track)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(progress, 1);
        assert_eq!(state(h).await?, (before.0 + 1, before.1 + 1, before.2 + 1));
    } else {
        assert_eq!(
            position.position(),
            super::quest_world_callbacks::local(
                oteryn_gameplay_quests::quest::world_callbacks::Position {
                    x: 32792,
                    y: 32331,
                    floor: -10
                }
            ),
            "refused STEP returns to its origin"
        );
        assert_eq!(
            state(h).await?,
            before,
            "refusal cannot advance progress or receipts"
        );
    }
    eprintln!("M03_DISPATCH_PG_EXECUTED {name}");
    Ok(())
}
