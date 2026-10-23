//! Dormant Game-side O2 composition. No observer authentication profile exists on main.
//! The sole ingress hook always refuses, before identifiers, player authority or actor creation.
use super::{
    connection::{AdmissionRefusal, ConnectionEnd},
    tcp_tls::write_frame,
};
use oteryn_gameplay_social::spectate::{
    Limits, ObserverCommand, ObserverIdentity, Reply, Request, SpectateRuntime,
};
use oteryn_protocol_oteryn::{
    MessageType, decode_wire_envelope,
    spectate::{ObserverEnvelope, SpectateMessage},
};
use std::sync::Mutex;
use tokio::io::AsyncWrite;

#[derive(Default)]
pub struct SpectateSession {
    // Named lock: every grant, session, projection, lifecycle and immediate egress write is
    // serialized here. Never held across an await or storage IO, or while locking the target.
    owner: Mutex<SpectateRuntime>,
}
impl From<Limits> for SpectateSession {
    fn from(limits: Limits) -> Self {
        Self {
            owner: Mutex::new(limits.into()),
        }
    }
}

pub enum SpectateDispatch<'a> {
    Admission,
    Owner(Box<Request>),
    ObserverFrame {
        observer: ObserverIdentity,
        frame: &'a [u8],
    },
    /// One immediate nonblocking write attempt. The O2 adapter must never wait for readiness.
    Flush {
        observer: ObserverIdentity,
        try_write: &'a mut dyn FnMut(&[u8]) -> std::io::Result<usize>,
    },
}
impl SpectateSession {
    /// Game-only composition seam, deliberately not reachable through player gameplay handlers.
    /// Platform authentication and measured limits are required before O2 can wire admission.
    pub fn dispatch(&self, input: SpectateDispatch<'_>) -> Reply {
        let Ok(mut owner) = self.owner.lock() else {
            return Reply::NotAvailable;
        };
        match input {
            SpectateDispatch::Admission => owner.apply(Request::Bootstrap),
            // Frames may leave the owner only during Flush's immediate write under this lock.
            SpectateDispatch::Owner(request) => match *request {
                Request::Drain(_) => Reply::NotAllowed,
                request => owner.apply(request),
            },
            SpectateDispatch::ObserverFrame { observer, frame } => {
                let command = match ObserverEnvelope::try_from(frame) {
                    Ok(envelope) if envelope.generation == observer.generation.get() => {
                        match envelope.message {
                            SpectateMessage::Stop => ObserverCommand::Stop,
                            SpectateMessage::Keepalive => ObserverCommand::Keepalive,
                            _ => ObserverCommand::Gameplay,
                        }
                    }
                    _ => match decode_wire_envelope(frame) {
                        Ok(envelope)
                            if envelope.message_type() == MessageType::LivenessAck
                                && envelope.liveness_ack(observer.generation.get()).is_ok() =>
                        {
                            ObserverCommand::Keepalive
                        }
                        // ClientCommand is rejected by type, before decoding its command payload.
                        // Malformed, future and unknown gameplay inputs take the same path.
                        _ => ObserverCommand::Gameplay,
                    },
                };
                owner.apply(Request::ObserverInput { observer, command })
            }
            SpectateDispatch::Flush {
                observer,
                try_write,
            } => {
                let Reply::Frames(frames) = owner.apply(Request::Drain(observer)) else {
                    return Reply::NotAvailable;
                };
                for message in frames {
                    let Ok(body) = Vec::<u8>::try_from(&ObserverEnvelope {
                        generation: observer.generation.get(),
                        message,
                    }) else {
                        owner.apply(Request::WriteFailed(observer));
                        owner.apply(Request::Drain(observer));
                        return Reply::NotAvailable;
                    };
                    let mut bytes = Vec::with_capacity(4 + body.len());
                    bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
                    bytes.extend_from_slice(&body);
                    if !matches!(try_write(&bytes), Ok(written) if written == bytes.len()) {
                        owner.apply(Request::WriteFailed(observer));
                        // Failed/partial writes close this observer; never retry a partial frame.
                        owner.apply(Request::Drain(observer));
                        return Reply::NotAvailable;
                    }
                }
                Reply::Applied
            }
        }
    }
}

#[allow(
    clippy::result_large_err,
    reason = "the same terminal end `admit_frame` returns; boxing would only add an allocation"
)]
pub(super) async fn refuse_observer_bootstrap<S: AsyncWrite + Unpin>(
    stream: &mut S,
    frame: &[u8],
) -> Result<(), ConnectionEnd> {
    if !ObserverEnvelope::is_ingress(frame) {
        return Ok(());
    }
    let sessions = SpectateSession::default();
    let reply = sessions.dispatch(SpectateDispatch::Admission);
    debug_assert_eq!(reply, Reply::NotAvailable);
    let envelope = ObserverEnvelope {
        generation: 0,
        message: SpectateMessage::NotAvailable,
    };
    if let Ok(bytes) = Vec::<u8>::try_from(&envelope) {
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            write_frame(stream, &bytes),
        )
        .await;
    }
    Err(ConnectionEnd::AdmissionRefused(
        AdmissionRefusal::Unavailable,
    ))
}

#[cfg(test)]
#[path = "spectate_session_tests.rs"]
mod tests;
