//! S05 client replay capture: the real Thais House catalogue pages and a guildhall bid refusal
//! as an ordinary player without guild-leader authority. The driver writes the raw page and
//! result payloads (`Session::gated_command`) that the native House panel replays
//! (`tools/dev-client/src/tests/e2e_s05.rs`). It changes no ownership.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::house;
use oteryn_dev_client::client_runtime::raw_feature::FeatureCommand;
use oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse;

const TOWN_KEY: &str = "oteryn:content.area.city.thais";

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn send(
    client: &mut DevClientSession,
    command: FeatureCommand,
) -> TestResult<ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?)
}

fn row_line(row: &house::Row) -> String {
    let optional = |value: Option<u64>| value.map_or_else(|| "-".to_owned(), |n| n.to_string());
    format!(
        "row\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
        row.house,
        row.kind,
        row.state,
        row.rent,
        row.price,
        row.revision,
        row.acl_revision,
        u8::from(row.owned),
        optional(row.maximum),
        optional(row.ends),
        optional(row.notice)
    )
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let world = *WorldId::decode(&clients.world)?.as_bytes();
    let mut client = connect_character(clients, 0x44, 1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let result: TestResult<String> = async {
            let mut cursor: Option<String> = None;
            let mut first: Option<(Vec<u8>, house::Page)> = None;
            let mut guildhall: Option<(Vec<u8>, house::Page)> = None;
            loop {
                let request = house::query(world, TOWN_KEY, cursor.as_deref())
                    .map_err(|error| format!("query encode: {error}"))?;
                let reply = send(&mut client, request).await?;
                if reply.status != CommandStatus::Accepted {
                    return Err(format!("step=catalogue reply {reply:?}").into());
                }
                let page = house::page(&reply.payload)
                    .map_err(|error| format!("step=catalogue payload: {error}"))?;
                cursor = page.cursor.clone();
                if guildhall.is_none() && page.rows.iter().any(|row| row.kind == 3) {
                    guildhall = Some((reply.payload.clone(), page.clone()));
                }
                if first.is_none() {
                    first = Some((reply.payload, page));
                }
                if cursor.is_none() {
                    break;
                }
            }
            let (page_bytes, page) = first.ok_or("step=catalogue no page")?;
            let (guild_bytes, guild_page) = guildhall.ok_or("step=catalogue no guildhall row")?;
            if page.rows.is_empty()
                || page
                    .rows
                    .iter()
                    .any(|row| row.rent == 0 || row.revision == 0)
            {
                return Err(format!(
                    "step=catalogue rows without rent or revision {:?}",
                    page.rows
                )
                .into());
            }
            let target = guild_page
                .rows
                .iter()
                .find(|row| row.kind == 3)
                .ok_or("step=refusal guildhall row vanished")?;
            let action = house::Action::Bid(target.price.max(target.rent).max(1));
            let request = house::mutation(world, &target.house, target.acl_revision, &action)
                .map_err(|error| format!("mutation encode: {error}"))?;
            let refusal = send(&mut client, request).await?;
            if refusal.status != CommandStatus::Accepted {
                return Err(format!("step=refusal reply {refusal:?}").into());
            }
            let (disposition, _) = house::result(&refusal.payload)
                .map_err(|error| format!("step=refusal payload: {error}"))?;
            if !(4..=10).contains(&disposition) {
                return Err(format!("step=refusal disposition {disposition}").into());
            }
            let mut expected: String = page.rows.iter().map(row_line).collect();
            expected.push_str(&format!("guildhall={}\n", target.house));
            capture("s05-page.bin", &page_bytes)?;
            capture("s05-guildhall-page.bin", &guild_bytes)?;
            capture("s05-refusal.bin", &refusal.payload)?;
            capture("s05-expected.txt", expected.as_bytes())?;
            Ok(format!(
                "rows={} guildhall={} disposition={disposition}",
                page.rows.len(),
                target.house
            ))
        }
        .await;

        result
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S05.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S05.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S05.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn house_evidence_distinguishes_absent_and_zero_optional_values() {
        let mut row = house::Row {
            name: String::new(),
            size: 0,
            beds: 0,
            entrance: None,
            house: "house".into(),
            state: 0,
            rent: 0,
            revision: u64::MAX,
            acl_revision: 0,
            owned: false,
            price: 0,
            notice: None,
            ends: None,
            maximum: None,
            kind: 0,
        };
        assert!(row_line(&row).ends_with("\t-\t-\t-\n"));
        row.maximum = Some(0);
        assert!(row_line(&row).ends_with("\t0\t-\t-\n"));
    }
}
