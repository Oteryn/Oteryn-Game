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
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
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
    decode_reply(input_selected(client), &payload)
}

fn input_selected(client: &DevClientSession) -> bool {
    client
        .selected_capabilities()
        .contains(&wire::input::CAPABILITY_SERVER_MODAL_INPUT_V1)
}

fn decode_reply(selected: bool, payload: &[u8]) -> TestResult<wire::Reply> {
    let (reply, input) = if selected {
        wire::input::decode_reply_with_input(payload)
    } else {
        wire::Reply::decode(payload).map(|reply| (reply, None))
    }
    .map_err(|error| format!("step=query observed=decode-{error}"))?;
    if input.is_some() {
        return Err(
            "step=query observed=dialog-requires-input needed=real-input-value-prerequisite".into(),
        );
    }
    Ok(reply)
}

fn answer(selected: bool, dialog_id: u32, button_id: u8) -> TestResult<Vec<u8>> {
    let answer = wire::Answer {
        dialog_id,
        button_id,
        choice_id: None,
    };
    if selected {
        Ok(wire::input::InputAnswer {
            answer,
            value: None,
            dont_show_again: false,
        }
        .encode()?)
    } else {
        Ok(answer.encode()?)
    }
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
    let selected = input_selected(client);
    let wrong_id = dialog.id.checked_add(1).unwrap_or(1);
    let wrong = answer(selected, wrong_id, button)?;
    let (status, payload) = send(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_SERVER_MODAL_ANSWER,
        &wrong,
    )
    .await?;
    if status == CommandStatus::Accepted
        && matches!(
            decode_reply(selected, &payload),
            Ok(wire::Reply::Answered(_))
        )
    {
        return Err("step=wrong-answer observed=accepted-for-unknown-dialog".into());
    }
    match query(client).await? {
        wire::Reply::Dialog(still) if still.id == dialog.id => {}
        other => return Err(format!("step=wrong-answer observed=dialog-changed:{other:?}").into()),
    }
    let right = answer(selected, dialog.id, button)?;
    let payload = accepted(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_SERVER_MODAL_ANSWER,
        &right,
    )
    .await?;
    match decode_reply(selected, &payload) {
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
    match decode_reply(selected, &payload) {
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
    #[test]
    fn selected_modal_roundtrip_and_wrong_identity_are_well_formed() -> TestResult {
        let dialog = wire::Dialog {
            id: 7,
            title: "Notice".into(),
            message: "Real modal codec regression".into(),
            choices: Vec::new(),
            buttons: vec![(1, "OK".into())],
            cancel_button: None,
            image: None,
        };
        let bytes = wire::input::encode_dialog_with_input(&dialog, None)?;
        assert_eq!(
            decode_reply(true, &bytes)?,
            wire::Reply::Dialog(dialog.clone())
        );
        assert!(decode_reply(false, &bytes).is_err());
        let legacy = wire::Reply::Dialog(dialog.clone()).encode()?;
        assert_eq!(decode_reply(false, &legacy)?, wire::Reply::Dialog(dialog));
        assert!(decode_reply(true, &legacy).is_err());
        for selected in [false, true] {
            let right = answer(selected, 7, 1)?;
            let wrong = answer(selected, 8, 1)?;
            let right = wire::input::decode_answer_selected(selected, &right)?;
            let wrong = wire::input::decode_answer_selected(selected, &wrong)?;
            assert_eq!((right.answer.dialog_id, wrong.answer.dialog_id), (7, 8));
            assert_eq!(right.answer.button_id, wrong.answer.button_id);
            assert_eq!(right.value, wrong.value);
            assert_eq!(right.dont_show_again, wrong.dont_show_again);
        }
        Ok(())
    }
}
