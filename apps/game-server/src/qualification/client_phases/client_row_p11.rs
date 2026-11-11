//! P11 client replay capture: the real account-achievements page 0 and the refusal of a
//! malformed query. The consumer is `Session::account_achievements` in
//! `crates/session/src/account_achievements.rs`.
use super::*;
use oteryn_protocol_oteryn::account_achievements::{
    self as wire, COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY,
};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_world(clients, 0x2f).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let page = match client.session_mut().account_achievements(0).await? {
        oteryn_dev_client::client_runtime::account_achievements::Response::Page(page) => page,
        oteryn_dev_client::client_runtime::account_achievements::Response::Refused => {
            return Err("page query was refused".into());
        }
    };
    let payload = wire::encode_account_achievements_result(&page)
        .map_err(|error| format!("page encode: {error:?}"))?;
    if payload.is_empty() {
        return Err("page query payload is empty".into());
    }
    capture("p11-page.bin", &payload)?;
    let refused = client
        .session_mut()
        .gated_command(0, COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY, &[0x08])
        .await?;
    if refused.status != CommandStatus::Rejected {
        return Err(format!("malformed query was not rejected: {refused:?}").into());
    }
    let rows: Vec<String> = page
        .rows
        .iter()
        .map(|row| format!("{}:{}:{}", row.key, row.grade, row.points))
        .collect();
    let expected = format!(
        "page={}\ntotal_points={}\nfact_count={}\nhas_more={}\nrows={}\nrefused_status=Rejected\n",
        page.page,
        page.total_points,
        page.fact_count,
        page.has_more,
        rows.join("|")
    );
    capture("p11-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P11.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P11.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P11.client status=FAIL evidence=timeout"),
    }
}
