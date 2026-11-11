//! E12 client replay capture: List the owned hirelings, then request an outfit the hireling is not
//! entitled to and capture the server refusal. Client owner: the Hireling management panel in
//! `crates/client-ui/src/features/0250-hireling-management`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::hireling::management;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn command(
    client: &mut DevClientSession,
    request: management::Request,
) -> TestResult<(management::Reply, Vec<u8>)> {
    let response = client
        .session_mut()
        .gated_command(
            management::CAPABILITY_HIRELING_MANAGEMENT_V1,
            management::COMMAND_TYPE_HIRELING_MANAGEMENT,
            &request.encode(),
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("hireling transport status {:?}", response.status).into());
    }
    let reply = management::Reply::decode(&response.payload).ok_or("hireling reply undecodable")?;
    Ok((reply, response.payload))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    let mut client = connect_world(clients, 0x3c).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    if !client
        .selected_capabilities()
        .contains(&management::CAPABILITY_HIRELING_MANAGEMENT_V1)
    {
        return Err("capability 250 not selected".into());
    }
    let (listed, list_bytes) = command(&mut client, management::Request::List).await?;
    let management::Reply::State(entries) = listed else {
        return Err("list reply is not State".into());
    };
    let entry = entries
        .first()
        .cloned()
        .ok_or("no owned hireling: needs a player-obtainable owned house and hireling lamp")?;
    // Dragon is the last entitlement bit; request it only when it is not owned.
    if entry.outfits & (1 << 8) != 0 {
        return Err("hireling owns every outfit; no refusal path to capture".into());
    }
    let look_type: u16 = if entry.sex == 2 { 1125 } else { 1126 };
    let (refused, refused_bytes) = command(
        &mut client,
        management::Request::Outfit {
            hireling: entry.hireling,
            look_type,
            colors: [1, 2, 3, 4],
        },
    )
    .await?;
    if refused != management::Reply::Refused(management::Refusal::OutfitNotOwned) {
        return Err(format!("outfit refusal reply {refused:?}").into());
    }
    let placed = entry
        .position
        .map_or_else(|| "none".to_owned(), |(x, y, z)| format!("{x},{y},{z}"));
    let expected = format!(
        "entries={}\nhireling={}\nname={}\nsex={}\nlook_type={}\ncolors={:?}\noutfits={}\nskills={}\nplaced={placed}\nrefused_look={look_type}\nrefusal=OutfitNotOwned\n",
        entries.len(),
        hex(&entry.hireling),
        entry.name,
        entry.sex,
        entry.look_type,
        entry.colors,
        entry.outfits,
        entry.skills
    );
    capture("e12-list.bin", &list_bytes)?;
    capture("e12-refused.bin", &refused_bytes)?;
    capture("e12-expected.txt", expected.as_bytes())?;

    Ok(format!(
        "hireling={} look_type={} refusal=OutfitNotOwned",
        hex(&entry.hireling),
        entry.look_type
    ))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E12.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E12.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E12.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_preserves_zero_padding_and_high_bytes() {
        assert_eq!(hex(&[]), "");
        assert_eq!(hex(&[0, 15, 128, 255]), "000f80ff");
    }
}
