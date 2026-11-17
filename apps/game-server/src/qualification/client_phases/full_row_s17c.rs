//! S17C (livestream viewer chat and moderation):
//! 1. A viewer joins the real listener with an unissued ticket; the join must be refused, so no
//!    viewer chat is possible.
//! 2. The admitted player session sends cast chat (capability 291) and the caster moderation
//!    command (505); without those capabilities the session must refuse both exactly with
//!    a sequenced Rejected result.
//!
//! The row then reports the missing precondition.
use super::full_row_s17b::{refused_by_session, viewer_join};
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::cast;

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xa3).await?;
    let chat = refused_by_session(
        &mut client,
        cast::CAPABILITY_CAST_CHAT_V1,
        cast::COMMAND_TYPE_CAST_CHAT,
        &cast::Message::Reply("hello".to_owned()),
    )
    .await?;
    let moderate = refused_by_session(
        &mut client,
        cast::CAPABILITY_CAST_CONTROL_V1,
        cast::COMMAND_TYPE_CAST_MODERATE,
        &cast::Message::Moderate {
            revision: 0,
            subject: [0; 16],
            action: 1,
            key: [0x35; 16],
        },
    )
    .await?;
    logout(&mut client).await?;
    evidence(&format!(
        "S17C_CHECK chat-dispatch={chat} moderation-dispatch={moderate}"
    ));
    viewer_join(clients).await
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let detail = match result {
        Ok(Ok(())) => "step=viewer-join;needed=platform-issued-viewer-ticket".to_owned(),
        Ok(Err(error)) => token(&error.to_string()),
        Err(_) => "step=journey;observed=timeout".to_owned(),
    };
    evidence(&format!(
        "e2e_all scenario=S17c.full status=FAIL seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

/// One evidence token: no whitespace, bounded length.
fn token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:/=<>,;".contains(character) {
                character
            } else {
                '-'
            }
        })
        .take(240)
        .collect()
}

fn expect(ok: bool, step: &str, observed: impl std::fmt::Debug) -> TestResult {
    if ok {
        Ok(())
    } else {
        Err(format!("step={step};observed={observed:?}").into())
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    expect(
        reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
        "logout",
        reply.result,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_no_whitespace_and_is_bounded() {
        let value = token(&format!("step=a b\tc\n{}", "x".repeat(400)));
        assert!(!value.contains(char::is_whitespace));
        assert!(value.len() <= 240);
        assert!(value.starts_with("step=a-b-c-"));
    }
}
