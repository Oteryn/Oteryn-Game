//! A later EXPLORE visit cannot replace its actual predecessor visit.
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.authored.elvenbane_quest";
const PREFIX: &str = "oteryn:quest-progress/authored/elvenbane_quest/";

#[test]
fn m01b_elvenbane_dragon_floor_requires_first_native_visit() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e::scenario_thread::run("m01b-elvenbane-dragon", || {
            super::super::quest_world_callbacks::scenario(
                "m01b/elvenbane/s2/entry",
                Position {
                    x: 32591,
                    y: 31645,
                    floor: -5,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_elvenbane_dragon duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    let bytes: [&'static [u8]; 2] = [
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z07-x127-y123.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z05-x127-y123.b3"
        )),
    ];
    let crops = [((32590, 31645), 7), ((32591, 31645), 5)]
        .into_iter()
        .enumerate()
        .map(|(i, ((x, y), z))| NativeCrop {
            bytes: bytes[i],
            region: (z, 127, 123),
            bounds: (32589, 31644, 32594, 31647),
            start: (x + 1, y),
            expected: vec![ExpectedTile {
                position: (x, y),
                ids: vec![436],
                counted_entry: None,
            }],
        })
        .collect();
    native_scene(native, crops, &[])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("Elvenbane actor")?;
    koshei::step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (0, 0, 0),
        "later visit cannot invent predecessor credit"
    );
    koshei::relocate(
        owner,
        actor,
        admitted.game_session_id,
        Position {
            x: 32591,
            y: 31645,
            floor: -7,
        },
    )
    .await?;
    koshei::step(owner, admitted, 2, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 0, 0));
    koshei::relocate(
        owner,
        actor,
        admitted.game_session_id,
        Position {
            x: 32592,
            y: 31645,
            floor: -5,
        },
    )
    .await?;
    expired_replay(owner, admitted).await?;
    assert_eq!(
        counts(h).await?,
        (1, 0, 0),
        "a previously refused command cannot gain credit after the predecessor"
    );
    koshei::step(owner, admitted, 3, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (1, 1, 0));
    koshei::step(owner, admitted, 3, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "same command replay cannot farm either stage"
    );
    koshei::step(owner, admitted, 4, StepDirection::East).await?;
    koshei::step(owner, admitted, 5, StepDirection::West).await?;
    assert_eq!(
        counts(h).await?,
        (1, 1, 0),
        "fresh revisit cannot farm the later stage or finish the quest"
    );
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1), (SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$2), (SELECT count(*) FROM game_character_quest_states WHERE quest_key=$3 AND completed_character_revision IS NOT NULL)")
        .bind(format!("{PREFIX}s1")).bind(format!("{PREFIX}s2")).bind(QUEST).fetch_one(&h.pool).await?)
}

// The transport retains only the latest result. Older commands must close with
// the exact expiry fault, rather than returning a movement result.
async fn expired_replay(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::AsyncWriteExt;
    let payload = oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::West);
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: 1,
            command_type: 1,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let next = admitted.continuity.next_command_id;
    match serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await {
        ConnectionEnd::AdmittedThenClosed(
            after,
            crate::foundation::FoundationProtocolError::CommandOutcomeExpired,
        ) => {
            assert_eq!(after.continuity.next_command_id, next);
            *admitted = after;
            Ok(())
        }
        _ => Err("old command must be rejected with CommandOutcomeExpired".into()),
    }
}
