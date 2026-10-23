mod spectate_ingress {
    use super::*;
    use oteryn_protocol_oteryn::spectate::{Audience, ObserverEnvelope, SpectateMessage};

    #[test]
    fn spectate_all_observer_bootstraps_fail_before_player_authority() -> Result<(), Box<dyn Error>>
    {
        run(async {
            let authority = Authority::new(Ok(()));
            for generation in [0, 1, u64::MAX] {
                for message in [
                    SpectateMessage::Stop,
                    SpectateMessage::GrantSet(Audience::Code),
                    SpectateMessage::GrantSet(Audience::Contacts),
                    SpectateMessage::Keepalive,
                    SpectateMessage::NotAvailable,
                ] {
                    let frame = Vec::<u8>::try_from(&ObserverEnvelope {
                        generation,
                        message,
                    })
                    .map_err(|error| format!("encode: {error:?}"))?;
                    let (end, frames) = drive(&authority, &[frame]).await?;
                    assert!(matches!(
                        end,
                        ConnectionEnd::AdmissionRefused(AdmissionRefusal::Unavailable)
                    ));
                    assert_eq!(frames.len(), 1);
                    let reply = ObserverEnvelope::try_from(frames[0].as_slice())
                        .map_err(|error| format!("decode: {error:?}"))?;
                    assert_eq!(
                        reply,
                        ObserverEnvelope {
                            generation: 0,
                            message: SpectateMessage::NotAvailable
                        }
                    );
                    assert_eq!(authority.calls.get(), 0);
                }
            }
            for malformed in [
                vec![8, 15],
                vec![8, 15, 34, 255],
                envelope(15, 0, b"no accepted observer profile"),
            ] {
                let (end, frames) = drive(&authority, &[malformed]).await?;
                assert!(matches!(
                    end,
                    ConnectionEnd::AdmissionRefused(AdmissionRefusal::Unavailable)
                ));
                assert_eq!(frames.len(), 1);
                assert_eq!(
                    ObserverEnvelope::try_from(frames[0].as_slice())
                        .map_err(|error| format!("decode: {error:?}"))?
                        .message,
                    SpectateMessage::NotAvailable
                );
                assert_eq!(authority.calls.get(), 0);
            }
            Ok(())
        })
    }
}
