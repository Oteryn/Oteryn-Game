use super::*;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted},
    tcp_tls::{read_frame, write_frame},
};
use oteryn_protocol_oteryn::{ClientCommandValue, CommandStatus, decode_command_result, encode_client_command};
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;

fn varint(out: &mut Vec<u8>, mut n: u64) {
    while n >= 128 {
        out.push((n as u8 & 127) | 128);
        n >>= 7;
    }
    out.push(n as u8);
}
pub(super) fn named(op: u8, name: &str) -> Vec<u8> {
    let mut out = vec![8, op, 26];
    varint(&mut out, name.len() as u64);
    out.extend(name.as_bytes());
    out
}
pub(super) fn numbered(op: u8, number: u64) -> Vec<u8> {
    let mut out = vec![8, op, 16];
    varint(&mut out, number);
    out
}
pub(super) fn edit(entry: u64) -> Vec<u8> {
    let mut out = numbered(3, entry);
    out.extend([26, 0, 32, 0, 40, 1]);
    out
}
fn read_varint(bytes: &[u8], at: &mut usize) -> TestResult<u64> {
    let mut value = 0;
    for shift in (0..70).step_by(7) {
        let byte = *bytes.get(*at).ok_or("VIP result truncated")?;
        *at += 1;
        if shift == 63 && byte > 1 { return Err("VIP varint overflow".into()); }
        value |= u64::from(byte & 127) << shift;
        if byte < 128 { return Ok(value); }
    }
    Err("VIP result varint".into())
}
pub(super) fn fields(bytes: &[u8]) -> TestResult<Value> {
    let mut at = 0;
    let mut value = json!({});
    while at < bytes.len() {
        let tag = read_varint(bytes, &mut at)?;
        match tag {
            8 | 16 | 24 | 32 => {
                let key = match tag { 8 => "code", 16 => "revision", 24 => "number", _ => "wait" };
                value[key] = json!(read_varint(bytes, &mut at)?);
            }
            42 => {
                let len = usize::try_from(read_varint(bytes, &mut at)?)?;
                let end = at.checked_add(len).ok_or("VIP result length overflow")?;
                value["view"] = serde_json::from_slice(bytes.get(at..end).ok_or("VIP result view")?)?;
                at = end;
            }
            _ => return Err("VIP unexpected result field".into()),
        }
    }
    Ok(value)
}

pub(super) async fn run(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession, kind: u32, payload: &[u8]) -> TestResult<Vec<u8>> {
    let session = *admitted;
    assert!(session.continuity.selected_capabilities.command_selected(kind),
        "VIP fixture command not selected: kind={kind}, command={}", session.continuity.next_command_id);
    oteryn_protocol_oteryn::vip::decode_request(kind, payload).map_err(|error|
        format!("VIP fixture malformed request: kind={kind}, command={}, error={error:?}", session.continuity.next_command_id))?;
    let (mut server, mut client) = tokio::io::duplex(128 * 1024);
    let serving = async move {
        let end = serve_admitted(&mut server, session, owner, IDLE_LIVENESS).await;
        // An early refusal must close the stream so the client sees EOF and the
        // returned ConnectionEnd explains it, rather than a hidden 30-second wait.
        let _ = server.shutdown().await;
        end
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type() == MessageType::SnapshotCommit { break; }
        }
        let frame = encode_client_command(session.continuity.connection_generation, &ClientCommandValue {
            command_id: session.continuity.next_command_id,
            command_type: kind,
            payload,
        }).map_err(debug)?;
        write_frame(&mut client, &frame).await?;
        let bytes = loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(reply.command_id, session.continuity.next_command_id,
                    "VIP wire command mismatch: kind={kind}");
                assert_eq!(reply.status, CommandStatus::Accepted,
                    "VIP wire outer refusal: kind={kind}, command={}", session.continuity.next_command_id);
                break reply.payload.to_vec();
            }
        };
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(bytes)
    };
    let (end, reply) = tokio::time::timeout(Duration::from_secs(30), super::super::fixture::join_two(serving, driving)).await?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(current) => *admitted = current,
        other => return Err(format!("VIP wire end: {other:?}").into()),
    }
    reply
}
