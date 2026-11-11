//! S11 client replay capture: query the real ranking snapshot as an ordinary player and capture
//! the raw page payload the server sent in its `CommandResult` (`Session::gated_command`), which
//! the native Highscores panel consumes through `apply_result`
//! (`crates/client-ui/src/features/0009-highscores/protocol.rs`).
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::highscores::{self, wire};
use oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn exchange(
    client: &mut DevClientSession,
    query: wire::HighscoresQuery,
) -> TestResult<ServerMechanicResponse> {
    let command =
        highscores::command(query).map_err(|error| format!("ranking input: {error:?}"))?;
    Ok(client
        .session_mut()
        .gated_command(command.capability, command.command_type, &command.payload)
        .await?)
}

fn row_text(row: &wire::HighscoreRow) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        row.rank, row.name, row.vocation, row.level, row.value
    )
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_world(clients, 0x40).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let query = wire::HighscoresQuery {
        world: None,
        details: false,
        category: 1,
        vocation: 0,
        page: 1,
        snapshot_id: 0,
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let accepted = loop {
        let reply = exchange(&mut client, query).await?;
        if reply.status == CommandStatus::Accepted {
            break reply;
        }
        // The durable root can be briefly busy while the snapshot job publishes: retry only an
        // explicit empty refusal, within a fixed deadline.
        if reply.status != CommandStatus::Rejected
            || !reply.payload.is_empty()
            || tokio::time::Instant::now() >= deadline
        {
            return Err(format!("step=query observed={reply:?}").into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    };
    let page = wire::decode_highscores_result(&accepted.payload)
        .map_err(|error| format!("step=decode observed={error:?}"))?;
    if page.category != 1 || page.page != 1 || page.snapshot_id == 0 || page.rows.len() > wire::MAX_HIGHSCORES_PAGE_ROWS {
        return Err(format!("step=identity observed={page:?}").into());
    }
    // Category 13 is reserved on the wire: the server must refuse it with an empty payload.
    let refused = exchange(
        &mut client,
        wire::HighscoresQuery {
            world: None,
            details: false,
            category: 13,
            ..query
        },
    )
    .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("step=refusal observed={refused:?}").into());
    }
    let mut expected = format!(
        "category={}\nvocation=0\npage={}\nsnapshot_id={}\ncomputed_at={}\nhas_more={}\nrows={}\n",
        page.category,
        page.page,
        page.snapshot_id,
        page.computed_at,
        u8::from(page.has_more),
        page.rows.len()
    );
    for (index, row) in page.rows.iter().enumerate() {
        expected.push_str(&format!("row{index}={}\n", row_text(row)));
    }
    expected.push_str(&format!(
        "own={}\nrefused={:?}\n",
        page.own_row.as_ref().map_or_else(|| "none".into(), row_text),
        refused.status
    ));
    capture("s11-page1.bin", &accepted.payload)?;
    capture("s11-expected.txt", expected.as_bytes())?;

    Ok(format!(
        "snapshot={} rows={} own={}",
        page.snapshot_id,
        page.rows.len(),
        page.own_row.is_some()
    ))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S11.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S11.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S11.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranking_evidence_keeps_full_numeric_width() {
        let row = wire::HighscoreRow {
            rank: u64::MAX,
            name: "name".into(),
            vocation: "knight".into(),
            level: u32::MAX,
            value: 0,
        };
        assert_eq!(
            row_text(&row),
            "18446744073709551615|name|knight|4294967295|0"
        );
    }
}
