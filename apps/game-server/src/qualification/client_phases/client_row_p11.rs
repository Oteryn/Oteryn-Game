//! P11 client replay capture: the real typed account-achievements page 0 and local codec
//! refusal of malformed query bytes, without a wire submission. The consumer is
//! `Session::account_achievements` in `crates/session/src/account_achievements.rs`.
use super::*;
use oteryn_dev_client::client_runtime::account_achievements::Response;
use oteryn_protocol_oteryn::account_achievements as wire;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "P11_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x2f).await?;
    let Response::Page(page) = client.session_mut().account_achievements(0).await? else {
        return Err("page query observed=refused".into());
    };
    // The typed core API never emits malformed query bytes. Exercise its codec
    // boundary explicitly; do not label this local check as a server rejection.
    let next_command = client.next_command_id();
    let sequence = client.last_server_sequence();
    if wire::decode_account_achievements_query(&[0x08]).is_ok()
        || client.next_command_id() != next_command
        || client.last_server_sequence() != sequence
    {
        return Err("malformed query was not refused locally without submission".into());
    }
    let reread = client.session_mut().account_achievements(0).await?;
    if reread != Response::Page(page.clone()) {
        return Err(format!("local malformed refusal changed the page: {reread:?}").into());
    }
    // The typed core owner has decoded the real reply; encode that exact value,
    // as other typed capture owners do, for the session replay consumer.
    capture(
        "p11-page.bin",
        &wire::encode_account_achievements_result(&page)
            .map_err(|error| format!("page capture encode: {error:?}"))?,
    );
    let rows: Vec<String> = page
        .rows
        .iter()
        .map(|row| format!("{}:{}:{}", row.key, row.grade, row.points))
        .collect();
    let expected = format!(
        "page={}\ntotal_points={}\nfact_count={}\nhas_more={}\nrows={}\nmalformed_query=local-codec-refusal\nmalformed_wire_submission=false\n",
        page.page,
        page.total_points,
        page.fact_count,
        page.has_more,
        rows.join("|")
    );
    capture("p11-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match tokio::time::timeout(Duration::from_secs(120), journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P11.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P11.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P11.client status=FAIL evidence=timeout"),
    }
}
