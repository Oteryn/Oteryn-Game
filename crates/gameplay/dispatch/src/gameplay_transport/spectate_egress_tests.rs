use super::*;

#[test]
fn spectate_fanout_preserves_public_payload_and_failed_partial_writes_close_only_observer() {
    for partial in [false, true] {
        let sessions = SpectateSession::from(limits(10, 40));
        start(&sessions);
        // Reuse the native public-chat fixture; there is no command-outcome source.
        sessions.dispatch(SpectateDispatch::Owner(Box::new(Request::Publish {
            facts: facts(1, 1, 1),
            projection: WorldProjection::Chat(
                oteryn_gameplay_social::spectate::spectate_test_support::local(),
            ),
        })));
        let reply = sessions.dispatch(SpectateDispatch::Flush {
            observer: observer(1),
            try_write: &mut |bytes| {
                let envelope = ObserverEnvelope::try_from(&bytes[4..]).expect("frame");
                assert!(matches!(
                    envelope.message,
                    SpectateMessage::Frame { domain: 12, .. }
                ));
                if partial {
                    Ok(bytes.len() - 1)
                } else {
                    Err(std::io::ErrorKind::WouldBlock.into())
                }
            },
        });
        assert_eq!(reply, Reply::NotAvailable);
        assert_eq!(
            sessions.dispatch(SpectateDispatch::Owner(Box::new(Request::Refresh(facts(
                1, 1, 1
            ))))),
            Reply::State {
                active: true,
                count: 0
            }
        );
        assert_eq!(
            sessions.dispatch(SpectateDispatch::Flush {
                observer: observer(1),
                try_write: &mut |_| unreachable!("closed observer must never write")
            }),
            Reply::NotAvailable
        );
    }
}

#[test]
fn spectate_revoke_discards_queued_projection_before_egress() {
    let sessions = SpectateSession::from(limits(10, 40));
    let (revision, _) = start(&sessions);
    sessions.dispatch(SpectateDispatch::Owner(Box::new(Request::Publish {
        facts: facts(1, 1, 1),
        projection: WorldProjection::Chat(
            oteryn_gameplay_social::spectate::spectate_test_support::local(),
        ),
    })));
    sessions.dispatch(SpectateDispatch::Owner(Box::new(Request::GrantRevoke {
        facts: facts(1, 1, 1),
        expected_revision: revision,
    })));
    let mut writes = 0;
    assert_eq!(
        sessions.dispatch(SpectateDispatch::Flush {
            observer: observer(1),
            try_write: &mut |bytes| {
                writes += 1;
                assert_eq!(
                    ObserverEnvelope::try_from(&bytes[4..])
                        .expect("end")
                        .message,
                    SpectateMessage::End(EndReason::Revoked)
                );
                Ok(bytes.len())
            }
        }),
        Reply::Applied
    );
    assert_eq!(writes, 1);
}
