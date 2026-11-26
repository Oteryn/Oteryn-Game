//! P06 client replay capture: real WHEEL_ALLOCATION replies for the current-state query and a
//! stale-revision Save, each with the current Wheel/Atelier/resource frames that
//! `wheel_allocation::send` collects. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0148-wheel-allocation/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::wheel_allocation::{self, WheelCommand, WheelResponse};
use oteryn_protocol_oteryn::wheel;
use std::{io::Write, path::PathBuf};

const CORE_SLOT: usize = 20;

fn capture(roots: &[PathBuf], name: &str, bytes: &[u8]) -> TestResult {
    for root in roots {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(name))?
            .write_all(bytes)?;
    }
    Ok(())
}

async fn send(
    client: &mut DevClientSession,
    payload: Vec<u8>,
    name: &str,
    roots: &[PathBuf],
) -> TestResult<(WheelResponse, wheel::WheelAllocationReply)> {
    let command = WheelCommand::new(wheel::COMMAND_TYPE_WHEEL_ALLOCATION, payload)
        .ok_or("Wheel runtime rejected the command")?;
    let response = wheel_allocation::send(client.session_mut(), &command).await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("{name} status={:?}", response.status).into());
    }
    capture(roots, &format!("p06-{name}-receipt.bin"), &response.payload)?;
    for (suffix, frame) in [
        ("wheel", &response.wheel),
        ("atelier", &response.atelier),
        ("resources", &response.resources),
    ] {
        if let Some(bytes) = frame {
            capture(roots, &format!("p06-{name}-{suffix}.bin"), bytes)?;
        }
    }
    let current = response
        .wheel
        .as_deref()
        .ok_or("current Wheel frame absent")?;
    let current = wheel::decode_wheel_reply(current)
        .map_err(|error| format!("{name} current Wheel: {error:?}"))?;
    Ok((response, current))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let roots: Vec<_> = ["E2E_ALL_CAPTURE_DIR", "NODE_BOOT_P06_CAPTURE_DIR"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    if roots.is_empty() {
        return Err("Wheel replay capture directory absent".into());
    }
    let mut client = connect_world(clients, 0x28).await?;
    if !client
        .selected_capabilities()
        .contains(&wheel::CAPABILITY_WHEEL_ALLOCATION_V1)
    {
        return Err("admission omitted Wheel capability 148".into());
    }
    let (query, initial) = send(&mut client, vec![8, 0], "query", &roots).await?;
    let query_receipt = wheel::decode_wheel_reply(&query.payload)
        .map_err(|error| format!("query receipt: {error:?}"))?;
    if initial.refused || query_receipt != initial || query.atelier.is_none() {
        return Err(format!(
            "query receipt or current projection disagrees: {query_receipt:?} {initial:?}"
        )
        .into());
    }
    let mut points = [0; 36];
    points[CORE_SLOT] = 25;
    let mut occurrence = [0x28; 16];
    occurrence[6] = 0x70;
    occurrence[8] = 0x80;
    let payload = wheel::encode_wheel_request(&wheel::WheelAllocationRequest {
        action: 1,
        occurrence,
        expected_revision: initial.revision + 99,
        points,
        vessels: None,
    })
    .map_err(|error| format!("save encoder: {error:?}"))?;
    let (stale, after) = send(&mut client, payload, "stale", &roots).await?;
    let receipt = wheel::decode_wheel_reply(&stale.payload)
        .map_err(|error| format!("stale receipt: {error:?}"))?;
    if !receipt.refused
        || (wheel::WheelAllocationReply {
            refused: false,
            ..receipt.clone()
        }) != initial
        || after != initial
    {
        return Err(format!("stale save was not refused unchanged: {receipt:?} {after:?}").into());
    }
    let expected = format!(
        "revision={}\nallocated={}\ncore_slot={}\nstale_refused={}\nstale_current_revision={}\n",
        initial.revision,
        initial.points.iter().map(|p| u32::from(*p)).sum::<u32>(),
        initial.points[CORE_SLOT],
        receipt.refused,
        after.revision
    );
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("Wheel capture logout observed {logout:?}").into());
    }
    capture(&roots, "p06-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P06.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P06.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P06.client status=FAIL evidence=timeout"),
    }
}
