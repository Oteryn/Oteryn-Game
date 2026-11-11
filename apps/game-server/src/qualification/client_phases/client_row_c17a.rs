//! Client replay capture for SCENARIO_MATRIX C17a (Haste / Strong Haste).
//! The owning client seam is `ConditionStatusPanel::finish` in
//! `crates/client-ui/src/features/0276-condition-status/state.rs`; it decodes the real condition
//! query reply, while `panel_text`/`status_text` in that feature's `view.rs` exposes Haste.

use super::*;
use oteryn_protocol_oteryn::chat::{ChatDisposition, ChatIntent, ChatSpeechMode};
use oteryn_protocol_oteryn::condition_status::{
    CAPABILITY_CONDITION_STATUS_V1, COMMAND_TYPE_CONDITION_STATUS_QUERY, ConditionStatus,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = row_complete(journey(clients)).await;
    match result {
        Ok(Ok(())) => evidence("SEAM_EVIDENCE e2e_all scenario=C17a.client status=PASS"),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C17a.client status=FAIL evidence={error}"
        )),
        Err(_) => {
            evidence("SEAM_EVIDENCE e2e_all scenario=C17a.client status=FAIL evidence=timeout")
        }
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut client = connect_character(clients, 0xca, 1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let (haste, haste_payload) = cast_and_query(&mut client, "utani hur").await?;
        wait_support_cooldown(&mut client).await?;
        let (strong, strong_payload) = cast_and_query(&mut client, "utani gran hur").await?;

        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let expected = format!("{}\n---\n{}\n", visible(&haste)?, visible(&strong)?);
            for (name, bytes) in [
                ("c17a-haste.bin", haste_payload),
                ("c17a-strong-haste.bin", strong_payload),
            ] {
                std::fs::write(Path::new(&path).join(name), bytes)?;
            }
            std::fs::write(Path::new(&path).join("c17a-expected.txt"), expected)?;
        }
        Ok(())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

async fn cast_and_query(
    client: &mut oteryn_dev_client::DevClientSession,
    words: &str,
) -> TestResult<(ConditionStatus, Vec<u8>)> {
    let mana_before = client
        .actor_vitals()
        .ok_or("C17a caster vitals absent before cast")?
        .mana;
    let cast = client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: words.into(),
        })
        .await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        return Err(format!("C17a support cast refused: {words} {cast:?}").into());
    }
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_CONDITION_STATUS_V1,
            COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("C17a condition query refused after {words}").into());
    }
    let status = ConditionStatus::decode(&reply.payload).ok_or("C17a malformed condition reply")?;
    let mana_after = client
        .actor_vitals()
        .ok_or("C17a caster vitals absent after cast")?
        .mana;
    if mana_after >= mana_before {
        return Err(format!("C17a support cast did not debit mana: {words}").into());
    }
    if !status
        .conditions
        .iter()
        .any(|condition| condition.icon == 13)
    {
        return Err(format!("C17a Haste icon absent after {words}").into());
    }
    Ok((status, reply.payload))
}

async fn wait_support_cooldown(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let until = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
}

fn visible(status: &ConditionStatus) -> TestResult<String> {
    status
        .conditions
        .iter()
        .find(|condition| condition.icon == 13)
        .ok_or("C17a Haste icon absent")?;
    expected_conditions(status)
}

// Capture oracle: format wire conditions independently of the client UI under test.
fn expected_conditions(status: &ConditionStatus) -> TestResult<String> {
    const LABELS: [(&str, &str); 27] = [
        ("☠", "Poison"),
        ("♨", "Burning"),
        ("ϟ", "Energy"),
        ("♥", "Bleeding"),
        ("≈", "Drowning"),
        ("❄", "Freezing"),
        ("☀", "Dazzled"),
        ("☾", "Cursed"),
        ("◉", "Invisible"),
        ("◌", "Drunk"),
        ("⌖", "Rooted"),
        ("!", "Feared"),
        ("»", "Haste"),
        ("≪", "Paralysed"),
        ("♧", "Food"),
        ("+", "Recovery"),
        ("◈", "Mana shield"),
        ("☼", "Light"),
        ("◆", "Attributes"),
        ("↑", "Skill bonus"),
        ("+", "Regeneration"),
        ("◇", "Outfit"),
        ("✦", "Intense Hex"),
        ("×", "Muted"),
        ("○", "Pacified"),
        ("−", "Powerless"),
        ("!", "Agony"),
    ];
    let mut text = String::new();
    for condition in &status.conditions {
        let (icon, label) = LABELS
            .get(usize::from(condition.icon).wrapping_sub(1))
            .ok_or("unknown condition icon")?;
        let duration = match condition.remaining_ms {
            -1 => "Permanent".to_owned(),
            ms if ms > 0 => format!("{}s", ms.div_euclid(1000) + i64::from(ms % 1000 != 0)),
            _ => return Err("invalid condition duration".into()),
        };
        text.push_str(&format!("{icon} {label}: {duration}\r\n"));
    }
    if text.is_empty() {
        text.push_str("No active conditions");
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_row_haste_display_preserves_other_active_conditions() -> TestResult {
        use oteryn_protocol_oteryn::condition_status::ActiveCondition;
        let status = ConditionStatus {
            conditions: vec![
                ActiveCondition { icon: 1, sub_id: 0, remaining_ms: 1001 },
                ActiveCondition { icon: 13, sub_id: 0, remaining_ms: 1000 },
                ActiveCondition { icon: 15, sub_id: 0, remaining_ms: -1 },
            ],
        };
        assert_eq!(visible(&status)?, "☠ Poison: 2s\r\n» Haste: 1s\r\n♧ Food: Permanent\r\n");
        Ok(())
    }
    #[test]
    fn haste_display_rounds_up_and_handles_permanent() -> TestResult {
        use oteryn_protocol_oteryn::condition_status::ActiveCondition;
        assert!(visible(&ConditionStatus::default()).is_err());
        for (remaining_ms, text) in [(1, "1s"), (1000, "1s"), (1001, "2s"), (-1, "Permanent")] {
            let status = ConditionStatus {
                conditions: vec![ActiveCondition {
                    icon: 13,
                    sub_id: 0,
                    remaining_ms,
                }],
            };
            assert_eq!(visible(&status)?, format!("» Haste: {text}\r\n"));
        }
        Ok(())
    }
}
