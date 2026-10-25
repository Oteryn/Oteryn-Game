//! Capability-gated tutorial notice framing. Mounted once by the connection owner.
use super::*;
use oteryn_protocol_oteryn::tutorial_notices::*;

pub(super) fn snapshot(admitted: &AdmittedSession) -> Option<DomainSnapshot<'static>> {
    admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_TUTORIAL_NOTICES_V1)
        .then_some(DomainSnapshot {
            domain_id: STATE_DOMAIN_TUTORIAL_NOTICES,
            revision: admitted.continuity.tutorial_revision,
            snapshot_type: 1,
            payload: TUTORIAL_NOTICES_SNAPSHOT,
        })
}

pub(super) async fn emit<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    admitted: &mut AdmittedSession,
    sequence: &mut u64,
) -> Option<()> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_TUTORIAL_NOTICES_V1)
    {
        return Some(());
    }
    while let Some(notice) = authority.take_tutorial_notice(admitted.game_session_id) {
        emit_notice(stream, admitted, sequence, &notice).await?;
    }
    Some(())
}

/// Shared status-text channel. USE-AUTOWALK-1 adds only the canonical no-path text;
/// hints remain produced exclusively by the tutorial rules.
pub(super) async fn emit_notice<S: AsyncWrite + Unpin>(
    stream: &mut S,
    admitted: &mut AdmittedSession,
    sequence: &mut u64,
    notice: &TutorialNotice,
) -> Option<()> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_TUTORIAL_NOTICES_V1)
    {
        return Some(());
    }
    let payload = encode_tutorial_notice(notice).ok()?;
    let next_sequence = sequence.checked_add(1)?;
    let revision = admitted.continuity.tutorial_revision;
    let next_revision = revision.checked_add(1)?;
    let frame = encode_state_delta(
        admitted.continuity.connection_generation,
        next_sequence,
        STATE_DOMAIN_TUTORIAL_NOTICES,
        revision,
        next_revision,
        1,
        &payload,
    )
    .ok()?;
    *sequence = next_sequence;
    admitted.continuity.server_sequence = next_sequence;
    admitted.continuity.tutorial_revision = next_revision;
    write_frame(stream, &frame).await.ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use oteryn_protocol_oteryn::{decode_state_delta, decode_wire_envelope};
    use tokio::io::AsyncReadExt;
    #[test]
    fn autowalk_no_path_notice_requires_capability_31() -> Result<(), Box<dyn std::error::Error>> {
        fn id(seed: u8) -> [u8; 16] {
            let mut bytes = [seed; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        }
        use crate::gameplay_transport::connection::SessionContinuity;
        use oteryn_protocol_oteryn::tutorial_notices::{TutorialNotice, decode_tutorial_notice};
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                for selected in [false, true] {
                    let mut admitted = AdmittedSession {
                        game_session_id: crate::foundation::GameSessionId::decode(&id(50))?,
                        world_id: crate::foundation::WorldId::decode(&id(42))?,
                        channel_id: crate::foundation::ChannelId::decode(&id(43))?,
                        runtime_actor: None,
                        first_entry: FirstEntryOutcome::NotApplicable,
                        controller: None,
                        continuity: SessionContinuity::FRESH,
                        item_fence: None,
                    };
                    if selected {
                        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES,
                        &[31],
                    )
                    .ok_or("notice capability")?;
                    }
                    admitted.continuity.tutorial_revision = 4;
                    admitted.continuity.server_sequence = 7;
                    let mut sequence = 7;
                    let notice = TutorialNotice::StatusText("There is no way.".into());
                    let (mut server, mut client) = tokio::io::duplex(1024);
                    super::emit_notice(&mut server, &mut admitted, &mut sequence, &notice)
                        .await
                        .ok_or("notice")?;
                    drop(server);
                    let mut bytes = Vec::new();
                    client.read_to_end(&mut bytes).await?;
                    if selected {
                        let size = usize::try_from(u32::from_be_bytes(bytes[..4].try_into()?))?;
                        assert_eq!(bytes.len(), size + 4);
                        let envelope = decode_wire_envelope(&bytes[4..])?;
                        assert_eq!(envelope.server_sequence(), 8);
                        let delta = decode_state_delta(envelope.payload())?;
                        assert_eq!(
                            (delta.domain_id, delta.base_revision, delta.new_revision),
                            (23, 4, 5)
                        );
                        assert_eq!(
                            decode_tutorial_notice(delta.payload)
                                .map_err(|error| format!("{error:?}"))?,
                            notice
                        );
                        assert_eq!((sequence, admitted.continuity.tutorial_revision), (8, 5));
                    } else {
                        assert!(bytes.is_empty());
                        assert_eq!((sequence, admitted.continuity.tutorial_revision), (7, 4));
                    }
                    assert_eq!(admitted.continuity.server_sequence, sequence);
                }
                Ok(())
            })
    }
}
