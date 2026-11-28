//! S11 client replay capture: query the real ranking snapshot as an ordinary player and capture
//! the original command and result envelopes, which
//! the native Highscores panel consumes through `apply_result`
//! (`crates/client-ui/src/features/0009-highscores/protocol.rs`).
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::highscores::{self, wire};
use oteryn_protocol_oteryn::{
    FoundationProtocolError, decode_command_result, decode_wire_envelope,
};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        std::fs::write(Path::new(&root).join(name), bytes)?;
    }
    Ok(())
}

async fn exchange(
    client: &mut DevClientSession,
    label: &str,
    query: wire::HighscoresQuery,
) -> TestResult<(CommandStatus, u32, Vec<u8>)> {
    let command =
        highscores::command(query).map_err(|error| format!("ranking input: {error:?}"))?;
    let exchange = client
        .session_mut()
        .capture_highscores_exchange(&command.payload)
        .await?;
    let request = decode_wire_envelope(&exchange.request_frame)?;
    let response = decode_wire_envelope(&exchange.result_frame)?;
    let result = decode_command_result(response.payload())?;
    let session = uuid_text(exchange.game_session_id.as_bytes());
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = Path::new(&root);
        let envelope = serde_json::json!({
            "command_id": result.command_id,
            "game_session_id": session,
            "payload": result.payload,
            "server_sequence": response.server_sequence(),
            "status": result.status as u32,
        });
        let wire_proof = serde_json::json!({
            "run": root.file_name().and_then(|name| name.to_str()).ok_or("S11 capture directory has no run name")?,
            "session": session,
            "generation": request.connection_generation(),
            "request": exchange.request_frame,
            "result": exchange.result_frame,
        });
        std::fs::write(root.join(format!("s11-{label}.json")), serde_json::to_vec(&envelope)?)?;
        std::fs::write(root.join(format!("s11-{label}-wire.json")), serde_json::to_vec(&wire_proof)?)?;
    }
    Ok((result.status, exchange.error_code, result.payload.to_vec()))
}

fn row_text(row: &wire::HighscoreRow) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        row.rank, row.name, row.vocation, row.level, row.value
    )
}

async fn experience_facts(
    clients: &SeamClients<'_>,
) -> TestResult<(Vec<wire::HighscoreRow>, wire::HighscoreRow, u32, bool)> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let rows = sqlx::query_as::<_, (String, String, i64, i64, i64, bool)>(
        r#"WITH facts AS (
             SELECT r.character_id, r.name, b.vocation,
                    COALESCE(p.level, birth.level)::bigint AS level,
                    COALESCE(p.total_experience, birth.total_experience)::bigint AS total_experience
             FROM game_character_roots r
             JOIN game_character_build_state b USING (character_id)
             LEFT JOIN game_character_progression_state p USING (character_id)
             LEFT JOIN game_character_creation_progression birth
                 ON birth.character_id = r.character_id AND r.character_revision = 1
             WHERE r.world_id = $1::uuid AND r.lifecycle = 1 AND b.vocation <> 'none'
               AND NOT game_account_closure_detached(r.character_id)
           )
           SELECT name, vocation, level, total_experience,
                  rank() OVER (ORDER BY total_experience DESC), character_id = $2::uuid
           FROM facts ORDER BY total_experience DESC, name COLLATE "C", character_id
           LIMIT 51"#,
    )
    .bind(uuid_text(&clients.world))
    .bind(uuid_text(&clients.characters[1]))
    .fetch_all(&mut connection)
    .await?;
    let bound = wire::MAX_HIGHSCORES_PAGE_ROWS;
    let mut own = None;
    let mut own_page = None;
    let mut expected = Vec::new();
    let has_more = rows.len() > bound;
    for (position, (name, vocation, level, value, rank, is_own)) in rows.into_iter().enumerate() {
        let row = wire::HighscoreRow {
            rank: rank.try_into()?,
            name,
            vocation,
            level: level.try_into()?,
            value: value.try_into()?,
        };
        if is_own {
            own_page = Some(u32::try_from(position / bound + 1)?);
            own = Some(row.clone());
        }
        if position < bound {
            expected.push(row);
        }
    }
    Ok((
        expected,
        own.ok_or("S11 qualification player lacks persisted ranking inputs")?,
        own_page.ok_or("S11 qualification player lacks rank position")?,
        has_more,
    ))
}

