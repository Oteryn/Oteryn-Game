//! Capture the real capability-64 receipts consumed by the native M01 client adapter.
use super::*;
use oteryn_dev_client::client_runtime::target_move::{WalkDisposition, WalkResponse, project};
use oteryn_dev_client::native_session::GatedCommandResult;
use oteryn_protocol_oteryn::target_move::*;

#[path = "../../../../../tools/qualification/e2e_all/m01_replay.rs"]
mod m01_replay;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::observe(journey(clients)).await {
        Ok(()) => evidence(
            "e2e_all scenario=M01.client status=PASS evidence=movement-relog-and-native-replay",
        ),
        Err(error) => {
            evidence(&format!("M01_CLIENT_CAPTURE_GAP {error}"));
            evidence(
                "e2e_all scenario=M01.client status=FAIL evidence=movement-relog-or-replay-failed",
            );
        }
    }
}

struct Capture {
    root: std::path::PathBuf,
    run: String,
    batch: String,
    session: String,
}

impl Capture {
    fn start(client: &Client) -> TestResult<Self> {
        let root = std::path::PathBuf::from(required("E2E_ALL_CAPTURE_DIR")?);
        let run = required("M01_CAPTURE_RUN")?;
        let batch = required("OTERYN_E2E_ROW_BATCH")?;
        if run.is_empty() || run.contains(char::is_whitespace) || !m01_replay::selects_m01(&batch) {
            return Err(
                "M01 requires a fresh run identity and a batch selecting M01.client".into(),
            );
        }
        let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
        let capture = Self {
            root,
            run,
            batch,
            session,
        };
        capture.write(
            "run",
            format!(
                "M01.client {} {} {}\n",
                capture.batch, capture.run, capture.session
            )
            .as_bytes(),
        )?;
        Ok(capture)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> TestResult {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join(format!("m01-{name}")))?
            .write_all(bytes)?;
        Ok(())
    }

    fn verify(self) -> TestResult {
        self.write("complete", b"M01.client\n")?;
        m01_replay::replay(&self.root, &self.run, &self.batch, &self.session)?;
        evidence(&format!("M01_CHECK replay PASS session={}", self.session));
        Ok(())
    }

    async fn command(
        &self,
        client: &mut oteryn_dev_client::DevClientSession,
        name: &str,
        intent: TargetMoveIntent,
    ) -> TestResult<u64> {
        let payload = encode_target_move_intent(intent)
            .map_err(|error| format!("M01 {name} encode: {error:?}"))?;
        let result = client
            .session_mut()
            .gated_command(
                CAPABILITY_TARGET_MOVE_V1,
                COMMAND_TYPE_TARGET_MOVE_INTENT,
                &payload,
            )
            .await?;
        let response = project(&result)?;
        let expected = WalkResponse {
            status: CommandStatus::Accepted,
            disposition: Some(WalkDisposition::Accepted),
        };
        if response != expected {
            return Err(format!("M01 {name} observed {response:?}, expected {expected:?}").into());
        }
        let mut bytes = format!(
            "{} {} {} {}\n",
            self.session, result.command_id, result.status as u32, result.server_sequence
        )
        .into_bytes();
        bytes.extend(&result.payload);
        self.write(name, &bytes)?;
        Ok(result.server_sequence)
    }
}

type Client = oteryn_dev_client::DevClientSession;
const LANDING_DEADLINE: Duration = Duration::from_secs(8);

// The retained temple corridor is x=32369, floor -7. Assert each fixed landing.
fn temple(north: u16) -> ActorPosition {
    ActorPosition {
        x: 32369,
        y: i32::from(32238 - north),
        floor: 7,
    }
}

fn position(client: &Client) -> ActorPosition {
    client.world_spatial().actor_position
}

fn own_motion(client: &Client) -> Option<oteryn_protocol_oteryn::actor_motion::ActorMotion> {
    let entities = client.world_entities()?;
    entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())?
        .motion
}

async fn step_once(
    client: &mut Client,
    direction: StepDirection,
    expected: ActorPosition,
) -> TestResult<Duration> {
    let forecast_index = match direction {
        StepDirection::North => 0,
        StepDirection::East => 1,
        StepDirection::South => 2,
        StepDirection::West => 3,
        _ => return Err("M01 requires a cardinal STEP".into()),
    };
    let forecast = own_motion(client)
        .ok_or("M01 STEP forecast absent")?
        .prewalk_ms[forecast_index];
    let outcome = client.step(direction).await?;
    if outcome.disposition != StepDisposition::Moved || position(client) != expected {
        return Err(format!(
            "M01 STEP {direction:?} disposition={:?} expected={expected:?} actual={:?}",
            outcome.disposition,
            position(client)
        )
        .into());
    }
    let committed = own_motion(client).ok_or("M01 STEP motion absent")?;
    if forecast == 0 || outcome.step_wait_ms != forecast || committed.step_duration_ms != forecast {
        return Err(format!(
            "M01 STEP pacing forecast={forecast} receipt={} motion={}",
            outcome.step_wait_ms, committed.step_duration_ms
        )
        .into());
    }
    Ok(Duration::from_millis(u64::from(outcome.step_wait_ms)))
}

