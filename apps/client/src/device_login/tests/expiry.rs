use super::*;

#[test]
fn expired_session_revocation_has_a_recovery_deadline() -> TestResult {
    blocked_expiry(false)
}

#[test]
fn player_cancel_interrupts_expired_session_revocation() -> TestResult {
    blocked_expiry(true)
}

fn blocked_expiry(cancel_during_revoke: bool) -> TestResult {
    tokio::runtime::Runtime::new()?.block_on(async {
        let (_release, blocked) = tokio::sync::oneshot::channel();
        let server =
            Server::start_with_blocked_ack(fixture("revoke-response.json")?, blocked).await?;
        let folder = tempfile::tempdir()?;
        let vault = Arc::new(MemoryVault::default());
        vault::Vault::write(vault.as_ref(), &enrollment_response()?)
            .map_err(std::io::Error::other)?;
        let login = manager(vault.clone(), folder.path().into());
        let cancel = CancellationToken::new();
        let mut attempt = login
            .begin(&server.client, options(), cancel.clone())
            .await?
            .ok_or("attempt")?;
        attempt
            .state
            .session
            .as_mut()
            .ok_or("session")?
            .idle_expires_at = now()?;
        let result = {
            let ticket = attempt.ticket(&server.client, cancel.clone());
            tokio::pin!(ticket);
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    tokio::select! {
                        _ = &mut ticket => return Err("ticket returned before revocation"),
                        _ = tokio::task::yield_now() => {
                            if !server.requests.lock().map_err(|_| "requests")?.is_empty() {
                                return Ok(());
                            }
                        }
                    }
                }
            })
            .await??;
            let wait = if cancel_during_revoke {
                cancel.cancel();
                Duration::from_millis(500)
            } else {
                Duration::from_secs(4)
            };
            tokio::time::timeout(wait, ticket).await?
        };
        if cancel_during_revoke {
            assert!(matches!(
                result,
                Err(NativeLoginError::Platform(
                    oteryn_platform_client::PlatformClientError::Cancelled
                ))
            ));
        } else {
            assert!(matches!(result, Err(NativeLoginError::TicketSpent)));
        }
        assert_eq!(attempt.state.pending.len(), 1);
        assert!(
            vault::Vault::read(vault.as_ref())
                .map_err(std::io::Error::other)?
                .is_none()
        );
        Ok(())
    })
}

#[test]
fn next_sign_in_bounds_deferred_revocation_and_retains_credential() -> TestResult {
    tokio::runtime::Runtime::new()?.block_on(async {
        let (_release, blocked) = tokio::sync::oneshot::channel();
        let server =
            Server::start_with_blocked_ack(fixture("revoke-response.json")?, blocked).await?;
        let folder = tempfile::tempdir()?;
        let vault = Arc::new(MemoryVault::default());
        vault::Vault::write(vault.as_ref(), &enrollment_response()?)
            .map_err(std::io::Error::other)?;
        let login = manager(vault, folder.path().into());
        let mut first = login
            .begin(&server.client, options(), CancellationToken::new())
            .await?
            .ok_or("attempt")?;
        first
            .state
            .session
            .as_mut()
            .ok_or("session")?
            .idle_expires_at = now()?;
        assert!(matches!(
            first.ticket(&server.client, CancellationToken::new()).await,
            Err(NativeLoginError::TicketSpent)
        ));
        first
            .state
            .pending
            .push(DeviceCredential::parse(&credential('A'))?);
        let pending = first.state.pending.clone();
        drop(first);
        let (_retry_release, retry_blocked) = tokio::sync::oneshot::channel();
        let retry =
            Server::start_with_blocked_ack(fixture("revoke-response.json")?, retry_blocked).await?;
        let next = tokio::time::timeout(
            Duration::from_secs(3),
            login.begin(&retry.client, options(), CancellationToken::new()),
        )
        .await??
        .ok_or("next attempt")?;
        assert_eq!(pending.len(), 2);
        assert!(next.state.pending == pending);
        Ok(())
    })
}

#[test]
fn remembered_intent_propagates_durable_rename_failure() -> TestResult {
    let root = tempfile::tempdir()?;
    let login = manager(Arc::new(MemoryVault::default()), root.path().into());
    crate::atomic_file::fail_durable_rename_to(&root.path().join("revoke.pending"));
    assert!(login.block_remembered().is_err());
    Ok(())
}
