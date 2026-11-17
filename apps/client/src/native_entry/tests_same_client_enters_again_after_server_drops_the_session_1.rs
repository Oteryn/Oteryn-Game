#[test]
fn same_client_enters_again_after_server_drops_the_session() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                let stack =
                    Stack::start_with_frames(vec![], super::test_support::admitted_frames())?;
                let client = crate::ClientBootstrap::new()?.with_native_login(stack.config());
                let mut entry = crate::reentry::Reentry::new(client);
                for _ in 0..2 {
                    assert!(entry.request());
                    assert!(!entry.request());
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                    let (mut view, link) = loop {
                        if let Some(result) = entry.poll() {
                            break result?;
                        }
                        if std::time::Instant::now() >= deadline {
                            return Err("entry timed out".into());
                        }
                        std::thread::yield_now();
                    };
                    assert_eq!(link.party_panel().own_character_name, "Ari");
                    loop {
                        if let Err(end) = view.tick(&link) {
                            assert_eq!(
                                end,
                                crate::play::PlayEnd::Ended(PublicClass::SessionUnavailable)
                            );
                            break;
                        }
                        if std::time::Instant::now() >= deadline {
                            return Err("session end timed out".into());
                        }
                        std::thread::yield_now();
                    }
                    drop(link);
                    entry.leave_play();
                }
                assert_eq!(stack.admissions.load(Ordering::SeqCst), 2);
                for target in [
                    "GET /v1/client/directory",
                    "POST /oauth/token",
                    "POST /api/v1/game-auth/tickets",
                    "POST /v1/login",
                ] {
                    assert_eq!(
                        super::test_support::count(&stack.requests(), target),
                        2,
                        "{target}"
                    );
                }
                entry.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "re-entry test thread failed")?
        .map_err(Into::into)
}
