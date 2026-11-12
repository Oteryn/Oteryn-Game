//! Real OAuth ticket issuance and the currently unavailable portal-rs Gateway.
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
            record_blocker(source)?;
            return Err("native_handler_missing".into());
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
