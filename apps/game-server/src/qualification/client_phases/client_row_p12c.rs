//! P12c client replay capture: real CHARACTER_INFO QuerySelf replies (full and paged) and the
//! refused QueryVisible of another character. The consumer is `apply` in
//! `apps/client/src/features/0097-character-info/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::character_info_panel as info;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn send(
    client: &mut DevClientSession,
    operation: info::CharacterInfoOperation,
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(
            info::CAPABILITY_CHARACTER_INFO_V1,
            info::COMMAND_TYPE_CHARACTER_INFO,
            &info::encode_operation(operation),
        )
        .await?)
}

/// The summary values, deaths and PvP kills of a QuerySelf reply.
fn summary(bytes: &[u8]) -> TestResult<String> {
    if bytes.first() != Some(&0) || bytes.len() < 71 {
        return Err(format!("malformed character-info reply len={}", bytes.len()).into());
    }
    let mut values = Vec::new();
    for at in 0..8 {
        values.push(i64::from_le_bytes(bytes[1 + at * 8..9 + at * 8].try_into()?).to_string());
    }
    Ok(format!(
        "values={}|deaths={}|kills={}",
        values.join(","),
        u16::from_le_bytes(bytes[65..67].try_into()?),
        u16::from_le_bytes(bytes[67..69].try_into()?)
    ))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_character(clients, 0x31, 0).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let mut summaries = Vec::new();
        for (name, offset, limit) in [("query", 0, 50), ("paged", 1, 1)] {
            let page = info::PageRequest::new(offset, limit).ok_or("page")?;
            let reply = send(&mut client, info::CharacterInfoOperation::QuerySelf(page)).await?;
            if reply.status != CommandStatus::Accepted {
                return Err(format!("QuerySelf {name} status={:?}", reply.status).into());
            }
            capture(&format!("p12c-{name}.bin"), &reply.payload)?;
            summaries.push(summary(&reply.payload)?);
        }
        let target = clients.characters[1];
        let refused = send(
            &mut client,
            info::CharacterInfoOperation::QueryVisible {
                character: target,
                page: info::PageRequest::new(0, 1).ok_or("page")?,
            },
        )
        .await?;
        if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
            return Err(
                format!("QueryVisible was not rejected without payload: {refused:?}").into(),
            );
        }
        let target: String = target.iter().map(|byte| format!("{byte:02x}")).collect();
        let expected = format!(
            "query={}\npaged={}\nvisible_target={target}\nrefused_status=Rejected\n",
            summaries[0], summaries[1]
        );
        capture("p12c-expected.txt", expected.as_bytes())?;
        Ok(expected.replace('\n', ";"))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P12c.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P12c.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P12c.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_rejects_truncated_or_extra_title_data() {
        assert!(summary(&[]).is_err());
        let mut payload = vec![0; 71];
        assert!(summary(&payload).is_ok());
        payload[69] = 1;
        assert!(summary(&payload).is_err());
        payload.push(0);
        assert!(summary(&payload).is_ok());
        payload.push(0);
        assert!(summary(&payload).is_err());
    }
}
