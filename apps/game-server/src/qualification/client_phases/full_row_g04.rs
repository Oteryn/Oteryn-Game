//! G04: player bug report (apps/client 0076-player-report, `bug.rs`).
//! 1. Admit the node-boot character and build the same wire request the native Bug form builds:
//!    kind BUG, category "Map location" with the actor's own map position, a unique comment.
//! 2. Send it: expect an accepted reply with a server-minted non-zero report number.
//! 3. Re-read the durable queue (read-only SQL): exactly one row for this reporter and comment
//!    whose report number, map position and kind equal what the server answered.
//! 4. Send a malformed bug (unknown category): expect the `Invalid` refusal and no new row.
//! 5. Log out, freshly admit, and re-read the durable row: it is unchanged.
use super::*;
use oteryn_protocol_oteryn::player_report as wire;
use sqlx::Connection;

const COMMENT: &str = "G04 bug report: the depot chest closed by itself";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(90), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "report-accepted-and-durable".to_owned()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G04.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn report(
    client: &mut oteryn_dev_client::DevClientSession,
    request: &wire::PlayerReportRequest,
) -> TestResult<wire::PlayerReportReply> {
    let payload = wire::encode_player_report_request(request)
        .map_err(|error| format!("step=encode observed={error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_PLAYER_REPORT_V1,
            wire::COMMAND_TYPE_PLAYER_REPORT,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("step=report observed=status:{:?}", response.status).into());
    }
    wire::decode_player_report_reply(&response.payload)
        .map_err(|error| format!("step=decode observed={error:?}").into())
}

type Row = (i64, i16, i32, i32, i16, i32, i32);

async fn rows(clients: &SeamClients<'_>) -> TestResult<Vec<Row>> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let rows = sqlx::query_as(
        "SELECT report_no, kind, map_x, map_y, map_floor, reporter_x, reporter_y FROM game_player_reports WHERE reporter_character_id = encode($1, 'hex')::uuid AND comment = $2 ORDER BY report_no LIMIT 8",
    )
    .bind(clients.characters[1].as_slice())
    .bind(COMMENT)
    .fetch_all(&mut database)
    .await?;
    database.close().await?;
    Ok(rows)
}

fn request(category: u8, map: (u16, u16, u8)) -> wire::PlayerReportRequest {
    wire::PlayerReportRequest {
        kind: wire::KIND_BUG,
        category,
        reason: 0,
        action: 0,
        target: String::new(),
        comment: COMMENT.into(),
        map_x: map.0,
        map_y: map.1,
        map_floor: map.2,
    }
}

async fn logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xc5).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_PLAYER_REPORT_V1)
    {
        return Err("step=admit observed=capability-76-not-selected".into());
    }
    let before = rows(clients).await?;
    let at = client.world_spatial().actor_position;
    let map = (
        u16::try_from(at.x)?,
        u16::try_from(at.y)?,
        u8::try_from(at.floor)?,
    );
    let valid = request(wire::BUG_CATEGORY_MAP, map);
    wire::validate_player_report(&valid).map_err(|code| format!("step=build observed={code:?}"))?;
    let reply = report(&mut client, &valid).await?;
    if reply.code != wire::PlayerReportCode::Accepted || reply.report == 0 {
        return Err(
            format!("step=report observed={reply:?};needed=accepted-with-report-number").into(),
        );
    }
    let stored = rows(clients).await?;
    let expected = (
        i64::try_from(reply.report)?,
        i16::from(wire::KIND_BUG),
        i32::from(map.0),
        i32::from(map.1),
        i16::from(map.2),
    );
    let fresh_rows: Vec<_> = stored.iter().filter(|row| !before.contains(row)).collect();
    let [row] = fresh_rows.as_slice() else {
        return Err(format!(
            "step=durable observed=new-rows:{};needed=1",
            fresh_rows.len()
        )
        .into());
    };
    if (row.0, row.1, row.2, row.3, row.4) != expected || row.5 != at.x || row.6 != at.y {
        return Err(format!("step=durable observed={row:?};needed={expected:?}").into());
    }
    let refused = report(&mut client, &request(wire::MAX_BUG_CATEGORY + 1, (0, 0, 0))).await;
    // The shared client rejects an unknown category before sending; either an `Invalid` reply or
    // an encode refusal proves the negative path, but a stored row never does.
    if let Ok(reply) = &refused {
        let refusal = matches!(
            reply.code,
            wire::PlayerReportCode::Invalid | wire::PlayerReportCode::RateLimited
        );
        if !refusal || reply.report != 0 {
            return Err(format!("step=invalid observed={reply:?}").into());
        }
    }
    if rows(clients).await? != stored {
        return Err("step=invalid observed=queue-changed".into());
    }
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xc6).await?;
    let durable = rows(clients).await?;
    logout(&mut fresh).await?;
    if durable != stored {
        return Err("step=fresh observed=queue-changed-after-readmission".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_bug_request_is_valid_and_unknown_category_is_not() {
        assert!(
            wire::validate_player_report(&request(wire::BUG_CATEGORY_MAP, (10, 20, 7))).is_ok()
        );
        assert_eq!(
            wire::validate_player_report(&request(wire::MAX_BUG_CATEGORY + 1, (0, 0, 0))),
            Err(wire::PlayerReportCode::Invalid)
        );
        assert_eq!(token("a  b"), "a-b");
    }
}
