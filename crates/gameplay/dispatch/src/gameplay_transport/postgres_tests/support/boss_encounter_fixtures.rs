use tokio::io::AsyncWriteExt;
use super::fixture::{self, debug};

use super::TestResult;

use crate::foundation::{MessageType, decode_wire_envelope};

use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS,
    serve_admitted,
};

use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};

use crate::gameplay_transport::ComposedFreshAdmission;

use oteryn_protocol_oteryn::boss_encounter::*;

use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};

// Direct connections have no node maintenance task. A bounded pass can retire
// its holder, or the live server can still be using it without any demand yet.
// Request and service readiness before a direct pass, without retrying that pass.
pub(super) async fn maintain_ready(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    if !owner.root.is_ready() {
        owner.root.request_ready();
    }
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    Ok(())
}

pub(super) async fn exchange_kind(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    maintain_ready(owner).await?;
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(4096);
    let serving = async move {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        write_frame(
            &mut client,
            &encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type: kind,
                    payload,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let reply = loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let r = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(r.command_id, id);
                assert!(r.payload.len() <= MAX_RESULT_BYTES);
                break (r.status, r.payload.to_vec());
            }
        };
        client.shutdown().await?;
        Ok(reply)
    };
    let (ended, result): (_, TestResult<(CommandStatus, Vec<u8>)>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    maintain_ready(owner).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    result.map_err(|error| format!("cooldown command {id} ended {ended:?}: {error}").into())
}

fn varint(bytes: &[u8], at: &mut usize) -> TestResult<u64> {
    let mut value = 0;
    for shift in (0..70).step_by(7) {
        let b = *bytes.get(*at).ok_or("short varint")?;
        *at += 1;
        if shift == 63 && b > 1 {
            return Err("varint overflow".into());
        }
        value |= u64::from(b & 127) << shift;
        if b < 128 {
            return Ok(value);
        }
    }
    Err("varint overflow".into())
}
fn bytes<'a>(input: &'a [u8], at: &mut usize) -> TestResult<&'a [u8]> {
    let len = usize::try_from(varint(input, at)?)?;
    let end = at.checked_add(len).ok_or("length overflow")?;
    let value = input.get(*at..end).ok_or("short bytes")?;
    *at = end;
    Ok(value)
}
pub(super) type CooldownPage = (u64, Vec<(String, u64)>, String);

pub(super) fn page(payload: &[u8]) -> TestResult<CooldownPage> {
    let mut at = 0;
    assert_eq!(varint(payload, &mut at)?, 8);
    let now = varint(payload, &mut at)?;
    let mut entries = Vec::new();
    let mut next = String::new();
    while at < payload.len() {
        match varint(payload, &mut at)? {
            18 => {
                let entry = bytes(payload, &mut at)?;
                let mut ea = 0;
                assert_eq!(varint(entry, &mut ea)?, 10);
                let key = std::str::from_utf8(bytes(entry, &mut ea)?)?.to_owned();
                assert_eq!(varint(entry, &mut ea)?, 16);
                let available = varint(entry, &mut ea)?;
                assert_eq!(ea, entry.len());
                entries.push((key, available));
            }
            26 => {
                next = std::str::from_utf8(bytes(payload, &mut at)?)?.to_owned();
            }
            34 => {
                // Bosstiary progress shares the authority-consistent snapshot. Cooldown
                // assertions below deliberately ignore its independently tested contents.
                let _ = bytes(payload, &mut at)?;
            }
            40 => {
                let _ = varint(payload, &mut at)?;
            }
            50 => {
                let _ = bytes(payload, &mut at)?;
            }
            _ => return Err("unknown result field".into()),
        }
    }
    Ok((now, entries, next))
}
