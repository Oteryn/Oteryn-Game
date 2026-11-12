//! P12a client replay capture: real CHARACTER_INFO QuerySelf replies and the refusal of selecting
//! a title the character has not earned. The consumer is `apply` in
//! `apps/client/src/features/0097-character-info/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::character_info_panel as info;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("P12A_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
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

/// The summary values, deaths, PvP kills, earned title ids and selected id of a QuerySelf reply.
fn summary(bytes: &[u8]) -> TestResult<String> {
    if bytes.first() != Some(&0) || bytes.len() < 71 {
        return Err(format!("malformed character-info reply len={}", bytes.len()).into());
    }
    let mut values = Vec::new();
    for at in 0..8 {
        values.push(i64::from_le_bytes(bytes[1 + at * 8..9 + at * 8].try_into()?).to_string());
    }
    let count = usize::from(bytes[69]);
    if bytes.len() != 71 + count {
        return Err(format!("title projection count={count} len={}", bytes.len()).into());
    }
    let earned: Vec<String> = bytes[70..70 + count].iter().map(u8::to_string).collect();
    Ok(format!(
        "values={}\ndeaths={}\nkills={}\nearned={}\nselected={}\n",
        values.join(","),
        u16::from_le_bytes(bytes[65..67].try_into()?),
        u16::from_le_bytes(bytes[67..69].try_into()?),
        earned.join(","),
        bytes[70 + count]
    ))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x30).await?;
    let page = info::PageRequest::new(0, 50).ok_or("page")?;
    let query = send(&mut client, info::CharacterInfoOperation::QuerySelf(page)).await?;
    if query.status != CommandStatus::Accepted {
        return Err(format!("QuerySelf status={:?}", query.status).into());
    }
    capture("p12a-query.bin", &query.payload);
    let before = summary(&query.payload)?;
    let earned: Vec<u8> = before
        .lines()
        .find_map(|line| line.strip_prefix("earned="))
        .unwrap_or("")
        .split(',')
        .filter_map(|id| id.parse().ok())
        .collect();
    let unearned = (1..=u8::try_from(info::TITLE_COUNT)?)
        .filter(|id| !earned.contains(id))
        .find_map(info::CharacterTitle::from_id)
        .ok_or("every title is earned")?;
    let refused = send(&mut client, info::CharacterInfoOperation::Select(Some(unearned))).await?;
    if refused.status != CommandStatus::Rejected {
        return Err(format!("unearned Select was not rejected: {refused:?}").into());
    }
    if !refused.payload.is_empty() {
        capture("p12a-refused.bin", &refused.payload);
    }
    let after = send(&mut client, info::CharacterInfoOperation::QuerySelf(page)).await?;
    if after.status != CommandStatus::Accepted || after.payload != query.payload {
        return Err("unearned Select changed the earned or selected title".into());
    }
    capture("p12a-query-after.bin", &after.payload);
    let expected = format!(
        "{before}refused_title={}\nrefused_status=Rejected\n",
        unearned.id()
    );
    capture("p12a-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P12a.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P12a.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P12a.client status=FAIL evidence=timeout"),
    }
}