async fn publish_ranking(clients: &SeamClients<'_>) -> TestResult<(u64, i64)> {
    use crate::durability::highscore_snapshot::{CutLimits, SnapshotCatalogues};
    let root = crate::durability::DurabilityRoot::connect_test_runtime(clients.url)?;
    root.await_ready().await?;
    let achievements = crate::achievement_catalogue::AchievementCatalogue::embedded()
        .map_err(|error| format!("S11 achievement catalogue: {error:?}"))?;
    let catalogues = SnapshotCatalogues::activated(std::sync::Arc::new(achievements))
        .map_err(|error| format!("S11 ranking catalogues: {error:?}"))?;
    let snapshots = root
        .refresh_highscore_world_within(
            wire::WorldId::decode(&clients.world)?,
            catalogues,
            CutLimits::REGISTERED,
        )
        .await
        .map_err(|error| format!("S11 ranking cut publication: {error:?}"))?;
    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.category == 1)
        .ok_or("S11 publication lacks the Experience cut")?;
    Ok((snapshot.snapshot_id, snapshot.computed_at_ms))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x40).await?;
    let query = wire::HighscoresQuery {
        world: None,
        details: true,
        category: 1,
        vocation: 0,
        page: 1,
        snapshot_id: 0,
    };
    let (expected_rows, expected_own, expected_own_page, expected_has_more) =
        experience_facts(clients).await?;
    let (status, _, before) = exchange(&mut client, "before", query).await?;
    require_mechanic_acceptance(status, "highscore-before-query")?;
    let before = wire::decode_highscores_result(&before)
        .map_err(|error| format!("step=before-decode observed={error:?}"))?;
    if before.rows == expected_rows && before.own_row.as_ref() == Some(&expected_own) {
        return Err("S11 expected ranking already present before publication".into());
    }
    let (published, computed_at) = publish_ranking(clients).await?;
    if published <= before.snapshot_id {
        return Err("S11 publication did not advance the Experience cut".into());
    }
    let (status, _, accepted) = exchange(&mut client, "experience", query).await?;
    require_mechanic_acceptance(status, "highscore-query")?;
    let page = wire::decode_highscores_result(&accepted)
        .map_err(|error| format!("step=decode observed={error:?}"))?;
    if page.category != 1
        || page.page != 1
        || page.snapshot_id != published
        || page.computed_at != computed_at
        || page.world != Some(wire::WorldId::decode(&clients.world)?)
        || page.own_page != Some(expected_own_page)
        || page.has_more != expected_has_more
        || page.rows != expected_rows
        || page.own_row.as_ref() != Some(&expected_own)
    {
        return Err(format!("step=persisted-ranking expected_rows={} expected_own={} observed_rows={} observed_own={} snapshot={}", expected_rows.len(), row_text(&expected_own), page.rows.len(), page.own_row.is_some(), page.snapshot_id).into());
    }
    // Global lists Goshnar's Taint and Loyalty as supported Highscores categories.
    // Probe unregistered wire category 18. https://tibia.fandom.com/wiki/Highscores
    let (refused, error_code, refusal_payload) = exchange(
        &mut client,
        "refused",
        wire::HighscoresQuery {
            world: None,
            details: true,
            category: 18,
            ..query
        },
    )
    .await?;
    if refused != CommandStatus::Rejected
        || !refusal_payload.is_empty()
        || error_code != FoundationProtocolError::NotSupported.code()
    {
        return Err(format!("step=refusal observed={refused:?} code={error_code}").into());
    }
    let (status, _, loyalty) = exchange(
        &mut client,
        "loyalty",
        wire::HighscoresQuery {
            category: 13,
            ..query
        },
    )
    .await?;
    require_mechanic_acceptance(status, "highscore-loyalty-query")?;
    let loyalty = wire::decode_highscores_result(&loyalty)
        .map_err(|error| format!("step=loyalty-decode observed={error:?}"))?;
    if loyalty.category != 13
        || loyalty.world != page.world
        || loyalty.page != 1
        || loyalty.snapshot_id == 0
        || loyalty.rows.len() > wire::MAX_HIGHSCORES_PAGE_ROWS
    {
        return Err(format!("step=loyalty-details observed={loyalty:?}").into());
    }
    let session = client.session_mut().join_snapshot().game_session_id;
    let mut expected = format!(
        "game_session_id={}\nworld={}\nown_page={}\ncategory={}\nvocation=0\npage={}\nsnapshot_id={}\ncomputed_at={}\nhas_more={}\nrows={}\n",
        uuid_text(session.as_bytes()),
        uuid_text(&clients.world),
        expected_own_page,
        page.category,
        page.page,
        page.snapshot_id,
        computed_at,
        u8::from(page.has_more),
        page.rows.len()
    );
    for (index, row) in expected_rows.iter().enumerate() {
        expected.push_str(&format!("row{index}={}\n", row_text(row)));
    }
    expected.push_str(&format!(
        "own={}\nrefused={:?}\n",
        row_text(&expected_own),
        refused
    ));
    let (logout, observed, _, _) = logout_once(&mut client).await?;
    if logout != Some(LogoutResult::Accepted) {
        return Err(format!("step=logout observed={observed}").into());
    }
    drop(client);
    let mut rejoined = connect_world(clients, 0x41).await?;
    let relog_session = rejoined.session_mut().join_snapshot().game_session_id;
    if relog_session == session {
        return Err("S11 relog reused the original session".into());
    }
    let (status, _, after) = exchange(&mut rejoined, "relog", query).await?;
    require_mechanic_acceptance(status, "highscore-relog-query")?;
    let after = wire::decode_highscores_result(&after)
        .map_err(|error| format!("step=relog-decode observed={error:?}"))?;
    if after != page {
        return Err("S11 persisted ranking changed after relog".into());
    }
    let (logout, observed, _, _) = logout_once(&mut rejoined).await?;
    if logout != Some(LogoutResult::Accepted) {
        return Err(format!("step=relog-logout observed={observed}").into());
    }
    expected.push_str(&format!("relog_session={}\n", uuid_text(relog_session.as_bytes())));
    capture("s11-expected.txt", expected.as_bytes())?;
    Ok(format!(
        "snapshot={} rows={} own={}",
        page.snapshot_id,
        page.rows.len(),
        page.own_row.is_some()
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients)))
        .await
    {
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
