//! S17C (livestream viewer chat and moderation):
//! 1. A viewer joins the real listener with an unissued ticket; the join must be refused, so no
//!    viewer chat is possible.
//! 2. The admitted player session sends cast chat (capability 291) and the caster moderation
//!    command (505); without those capabilities the session must refuse both exactly with
//!    `CapabilityNotSelected`.
//!
//! The row then reports the missing precondition.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::CastViewerSession;
use oteryn_protocol_oteryn::cast;
use tokio_rustls::rustls::pki_types::ServerName;

/// The viewer's first frame, a cast Join with a ticket Platform never issued, on the real
/// listener. Returns the session error of the refused join.
async fn viewer_join(clients: &SeamClients<'_>) -> TestResult<String> {
    let connector =
        crate::qualification::connector(clients.certificate, &crate::qualification::EXACT)?;
    let tcp = tokio::net::TcpStream::connect(clients.address).await?;
    let stream = connector
        .connect(ServerName::try_from("localhost")?, tcp)
        .await?;
    match CastViewerSession::join(
        stream,
        b"oteryn-qualification-unissued-ticket",
        Duration::from_secs(5),
    )
    .await
    {
        Ok(_) => Err("step=viewer-join;observed=joined-with-an-unissued-ticket".into()),
        Err(error) => Ok(format!("{error:?}")),
    }
}

/// A gated cast command from the admitted player session must fail before the wire.
async fn refused_by_session(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    message: &cast::Message,
) -> TestResult<String> {
    let payload = message
        .encode()
        .map_err(|error| format!("step=encode;observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, &payload)
        .await;
    let error = reply
        .err()
        .map(|error| format!("{error:?}"))
        .ok_or("step=cast-command;observed=accepted-without-capability")?;
    expect(
        error.contains("CapabilityNotSelected"),
        "cast-command",
        &error,
    )?;
    Ok(error)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let refused = viewer_join(clients).await?;
    let mut client = connect_world(clients, 0xa3).await?;
    let chat = refused_by_session(
        &mut client,
        cast::CAPABILITY_CAST_CHAT_V1,
        cast::COMMAND_TYPE_CAST_CHAT,
        &cast::Message::Chat("hello".to_owned()),
    )
    .await?;
    let moderate = refused_by_session(
        &mut client,
        cast::CAPABILITY_CAST_CONTROL_V1,
        cast::COMMAND_TYPE_CAST_MODERATE,
        &cast::Message::Moderate {
            revision: 0,
            subject: [0; 16],
            action: 0,
            key: [0; 16],
        },
    )
    .await?;
    logout(&mut client).await?;
    Ok((
        "FAIL",
        format!(
            "step=viewer-chat;observed=join-refused:{refused},chat:{chat},moderate:{moderate};needed=cast-listener-and-platform-issued-viewer-ticket"
        ),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(90), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=S17C.full status={status} seconds={:.3} evidence={detail}",
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
