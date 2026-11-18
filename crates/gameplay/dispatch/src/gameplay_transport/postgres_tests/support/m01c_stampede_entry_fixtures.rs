use super::fixture::{Harness, debug};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) const NAME: &str = "m01b/m01c-stampede-entry";

pub(super) const CALLBACK: &str = "m01c-stampede-entry";

const KEY: &str = "oteryn:quest-world/m01c-stampede-entry";

const TRACK: &str = "oteryn:quest-progress/m01c-stampede-route";

const ELEPHANT: &str = "oteryn:creature.terrified_elephant";

pub(super) const SELECTOR: Position = Position {
    x: 32850,
    y: 32671,
    floor: -7,
};

/// The player starts on the walkable ground south-east of the source STEP tile
/// (the tile north of it holds no ground); it leaves before the spawn is due.
pub(super) fn start() -> Position {
    Position {
        x: SELECTOR.x + 1,
        y: SELECTOR.y + 1,
        ..SELECTOR
    }
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_crop};
    let ground = |x, y| ExpectedTile {
        position: (x, y),
        ids: vec![1019],
        counted_entry: None,
    };
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x128-y127.b3"
            )),
            region: (7, 128, 127),
            bounds: (32850, 32670, 32855, 32674),
            start: (32851, 32672),
            expected: vec![
                ExpectedTile {
                    position: (32850, 32670),
                    ids: vec![1020, 1060],
                    counted_entry: None,
                },
                ground(32850, 32671),
                ground(32850, 32673),
                ground(32851, 32672),
                ground(32852, 32673),
                ground(32853, 32672),
                ground(32854, 32673),
            ],
        },
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
    assert!(
        owner.premium_coordinator.is_some(),
        "the Premium trial runs with the current Premium fixture"
    );
    assert_eq!(counts(h).await?, (0, 0));
    assert!(
        owner
            .states
            .quest_callbacks
            .next_encounter_due_us()
            .is_none()
    );
    // The source approach is a diagonal STEP; the wire route refuses it unless
    // the session negotiated the real diagonal (and paced) movement capability.
    let offered = owner.offered_capabilities();
    let mut pending = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    pending.extend([
        oteryn_protocol_oteryn::world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1,
        oteryn_protocol_oteryn::world_spatial::CAPABILITY_PACED_MOVEMENT_V1,
    ]);
    let mut supported = std::collections::BTreeSet::new();
    while let Some(id) = pending.pop() {
        if supported.insert(id)
            && let Some(capability) = offered.iter().find(|capability| capability.id == id)
        {
            pending.extend_from_slice(capability.requires);
        }
    }
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            offered,
            &supported.into_iter().collect::<Vec<_>>(),
        )
        .ok_or("Stampede diagonal approach capabilities must be offered")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let reply = super::quest_world_callbacks_fixtures::wire(
        owner,
        admitted,
        1,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::NorthWest),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        counts(h).await?,
        (1, 1),
        "the timed personal trial passes preflight and commits its entry receipt"
    );
    let spawn_at = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("the committed entry schedules its native trial")?;
    owner.drain_quest_encounter_deadlines().await;
    if owner.owner_now().get() < spawn_at {
        assert_eq!(
            elephants(owner).await?,
            0,
            "no native spawn before its deadline"
        );
    }
    let due = spawn_at.checked_add(100_000).ok_or("spawn clock")?;
    if due > owner.owner_now().get() {
        owner.clock_origin = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_micros(due))
            .ok_or("semantic clock")?;
    }
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        elephants(owner).await?,
        5,
        "the scheduled trial spawns its five native elephants"
    );
    assert_eq!(
        counts(h).await?,
        (1, 1),
        "the spawn writes no further progress"
    );
    eprintln!("M01C_STAMPEDE_TIMED_PERSONAL_PREFLIGHT_EXECUTED");
    Ok(())
}

async fn elephants(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<usize> {
    let runtime = owner.runtime.lock().await;
    Ok(runtime
        .current_live_creature_census()
        .map_err(debug)?
        .into_iter()
        .filter(|actor| {
            runtime
                .companion_snapshot(*actor)
                .is_ok_and(|native| native.state.policy.definition_key == ELEPHANT)
        })
        .count())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1), \
         (SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$2)",
    )
    .bind(KEY)
    .bind(TRACK)
    .fetch_one(&h.pool)
    .await?)
}
