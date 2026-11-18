use super::*;
use crate::foundation::{ChannelRuntimeV1, MovementPositionSnapshot, NodeId};
use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
use oteryn_protocol_oteryn::tutorial_notices::{TutorialNotice, decode_tutorial_notice};
use oteryn_protocol_oteryn::{decode_state_delta, decode_wire_envelope};
use std::cell::Cell;
use tokio::io::AsyncReadExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Copy)]
enum WalkCase {
    InvalidActor,
    FloorChange,
    Recovered,
    Refused,
    RetryUnreachable,
    Unreachable,
}

struct WalkOwner {
    position: MovementPositionSnapshot,
    routes: Cell<usize>,
    steps: Cell<usize>,
    case: WalkCase,
}

impl FreshAdmissionAuthority for WalkOwner {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        let at = self.position.position();
        Some(WorldSpatialObservation {
            content_generation: [7; 32],
            actor_position: crate::gameplay_transport::world_spatial::ActorPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        })
    }

    async fn item_walk_route(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        _: AutoWalkTarget,
    ) -> Result<Option<AutoWalkRoute>, ()> {
        let previous = self.routes.get();
        self.routes.set(previous + 1);
        if matches!(self.case, WalkCase::InvalidActor) {
            return Ok(None);
        }
        if matches!(self.case, WalkCase::Unreachable)
            || (previous > 0 && matches!(self.case, WalkCase::RetryUnreachable))
        {
            return Err(());
        }
        Ok(Some(AutoWalkRoute {
            deferred: false,
            follow: None,
            expected: self.position,
            steps: [StepDirection::East].into(),
        }))
    }

    async fn item_walk_valid(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        expected: MovementPositionSnapshot,
    ) -> bool {
        expected == self.position
    }

    async fn item_walk_step(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        expected: MovementPositionSnapshot,
        _: StepDirection,
        _: Option<(ExactActorRef, MovementPositionSnapshot)>,
    ) -> (
        StepOutcome,
        Option<Duration>,
        Option<MovementPositionSnapshot>,
    ) {
        assert_eq!(expected, self.position);
        let previous = self.steps.get();
        self.steps.set(previous + 1);
        let mut outcome = StepOutcome::rejected();
        if matches!(self.case, WalkCase::FloorChange)
            || (matches!(self.case, WalkCase::Recovered) && previous > 0)
        {
            outcome.disposition = StepDisposition::Moved;
        }
        // A floor-changing step deliberately has no continuation snapshot, like the owner.
        let current =
            (matches!(self.case, WalkCase::Recovered) && previous > 0).then_some(self.position);
        (outcome, None, current)
    }
}

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

