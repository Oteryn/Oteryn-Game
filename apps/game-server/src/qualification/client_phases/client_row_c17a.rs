//! Client replay capture for SCENARIO_MATRIX C17a (Haste / Strong Haste).
//! The owning client seam is `ConditionStatusPanel::finish` in
//! `crates/client-ui/src/features/0276-condition-status/state.rs`; it decodes the real condition
//! query reply, while `panel_text`/`status_text` in that feature's `view.rs` exposes Haste.

use super::*;
use oteryn_protocol_oteryn::condition_status::{
    CAPABILITY_CONDITION_STATUS_V1, COMMAND_TYPE_CONDITION_STATUS_QUERY, ConditionStatus,
};
use oteryn_protocol_oteryn::chat::{ChatDisposition, ChatIntent, ChatSpeechMode};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(95), verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(())) => evidence("SEAM_EVIDENCE e2e_all scenario=C17a.client status=PASS"),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C17a.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=C17a.client status=FAIL evidence=timeout",
        ),
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_character(clients, 0xca, 1).await?;
    let (haste, haste_payload) = cast_and_query(&mut client, "utani hur").await?;
    let (strong, strong_payload) = cast_and_query(&mut client, "utani gran hur").await?;

    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let expected = format!("{}\n---\n{}\n", visible(&haste)?, visible(&strong)?);
        for (name, bytes) in [
            ("c17a-haste.bin", haste_payload),
            ("c17a-strong-haste.bin", strong_payload),
        ] {
            if let Err(error) = std::fs::write(Path::new(&path).join(name), bytes) {
                evidence(&format!("C17A_CAPTURE_WRITE_ERROR file={name} error={error}"));
            }
        }
        if let Err(error) = std::fs::write(Path::new(&path).join("c17a-expected.txt"), expected) {
            evidence(&format!(
                "C17A_CAPTURE_WRITE_ERROR file=c17a-expected.txt error={error}"
            ));
        }
    }
    Ok(())
}

fn has_haste(status: &ConditionStatus) -> bool {
    status.conditions.iter().any(|condition| condition.icon == 13)
}

fn haste_transition(before: &ConditionStatus, after: &ConditionStatus) -> bool {
    !has_haste(before) && has_haste(after)
}

async fn cast_and_query(
    client: &mut oteryn_dev_client::DevClientSession,
    words: &str,
) -> TestResult<(ConditionStatus, Vec<u8>)> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(42);
    let before = loop {
        let (status, _) = query(client).await?;
        if !has_haste(&status) {
            break status;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C17a existing Haste did not expire before {words}").into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
        client.take_events();
    };
    let cast = client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: words.into(),
        })
        .await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        return Err(format!("C17a support cast refused: {words} {cast:?}").into());
    }
    let (status, payload) = query(client).await?;
    if !haste_transition(&before, &status) {
        return Err(format!("C17a Haste icon absent after {words}").into());
    }
    Ok((status, payload))
}

pub(super) async fn query(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<(ConditionStatus, Vec<u8>)> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_CONDITION_STATUS_V1,
            COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err("condition-status query refused".into());
    }
    let status = ConditionStatus::decode(&reply.payload).ok_or("malformed condition-status reply")?;
    Ok((status, reply.payload))
}

fn visible(status: &ConditionStatus) -> TestResult<String> {
    let condition = status
        .conditions
        .iter()
        .find(|condition| condition.icon == 13)
        .ok_or("C17a Haste icon absent")?;
    let duration = if condition.remaining_ms == -1 {
        "Permanent".into()
    } else {
        format!(
            "{}s",
            condition.remaining_ms / 1_000 + i64::from(condition.remaining_ms % 1_000 != 0)
        )
    };
    Ok(format!("» Haste: {duration}\r\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::condition_status::ActiveCondition;

    #[test]
    fn haste_requires_a_new_condition() {
        let active = ConditionStatus { conditions: vec![ActiveCondition { icon: 13, sub_id: 0, remaining_ms: 10_000 }] };
        assert!(haste_transition(&ConditionStatus::default(), &active));
        assert!(!haste_transition(&active, &active));
    }
}
