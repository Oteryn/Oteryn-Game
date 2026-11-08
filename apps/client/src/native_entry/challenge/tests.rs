use crate::native_entry::test_support::{self, MockChallengeVerifier, Stack};
use std::sync::{Arc, Mutex};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn login_challenge_browser_return_retries_then_enters_real_admission() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                let stack = Stack::start_with_challenge()?;
                let mut config = stack.config();
                let original_browser = config.open_browser.clone();
                let opened = Arc::new(Mutex::new(Vec::new()));
                let recorded = opened.clone();
                config.open_browser = Arc::new(move |url| {
                    if let Ok(mut log) = recorded.lock() {
                        log.push(url.to_owned());
                    }
                    original_browser(url)
                });
                let client = crate::ClientBootstrap::new()?.with_native_login(config);
                let admitted = client.request_gameplay_entry()?;
                assert_eq!(admitted.character_name, "Ari");
                client.shutdown();
                let opened = opened.lock().map_err(|_| "browser log")?;
                assert_eq!(opened.len(), 2);
                assert!(opened[0].contains("/oauth/authorize?"));
                assert!(opened[1].contains("/oauth/challenge?"));
                assert!(!opened[1].contains("code_verifier"));
                assert!(!opened[1].contains("code-1"));
                let state = |url: &str| {
                    url.split(['?', '&'])
                        .find_map(|pair| pair.strip_prefix("state="))
                        .map(str::to_owned)
                };
                assert_ne!(state(&opened[0]), state(&opened[1]));
                let requests = stack.requests();
                assert_eq!(test_support::count(&requests, "POST /oauth/token"), 2);
                assert_eq!(
                    test_support::count(&requests, "POST /api/v1/game-auth/tickets"),
                    1
                );
                assert!(
                    requests
                        .iter()
                        .any(|request| request.contains("challenge_token=challenge-proof-1"))
                );
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "challenge test thread failed")?
        .map_err(Into::into)
}

#[test]
fn mock_challenge_verifier_rejects_wrong_binding_and_replay() {
    let mut verifier = MockChallengeVerifier::default();
    assert_eq!(verifier.exchange("code=code-1&code_verifier=pkce").0, 403);
    let proof = "code=code-1&code_verifier=pkce&challenge_token=challenge-proof-1&challenge_id=01890a5d-ac96-774b-bcce-b302099a8057";
    assert_eq!(
        verifier
            .exchange(&proof.replace("code=code-1", "code=other"))
            .0,
        403
    );
    assert_eq!(
        verifier
            .exchange(&proof.replace("challenge_token=challenge-proof-1", "challenge_token=wrong"))
            .0,
        403
    );
    assert_eq!(
        verifier
            .exchange(&proof.replace("code_verifier=pkce", "code_verifier=wrong"))
            .0,
        403
    );
    assert_eq!(verifier.exchange(proof).0, 200);
    assert_eq!(verifier.exchange(proof).0, 403);
}

#[test]
fn login_challenge_browser_failure_or_wrong_state_never_issues_ticket() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                for wrong_state in [false, true] {
                    let stack = Stack::start_with_challenge()?;
                    let mut config = stack.config();
                    let original = config.open_browser.clone();
                    config.open_browser = Arc::new(move |url| {
                        if url.contains("/oauth/challenge?") {
                            return wrong_state
                                && original(&format!(
                                    "{}&state=wrong",
                                    url.split("&state=").next().unwrap_or_default()
                                ));
                        }
                        original(url)
                    });
                    assert_eq!(
                        stack.entry(config)?,
                        crate::GameplayEntryError::Rejected(
                            oteryn_platform_client::native_login::PublicClass::RetryLogin
                        )
                    );
                    let requests = stack.requests();
                    assert_eq!(test_support::count(&requests, "POST /oauth/token"), 1);
                    assert_eq!(
                        test_support::count(&requests, "POST /api/v1/game-auth/tickets"),
                        0
                    );
                    stack.finish();
                }
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "challenge failure test thread failed")?
        .map_err(Into::into)
}
