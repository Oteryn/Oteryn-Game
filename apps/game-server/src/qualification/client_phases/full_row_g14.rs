//! G14 server modal dialog query and answer (capability 450, commands 736 and 737).
//! 1. Admit the node-boot character and query the pending modal: the server notice dialog must
//!    be offered (the node-boot configuration supplies the notice text).
//! 2. A repeat query returns the same dialog while it is pending.
//! 3. An answer carrying another dialog identity is refused and the dialog stays pending.
//! 4. Answer the first button: the reply echoes the answer, a later query returns no dialog,
//!    and the identical answer replayed is acknowledged without re-opening the dialog.
//!
//! The modal state is scoped to the connection, so no cross-session assertion is made.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::server_modal as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "server-modal-offered-answered-and-cleared".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G14.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, payload) = send(client, capability, command, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=command-{command} observed=status:{status:?} needed=accepted-reply"
        )
        .into());
    }
    Ok(payload)
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

const CAPABILITY: u32 = wire::CAPABILITY_SERVER_MODAL_V1;

async fn query(client: &mut DevClientSession) -> TestResult<wire::Reply> {
    let payload = accepted(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_SERVER_MODAL_QUERY,
        &[],
    )
    .await?;
    let reply = wire::Reply::decode(&payload)
        .map_err(|error| format!("step=query observed=decode-{error}"))?;
    Ok(reply)
}

fn answer(dialog_id: u32, button_id: u8) -> TestResult<Vec<u8>> {
    Ok(wire::Answer {
        dialog_id,
        button_id,
        choice_id: None,
    }
    .encode()?)
}

async fn body(client: &mut DevClientSession, _clients: &SeamClients<'_>) -> TestResult {
    let wire::Reply::Dialog(dialog) = query(client).await? else {
        return Err("step=query observed=no-dialog needed=non-empty-server-motd".into());
    };
    let Some(button) = dialog.buttons.first().map(|button| button.0) else {
        return Err("step=query observed=dialog-without-buttons".into());
    };
    match query(client).await? {
        wire::Reply::Dialog(again) if format!("{again:?}") == format!("{dialog:?}") => {}
        other => return Err(format!("step=repeat observed={other:?}").into()),
    }
    let wrong = answer(dialog.id.wrapping_add(7), button)?;
    let (status, payload) = send(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_SERVER_MODAL_ANSWER,
        &wrong,
    )
    .await?;
    if status == CommandStatus::Accepted
        && matches!(wire::Reply::decode(&payload), Ok(wire::Reply::Answered(_)))
    {
        return Err("step=wrong-answer observed=accepted-for-unknown-dialog".into());
    }
    match query(client).await? {
        wire::Reply::Dialog(still) if still.id == dialog.id => {}
        other => return Err(format!("step=wrong-answer observed=dialog-changed:{other:?}").into()),
    }
    let right = answer(dialog.id, button)?;
    let payload = accepted(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_SERVER_MODAL_ANSWER,
        &right,
    )
    .await?;
    match wire::Reply::decode(&payload) {
        Ok(wire::Reply::Answered(given))
            if given.dialog_id == dialog.id && given.button_id == button => {}
        other => return Err(format!("step=answer observed={other:?}").into()),
    }
    if !matches!(query(client).await?, wire::Reply::None) {
        return Err("step=cleared observed=dialog-still-pending".into());
    }
    let payload = accepted(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_SERVER_MODAL_ANSWER,
        &right,
    )
    .await?;
    match wire::Reply::decode(&payload) {
        Ok(wire::Reply::Answered(_)) => {}
        other => return Err(format!("step=replay observed={other:?}").into()),
    }
    if !matches!(query(client).await?, wire::Reply::None) {
        return Err("step=replay observed=dialog-reopened".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x76).await?;
    let outcome = verdict::observe(body(&mut client, clients)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    outcome?;
    closed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }
}