async fn observe(
    client: &mut Client,
    label: &str,
    condition: impl Fn(&Client) -> bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !condition(client) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("M01 {label} not observed at {:?}", position(client)).into());
        }
        client.service_liveness(Duration::from_millis(10)).await?;
    }
    Ok(())
}

fn landing_is_causal(sequence: u64, boundary: u64, peer_seen: bool) -> bool {
    sequence > boundary && peer_seen
}

async fn expect_path(
    client: &mut Client,
    label: &str,
    start: ActorPosition,
    receipt_sequence: u64,
    landings: &[ActorPosition],
    peer_landing: Option<(
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
        ActorPosition,
    )>,
) -> TestResult {
    use oteryn_dev_client::native_session::SessionEvent;
    use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
    let mut deadline = tokio::time::Instant::now() + LANDING_DEADLINE;
    let mut previous = start;
    let mut remaining = landings;
    let mut peer_seen = peer_landing.is_none();
    loop {
        for event in client.take_events() {
            let (sequence, at) = match event {
                SessionEvent::WorldSpatial(delta) => {
                    (delta.server_sequence, delta.value.actor_position)
                }
                SessionEvent::WorldSpatialEntities(delta) => {
                    if let Some((identity, expected)) = peer_landing {
                        peer_seen |= delta.server_sequence > receipt_sequence
                            && delta.value.update.iter().any(|entity| {
                                entity.entity == identity && entity.position == expected
                            });
                    }
                    (delta.server_sequence, delta.value.actor_position)
                }
                SessionEvent::TutorialNotice(delta) if matches!(&delta.value, TutorialNotice::StatusText(text) if text == "There is no way.") =>
                {
                    return Err(format!("M01 {label}: There is no way.").into());
                }
                _ => continue,
            };
            if at == previous {
                continue;
            }
            if !landing_is_causal(sequence, receipt_sequence, peer_seen) {
                return Err(format!("M01 {label} premature landing {at:?} sequence={sequence} cutoff={receipt_sequence} peer_seen={peer_seen}").into());
            }
            let Some((expected, rest)) = remaining.split_first() else {
                return Err(format!("M01 {label} moved beyond the expected path to {at:?}").into());
            };
            if &at != expected {
                return Err(
                    format!("M01 {label} expected landing {expected:?}, observed {at:?}").into(),
                );
            }
            previous = at;
            remaining = rest;
            deadline = tokio::time::Instant::now() + LANDING_DEADLINE;
        }
        if remaining.is_empty() {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("M01 {label} did not reach {remaining:?} from {previous:?}").into(),
            );
        }
        client.service_liveness(Duration::from_millis(10)).await?;
    }
}

fn assert_stationary(client: &mut Client, label: &str, expected: ActorPosition) -> TestResult {
    use oteryn_dev_client::native_session::SessionEvent;
    for event in client.take_events() {
        let at = match event {
            SessionEvent::WorldSpatial(delta) => delta.value.actor_position,
            SessionEvent::WorldSpatialEntities(delta) => delta.value.actor_position,
            _ => continue,
        };
        if at != expected {
            return Err(format!("M01 {label} translated to {at:?}").into());
        }
    }
    if position(client) != expected {
        return Err(format!("M01 {label} translated to {:?}", position(client)).into());
    }
    Ok(())
}

