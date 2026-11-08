//! N06 (client-only; Platform challenge issuance is a recorded Platform blocker): the production
//! authorize path through the challenge step against the `test_support` Platform responses.
//! Platform issues a challenge, the right answer is accepted and admission proceeds; a wrong
//! answer and an unanswered (expired) challenge stop before any ticket or admission.
use crate::GameplayEntryError;
use crate::native_entry::test_support::{self, Stack};
use oteryn_platform_client::native_login::PublicClass;
use std::io::{Read as _, Write as _};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const CHALLENGE_ID: &str = "challenge_id=01890a5d-ac96-774b-bcce-b302099a8057";

/// Opens the sign-in URL like the test browser does, but returns `answer` for a challenge URL;
/// `None` opens it without ever answering.
fn answer_browser(url: &str, answer: Option<&str>) -> bool {
    let query = |name: &str| {
        url.split(['?', '&'])
            .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
            .map(|value| value.replace("%3A", ":").replace("%2F", "/"))
    };
    let (Some(redirect), Some(state)) = (query("redirect_uri"), query("state")) else {
        return false;
    };
    let Some(authority) = redirect
        .strip_prefix("http://")
        .and_then(|rest| rest.strip_suffix("/callback"))
        .map(str::to_owned)
    else {
        return false;
    };
    let Some(answer) = answer.map(str::to_owned) else {
        return true;
    };
    std::thread::spawn(move || {
        if let Ok(mut stream) = std::net::TcpStream::connect(&authority) {
            let _ = write!(
                stream,
                "GET /callback?code={answer}&state={state} HTTP/1.1\r\nhost: {authority}\r\n\r\n"
            );
            let _ = stream.read_to_end(&mut Vec::new());
        }
    });
    true
}

fn position(requests: &[String], needle: &str) -> Option<usize> {
    requests.iter().position(|request| request.contains(needle))
}

fn run() -> TestResult {
    // Challenge issued, right answer accepted, admission proceeds.
    let stack = Stack::start_with_challenge()?;
    let client = crate::ClientBootstrap::new()?.with_native_login(stack.config());
    let admitted = client.request_gameplay_entry()?;
    client.shutdown();
    assert_eq!(admitted.character_name, "Ari");
    let requests = stack.requests();
    let tokens: Vec<_> = requests
        .iter()
        .filter(|request| request.starts_with("POST /oauth/token"))
        .collect();
    assert_eq!(tokens.len(), 2);
    assert!(!tokens[0].contains("challenge_token="));
    assert!(tokens[1].contains("challenge_token=challenge-proof-1"));
    assert!(tokens[1].contains(CHALLENGE_ID));
    assert!(tokens[1].contains("code=code-1"));
    let ticket =
        position(&requests, "POST /api/v1/game-auth/tickets").ok_or("no ticket request")?;
    let answered = position(&requests, "challenge_token=challenge-proof-1").ok_or("no answer")?;
    assert!(
        answered < ticket,
        "the ticket was issued before the challenge was answered"
    );
    assert_eq!(
        test_support::count(&requests, "POST /api/v1/game-auth/tickets"),
        1
    );
    assert!(stack.admissions.load(Ordering::SeqCst) >= 1);
    stack.finish();

    // A wrong answer is refused by Platform; an unanswered challenge expires locally.
    for answer in [Some("wrong-answer"), None] {
        let stack = Stack::start_with_challenge()?;
        let mut config = stack.config();
        let original = config.open_browser.clone();
        let opened = Arc::new(Mutex::new(0_usize));
        let counter = opened.clone();
        config.callback_timeout = Duration::from_millis(600);
        config.open_browser = Arc::new(move |url| {
            if url.contains("/oauth/challenge?") {
                if let Ok(mut count) = counter.lock() {
                    *count += 1;
                }
                return answer_browser(url, answer);
            }
            original(url)
        });
        let error = stack.entry(config)?;
        assert_eq!(*opened.lock().map_err(|_| "browser count")?, 1);
        let requests = stack.requests();
        let challenged = test_support::count(&requests, "POST /oauth/token");
        match answer {
            Some(wrong) => {
                assert_eq!(challenged, 2, "the wrong answer must reach Platform once");
                assert!(position(&requests, &format!("challenge_token={wrong}")).is_some());
            }
            None => assert_eq!(
                challenged, 1,
                "an unanswered challenge must not retry the exchange"
            ),
        }
        assert_eq!(error, GameplayEntryError::Rejected(PublicClass::RetryLogin));
        assert_eq!(
            test_support::count(&requests, "POST /api/v1/game-auth/tickets"),
            0
        );
        assert_eq!(stack.admissions.load(Ordering::SeqCst), 0);
        stack.finish();
    }
    Ok(())
}

#[test]
fn e2e_n06_login_challenge_answer_accepted_wrong_or_expired_stops_before_admission() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| run().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "challenge test thread failed")?
        .map_err(Into::into)
}