fn id(tag: u8) -> [u8; 16] {
    let mut id = [tag; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

async fn scenario(case: WalkCase) -> TestResult {
    let world = WorldId::decode(&id(1))?;
    let channel = ChannelId::decode(&id(2))?;
    let session_id = GameSessionId::decode(&id(3))?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        NodeId::decode(&id(4))?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::test(world),
    )
    .map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session_id).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(debug)?;
    let position = runtime
        .borrow_movement_position()
        .read(actor)
        .map_err(debug)?;
    let owner = WalkOwner {
        position,
        routes: Cell::new(0),
        steps: Cell::new(0),
        case,
    };
    let mut admitted = AdmittedSession {
        game_session_id: session_id,
        world_id: world,
        channel_id: channel,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        item_fence: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[6, 31, 64],
            )
            .ok_or("capabilities")?,
            ..SessionContinuity::FRESH
        },
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let mut session = setup::prepare(
        &mut server,
        &mut admitted,
        &owner,
        IDLE_LIVENESS,
        actor,
        owner.observe(actor).await.ok_or("observation")?,
        integrity::Integrity::default(),
    )
    .await
    .map_err(|_| "prepare")?;
    let payload = oteryn_protocol_oteryn::target_move::encode_target_move_intent(
        TargetMoveIntent::Walk(oteryn_protocol_oteryn::world_spatial::ActorPosition {
            x: 10,
            y: 10,
            floor: 0,
        }),
    )
    .map_err(debug)?;
    let dispatch = route(
        &owner,
        &mut session,
        oteryn_protocol_oteryn::ClientCommandView {
            command_id: 1,
            command_type: 81,
            payload: &payload,
        },
    )
    .await
    .map_err(|_| "route")?;
    if matches!(case, WalkCase::InvalidActor) {
        assert!(
            matches!(dispatch, Dispatch::Reply(CommandStatus::Rejected, bytes) if bytes == [8, 3])
        );
    } else {
        assert!(matches!(dispatch, Dispatch::Reply(_, _)));
    }
    tick(&mut server, &owner, &mut session)
        .await
        .map_err(|_| "first tick")?;
    if !matches!(
        case,
        WalkCase::Unreachable | WalkCase::FloorChange | WalkCase::InvalidActor
    ) {
        assert_eq!(
            owner.routes.get(),
            2,
            "one refused cell step must recompute once"
        );
        if !matches!(case, WalkCase::RetryUnreachable) {
            assert!(
                session.target_walk.is_some(),
                "replacement route must be scheduled"
            );
            tick(&mut server, &owner, &mut session)
                .await
                .map_err(|_| "second tick")?;
            tick(&mut server, &owner, &mut session)
                .await
                .map_err(|_| "completion tick")?;
        }
    }
    assert!(session.target_walk.is_none());
    assert_eq!(
        owner.routes.get(),
        if matches!(
            case,
            WalkCase::Unreachable | WalkCase::FloorChange | WalkCase::InvalidActor
        ) {
            1
        } else {
            2
        }
    );
    assert_eq!(
        owner.steps.get(),
        if matches!(case, WalkCase::Unreachable | WalkCase::InvalidActor) {
            0
        } else if matches!(case, WalkCase::RetryUnreachable | WalkCase::FloorChange) {
            1
        } else {
            2
        }
    );
    drop(server);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await?;
    let mut notices = Vec::new();
    let mut frames = bytes.as_slice();
    while !frames.is_empty() {
        let size = usize::try_from(u32::from_be_bytes(frames[..4].try_into()?))?;
        let envelope = decode_wire_envelope(&frames[4..4 + size])?;
        if envelope.message_type() == oteryn_protocol_oteryn::MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload())?;
            if delta.domain_id == 23 {
                notices.push(decode_tutorial_notice(delta.payload).map_err(debug)?);
            }
        }
        frames = &frames[4 + size..];
    }
    assert_eq!(
        notices,
        if matches!(
            case,
            WalkCase::Recovered | WalkCase::FloorChange | WalkCase::InvalidActor
        ) {
            vec![]
        } else {
            vec![TutorialNotice::StatusText("There is no way.".into())]
        }
    );
    Ok(())
}

fn run(case: WalkCase) -> TestResult {
    match std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .start_paused(true)
                .build()?
                .block_on(scenario(case))
        })?
        .join()
    {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

#[test]
fn autowalk_refusal_retries_once_then_emits_no_way() -> TestResult {
    run(WalkCase::Refused)
}
#[test]
fn autowalk_refusal_recomputes_once_and_recovers_without_notice() -> TestResult {
    run(WalkCase::Recovered)
}
#[test]
fn autowalk_failed_recomputation_emits_no_way() -> TestResult {
    run(WalkCase::RetryUnreachable)
}
#[test]
fn autowalk_unreachable_target_emits_no_way_without_steps() -> TestResult {
    run(WalkCase::Unreachable)
}

#[test]
fn autowalk_floor_change_ends_without_retry_or_notice() -> TestResult {
    run(WalkCase::FloorChange)
}

#[test]
fn autowalk_invalid_actor_refusal_emits_no_pathfinding_notice() -> TestResult {
    run(WalkCase::InvalidActor)
}
