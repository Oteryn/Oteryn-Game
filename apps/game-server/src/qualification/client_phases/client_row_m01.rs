//! Capture the real capability-64 receipts consumed by the native M01 client adapter.
use super::*;
use oteryn_dev_client::client_runtime::target_move::{WalkDisposition, WalkResponse, project};
use oteryn_protocol_oteryn::target_move::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::observe(journey(clients)).await {
        Ok(()) => {
            evidence("e2e_all scenario=M01.client status=PASS evidence=real-target-move-replies")
        }
        Err(error) => evidence(&format!("M01_CLIENT_CAPTURE_GAP {error}")),
    }
}

struct Capture {
    root: std::path::PathBuf,
}

impl Capture {
    fn start() -> TestResult<Self> {
        let root = std::path::PathBuf::from(required("E2E_ALL_CAPTURE_DIR")?);
        let run = required("M01_CAPTURE_RUN")?;
        let batch = required("OTERYN_E2E_ROW_BATCH")?;
        if run.is_empty() || run.contains(char::is_whitespace) || batch != "128:1" {
            return Err("M01 requires a fresh run identity and batch 128:1".into());
        }
        let capture = Self { root };
        capture.write("run", format!("M01.client {batch} {run}\n").as_bytes())?;
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

    async fn command(
        &self,
        client: &mut oteryn_dev_client::DevClientSession,
        name: &str,
        intent: TargetMoveIntent,
    ) -> TestResult {
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
            "{} {} {}\n",
            result.command_id, result.status as u32, result.server_sequence
        )
        .into_bytes();
        bytes.extend(&result.payload);
        self.write(name, &bytes)
    }
}

type Client = oteryn_dev_client::DevClientSession;

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

async fn step_once(
    client: &mut Client,
    direction: StepDirection,
    expected: ActorPosition,
) -> TestResult<Duration> {
    let outcome = client.step(direction).await?;
    if outcome.disposition != StepDisposition::Moved || position(client) != expected {
        return Err(format!(
            "M01 STEP {direction:?} disposition={:?} expected={expected:?} actual={:?}",
            outcome.disposition,
            position(client)
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
    let capture = Capture::start()?;
    let mut client = connect_world(clients, 0x6d).await?;
    if position(&client) != temple(0) {
        return Err(format!("M01 town entry {:?}", position(&client)).into());
    }
    step_once(&mut client, StepDirection::North, temple(1)).await?;
    capture
        .command(&mut client, "autowalk", TargetMoveIntent::Walk(temple(3)))
        .await?;
    observe(&mut client, "autowalk arrival", |client| {
        position(client) == temple(3)
    })
    .await?;
    evidence("M01_CHECK autowalk PASS landed=32369,32235,7");

    capture
        .command(
            &mut client,
            "interrupt-walk",
            TargetMoveIntent::Walk(temple(6)),
        )
        .await?;
    observe(&mut client, "active route intermediate landing", |client| {
        position(client) == temple(4)
    })
    .await?;
    capture
        .command(&mut client, "stop", TargetMoveIntent::Stop)
        .await?;
    let stopped = temple(4);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(10)).await?;
        if position(&client) != stopped {
            return Err(
                format!("M01 stop moved from {stopped:?} to {:?}", position(&client)).into(),
            );
        }
    }
    evidence("M01_CHECK stop PASS intermediate=32369,32234,7");

    for (name, facing) in [
        ("turn-east", StepDirection::East),
        ("turn-south", StepDirection::South),
        ("turn-west", StepDirection::West),
        ("turn-north", StepDirection::North),
    ] {
        if own_facing(&client) == Some(facing) {
            return Err(format!("M01 {name} had no facing transition").into());
        }
        capture
            .command(&mut client, name, TargetMoveIntent::Turn(facing))
            .await?;
        observe(&mut client, name, |client| {
            own_facing(client) == Some(facing)
        })
        .await?;
        if position(&client) != stopped {
            return Err(format!("M01 {name} translated to {:?}", position(&client)).into());
        }
        evidence(&format!(
            "M01_CHECK {name} PASS facing={facing:?} position={stopped:?}"
        ));
    }

    let mut peer =
        provisioning::with_peer_liveness(&mut client, connect_character(clients, 0x6e, 0)).await?;
    if position(&peer) != temple(0) {
        return Err(format!("M01 peer town entry {:?}", position(&peer)).into());
    }
    step_once(&mut peer, StepDirection::North, temple(1)).await?;
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
    capture
        .command(&mut client, "follow", TargetMoveIntent::Follow(target))
        .await?;
    observe(&mut client, "initial pursuit", |client| {
        position(client) == temple(2)
    })
    .await?;
    evidence("M01_CHECK initial-follow PASS landed=32369,32236,7");

    step_once(&mut peer, StepDirection::South, temple(0)).await?;
    observe(&mut client, "pursuit of moved peer", |client| {
        position(client) == temple(1)
    })
    .await?;
    evidence("M01_CHECK continued-follow PASS landed=32369,32237,7");

    let saved = position(&client);
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("M01 logout refused: {:?}", logout.result).into());
    }
    drop(client);
    let relog = connect_world(clients, 0x6f).await?;
    if position(&relog) != saved {
        return Err(format!("M01 relog expected {saved:?}, got {:?}", position(&relog)).into());
    }
    evidence(&format!("M01_CHECK relog PASS position={saved:?}"));
    capture.write("complete", b"M01.client\n")
}
