use super::test_support::Stack;
use super::*;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_session_tcp::connection::connect_any;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn an_unreachable_first_address_falls_through_to_the_next() -> TestResult {
    let stack = Stack::start(vec![1100])?;
    let root = CertificateDer::from(std::fs::read(&stack.root_path)?);
    let dead = std::net::TcpListener::bind("127.0.0.1:0")?.local_addr()?;
    let live = SocketAddr::from(([127, 0, 0, 1], stack.game_port));
    let connected = stack.servers.block_on(async {
        (
            connect_any(&[dead], "localhost", &root).await.is_some(),
            connect_any(&[dead, live], "localhost", &root)
                .await
                .is_some(),
        )
    })?;
    stack.finish();
    assert_eq!(connected, (false, true));
    Ok(())
}

#[test]
fn absent_world_stops_before_any_credential_request() -> TestResult {
    let stack = Stack::start(vec![1100])?;
    let mut config = stack.config();
    config.world_ref = "w-other".to_owned();
    assert_eq!(stack.entry(config)?, GameplayEntryError::WorldUnavailable);
    let requests = stack.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        super::test_support::count(&requests, "GET /v1/client/directory"),
        1
    );
    assert_eq!(stack.admissions.load(Ordering::SeqCst), 0);
    stack.finish();
    Ok(())
}

#[test]
fn terminal_refusal_is_its_public_class_without_retry() -> TestResult {
    // 1105 ADMISSION_GRANT_REPLAYED: SECURITY_TERMINAL, SESSION_UNAVAILABLE.
    let stack = Stack::start(vec![1105])?;
    assert_eq!(
        stack.entry(stack.config())?,
        GameplayEntryError::Rejected(PublicClass::SessionUnavailable)
    );
    let requests = stack.requests();
    for target in [
        "GET /v1/client/directory",
        "POST /oauth/token",
        "POST /v1/game-auth/tickets",
        "POST /v1/login",
    ] {
        assert_eq!(super::test_support::count(&requests, target), 1, "{target}");
    }
    let token = requests
        .iter()
        .find(|request| request.starts_with("POST /oauth/token"))
        .cloned()
        .unwrap_or_default();
    assert!(token.contains("code=code-1"));
    assert!(token.contains("code_verifier="));
    assert_eq!(stack.admissions.load(Ordering::SeqCst), 1);
    stack.finish();
    Ok(())
}

#[test]
fn not_yet_valid_retries_the_same_grant_three_times() -> TestResult {
    let stack = Stack::start(vec![1103])?;
    assert_eq!(
        stack.entry(stack.config())?,
        GameplayEntryError::Rejected(PublicClass::TemporarilyUnavailable)
    );
    assert_eq!(stack.admissions.load(Ordering::SeqCst), 4);
    assert_eq!(
        super::test_support::count(&stack.requests(), "POST /v1/login"),
        1
    );
    stack.finish();
    Ok(())
}

#[test]
fn capacity_exceeded_retries_once() -> TestResult {
    let stack = Stack::start(vec![1115])?;
    assert_eq!(
        stack.entry(stack.config())?,
        GameplayEntryError::Rejected(PublicClass::TemporarilyUnavailable)
    );
    assert_eq!(stack.admissions.load(Ordering::SeqCst), 2);
    stack.finish();
    Ok(())
}

#[test]
fn reconciliation_repeats_the_gateway_login_with_the_same_attempt() -> TestResult {
    let stack = Stack::start(vec![1106, 1105])?;
    assert_eq!(
        stack.entry(stack.config())?,
        GameplayEntryError::Rejected(PublicClass::SessionUnavailable)
    );
    let logins = stack
        .requests()
        .into_iter()
        .filter(|request| request.starts_with("POST /v1/login"))
        .collect::<Vec<_>>();
    assert_eq!(logins.len(), 2);
    assert_eq!(logins.first(), logins.get(1));
    assert_eq!(
        super::test_support::count(&stack.requests(), "POST /v1/game-auth/tickets"),
        1
    );
    assert_eq!(stack.admissions.load(Ordering::SeqCst), 2);
    stack.finish();
    Ok(())
}

#[test]
fn configuration_is_read_only_when_the_platform_is_set() -> TestResult {
    assert!(NativeLoginConfig::from_env(|_name| None)?.is_none());
    let partial = NativeLoginConfig::from_env(|name| {
        (name == "OTERYN_PLATFORM_URL").then(|| "https://platform.test/".to_owned())
    });
    assert_eq!(
        partial.err(),
        Some(NativeLoginSettingError("OTERYN_GATEWAY_URL"))
    );
    let config =
        NativeLoginConfig::from_env(|name| Some(format!("{name}-value")))?.ok_or("configured")?;
    assert_eq!(config.character_id, "OTERYN_CHARACTER_ID-value");
    assert!(!format!("{config:?}").contains("OTERYN_OAUTH_CLIENT_ID"));
    Ok(())
}
