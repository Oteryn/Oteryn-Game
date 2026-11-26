//! G14 server modal dialog query and answer (capability 450, commands 736 and 737).
//! 1. Admit the node-boot character and query the pending modal: the server notice dialog must
//!    be offered (the node-boot configuration supplies the notice text).
//! 2. A repeat query returns the same dialog while it is pending.
//! 3. An answer carrying another dialog identity is refused and the dialog stays pending.
//! 4. Answer Acknowledge: the reply echoes the answer, a later query returns no dialog,
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
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={reply:?} needed=first-attempt-accepted").into());
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

const ACKNOWLEDGE: u8 = 1;
const CANCEL: u8 = 2;

fn require_notice(reply: wire::Reply, defaults_selected: bool) -> TestResult<wire::Dialog> {
    let wire::Reply::Dialog(dialog) = reply else {
        return Err("step=query observed=no-dialog needed=non-empty-server-motd".into());
    };
    if dialog.title != "Server notice"
        || dialog.message != "Welcome to the node-boot qualification world."
    {
        return Err("step=query observed=unexpected-dialog needed=configured-server-motd".into());
    }
    if dialog.buttons.is_empty() {
        return Err("step=query observed=dialog-without-buttons".into());
    }
    if !dialog
        .buttons
        .iter()
        .map(|(id, label)| (*id, label.as_str()))
        .eq([(ACKNOWLEDGE, "Acknowledge"), (CANCEL, "Cancel")])
        || dialog.default_button != defaults_selected.then_some(ACKNOWLEDGE)
        || dialog.cancel_button != Some(CANCEL)
        || !dialog.choices.is_empty()
        || dialog.image.is_some()
    {
        return Err(
            "step=query observed=unexpected-notice-controls needed=exact-notice-descriptor".into(),
        );
    }
    Ok(dialog)
}

async fn body(client: &mut DevClientSession, _clients: &SeamClients<'_>) -> TestResult {
    let defaults_selected = client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_SERVER_MODAL_DEFAULTS_V1);
    let dialog = require_notice(query(client).await?, defaults_selected)?;
    match query(client).await? {
        wire::Reply::Dialog(again) if again == dialog => {}
        other => return Err(format!("step=repeat observed={other:?}").into()),
    }
    let selected = input_selected(client);
    let wrong_id = dialog.id.checked_add(1).unwrap_or(1);
    let wrong = answer(selected, wrong_id, ACKNOWLEDGE)?;
    let (status, payload) = send(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_SERVER_MODAL_ANSWER,
        &wrong,
    )
    .await?;
    if status != CommandStatus::Rejected || !payload.is_empty() {
        return Err(format!(
            "step=wrong-answer observed=status:{status:?},payload-len:{} needed=empty-rejection",
            payload.len()
        )
        .into());
    }
    match query(client).await? {
        wire::Reply::Dialog(still) if still == dialog => {}
        other => return Err(format!("step=wrong-answer observed=dialog-changed:{other:?}").into()),
    }
    let right = answer(selected, dialog.id, ACKNOWLEDGE)?;
    let expected_answer = wire::Answer {
        dialog_id: dialog.id,
        button_id: ACKNOWLEDGE,
        choice_id: None,
    };
    let payload = accepted(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_SERVER_MODAL_ANSWER,
        &right,
    )
    .await?;
    match decode_reply(selected, &payload) {
        Ok(wire::Reply::Answered(given)) if given == expected_answer => {}
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
        Ok(wire::Reply::Answered(given)) if given == expected_answer => {}
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
    fn rejects_missing_notice_and_consistently_wrong_controls() -> TestResult {
        let notice = wire::Dialog {
            id: 7,
            title: "Server notice".into(),
            message: "Welcome to the node-boot qualification world.".into(),
            choices: Vec::new(),
            buttons: vec![(1, "Acknowledge".into()), (2, "Cancel".into())],
            default_button: Some(1),
            cancel_button: Some(2),
            image: None,
        };
        assert_eq!(
            require_notice(wire::Reply::Dialog(notice.clone()), true)?,
            notice
        );
        assert!(require_notice(wire::Reply::None, true).is_err());
        let mut swapped = notice.clone();
        swapped.buttons.swap(0, 1);
        let mut relabeled = notice.clone();
        relabeled.buttons[0].1 = "OK".into();
        let mut wrong_id = notice.clone();
        wrong_id.buttons[0].0 = 3;
        let mut wrong_default = notice.clone();
        wrong_default.default_button = Some(2);
        let mut wrong_cancel = notice.clone();
        wrong_cancel.cancel_button = Some(1);
        for malformed in [swapped, relabeled, wrong_id, wrong_default, wrong_cancel] {
            assert!(require_notice(wire::Reply::Dialog(malformed), true).is_err());
        }
        let mut legacy = notice;
        legacy.default_button = None;
        assert!(require_notice(wire::Reply::Dialog(legacy.clone()), true).is_err());
        assert_eq!(
            require_notice(wire::Reply::Dialog(legacy.clone()), false)?,
            legacy
        );
        Ok(())
    }

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
            default_button: None,
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
