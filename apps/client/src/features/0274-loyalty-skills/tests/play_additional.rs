use super::*;

#[test]
fn loyalty_skills_refusal_and_malformed_reply_keep_confirmed_values_and_allow_retry()
-> Result<(), Error> {
    scenario(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            let confirmed = boosted_snapshot();
            let mut refreshed = confirmed.clone();
            refreshed.points = 7_000;
            refreshed.title = Some("Enlightened of Tibia".into());
            let mut truncated = refreshed.encode().ok_or("malformed fixture")?;
            truncated.pop();
            let peer = Peer::new(
                true,
                [
                    Reply::new(
                        CommandStatus::Accepted,
                        confirmed.encode().ok_or("initial")?,
                    ),
                    // A syntactically valid payload accompanying a refusal is not an
                    // authoritative replacement for the last accepted snapshot.
                    Reply::new(
                        CommandStatus::Rejected,
                        refreshed.encode().ok_or("refused")?,
                    ),
                    Reply::new(CommandStatus::Accepted, truncated),
                    Reply::new(CommandStatus::Accepted, refreshed.encode().ok_or("retry")?),
                ],
            );
            let commands = Arc::clone(&peer.commands);
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(peer, incoming, outbox));
            ready(&link, |panel| panel.selected).await?;
            link.loyalty_skills_request();
            ready(&link, |panel| panel.snapshot.is_some() && !panel.pending).await?;
            for _ in 0..2 {
                link.loyalty_skills_request();
                let refused = ready(&link, |panel| panel.error.is_some() && !panel.pending).await?;
                assert_eq!(refused.snapshot.as_ref(), Some(&confirmed));
                assert!(refused.selected);
            }
            link.loyalty_skills_request();
            let recovered = ready(&link, |panel| !panel.pending).await?;
            assert_eq!(recovered.snapshot.as_ref(), Some(&refreshed));
            assert!(recovered.error.is_none());
            assert!(panel_text(&recovered).contains("Enlightened of Tibia"));
            assert_eq!(
                commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .len(),
                4
            );
            link.request_logout();
            task.await?;
            Ok::<(), Error>(())
        })?
    })
}

#[test]
fn loyalty_skills_unselected_session_receives_no_skills_command() -> Result<(), Error> {
    scenario(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            let peer = Peer::new(false, []);
            let commands = Arc::clone(&peer.commands);
            let served = Arc::clone(&peer.served);
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(peer, incoming, outbox));
            tokio::time::timeout(Duration::from_secs(3), served.notified()).await?;
            link.loyalty_skills_request();
            link.loyalty_skills_request();
            let panel = link.loyalty_skills_panel();
            assert!(!panel.selected);
            assert!(!panel.pending);
            assert!(panel.snapshot.is_none());
            assert!(panel_text(&panel).contains("unavailable"));
            link.request_logout();
            task.await?;
            assert!(
                commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .is_empty()
            );
            assert_eq!(link.loyalty_skills_panel(), LoyaltySkillsPanel::default());
            Ok::<(), Error>(())
        })?
    })
}

#[test]
fn loyalty_skills_closed_command_queue_releases_pending_request() {
    let (link, incoming, outbox) = play_channel();
    outbox.loyalty_skills_initialize(true);
    drop(incoming);
    link.loyalty_skills_request();
    let panel = link.loyalty_skills_panel();
    assert!(!panel.pending);
    assert_eq!(panel.error, Some("Session ended"));
    assert!(panel.snapshot.is_none());
}
