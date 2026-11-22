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

#[test]
fn durable_recovery_deadline_on_replay() -> TestResult {
    blocked_durable_recovery(false, false)
}

#[test]
fn durable_recovery_deadline_on_forget() -> TestResult {
    blocked_durable_recovery(true, false)
}

#[test]
fn durable_recovery_deadline_on_cancel() -> TestResult {
    blocked_durable_recovery(false, true)
}

fn blocked_durable_recovery(forget: bool, cancelled: bool) -> TestResult {
    tokio::runtime::Runtime::new()?.block_on(async {
        let folder = tempfile::tempdir()?;
        let login = manager(Arc::new(MemoryVault::default()), folder.path().into());
        let credential = DeviceCredential::parse(&credential('A'))?;
        login.journal.stage(&credential)?;
        if forget {
            login.block_remembered()?;
        }
        let cancel = CancellationToken::new();
        if cancelled {
            cancel.cancel();
        }
        let (_release, blocked) = tokio::sync::oneshot::channel();
        let server =
            Server::start_with_blocked_ack(fixture("revoke-response.json")?, blocked).await?;
        let result = tokio::time::timeout(
            Duration::from_millis(2200),
            login.begin(&server.client, options(), cancel),
        )
        .await?;
        if cancelled {
            assert!(result.is_err());
        } else {
            assert!(result?.is_none());
        }
        assert_eq!(server.requests.lock().map_err(|_| "requests")?.len(), 1);
        let retry = Server::start(vec![(200, fixture("revoke-response.json")?)]).await?;
        drop(
            login
                .begin(&retry.client, options(), CancellationToken::new())
                .await?,
        );
        assert!(
            retry
                .requests
                .lock()
                .map_err(|_| "requests")?
                .iter()
                .any(|request| request.contains(credential.expose()))
        );
        retry.finish().await
    })
}