fn own_facing(client: &Client) -> Option<StepDirection> {
    use oteryn_protocol_oteryn::world_spatial_entities::EntityDetail;
    let entities = client.world_entities()?;
    match entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())?
        .detail
    {
        EntityDetail::Actor { direction, .. } | EntityDetail::ActorWithIcons { direction, .. } => {
            Some(direction)
        }
        _ => None,
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x6d).await?;
    let capture = Capture::start(&client)?;
    if position(&client) != temple(0) {
        return Err(format!("M01 town entry {:?}", position(&client)).into());
    }
    step_once(&mut client, StepDirection::North, temple(1)).await?;
    evidence("M01_CHECK step PASS landed=32369,32237,7");
    client.take_events();
    let receipt_sequence = capture
        .command(&mut client, "autowalk", TargetMoveIntent::Walk(temple(6)))
        .await?;
    expect_path(
        &mut client,
        "autowalk arrival",
        temple(1),
        receipt_sequence,
        &[temple(2), temple(3), temple(4), temple(5), temple(6)],
        None,
    )
    .await?;
    evidence("M01_CHECK autowalk PASS landed=32369,32232,7");

    let receipt_sequence = capture
        .command(
            &mut client,
            "return-walk",
            TargetMoveIntent::Walk(temple(3)),
        )
        .await?;
    expect_path(
        &mut client,
        "return walk arrival",
        temple(6),
        receipt_sequence,
        &[temple(5), temple(4), temple(3)],
        None,
    )
    .await?;
    evidence("M01_CHECK return-walk PASS landed=32369,32235,7");

    let receipt_sequence = capture
        .command(
            &mut client,
            "interrupt-walk",
            TargetMoveIntent::Walk(temple(6)),
        )
        .await?;
    expect_path(
        &mut client,
        "active route intermediate landing",
        temple(3),
        receipt_sequence,
        &[temple(4)],
        None,
    )
    .await?;
    capture
        .command(&mut client, "stop", TargetMoveIntent::Stop)
        .await?;
    let stopped = temple(4);
    let deadline = tokio::time::Instant::now() + LANDING_DEADLINE;
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(10)).await?;
        assert_stationary(&mut client, "stop", stopped)?;
    }
    evidence("M01_CHECK stop PASS intermediate=32369,32234,7");

    for (name, facing) in [
        ("turn-east", StepDirection::East),
        ("turn-south", StepDirection::South),
        ("turn-west", StepDirection::West),
        ("turn-north", StepDirection::North),
    ] {
        let before = own_facing(&client).ok_or("M01 facing pre-state absent")?;
        if before == facing {
            return Err(format!("M01 {name} had no facing transition").into());
        }
        capture
            .command(&mut client, name, TargetMoveIntent::Turn(facing))
            .await?;
        observe(&mut client, name, |client| {
            own_facing(client) == Some(facing)
        })
        .await?;
        assert_stationary(&mut client, name, stopped)?;
        evidence(&format!(
            "M01_CHECK {name} PASS facing={facing:?} position={stopped:?}"
        ));
    }

    let mut peer = super::mp::observations::while_servicing_liveness(
        connect_character(clients, 0x6e, 0),
        async || {
            client.service_liveness(Duration::from_millis(25)).await?;
            assert_stationary(&mut client, "after TURN", stopped)
        },
    )
    .await??;
    if position(&peer) != temple(0) {
        return Err(format!("M01 peer town entry {:?}", position(&peer)).into());
    }
    step_once(&mut peer, StepDirection::North, temple(1)).await?;
    evidence("M01_CHECK peer-step-north PASS landed=32369,32237,7");
    let entities = peer.world_entities().ok_or("M01 peer entity view")?;
    let target = entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .ok_or("M01 peer identity absent")?
        .entity;
    observe(&mut client, "visible follow target", |client| {
        client
            .world_entities()
            .and_then(|entities| entities.get(&target))
            .is_some_and(|entity| entity.position == temple(1))
    })
    .await?;
    assert_stationary(&mut client, "before FOLLOW", stopped)?;
    let receipt_sequence = capture
        .command(&mut client, "follow", TargetMoveIntent::Follow(target))
        .await?;
    expect_path(
        &mut client,
        "initial pursuit",
        temple(4),
        receipt_sequence,
        &[temple(3), temple(2)],
        None,
    )
    .await?;
    evidence("M01_CHECK initial-follow PASS landed=32369,32236,7");

    client.service_liveness(Duration::from_millis(25)).await?;
    assert_stationary(&mut client, "before peer STEP", temple(2))?;
    let before_peer_step = client.last_server_sequence();
    step_once(&mut peer, StepDirection::South, temple(0)).await?;
    evidence("M01_CHECK peer-step-south PASS landed=32369,32238,7");
    expect_path(
        &mut client,
        "pursuit of moved peer",
        temple(2),
        before_peer_step,
        &[temple(1)],
        Some((target, temple(0))),
    )
    .await?;
    evidence("M01_CHECK continued-follow PASS landed=32369,32237,7");

    let saved = temple(1);
    if position(&client) != saved {
        return Err(format!(
            "M01 pre-logout expected {saved:?}, got {:?}",
            position(&client)
        )
        .into());
    }
    let client_session = client.join_snapshot().game_session_id;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!(
            "M01 logout refused: {:?} command_id={} server_sequence={}",
            logout.result, logout.command_id, logout.result_server_sequence
        )
        .into());
    }
    drop(client);
    let relog = connect_world(clients, 0x6f).await?;
    if relog.join_snapshot().game_session_id == client_session || position(&relog) != saved {
        return Err(format!("M01 relog expected {saved:?}, got {:?}", position(&relog)).into());
    }
    evidence(&format!("M01_CHECK relog PASS position={saved:?}"));
    capture.verify()
}

#[test]
fn m01_rejects_early_and_unwitnessed_landings() {
    assert!(!landing_is_causal(9, 10, true));
    assert!(!landing_is_causal(10, 10, true));
    assert!(!landing_is_causal(11, 10, false));
    assert!(landing_is_causal(11, 10, true));
}
