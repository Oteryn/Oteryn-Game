use super::test_server::*;
use super::*;

mod pagination;
mod recovery;
mod replay;
mod transfer;

#[test]
fn account_services_scope_is_opt_in() -> TestResult {
    let mock = MockPlatform::start()?;
    let client = mock.client()?;
    let game = client.authorization_url("http://127.0.0.1:1/callback", "s", "p")?;
    let services =
        client.account_services_authorization_url("http://127.0.0.1:1/callback", "s", "p")?;
    assert!(
        game.query_pairs()
            .any(|(k, v)| k == "scope" && v == "game:ticket")
    );
    assert!(
        services
            .query_pairs()
            .any(|(k, v)| k == "scope" && v == "game:account-services")
    );
    Ok(())
}
