//! W10 client replay capture: the real Hazard query reply (`[unlocked, selected]`), the level-1
//! selection reply and the locked level-2 refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0117-boss-encounter/protocol.rs` (hazard summary in
//! `protocol/auxiliary.rs`).
use super::*;
use oteryn_dev_client::client_runtime::boss_encounter as wire;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "W10_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

async fn hazard(
    client: &mut oteryn_dev_client::DevClientSession,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BOSS_ENCOUNTER_V1,
            wire::hazard::COMMAND_TYPE_HAZARD,
            payload,
        )
        .await?)
}

fn selected_hazard(before: &[u8], after: &[u8], requested: u8) -> bool {
    matches!((before, after), ([unlocked_before, selected_before], [unlocked_after, selected_after])
        if unlocked_before == unlocked_after
            && *selected_before != requested
            && *selected_after == requested)
}

#[test]
fn hazard_select_must_change_to_the_requested_level() {
    assert!(!selected_hazard(&[2, 0], &[2, 0], 1));
    assert!(!selected_hazard(&[2, 0], &[2, 2], 1));
    assert!(selected_hazard(&[2, 2], &[2, 1], 1));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x05).await?;
    let mut query = hazard(&mut client, &[]).await?;
    let [unlocked, current] = query.payload.as_slice() else {
        return Err(format!("hazard query malformed: {query:?}").into());
    };
    if query.status != CommandStatus::Accepted
        || !(2..12).contains(unlocked)
        || !(1..=*unlocked).contains(current)
    {
        return Err(format!(
            "hazard fixture needs both a second unlocked level and a locked level: {query:?}"
        )
        .into());
    }
    let (unlocked, current) = (*unlocked, *current);
    if current == 1 {
        let setup = hazard(&mut client, &[2]).await?;
        if setup.status != CommandStatus::Accepted
            || !selected_hazard(&query.payload, &setup.payload, 2)
        {
            return Err(
                format!("hazard setup selection did not change to level 2: {setup:?}").into(),
            );
        }
        query = hazard(&mut client, &[]).await?;
        if query.status != CommandStatus::Accepted || query.payload != [unlocked, 2] {
            return Err(format!("hazard setup selection not reflected in query: {query:?}").into());
        }
    }
    let select = hazard(&mut client, &[1]).await?;
    let locked = hazard(&mut client, &[unlocked + 1]).await?;
    if select.status != CommandStatus::Accepted || select.payload.len() != 2 {
        return Err(format!("hazard select reply={select:?}").into());
    }
    if !selected_hazard(&query.payload, &select.payload, 1) {
        return Err(format!("hazard selection did not change to level 1: {select:?}").into());
    }
    if locked.status != CommandStatus::Rejected || !locked.payload.is_empty() {
        return Err(format!("locked hazard level was not refused: {locked:?}").into());
    }
    let confirmed = hazard(&mut client, &[]).await?;
    if confirmed.status != CommandStatus::Accepted || confirmed.payload != select.payload {
        return Err(format!("confirmed hazard state differs from selection: {confirmed:?}").into());
    }
    capture("w10-query.bin", &query.payload);
    capture("w10-select.bin", &select.payload);
    let expected = format!(
        "query_unlocked={}\nquery_selected={}\nselect_unlocked={}\nselect_selected={}\n",
        query.payload[0], query.payload[1], select.payload[0], select.payload[1]
    );
    capture("w10-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W10.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W10.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W10.client status=FAIL evidence=timeout"),
    }
}
