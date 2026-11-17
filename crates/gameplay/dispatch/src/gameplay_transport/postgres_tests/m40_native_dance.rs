//! All sixteen source dance entries through native STEP; no intermediate stages seeded.
//! Crystal201189f movements_mission3_dance_dance_evolution.lua:1-67.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::wire;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::{unnatural_selection as dance, world_callbacks::Position};
use oteryn_protocol_oteryn::world_spatial::{
    self as codec, StepDirection as Dir, StepDisposition as D,
};

#[path = "m40_native_dance/rollout.rs"]
mod rollout;

#[test]
fn m40_native_diagonal_dance_full_sequence_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    if rollout::receipt_process(concat!(
        module_path!(),
        "::m40_native_diagonal_dance_full_sequence_and_replay"
    ))? {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m40-native-dance", || {
        super::quest_world_callbacks::scenario(
            "unnatural-selection-dance-native",
            Position {
                x: 32991,
                y: 31496,
                floor: -1,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    println!("POSTGRES_PLAYER_JOURNEY M40 server_version={version}");
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    {
        use crate::durability::quest_state::{
            QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
        };
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key:
                        "oteryn:quest-transition/quest/u8_54/unnatural_selection/dance_status/npc_1"
                            .into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(950).map_err(debug)?,
                    )),
                },
                owner.quest_catalogue.clone().ok_or("catalogue")?,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    select(owner, admitted, false)?;
    let before = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    let (probe, _, _) = step(owner, admitted, Dir::SouthEast).await?;
    assert_eq!(
        probe.disposition,
        D::Rejected,
        "unselected diagonal cannot reach Movement"
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?,
        before
    );
    assert_eq!(stage(h, dance::DANCE_STATUS_TRACK).await?, Some(1));
    select(owner, admitted, true)?;
    // Wrong real tile entries reset the active dance instead of granting progress.
    let (cardinal, _, _) = step(owner, admitted, Dir::East).await?;
    assert_eq!(cardinal.disposition, D::Moved);
    assert!(cardinal.step_wait_ms > 0);
    assert_eq!(stage(h, dance::DANCE_STATUS_TRACK).await?, Some(1));
    assert_eq!(
        step(owner, admitted, Dir::West).await?.0.disposition,
        D::Moved
    );
    let mut at = dance::TILES[0];
    let mut diagonals = std::collections::BTreeSet::new();
    for (index, tile) in dance::SEQUENCE.into_iter().enumerate() {
        let next = dance::TILES[usize::from(tile - 1)];
        let direction = direction(
            i32::from(next.0) - i32::from(at.0),
            i32::from(next.1) - i32::from(at.1),
        )?;
        let (result, command, payload) = step(owner, admitted, direction).await?;
        assert_eq!(result.disposition, D::Moved, "source dance step {index}");
        let scale = if direction.is_diagonal() {
            diagonals.insert(direction as u8);
            3
        } else {
            1
        };
        assert_eq!(result.step_wait_ms, cardinal.step_wait_ms * scale);
        let snapshot = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        assert_eq!(
            (snapshot.position().x, snapshot.position().y),
            (i32::from(next.0), i32::from(next.1))
        );
        assert_eq!(
            stage(h, dance::DANCE_STATUS_TRACK).await?,
            Some(i64::try_from(index)? + 2)
        );
        wire(owner, admitted, command, 1, payload).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?,
            snapshot
        );
        assert_eq!(
            stage(h, dance::DANCE_STATUS_TRACK).await?,
            Some(i64::try_from(index)? + 2)
        );
        at = next;
    }
    assert_eq!(diagonals.len(), 4);
    assert_eq!(stage(h, dance::MISSION03_TRACK).await?, Some(3));
    assert_eq!(stage(h, dance::QUESTLINE_TRACK).await?, Some(7));
    let (requests,grants):(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key='oteryn:achievement/talented_dancer'),(SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/talented_dancer')").fetch_one(&h.pool).await?;
    assert_eq!((requests, grants), (1, 1));
    println!(
        "M40_NATIVE_DANCE sixteen_steps four_diagonals cost_x3 unselected_refused replay_exact achievement_once"
    );
    Ok(())
}

fn select(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    diagonal: bool,
) -> TestResult {
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.retain(|id| *id != codec::CAPABILITY_DIAGONAL_MOVEMENT_V1);
    selected.push(13);
    if diagonal {
        selected.push(codec::CAPABILITY_DIAGONAL_MOVEMENT_V1);
    }
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &selected,
        )
        .ok_or("dance capabilities")?;
    Ok(())
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: Dir,
) -> TestResult<(codec::StepResult, u64, Vec<u8>)> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(5))
        .ok_or("clock")?;
    let command = admitted.continuity.next_command_id;
    let payload = codec::encode_step_intent(direction);
    let result = wire(owner, admitted, command, 1, payload.clone()).await?;
    Ok((
        codec::decode_step_result_receipt_with_diagonal(
            &result,
            true,
            admitted
                .continuity
                .selected_capabilities
                .contains(codec::CAPABILITY_DIAGONAL_MOVEMENT_V1),
        )
        .map_err(debug)?,
        command,
        payload,
    ))
}
async fn stage(h: &Harness, track: &str) -> TestResult<Option<i64>> {
    super::m39_bone_journey::stage(h, track).await
}
fn direction(dx: i32, dy: i32) -> TestResult<Dir> {
    Ok(match (dx, dy) {
        (0, -1) => Dir::North,
        (1, 0) => Dir::East,
        (0, 1) => Dir::South,
        (-1, 0) => Dir::West,
        (1, -1) => Dir::NorthEast,
        (1, 1) => Dir::SouthEast,
        (-1, 1) => Dir::SouthWest,
        (-1, -1) => Dir::NorthWest,
        _ => return Err("source dance contains a non-neighbor step".into()),
    })
}
