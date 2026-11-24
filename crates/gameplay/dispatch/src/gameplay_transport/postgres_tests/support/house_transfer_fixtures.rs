use tokio::io::AsyncReadExt;
use crate::foundation::{MessageType, decode_wire_envelope};
use super::*;

#[path = "../../../../../../../apps/game-server/tests/support/recovery_source_fixture.rs"]
mod shared;

pub(super) use shared::{RecoverySource, prepare_owned_session, prepare_owned_session_request};

pub(super) async fn resume_same_session(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: crate::foundation::GameSessionId,
    token: &str,
    transport: u8,
    supported: &[u32],
) -> TestResult<AdmittedSession> {
    let mut body = Vec::new();
    bytes(&mut body, 1, source.as_bytes());
    bytes(&mut body, 2, token.as_bytes());
    body.extend([24, 0]);
    scalar(&mut body, 4, 1);
    scalar(&mut body, 5, 1);
    scalar(&mut body, 6, 1);
    for id in supported {
        scalar(&mut body, 7, u64::from(*id));
    }
    bytes(&mut body, 8, b"resume-busy-holder-test");
    let mut frame = vec![8, MessageType::ClientResume as u8];
    bytes(&mut frame, 4, &body);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let admitted = crate::gameplay_transport::connection::admit_frame(
        &mut server,
        &frame,
        owner,
        &RecoveryIdentifiers(transport),
    )
    .await
    .map_err(|e| format!("same-session resume {e:?}"))?;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let n = u32::from_be_bytes(output.get(..4).ok_or("no reply")?.try_into()?) as usize;
    let envelope = decode_wire_envelope(&output[4..4 + n]).map_err(debug)?;
    assert_eq!(envelope.message_type(), MessageType::ServerResumeAccepted);
    Ok(admitted)
}

pub(super) struct RecoveryIdentifiers(pub(super) u8);
impl crate::gameplay_transport::connection::ConnectionIdentifiers for RecoveryIdentifiers {
    fn game_session_id(&self) -> Option<crate::foundation::GameSessionId> {
        None
    }
    fn transport_ref(&self) -> Option<crate::foundation::AuthenticatedTransportRefV1> {
        crate::foundation::AuthenticatedTransportRefV1::decode(&[self.0; 16]).ok()
    }
}

pub(super) fn scalar(out: &mut Vec<u8>, tag: u8, mut value: u64) {
    out.push(tag << 3);
    while value >= 128 {
        out.push(value as u8 | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
pub(super) fn bytes(out: &mut Vec<u8>, tag: u8, value: &[u8]) {
    out.push(tag << 3 | 2);
    let mut n = value.len();
    while n >= 128 {
        out.push(n as u8 | 128);
        n >>= 7;
    }
    out.push(n as u8);
    out.extend_from_slice(value);
}
