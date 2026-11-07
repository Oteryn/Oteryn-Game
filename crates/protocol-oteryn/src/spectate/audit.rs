//! D1118 registered ANL-01 type-4 payload, independent of transport message IDs.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationKind {
    GrantCreated = 1,
    GrantEnded = 2,
    ObserverAdmitted = 3,
    ObserverEnded = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpectateObservation {
    pub kind: ObservationKind,
    pub observer_session_id: Option<crate::GameSessionId>,
    pub end_reason: Option<EndReason>,
}

impl TryFrom<&SpectateObservation> for Vec<u8> {
    type Error = SpectateWireError;
    fn try_from(value: &SpectateObservation) -> Result<Self, Self::Error> {
        if matches!(
            value.kind,
            ObservationKind::ObserverAdmitted | ObservationKind::ObserverEnded
        ) != value.observer_session_id.is_some()
            || matches!(
                value.kind,
                ObservationKind::GrantEnded | ObservationKind::ObserverEnded
            ) != value.end_reason.is_some()
            || (value.kind == ObservationKind::GrantEnded
                && value.end_reason == Some(EndReason::ObserverStopped))
        {
            return Err(SpectateWireError::Malformed);
        }
        let mut out = Vec::new();
        push_varint_field(&mut out, 1, value.kind as u64);
        if let Some(session) = value.observer_session_id {
            push_message_field(&mut out, 2, session.as_bytes());
        }
        if let Some(reason) = value.end_reason {
            push_varint_field(&mut out, 3, reason as u64);
        }
        Ok(out)
    }
}

impl TryFrom<&[u8]> for SpectateObservation {
    type Error = SpectateWireError;
    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        if input.len() > 22 {
            return Err(SpectateWireError::LimitExceeded);
        }
        let (mut kind, mut session, mut reason) = (None, None, None);
        let mut cursor = 0;
        while cursor < input.len() {
            match read_varint(input, &mut cursor)? {
                8 => set_once(&mut kind, read_varint(input, &mut cursor)?)?,
                18 => set_once(
                    &mut session,
                    crate::GameSessionId::decode(read_bytes(input, &mut cursor)?)
                        .map_err(|_| SpectateWireError::Malformed)?,
                )?,
                24 => set_once(
                    &mut reason,
                    EndReason::try_from(read_varint(input, &mut cursor)?)?,
                )?,
                _ => return Err(SpectateWireError::Malformed),
            }
        }
        let kind = match kind {
            Some(1) => ObservationKind::GrantCreated,
            Some(2) => ObservationKind::GrantEnded,
            Some(3) => ObservationKind::ObserverAdmitted,
            Some(4) => ObservationKind::ObserverEnded,
            _ => return Err(SpectateWireError::Malformed),
        };
        let value = Self {
            kind,
            observer_session_id: session,
            end_reason: reason,
        };
        Vec::<u8>::try_from(&value)?;
        Ok(value)
    }
}
