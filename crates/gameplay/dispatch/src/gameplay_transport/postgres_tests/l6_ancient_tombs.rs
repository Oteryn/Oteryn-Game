//! L6 instrument progression through actual USE dispatch and the fenced Quest owner.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::wire;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const INSTRUMENTS: [(u16, Position); 5] = [
    (
        3255,
        Position {
            x: 33188,
            y: 32660,
            floor: -15,
        },
    ),
    (
        3261,
        Position {
            x: 33190,
            y: 32669,
            floor: -15,
        },
    ),
    (
        3258,
        Position {
            x: 33190,
            y: 32660,
            floor: -15,
        },
    ),
    (
        3260,
        Position {
            x: 33189,
            y: 32669,
            floor: -15,
        },
    ),
    (
        3257,
        Position {
            x: 33191,
            y: 32669,
            floor: -15,
        },
    ),
];

const PREFIX: &str = "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/";

#[test]
fn l6_ancient_tombs_instruments_real_use_persists_order_and_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6_ancient_tombs_instruments");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks::scenario(
                "l6-ancient-instruments",
                INSTRUMENTS[0].1,
                false,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "L6 instrument scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED l6_ancient_tombs_instruments duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterGameplayFence,
    _trigger: Position,
) -> TestResult {
    eprintln!("POSTGRES_DATABASE_READY l6_ancient_tombs_instruments");
    assert_native_bindings()?;
    let mut command = 1;
    let digest = owner
        .bundle_map()
        .ok_or("instrument map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    let payload = |index: usize, revision| -> TestResult<Vec<u8>> {
        let (_, position) = INSTRUMENTS[index];
        let key =
            oteryn_world_bundle::bundle::placement_key(position.floor, position.x, position.y, 1)
                .ok_or("instrument placement")?;
        let mut placement = digest.clone();
        placement.extend_from_slice(&key.to_be_bytes());
        oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: revision,
        })
        .map_err(debug)
    };
    approach(owner, admitted, INSTRUMENTS[1].1, &mut command).await?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let refused = wire(owner, admitted, command, 2, payload(1, 0)?).await?;
    command += 1;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&refused).map_err(debug)?,
        UseDisposition::Rejected
    );
    let before = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        before
            .tracks()
            .get(&format!("{PREFIX}vashresamun_instruments"))
            .copied()
            .unwrap_or(0),
        0
    );
    // Start with no injected quest progress. Each earned step is a literal USE.
    for (index, (_, position)) in INSTRUMENTS.into_iter().enumerate() {
        approach(owner, admitted, position, &mut command).await?;
        // Exercise the next action after the production use cadence permits it.
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let intent = payload(index, 0)?;
        let reply = wire(owner, admitted, command, 2, intent.clone()).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed,
            "instrument {}",
            index + 1
        );
        let state = owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(
            state
                .tracks()
                .get(&format!("{PREFIX}vashresamun_instruments")),
            Some(&(i64::try_from(index)? + 1))
        );
        // Receipt replay returns the retained success with the original stale map revision.
        let replay = wire(owner, admitted, command, 2, intent).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&replay).map_err(debug)?,
            UseDisposition::Committed
        );
        command += 1;
    }
    let state = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        state.tracks().get(&format!("{PREFIX}vashresamuns_door")),
        Some(&1)
    );
    assert_eq!(
        state
            .tracks()
            .get(&format!("{PREFIX}vashresamuns_treasure")),
        Some(&2)
    );
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-callback/ancient-tombs-instrument-%'")
        .fetch_one(&harness.pool).await?;
    assert_eq!(receipts, 5);
    owner.clock_origin -= std::time::Duration::from_secs(2);
    approach(owner, admitted, INSTRUMENTS[0].1, &mut command).await?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let refused = wire(owner, admitted, command, 2, payload(0, 1)?).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&refused).map_err(debug)?,
        UseDisposition::Rejected
    );
    let after = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(after.tracks(), state.tracks());
    Ok(())
}

// Validate against the actual repository B3, independently of the fixture and callback JSON.
fn assert_native_bindings() -> TestResult {
    let index: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements/index.json"
    ))?)?;
    let palette = index["palette"].as_array().ok_or("native palette")?;
    let region = oteryn_world_bundle_compiler::b3::decode_region(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z15-x129-y127.b3"
        )),
        oteryn_world_bundle::bundle::TILE_LIMITS,
        &mut oteryn_world_bundle::sector::Budget {
            tiles: 65536,
            entries: 1_000_000,
        },
    )?;
    for (appearance, position) in INSTRUMENTS {
        let tile = region
            .sectors
            .iter()
            .flat_map(|(_, _, tiles)| tiles)
            .find(|tile| tile.x == position.x && tile.y == position.y)
            .ok_or("native instrument tile")?;
        assert_eq!(tile.house, 0);
        let matches: Vec<_> = tile
            .items
            .iter()
            .filter(|item| {
                item.depth == 0
                    && palette
                        .get(item.palette as usize)
                        .and_then(|row| row["source_item_id"].as_u64())
                        == Some(u64::from(appearance))
            })
            .collect();
        let [item] = matches.as_slice() else {
            return Err("native instrument is absent or ambiguous".into());
        };
        assert_eq!(item.attrs.action, None);
        assert_eq!(item.attrs.unique, None);
    }
    Ok(())
}

async fn approach(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: Position,
    command: &mut u64,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        StepDirection, StepDisposition, decode_step_result, encode_step_intent,
    };
    let destination = crate::foundation::MovementLocalPosition {
        x: i32::from(target.x) - 1,
        y: i32::from(target.y),
        floor: -i16::from(target.floor),
    };
    for _ in 0..64 {
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        if at == destination {
            return Ok(());
        }
        let direction = if at.x < destination.x {
            StepDirection::East
        } else if at.x > destination.x {
            StepDirection::West
        } else if at.y < destination.y {
            StepDirection::South
        } else {
            StepDirection::North
        };
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let result = wire(owner, admitted, *command, 1, encode_step_intent(direction)).await?;
        assert_eq!(
            decode_step_result(&result).map_err(debug)?,
            StepDisposition::Moved
        );
        *command += 1;
    }
    Err("instrument approach exceeded its bounded path".into())
}
