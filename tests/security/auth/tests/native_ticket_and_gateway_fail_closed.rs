//! Real OAuth/Gateway refusal before grants, followed by one TLS Game admission.
mod support;
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::native_login::{
    AttemptRef, GatewayErrorCode, GatewayLogin, NativeLoginError,
};
use support::*;

async fn journey(t: Topology) -> TestResult {
    t.no_admissions()?;
    // Missing product routes are an upstream blocker, never a successful refusal.
    for (base, path, source) in [
        (
            "public",
            "/api/v1/game-auth/tickets",
            "portal-rs/crates/player-api/src/lib.rs:47",
        ),
        (
            "gateway",
            "/v1/login",
            "portal-rs/crates/gateway/src/lib.rs:54",
        ),
    ] {
        let response = reqwest::Client::new()
            .get(format!("{}{path}", text(&t.report["baseUrls"], base)?))
            .send()
            .await
            .map_err(|_| "route_probe_failed")?;
        if response.status() == 404 {
            return Err(format!("native_handler_missing:{source}").into());
        }
        check(
            response.status() == 405,
            "native_route_probe_unexpected_status",
        )?;
    }
    let account = &t.input.accounts[0].account_id;
    let before = sql(
        &t.database,
        "ticket_count",
        &format!("SELECT count(*) FROM game_login_tickets WHERE account_id='{account}';"),
    )?
    .parse::<u64>()
    .map_err(|_| "ticket_count_invalid")?;
    let native = t.native(text(&t.report["baseUrls"], "public")?)?;
    let token = t.oauth(0).await?;
    let ticket = native
        .issue_native_ticket(token, CancellationToken::new())
        .await
        .map_err(|_| "native_ticket_refused")?;
    check(sql(&t.database, "ticket_issued", &format!("SELECT count(*) FROM game_login_tickets WHERE account_id='{account}' AND audience='oteryn-native-game-gateway' AND used_at IS NULL AND attempt_ref IS NULL;"))?.parse::<u64>().map_err(|_| "ticket_count_invalid")? == before + 1, "ticket_not_persisted_for_oauth_account")?;
    // Deliberately absent character: this lane has no LCFA owner snapshot yet.
    // Prove absence independently, rather than pretending it is a selected character.
    let character = AttemptRef::generate().map_err(|_| "random_identifier_failed")?;
    check(
        sql(
            &t.database,
            "character_absent",
            &format!(
                "SELECT count(*) FROM account_characters WHERE character_id='{}';",
                character.as_str()
            ),
        )? == "0",
        "negative_character_exists",
    )?;
    let attempt = AttemptRef::generate().map_err(|_| "random_identifier_failed")?;
    let login = GatewayLogin {
        attempt_ref: &attempt,
        character_id: character.as_str(),
        channel_id: None,
        client_build: "game-platform-qualification",
    };
    // The live composition has LCFA enabled but no published snapshot: exact
    // protocol-2 503 ROUTE_UNAVAILABLE, parsed/validated by the shipping client.
    check(
        matches!(
            native
                .gateway_login(&ticket, &login, CancellationToken::new())
                .await,
            Err(NativeLoginError::Gateway(
                GatewayErrorCode::RouteUnavailable
            ))
        ),
        "gateway_wrong_refusal",
    )?;
    t.no_admissions()?;
    check(sql(&t.database, "redemption_rolled_back", &format!("SELECT count(*) FROM game_login_tickets WHERE account_id='{account}' AND used_at IS NULL AND attempt_ref IS NULL;"))?.parse::<u64>().map_err(|_| "ticket_count_invalid")? == before + 1, "failed_issue_consumed_ticket")?;
    println!("NODE_BOOT_EVIDENCE flow=native_ticket_gateway result=fail_closed game_admissions=0");
    Ok(())
}
#[test]
fn native_ticket_and_gateway_fail_closed() -> TestResult {
    run("native_ticket_gateway", 60, journey)
}

