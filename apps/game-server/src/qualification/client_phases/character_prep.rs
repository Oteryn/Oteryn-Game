//! Disposable character preparation through the node control socket.
use super::*;

pub(super) async fn grant_premium(client: &oteryn_dev_client::DevClientSession) -> TestResult {
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "premium".to_owned(),
    )
    .await?;
    Ok(())
}
