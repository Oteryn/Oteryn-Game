//! N07a client replay capture: the real CHARACTER_INFO QuerySelf, accepted Select, refused
//! Select of an unearned title and follow-up QuerySelf that feed the Cyclopedia title selector.
//! The consumer is `apply` in `apps/client/src/features/0097-character-info/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::character_info_panel as info;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "N07A_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
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

/// The earned title ids and the selected id of a QuerySelf reply.
fn titles(bytes: &[u8]) -> TestResult<(Vec<u8>, u8)> {
    if bytes.first() != Some(&0) || bytes.len() < 71 {
        return Err(format!("malformed character-info reply len={}", bytes.len()).into());
    }
    let count = usize::from(bytes[69]);
    if bytes.len() != 71 + count {
        return Err(format!("title projection count={count} len={}", bytes.len()).into());
    }
    Ok((bytes[70..70 + count].to_vec(), bytes[70 + count]))
}

fn choose_title(earned: &[u8], selected: u8) -> Option<u8> {
    earned
        .iter()
        .copied()
        .find(|id| *id != 0 && *id != selected)
}

#[test]
fn title_selection_requires_an_earned_change() {
    assert!(choose_title(&[], 0).is_none());
    assert!(choose_title(&[1], 1).is_none());
    assert_eq!(choose_title(&[1, 2], 1), Some(2));
}

fn ids(values: &[u8]) -> String {
    values
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xf7).await?;
    // The page the titles selector requests (offset 0, limit 1).
    let page = info::PageRequest::new(0, 1).ok_or("page")?;
    let mut query = send(&mut client, info::CharacterInfoOperation::QuerySelf(page)).await?;
    if query.status != CommandStatus::Accepted {
        return Err(format!("QuerySelf status={:?}", query.status).into());
    }
    let (earned, mut selected_before) = titles(&query.payload)?;
    if choose_title(&earned, selected_before).is_none() && earned.contains(&selected_before) {
        let clear = send(&mut client, info::CharacterInfoOperation::Select(None)).await?;
        if clear.status != CommandStatus::Accepted
            || clear.payload.len() != 9
            || clear.payload.first() != Some(&3)
        {
            return Err(
                format!("could not clear title before exercising selection: {clear:?}").into(),
            );
        }
        query = send(&mut client, info::CharacterInfoOperation::QuerySelf(page)).await?;
        let (reread_earned, reread_selected) = titles(&query.payload)?;
        if query.status != CommandStatus::Accepted
            || reread_earned != earned
            || reread_selected != 0
        {
            return Err("title clear setup was not reflected by QuerySelf".into());
        }
        selected_before = reread_selected;
    }
    let title = choose_title(&earned, selected_before)
        .and_then(info::CharacterTitle::from_id)
        .ok_or("no different earned title")?;
    capture("n07a-query.bin", &query.payload);
    let chosen = title.id();
    let operation = info::CharacterInfoOperation::Select(Some(title));
    let select = send(&mut client, operation).await?;
    if select.status != CommandStatus::Accepted
        || select.payload.len() != 9
        || select.payload.first() != Some(&3)
    {
        return Err(format!("Select {chosen} reply {select:?}").into());
    }
    capture("n07a-select.bin", &select.payload);
    let revision = u64::from_le_bytes(select.payload[1..9].try_into()?);
    let unearned = (1..=u8::try_from(info::TITLE_COUNT)?)
        .filter(|id| !earned.contains(id))
        .find_map(info::CharacterTitle::from_id)
        .ok_or("every title is earned")?;
    let refused = send(
        &mut client,
        info::CharacterInfoOperation::Select(Some(unearned)),
    )
    .await?;
    if refused.status != CommandStatus::Rejected {
        return Err(format!("unearned Select was not rejected: {refused:?}").into());
    }
    let after = send(&mut client, info::CharacterInfoOperation::QuerySelf(page)).await?;
    if after.status != CommandStatus::Accepted {
        return Err(format!("follow-up QuerySelf status={:?}", after.status).into());
    }
    capture("n07a-query-after.bin", &after.payload);
    let (earned_after, selected_after) = titles(&after.payload)?;
    if earned_after != earned || selected_after != chosen {
        return Err(format!(
            "follow-up QuerySelf earned={earned_after:?} selected={selected_after}; need earned={earned:?} selected={chosen}"
        )
        .into());
    }
    let expected = format!(
        "earned={}\nselected_before={selected_before}\nselect_title={chosen}\nselect_revision={revision}\nrefused_title={}\nrefused_status=Rejected\nselected_after={selected_after}\n",
        ids(&earned),
        unearned.id()
    );
    capture("n07a-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=N07a.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N07a.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=N07a.client status=FAIL evidence=timeout"),
    }
}
