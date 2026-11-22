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
