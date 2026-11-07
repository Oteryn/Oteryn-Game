//! F25 support: exercise the production retained lifecycle, without inventing queues.
use super::*;

struct RetryAuthority {
    session: AdmittedSession,
    loss_unknowns: usize,
    terminal_unknowns: usize,
    losses: AtomicUsize,
    reconciliations: AtomicUsize,
}

impl RetryAuthority {
    fn new(loss_unknowns: usize, terminal_unknowns: usize) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            session: AdmittedSession {
                game_session_id: GameSessionId::decode(&CHARACTER)?,
                world_id: crate::foundation::WorldId::decode(&CHARACTER)?,
                channel_id: crate::foundation::ChannelId::decode(&CHARACTER)?,
                runtime_actor: None,
                first_entry: FirstEntryOutcome::NotApplicable,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            },
            loss_unknowns,
            terminal_unknowns,
            losses: AtomicUsize::new(0),
            reconciliations: AtomicUsize::new(0),
        })
    }

    fn lost(&self) -> LostControl {
        LostControl {
            session: self.session,
            wait: Duration::from_millis(17),
            reset: None,
        }
    }
}

impl FreshAdmissionAuthority for RetryAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        unreachable!("cleanup must not re-admit a session")
    }

    async fn lose_control(
        &self,
        session: AdmittedSession,
        wait: Duration,
        reset: Option<&crate::durability::world_reset::RetiringWorldReset>,
    ) -> ControlLossResult {
        assert_eq!(
            session, self.session,
            "retry retains the exact controller binding"
        );
        assert!(reset.is_none());
        let attempt = self.losses.fetch_add(1, Ordering::SeqCst);
        assert_eq!(
            wait,
            if attempt == 0 {
                Duration::from_millis(17)
            } else {
                Duration::ZERO
            }
        );
        if attempt < self.loss_unknowns {
            ControlLossResult::Unknown
        } else {
            ControlLossResult::Terminal
        }
    }

    async fn reconcile_terminal(&self, session: AdmittedSession) -> GraceExpiryResult {
        assert_eq!(session, self.session);
        if self.reconciliations.fetch_add(1, Ordering::SeqCst) < self.terminal_unknowns {
            GraceExpiryResult::Unknown
        } else {
            GraceExpiryResult::Released
        }
    }
}

fn poll_once(future: Pin<&mut impl Future<Output = ()>>) -> Poll<()> {
    future.poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
}

#[test]
fn f25_unknown_cleanup_retries_reconcile_the_same_session() -> Result<(), Box<dyn Error>> {
    runtime()?.block_on(async {
        let authority = RetryAuthority::new(1, 1)?;
        tokio::time::timeout(
            Duration::from_secs(5),
            control_loss_lifecycle(&authority, authority.lost(), &CancellationToken::new()),
        )
        .await?;
        assert_eq!(authority.losses.load(Ordering::SeqCst), 2);
        assert_eq!(authority.reconciliations.load(Ordering::SeqCst), 2);
        Ok(())
    })
}

#[test]
fn f25_shutdown_ends_pending_loss_and_terminal_retry_futures() -> Result<(), Box<dyn Error>> {
    runtime()?.block_on(async {
        // Both retry sites must register cancellation while they are sleeping.
        for loss_unknowns in [0, usize::MAX] {
            let authority = RetryAuthority::new(loss_unknowns, usize::MAX)?;
            let shutdown = CancellationToken::new();
            let mut lifecycle = Box::pin(control_loss_lifecycle(
                &authority,
                authority.lost(),
                &shutdown,
            ));
            assert!(poll_once(lifecycle.as_mut()).is_pending());
            let losses = authority.losses.load(Ordering::SeqCst);
            let reconciliations = authority.reconciliations.load(Ordering::SeqCst);
            assert_eq!(losses, 1);
            assert_eq!(reconciliations, usize::from(loss_unknowns == 0));
            shutdown.cancel();
            assert!(poll_once(lifecycle.as_mut()).is_ready());
            drop(lifecycle);
            // No spawned retry may outlive the lifecycle, even after its timer was due.
            tokio::time::sleep(RECONCILE_BACKOFF * 2).await;
            assert_eq!(authority.losses.load(Ordering::SeqCst), losses);
            assert_eq!(
                authority.reconciliations.load(Ordering::SeqCst),
                reconciliations
            );
        }
        Ok(())
    })
}

#[test]
fn f25_retry_source_has_one_attempt_per_backoff() -> Result<(), Box<dyn Error>> {
    runtime()?.block_on(async {
        for loss_unknowns in [0, usize::MAX] {
            let authority = RetryAuthority::new(loss_unknowns, usize::MAX)?;
            let shutdown = CancellationToken::new();
            let mut lifecycle = pin!(control_loss_lifecycle(
                &authority,
                authority.lost(),
                &shutdown
            ));
            assert!(poll_once(lifecycle.as_mut()).is_pending());
            let attempts = || {
                if loss_unknowns == 0 {
                    assert_eq!(authority.losses.load(Ordering::SeqCst), 1);
                    authority.reconciliations.load(Ordering::SeqCst)
                } else {
                    assert_eq!(authority.reconciliations.load(Ordering::SeqCst), 0);
                    authority.losses.load(Ordering::SeqCst)
                }
            };
            assert_eq!(attempts(), 1);
            // Spurious polls cannot fan out the retained operation or bypass its timer.
            for _ in 0..32 {
                assert!(poll_once(lifecycle.as_mut()).is_pending());
            }
            assert_eq!(attempts(), 1);
            tokio::time::sleep(RECONCILE_BACKOFF + Duration::from_millis(10)).await;
            assert!(poll_once(lifecycle.as_mut()).is_pending());
            assert_eq!(attempts(), 2);
            for _ in 0..32 {
                assert!(poll_once(lifecycle.as_mut()).is_pending());
            }
            assert_eq!(attempts(), 2);
            shutdown.cancel();
            assert!(poll_once(lifecycle.as_mut()).is_ready());
        }
        Ok(())
    })
}
