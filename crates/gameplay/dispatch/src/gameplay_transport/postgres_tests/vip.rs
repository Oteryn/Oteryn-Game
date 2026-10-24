//! VIP-1 through the actual admitted wire dispatch and the installed production migration.
use super::fixture::{self, Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::CharacterId;
use crate::durability::{DurabilityRoot, guild_chat_fixture as characters};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use serde_json::{Value, json};
use sqlx::ConnectOptions;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

#[path = "vip/negatives.rs"]
mod negatives;

#[path = "vip/scenario.rs"]
mod scenario;

#[test]
fn vip_postgres_real_wire_consent_replay_restart_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario::run_scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "VIP PostgreSQL scenario panicked")??;
    Ok(())
}

fn push_varint(out: &mut Vec<u8>, mut n: u64) {
    while n >= 128 {
        out.push((n as u8 & 127) | 128);
        n >>= 7;
    }
    out.push(n as u8);
}
fn named(op: u8, name: &str) -> Vec<u8> {
    let mut out = vec![8, op, 26];
    push_varint(&mut out, name.len() as u64);
    out.extend(name.as_bytes());
    out
}
fn numbered(op: u8, n: u64) -> Vec<u8> {
    let mut out = vec![8, op, 16];
    push_varint(&mut out, n);
    out
}
fn edit(entry: u64, text: &str, icon: u8, notify: bool, groups: &[u8]) -> Vec<u8> {
    let mut out = numbered(3, entry);
    out.push(26);
    push_varint(&mut out, text.len() as u64);
    out.extend(text.as_bytes());
    out.extend([32, icon, 40, u8::from(notify)]);
    for g in groups {
        out.extend([48, *g]);
    }
    out
}
async fn login(client: &mut tokio::io::DuplexStream) -> TestResult {
    loop {
        let frame = read_frame(client).await?;
        let env = decode_wire_envelope(&frame).map_err(debug)?;
        if env.message_type() == MessageType::SnapshotCommit {
            return Ok(());
        }
    }
}
fn take_varint(bytes: &[u8], cursor: &mut usize) -> TestResult<u64> {
    let mut n = 0;
    for shift in (0..70).step_by(7) {
        let b = *bytes.get(*cursor).ok_or("truncated VIP result")?;
        *cursor += 1;
        if shift == 63 && b > 1 {
            return Err("VIP result overflow".into());
        }
        n |= u64::from(b & 127) << shift;
        if b < 128 {
            return Ok(n);
        }
    }
    Err("VIP result varint".into())
}
fn result_fields(bytes: &[u8]) -> TestResult<Value> {
    let mut cursor = 0;
    let mut out = json!({});
    while cursor < bytes.len() {
        let tag = take_varint(bytes, &mut cursor)?;
        match tag {
            8 | 16 | 24 | 32 => {
                let field = match tag {
                    8 => "code",
                    16 => "revision",
                    24 => "number",
                    _ => "wait",
                };
                assert!(out.get(field).is_none());
                out[field] = json!(take_varint(bytes, &mut cursor)?);
            }
            42 => {
                let len = usize::try_from(take_varint(bytes, &mut cursor)?)?;
                let end = cursor.checked_add(len).ok_or("VIP length overflow")?;
                out["view"] =
                    serde_json::from_slice(bytes.get(cursor..end).ok_or("VIP result view")?)?;
                cursor = end;
            }
            _ => return Err("unexpected VIP result field".into()),
        }
    }
    for field in ["code", "revision", "number", "wait"] {
        assert!(out[field].is_u64());
    }
    Ok(out)
}
fn code(bytes: &[u8]) -> TestResult<u8> {
    Ok(u8::try_from(
        result_fields(bytes)?["code"].as_u64().ok_or("code")?,
    )?)
}
fn number(bytes: &[u8]) -> TestResult<u64> {
    result_fields(bytes)?["number"]
        .as_u64()
        .ok_or_else(|| "number".into())
}
fn wait(bytes: &[u8]) -> TestResult<u8> {
    Ok(u8::try_from(
        result_fields(bytes)?["wait"].as_u64().ok_or("wait")?,
    )?)
}
fn view(bytes: &[u8]) -> TestResult<Value> {
    Ok(result_fields(bytes)?["view"].clone())
}
async fn command(
    client: &mut tokio::io::DuplexStream,
    sequence: &mut u64,
    payload: &[u8],
) -> TestResult<(u8, Vec<u8>)> {
    let command_id = *sequence;
    *sequence += 1;
    let (status, bytes) = reply(client, command_id, payload).await?;
    assert_eq!(status, CommandStatus::Accepted);
    Ok((code(&bytes)?, bytes))
}
async fn reply(
    client: &mut tokio::io::DuplexStream,
    command_id: u64,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let kind = match payload[1] {
        1..=6 => 48,
        7..=11 => 49,
        _ => 50,
    };
    let frame = encode_client_command(
        1,
        &ClientCommandValue {
            command_id,
            command_type: kind,
            payload,
        },
    )
    .map_err(debug)?;
    write_frame(client, &frame).await?;
    loop {
        let frame = read_frame(client).await.map_err(|e| {
            format!(
                "VIP operation {} command {}: {e}",
                payload[1],
                command_id
            )
        })?;
        let env = decode_wire_envelope(&frame).map_err(debug)?;
        if env.message_type() == MessageType::CommandResult {
            let r = decode_command_result(env.payload()).map_err(debug)?;
            assert_eq!(r.command_id, command_id);
            assert!(r.payload.len() <= oteryn_protocol_oteryn::vip::MAX_RESULT_BYTES);
            return Ok((r.status, r.payload.to_vec()));
        }
    }
}
async fn ok(
    client: &mut tokio::io::DuplexStream,
    sequence: &mut u64,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (code, bytes) = command(client, sequence, payload).await?;
    assert_eq!(code, 1, "VIP result {code}");
    Ok(bytes)
}
fn selected(mut admitted: AdmittedSession) -> TestResult<AdmittedSession> {
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[43])
            .ok_or("VIP offer")?;
    Ok(admitted)
}
