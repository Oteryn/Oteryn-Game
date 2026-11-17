//! S17B (livestream read-only viewer login and fan-out):
//! 1. A viewer opens the real listener and sends the cast Join with a ticket (`CastViewerSession`).
//!    Without the cast listener and a Platform-issued ticket the join must be refused.
//! 2. The admitted player session sends the cast consent command; without capability 292 the
//!    server must send a sequenced Rejected result.
//!
//! The row then reports the missing precondition.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::CastViewerSession;
use oteryn_protocol_oteryn::cast;
use tokio_rustls::rustls::pki_types::ServerName;

/// The viewer's first frame, a cast Join with a ticket Platform never issued, on the real
/// listener. The current protocol has no typed ticket-refusal frame.
pub(super) async fn viewer_join(clients: &SeamClients<'_>) -> TestResult {
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
        Err(error) => viewer_join_error(error),
    }
}

fn viewer_join_error(error: oteryn_dev_client::SessionError) -> TestResult {
    Err(format!(
        "step=viewer-join;observed=unclassified-join-error:{error:?};needed=typed-ticket-refusal"
    )
    .into())
}

pub(super) fn refusal_is_from_dispatch(
    result: &Result<
        oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse,
        oteryn_dev_client::SessionError,
    >,
) -> bool {
    matches!(result, Ok(reply) if reply.status == CommandStatus::Rejected && reply.payload.is_empty())
}

/// A correctly encoded unnegotiated cast command must be rejected by Game dispatch.
pub(super) async fn refused_by_session(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    message: &cast::Message,
) -> TestResult<String> {
    if client.selected_capabilities().contains(&capability) {
        return Err(format!("step=cast-command;observed=capability-{capability}-selected").into());
    }
    let payload = message
        .encode()
        .map_err(|error| format!("step=encode;observed={error:?}"))?;
    let reply = client
        .session_mut()
        .qualification_command(command, &payload)
        .await;
    let refused = refusal_is_from_dispatch(&reply);
    let error = format!("{reply:?}");
    expect(refused, "cast-command", &error)?;
    Ok(error)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xa2).await?;
    let consent = refused_by_session(
        &mut client,
        cast::CAPABILITY_CAST_CONSENT_V1,
        cast::COMMAND_TYPE_CAST_CONSENT,
        &cast::Message::ConsentSet {
            revision: 0,
            allowed: true,
        },
    )
    .await?;
    logout(&mut client).await?;
    evidence(&format!("S17B_CHECK consent-dispatch={consent}"));
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
        "e2e_all scenario=S17b.full status=FAIL seconds={:.3} evidence={detail}",
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
    fn viewer_transport_failures_do_not_prove_ticket_refusal() {
        use oteryn_dev_client::SessionError;
        for error in [
            SessionError::SessionUnusable,
            SessionError::Timeout("cast accept"),
            SessionError::Io(std::io::ErrorKind::UnexpectedEof.into()),
        ] {
            assert!(viewer_join_error(error).is_err());
        }
    }

    #[test]
    fn capability_refusal_requires_rejected_wire_result() {
        use oteryn_dev_client::SessionError;
        let result = oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 2,
            server_sequence: 7,
            status: CommandStatus::Rejected,
            payload: vec![],
        };
        assert!(refusal_is_from_dispatch(&Ok(result.clone())));
        assert!(!refusal_is_from_dispatch(&Ok(
            oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse {
                status: CommandStatus::Accepted,
                ..result
            }
        )));
        assert!(!refusal_is_from_dispatch(&Err(
            SessionError::CapabilityNotSelected { capability: 292 }
        )));
    }

    #[test]
    fn token_has_no_whitespace_and_is_bounded() {
        let value = token(&format!("step=a b\tc\n{}", "x".repeat(400)));
        assert!(!value.contains(char::is_whitespace));
        assert!(value.len() <= 240);
        assert!(value.starts_with("step=a-b-c-"));
    }
}