async fn admitted_journey(t: Topology) -> TestResult {
    use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES, CharacterId};
    use std::time::{Duration, Instant};
    t.no_admissions()?;
    let character = std::env::var("GAME_PLATFORM_CHARACTER_ID").map_err(|_| "character_missing")?;
    let character_bytes = oteryn_platform_client::native_login::parse_uuid_v7(&character)
        .ok_or("character_invalid")?;
    let route = &t.report["seed"]["route"];
    let world = text(route, "worldId")?;
    let channel = text(route, "channelId")?;
    check(
        oteryn_platform_client::native_login::parse_uuid_v7(world).is_some()
            && oteryn_platform_client::native_login::parse_uuid_v7(channel).is_some(),
        "route_scope_invalid",
    )?;
    let account = &t.input.accounts[1].account_id;
    check(
        sql(
            &t.game_database,
            "bootstrap_owner",
            &format!(
                "SELECT count(*) FROM game_character_roots WHERE character_id='{character}' AND account_id='{account}' AND world_id='{world}';"
            ),
        )? == "1",
        "game_character_binding_mismatch",
    )?;
    // Wait for Game's real outbox publisher and runtime reporter. Neither the
    // harness nor this test inserts projection, assignment-report or readiness rows.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let projected = sql(
            &t.database,
            "owner_projection",
            &format!(
                "SELECT count(*) FROM account_characters WHERE account_id='{account}' AND character_id='{character}' AND world_id='{world}' AND availability='AVAILABLE';"
            ),
        )? == "1";
        let ready = sql(
            &t.database,
            "runtime_report",
            &format!(
                "SELECT count(*) FROM native_runtime_status_reports WHERE world_id='{world}' AND channel_id='{channel}' AND invalid=false AND content_json::jsonb->>'ready'='true' AND observed_at >= extract(epoch FROM clock_timestamp())::bigint - 10;"
            ),
        )? == "1";
        if projected && ready {
            break;
        }
        check(
            Instant::now() < deadline,
            "owner_projection_or_runtime_not_ready",
        )?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let native = t.native(text(&t.report["baseUrls"], "public")?)?;
    let token = t.oauth(1).await?;
    let ticket = native
        .issue_native_ticket(token, CancellationToken::new())
        .await
        .map_err(|_| "positive_ticket_refused")?;
    let attempt = AttemptRef::generate().map_err(|_| "random_identifier_failed")?;
    let grant = native
        .gateway_login(
            &ticket,
            &GatewayLogin {
                attempt_ref: &attempt,
                character_id: &character,
                channel_id: Some(channel),
                client_build: "game-platform-qualification",
            },
            CancellationToken::new(),
        )
        .await
        .map_err(|error| match error {
            NativeLoginError::Gateway(code) => format!("positive_gateway_refused:{code:?}"),
            _ => "positive_gateway_transport_or_payload_failed".to_owned(),
        })?;
    check(
        grant.world_id == world && grant.channel_id == channel,
        "gateway_scope_mismatch",
    )?;
    check(
        grant.endpoint.host == text(route, "host")?
            && u64::from(grant.endpoint.port)
                == route["port"].as_u64().ok_or("route_port_invalid")?
            && grant.endpoint.tls_server_name == text(route, "tlsServerName")?,
        "gateway_endpoint_mismatch",
    )?;
    let root = oteryn_session_tcp::load_root_certificate(std::path::Path::new(
        &std::env::var("GAME_PLATFORM_GAMEPLAY_CERT")
            .map_err(|_| "gameplay_certificate_missing")?,
    ))
    .map_err(|_| "gameplay_certificate_invalid")?;
    let session = oteryn_session_tcp::connection::admit(
        &grant.endpoint.host,
        grant.endpoint.port,
        &grant.endpoint.tls_server_name,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&character_bytes).map_err(|_| "character_invalid")?,
            admission_material: grant.grant_bytes(),
            client_build_id: "game-platform-qualification",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(10),
        },
        &root,
    )
    .await
    .map_err(|code| match code {
        Some(code) => format!("game_admission_refused:{code}"),
        None => "game_connection_failed".to_owned(),
    })?;
    check(
        sql(
            &t.game_database,
            "exact_admission",
            &format!(
                "SELECT count(*) FROM game_durability_fresh_admission_receipts WHERE account_id='{account}' AND character_id='{character}' AND world_id='{world}' AND channel_id='{channel}' AND connection_generation=1;"
            ),
        )? == "1",
        "admission_binding_mismatch",
    )?;
    check(
        sql(
            &t.game_database,
            "only_admission",
            "SELECT count(*) FROM game_durability_fresh_admission_receipts;",
        )? == "1",
        "unexpected_extra_game_admission",
    )?;
    check(
        sql(
            &t.database,
            "platform_admission_owner",
            &format!(
                "SELECT count(*) FROM native_admissions WHERE account_id='{account}' AND attempt_ref='{}';",
                attempt.as_str()
            ),
        )? == "1",
        "platform_admission_binding_mismatch",
    )?;
    check(
        sql(
            &t.database,
            "only_platform_admission",
            "SELECT count(*) FROM native_admissions;",
        )? == "1",
        "unexpected_extra_platform_admission",
    )?;
    let recorded_session = sql(
        &t.game_database,
        "admitted_session",
        "SELECT game_session_id::text FROM game_durability_fresh_admission_receipts;",
    )?;
    check(
        oteryn_platform_client::native_login::parse_uuid_v7(&recorded_session).as_ref()
            == Some(session.join_snapshot().game_session_id.as_bytes()),
        "join_session_receipt_mismatch",
    )?;
    drop(session);
    println!(
        "NODE_BOOT_EVIDENCE flow=native_ticket_gateway result=pass game_admissions=1 identity_character_scope=exact"
    );
    Ok(())
}

#[test]
fn native_ticket_gateway_admits() -> TestResult {
    run("native_ticket_gateway_admits", 60, admitted_journey)
}
