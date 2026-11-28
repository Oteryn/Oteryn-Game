//! P12c client replay capture: real CHARACTER_INFO QuerySelf replies (full and paged) and the
//! refused QueryVisible of another character. The consumer is `apply` in
//! `apps/client/src/features/0097-character-info/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::character_info_panel as info;

fn frame(
    reply: &oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse,
) -> serde_json::Value {
    serde_json::json!({
        "command_id": reply.command_id,
        "server_sequence": reply.server_sequence,
        "status": format!("{:?}", reply.status),
        "payload": reply.payload,
    })
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

async fn expected_self(client: &DevClientSession, clients: &SeamClients<'_>) -> TestResult<String> {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let (level, experience): (i64, i64) = sqlx::query_as(
        "SELECT level, total_experience FROM game_character_progression_state
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_one(&mut db)
    .await?;
    let inventory = client
        .inventory()
        .ok_or("P12c independent inventory absent")?;
    let carried = usize::from(inventory.main_backpack.is_some())
        + inventory.equipment.len()
        + inventory.entries.len();
    Ok(format!("values={level},{experience},0,0,{carried},0,0,0"))
}

async fn authorize_foreign_fixture(clients: &SeamClients<'_>) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let result = sqlx::query(
        "WITH list AS (
            INSERT INTO game_vip_lists(account_id,world_id,next_entry_no)
            SELECT account_id,world_id,2 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
            RETURNING account_id,world_id
        ), entry AS (
            INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no)
            SELECT account_id,world_id,encode($2,'hex')::uuid,1 FROM list
            RETURNING account_id,world_id,character_id
        )
        INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at)
        SELECT account_id,world_id,encode($1,'hex')::uuid,character_id,2,now()+interval '1 day' FROM entry",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.characters[1].as_slice())
    .execute(&mut db)
    .await?;
    if result.rows_affected() != 1 {
        return Err("P12c consent fixture did not provision one grant".into());
    }
    evidence("P12C_FIXTURE scope=foreign-character-info friendship=consented");
    Ok(())
}

fn confirm_summary(
    reply: &oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse,
    expected: &str,
    step: &str,
) -> TestResult {
    if reply.status != CommandStatus::Accepted {
        return Err(format!("P12c {step} refused: {reply:?}").into());
    }
    let observed = summary(&reply.payload)?;
    if observed != expected {
        return Err(format!("P12c {step}: expected {expected}, observed {observed}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x31, 0).await?;
    let session_id = client.join_snapshot().game_session_id;
    let session = uuid_text(session_id.as_bytes());
    let self_query =
        info::CharacterInfoOperation::QuerySelf(info::PageRequest::new(0, 50).ok_or("full page")?);
    let empty = format!(
        "{}|deaths=0|kills=0",
        expected_self(&client, clients).await?
    );
    confirm_summary(
        &send(&mut client, self_query).await?,
        &empty,
        "pre-death self",
    )?;

    let world = full_e2ef7::qualification_world(clients)?;
    let world_bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&world_bytes, |_, _| Ok(()))?;
    full_row_p12c::die_to_spider(&mut client, world, &metadata.spawns).await?;

    let self_values = expected_self(&client, clients).await?;
    let full = format!("{self_values}|deaths=1|kills=0");
    let paged = format!("{self_values}|deaths=0|kills=0");
    let query = send(&mut client, self_query).await?;
    confirm_summary(&query, &full, "full self after death")?;
    let page_query =
        info::CharacterInfoOperation::QuerySelf(info::PageRequest::new(1, 1).ok_or("offset page")?);
    let page = send(&mut client, page_query).await?;
    confirm_summary(&page, &paged, "offset one after death")?;

    let target = clients.characters[1];
    let foreign = info::CharacterInfoOperation::QueryVisible {
        character: target,
        page: info::PageRequest::new(0, 1).ok_or("foreign page")?,
    };
    let refused = send(&mut client, foreign).await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("QueryVisible before consent: {refused:?}").into());
    }
    authorize_foreign_fixture(clients).await?;
    let visible = send(&mut client, foreign).await?;
    let target_summary = "values=1,0,0,0,15,0,0,0|deaths=0|kills=0";
    confirm_summary(&visible, target_summary, "same foreign query after consent")?;

    full_row_p12c::logout(&mut client).await?;
    let mut fresh = connect_character(clients, 0x32, 0).await?;
    let fresh_id = fresh.join_snapshot().game_session_id;
    if fresh_id == session_id {
        return Err("P12c replay reused the original session".into());
    }
    let durable = send(&mut fresh, self_query).await?;
    confirm_summary(&durable, &full, "fresh-session history")?;
    full_row_p12c::logout(&mut fresh).await?;

    let target: String = target.iter().map(|byte| format!("{byte:02x}")).collect();
    let expected = format!(
        "query={full}\npaged={paged}\nvisible_target={target}\nrefused_status=Rejected\nvisible={target_summary}\n",
    );
    let frames = [
        frame(&query),
        frame(&page),
        frame(&refused),
        frame(&visible),
        frame(&durable),
    ];
    let root = if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        std::path::PathBuf::from(root)
    } else {
        let root = std::env::temp_dir().join(format!("p12c-live-{session}"));
        std::fs::create_dir(&root)?;
        root
    };
    let capture = serde_json::json!({
        "session": session,
        "relog_session": uuid_text(fresh_id.as_bytes()),
        "frames": frames,
        "expected": expected,
    });
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("p12c-capture.json"))?;
    serde_json::to_writer(file, &capture)?;
    evidence(&format!(
        "P12C_CLIENT_CAPTURE session={session} last_command={} last_sequence={}",
        visible.command_id, visible.server_sequence
    ));

    let binary = std::env::var_os("E2E_P12C_REPLAY_BIN");
    let output = tokio::task::spawn_blocking(move || {
        let mut command = if let Some(binary) = binary {
            let mut command = std::process::Command::new(binary);
            command.arg("e2e_p12c_replays_paged_summary_and_refused_visible_query");
            command
        } else {
            let mut cargo = std::process::Command::new("cargo");
            cargo
                .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
                .args([
                    "test",
                    "--locked",
                    "-p",
                    "oteryn-client",
                    "--lib",
                    "e2e_p12c_replays_paged_summary_and_refused_visible_query",
                    "--",
                ]);
            cargo
        };
        command
            .args(["--ignored", "--nocapture"])
            .env("E2E_ALL_CAPTURE_DIR", root)
            .env("E2E_P12C_CAPTURE_SESSION", session)
            .output()
    })
    .await??;
    let stdout = String::from_utf8(output.stdout)?;
    let result = stdout
        .lines()
        .find(|line| line.starts_with("test result: ok. 1 passed; 0 failed;"));
    if !output.status.success()
        || !stdout.lines().any(|line| {
            line.starts_with("test ")
                && line
                    .ends_with("::e2e_p12c_replays_paged_summary_and_refused_visible_query ... ok")
        })
    {
        return Err(format!(
            "P12c client replay failed: {stdout} {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    evidence(&format!(
        "P12C_CLIENT_REPLAY {}",
        result.ok_or("P12c replay result absent")?
    ));

    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P12c.client status=PASS evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P12c.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P12c.client status=FAIL evidence=timeout"),
    }
}
