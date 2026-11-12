//! N02 client replay capture: a real accepted Wheel allocation Save (and the current-state query
//! before it) for the named-build, export and import path. Client owner:
//! `apps/client/src/features/0148-wheel-allocation/presets.rs` over client-ui `settings.rs`.
//!
//! The Save writes the core slot, the replies are captured, then the original allocation is
//! restored so later rows start from the same Wheel.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::wheel_allocation::{self, WheelCommand, WheelResponse};
use oteryn_protocol_oteryn::wheel;

const CORE_SLOT: usize = 20;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("N02_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

fn request(action: u8, occurrence: [u8; 16], revision: u64, points: [u16; 36]) -> Vec<u8> {
    wheel::encode_wheel_request(&wheel::WheelAllocationRequest {
        action,
        occurrence,
        expected_revision: revision,
        points,
        vessels: None,
    })
    .unwrap_or_default()
}

async fn send(
    client: &mut DevClientSession,
    payload: Vec<u8>,
    name: &str,
    keep: bool,
) -> TestResult<(WheelResponse, wheel::WheelAllocationReply)> {
    if payload.is_empty() {
        return Err(format!("{name} request did not encode").into());
    }
    let command = WheelCommand::new(wheel::COMMAND_TYPE_WHEEL_ALLOCATION, payload)
        .ok_or("Wheel runtime rejected the command")?;
    let response = wheel_allocation::send(client.session_mut(), &command).await?;
    if !matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Err(format!("{name} status={:?}", response.status).into());
    }
    if keep {
        capture(&format!("n02-{name}-receipt.bin"), &response.payload);
        for (suffix, frame) in [
            ("wheel", &response.wheel),
            ("atelier", &response.atelier),
            ("resources", &response.resources),
        ] {
            if let Some(bytes) = frame {
                capture(&format!("n02-{name}-{suffix}.bin"), bytes);
            }
        }
    }
    let receipt = wheel::decode_wheel_reply(&response.payload)
        .map_err(|error| format!("{name} receipt: {error:?}"))?;
    let current = response.wheel.as_deref().ok_or("current Wheel frame absent")?;
    let current = wheel::decode_wheel_reply(current)
        .map_err(|error| format!("{name} current Wheel: {error:?}"))?;
    if response.atelier.is_none() || (!receipt.refused && receipt != current) {
        return Err(format!("{name} frames inconsistent: {receipt:?} {current:?}").into());
    }
    Ok((response, receipt))
}

fn csv(points: &[u16; 36]) -> String {
    points.iter().map(u16::to_string).collect::<Vec<_>>().join(",")
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xad).await?;
    if !client
        .selected_capabilities()
        .contains(&wheel::CAPABILITY_WHEEL_ALLOCATION_V1)
    {
        return Err("admission omitted Wheel capability 148".into());
    }
    let (_, initial) = send(&mut client, vec![8, 0], "query", true).await?;
    if initial.refused {
        return Err(format!("query refused: {initial:?}").into());
    }
    let mut target = initial.points;
    target[CORE_SLOT] = if initial.points[CORE_SLOT] == 25 { 0 } else { 25 };
    let (_, saved) = send(
        &mut client,
        request(1, v7(0xad, 1), initial.revision, target),
        "save",
        true,
    )
    .await?;
    if saved.refused || saved.revision != initial.revision + 1 || saved.points != target {
        return Err(format!("save was not accepted exactly: {saved:?}").into());
    }
    let expected = format!(
        "revision={}\npoints={}\nsaved_revision={}\nsaved_points={}\n",
        initial.revision,
        csv(&initial.points),
        saved.revision,
        csv(&saved.points)
    );
    capture("n02-expected.txt", expected.as_bytes());
    // Restore the original allocation; a failure is evidence only, the capture is complete.
    match send(
        &mut client,
        request(1, v7(0xad, 2), saved.revision, initial.points),
        "restore",
        false,
    )
    .await
    {
        Ok((_, restored)) if !restored.refused && restored.points == initial.points => {}
        other => evidence(&format!("N02_RESTORE_UNCONFIRMED {other:?}").replace(' ', "_")),
    }
    let _ = client.session_mut().logout().await;
    Ok(format!(
        "revision={}->{}|core_slot={}->{}",
        initial.revision, saved.revision, initial.points[CORE_SLOT], saved.points[CORE_SLOT]
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N02.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N02.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=N02.client status=FAIL evidence=timeout"),
    }
}
