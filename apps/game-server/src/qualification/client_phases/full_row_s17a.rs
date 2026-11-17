//! S17a (livestream, Canary only: caster on/off, password, description):
//! 1. Admit the node-boot character and require the cast control capability (290).
//! 2. Send cast Start and require the accepted caster state, then Stop.
//! 3. The wire `Start`/`Update` carry a password flag but no password or description text
//!    (crates/protocol-oteryn/src/cast.rs:81), so that part of the row has no server route.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{CommandStatus, cast};

fn cast_selected(client: &DevClientSession) -> Vec<u32> {
    client
        .selected_capabilities()
        .iter()
        .copied()
        .filter(|capability| {
            (cast::CAPABILITY_CAST_VIEW_V1..=cast::CAPABILITY_CAST_CONSENT_V1).contains(capability)
        })
        .collect()
}

const TEXT_GAP: &str = "crates/protocol-oteryn/src/cast.rs:81";

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let mut client = connect_world(clients, 0xa1).await?;
    let offered = cast_selected(&client);
    if !offered.contains(&cast::CAPABILITY_CAST_CONTROL_V1) {
        logout(&mut client).await?;
        return Ok((
            "FAIL",
            format!(
                "step=capability;observed=cast-capabilities-selected:{offered:?};needed=capability-290-selected(OTERYN_CAST_ENABLED+platform-cast-mTLS);also-missing-server:{TEXT_GAP}"
            ),
        ));
    }
    let start = cast::Message::Start {
        revision: 0,
        access_revision: 0,
        delay: 0,
        cap: 0,
        password: false,
    }
    .encode()
    .map_err(|error| format!("step=encode-start;observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            cast::CAPABILITY_CAST_CONTROL_V1,
            cast::COMMAND_TYPE_CAST_CONTROL,
            &start,
        )
        .await?;
    expect(
        reply.status == CommandStatus::Accepted,
        "start",
        reply.status,
    )?;
    let state = cast::Message::decode(&reply.payload)
        .map_err(|error| format!("step=start-decode;observed={error:?}"))?;
    let revision = match state {
        cast::Message::CasterState { revision, .. } => revision,
        other => return Err(format!("step=start;observed={other:?}").into()),
    };
    let stop = cast::Message::Stop { revision }
        .encode()
        .map_err(|error| format!("step=encode-stop;observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            cast::CAPABILITY_CAST_CONTROL_V1,
            cast::COMMAND_TYPE_CAST_CONTROL,
            &stop,
        )
        .await?;
    expect(
        reply.status == CommandStatus::Accepted,
        "stop",
        reply.status,
    )?;
    logout(&mut client).await?;
    Ok(("MISSING-SERVER", format!("source:{TEXT_GAP}")))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=S17a.full status={status} seconds={:.3} evidence={detail}",
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
