//! G10: Cyclopedia titles, analyser, boss tracker and minimap markers.
//! 1. Admit, require the analyser capability (10) in the selected set.
//! 2. QuerySelf; select an earned title (needs at least one earned title), then a refused
//!    unearned Select, then QuerySelf again for the selected delta.
//! 3. BOSS_COOLDOWNS twice about one second apart; the server clock must advance.
//! 4. Log out, re-admit and QuerySelf: the selected title must persist.
//!
//! The server-driven parts are judged on real frames. Minimap markers are client-local state
//! (`apps/client/src/features/local-minimap/state.rs`, `Marker`): no protocol command carries
//! them and the dev client links no marker store, so a row whose server parts pass cannot prove
//! the markers and ends FAIL with `needed:` naming the missing local driver (never a claimed
//! missing client surface).
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::boss_encounter as boss;
use oteryn_dev_client::client_runtime::character_info_panel as info;
use oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse;

const NEEDED: &str = "needed:minimap-marker-add-remove-driver;server-parts=pass";
const ANALYSER_CAPABILITY: u32 = 10;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("FAIL", NEEDED.to_owned()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G10.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn send(
    client: &mut DevClientSession,
    operation: info::CharacterInfoOperation,
) -> TestResult<ServerMechanicResponse> {
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
        return Err(format!("step=query;observed=malformed-len-{}", bytes.len()).into());
    }
    let count = usize::from(bytes[69]);
    if bytes.len() != 71 + count {
        return Err(format!("step=query;observed=count-{count}-len-{}", bytes.len()).into());
    }
    Ok((bytes[70..70 + count].to_vec(), bytes[70 + count]))
}

async fn query(client: &mut DevClientSession) -> TestResult<(Vec<u8>, u8)> {
    let page = info::PageRequest::new(0, 1).ok_or("step=query;observed=page")?;
    let reply = send(client, info::CharacterInfoOperation::QuerySelf(page)).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=query;observed={:?}", reply.status).into());
    }
    titles(&reply.payload)
}

async fn cooldown_now(client: &mut DevClientSession) -> TestResult<u64> {
    let query = boss::query_taints("").ok_or("step=cooldowns;observed=encoder")?;
    let reply = client
        .session_mut()
        .gated_command(query.capability, query.kind, &query.payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=cooldowns;observed={:?}", reply.status).into());
    }
    let snapshot = boss::decode(&reply.payload).ok_or("step=cooldowns;observed=malformed")?;
    Ok(snapshot.now)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout;observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xd1).await?;
    if !client
        .selected_capabilities()
        .contains(&ANALYSER_CAPABILITY)
    {
        return Err("step=admission;needed:analyser-capability-10-selected".into());
    }
    let (earned, _before) = query(&mut client).await?;
    let Some(chosen) = earned.first().copied() else {
        return Err("step=select;needed:earned-title-reachable-by-ordinary-play".into());
    };
    let select = send(
        &mut client,
        info::CharacterInfoOperation::Select(info::CharacterTitle::from_id(chosen)),
    )
    .await?;
    if select.status != CommandStatus::Accepted {
        return Err(format!("step=select;observed={:?}", select.status).into());
    }
    let unearned = (1..=u8::try_from(info::TITLE_COUNT)?)
        .filter(|id| !earned.contains(id))
        .find_map(info::CharacterTitle::from_id)
        .ok_or("step=refuse;needed:an-unearned-title")?;
    let refused = send(
        &mut client,
        info::CharacterInfoOperation::Select(Some(unearned)),
    )
    .await?;
    if refused.status != CommandStatus::Rejected {
        return Err(format!("step=refuse;observed={:?}", refused.status).into());
    }
    let (_, selected) = query(&mut client).await?;
    if selected != chosen {
        return Err(format!("step=select-delta;observed={selected};need={chosen}").into());
    }
    let first = cooldown_now(&mut client).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_millis(1_100);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    let second = cooldown_now(&mut client).await?;
    if second <= first {
        return Err(format!("step=cooldowns;observed=clock-{first}-then-{second}").into());
    }
    logout(&mut client).await?;
    let mut again = connect_world(clients, 0xd2).await?;
    let (_, persisted) = query(&mut again).await?;
    if persisted != chosen {
        return Err(format!("step=persist;observed={persisted};need={chosen}").into());
    }
    logout(&mut again).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_rejects_a_short_reply_and_reads_a_valid_one() {
        assert!(titles(&[0; 10]).is_err());
        let mut reply = vec![0u8; 70];
        reply[69] = 1;
        reply.extend_from_slice(&[7, 7]);
        assert_eq!(titles(&reply).ok(), Some((vec![7], 7)));
    }

    #[test]
    fn needed_token_has_no_spaces() {
        assert!(NEEDED.starts_with("needed:") && !NEEDED.contains(' '));
        assert_eq!(token("a\nb"), "a-b");
    }
}
