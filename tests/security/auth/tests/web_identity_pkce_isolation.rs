//! Real portal-rs journey, invoked only by tools/qualification/game_platform/run.sh.
//! No HTTP server, identity writer, or token producer is substituted here.
mod support;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::{
    PlatformClientConfig,
    native_login::{NativeLoginClient, NativeLoginError},
};
use reqwest::Url;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, time::Duration};
use support::*;
use tokio::net::TcpListener;
async fn journey(input: Input, report: Value, game_database: String) -> TestResult {
    for account in &input.accounts {
        check(
            account.account_id.len() == 36
                && account
                    .account_id
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() || b == b'-'),
            "account_id_invalid",
        )?;
    }
    let portal_env = read_json(&format!(
        "{}/secrets/env.json",
        text(&report, "outputDirectory")?
    ))?;
    let database = text(&portal_env, "OTERYN_DATABASE_URL")?;
    check(database != game_database, "shared_database_forbidden")?;
    let public = text(&report["baseUrls"], "public")?;
    let config = PlatformClientConfig::new(public).map_err(|_| "native_config_invalid")?;
    let gateway = PlatformClientConfig::new(text(&report["baseUrls"], "gateway")?)
        .map_err(|_| "gateway_config_invalid")?;
    let native = NativeLoginClient::new(&config, &gateway, &input.client)
        .map_err(|_| "native_client_creation_failed")?;
    let callback = Url::parse(&input.callback).map_err(|_| "callback_invalid")?;
    let listener =
        TcpListener::bind(("127.0.0.1", callback.port().ok_or("callback_port_missing")?))
            .await
            .map_err(|_| "loopback_bind_failed")?;
    let mut a = Browser::new(public)?;
    let csrf = a.csrf("/en/login").await?;
    let anonymous = cookie_hash(&a)?;
    let response = a.login(&input.accounts[0], &random()?, Some(&csrf)).await?;
    check(
        response.status().is_redirection() && location(&response)? == "/en/login",
        "wrong_password_not_generic",
    )?;
    check(
        session_owner(database, &a)? == "anonymous",
        "wrong_password_created_authenticated_session",
    )?;
    let failed = a
        .request("/en/login", None)
        .await?
        .text()
        .await
        .map_err(|_| "failure_page_read_failed")?;
    check(
        failed.contains("role=\"alert\"") && !failed.contains(&input.accounts[0].email),
        "failure_page_not_generic",
    )?;
    for token in [None, Some("invalid")] {
        check(
            a.login(&input.accounts[0], &input.accounts[0].password, token)
                .await?
                .status()
                == 403,
            "csrf_not_rejected",
        )?;
        check(
            session_owner(database, &a)? == "anonymous",
            "csrf_created_session",
        )?;
    }
    let csrf = a.csrf("/en/login").await?;
    check(
        a.login(&input.accounts[0], &input.accounts[0].password, Some(&csrf))
            .await?
            .status()
            .is_redirection(),
        "real_login_refused",
    )?;
    check(
        cookie_hash(&a)? != anonymous,
        "login_did_not_rotate_session",
    )?;
    check(
        session_owner(database, &a)? == input.accounts[0].account_id,
        "wrong_account_session",
    )?;
    check(
        sql(
            database,
            "old_session",
            &format!(
                "SELECT count(*) FROM web_sessions WHERE id_hash=decode('{anonymous}','hex');"
            ),
        )? == "0",
        "old_session_survived_login",
    )?;
    let verifier = random()?;
    let code = authorize(&mut a, &native, &input, &listener, &random()?, &verifier).await?;
    check(
        matches!(
            native
                .exchange_code(&code, &random()?, &input.callback, CancellationToken::new())
                .await,
            Err(NativeLoginError::TokenRefused)
        ),
        "pkce_mismatch_accepted",
    )?;
    check(
        sql(
            database,
            "unused_code",
            &format!(
                "SELECT count(*) FROM oauth_authorization_codes WHERE code_hash=decode('{}','hex') AND used_at IS NULL;",
                digest(&code)
            ),
        )? == "1",
        "negative_case_consumed_code",
    )?;
    check(
        sql(
            database,
            "negative_tokens",
            "SELECT count(*) FROM oauth_tokens;",
        )? == "0",
        "negative_case_issued_token",
    )?;
    let _token = native
        .exchange_code(&code, &verifier, &input.callback, CancellationToken::new())
        .await
        .map_err(|_| "native_code_exchange_refused")?;
    check(
        matches!(
            native
                .exchange_code(&code, &verifier, &input.callback, CancellationToken::new())
                .await,
            Err(NativeLoginError::TokenRefused)
        ),
        "authorization_code_reused",
    )?;
    check(
        sql(
            database,
            "account_a_tokens",
            &format!(
                "SELECT count(*) FROM oauth_tokens t JOIN oauth_token_families f ON f.id=t.family_id WHERE f.account_id='{}';",
                input.accounts[0].account_id
            ),
        )? == "1",
        "token_not_bound_to_account_a",
    )?;
    check(
        sql(
            database,
            "account_b_families",
            &format!(
                "SELECT count(*) FROM oauth_token_families WHERE account_id='{}';",
                input.accounts[1].account_id
            ),
        )? == "0",
        "account_b_received_account_a_token",
    )?;
    // Another account's credentials cannot relabel an authenticated jar.
    let csrf = csrf_for_logout(&mut a, &native, &input).await?;
    a.login(&input.accounts[1], &input.accounts[1].password, Some(&csrf))
        .await?;
    check(
        session_owner(database, &a)? == input.accounts[0].account_id,
        "account_b_rebound_account_a_session",
    )?;
    let mut b = Browser::new(public)?;
    let csrf = b.csrf("/en/login").await?;
    b.login(&input.accounts[1], &input.accounts[1].password, Some(&csrf))
        .await?;
    check(
        session_owner(database, &b)? == input.accounts[1].account_id,
        "account_b_login_not_isolated",
    )?;
    let old_a_cookie = a.cookie.clone();
    let old_a_hash = cookie_hash(&a)?;
    let csrf = csrf_for_logout(&mut a, &native, &input).await?;
    check(
        a.request("/en/logout", Some(&[("_csrf".into(), csrf)]))
            .await?
            .status()
            .is_redirection(),
        "logout_refused",
    )?;
    check(
        cookie_hash(&a)? != old_a_hash,
        "logout_did_not_rotate_session",
    )?;
    check(
        session_owner(database, &a)? == "anonymous",
        "logout_kept_authenticated_session",
    )?;
    let mut replay = Browser::new(public)?;
    replay.cookie = old_a_cookie;
    check(
        sql(
            database,
            "logout_old_session",
            &format!(
                "SELECT count(*) FROM web_sessions WHERE id_hash=decode('{old_a_hash}','hex');"
            ),
        )? == "0",
        "logout_session_row_survived",
    )?;
    let response = replay.request("/en/login", None).await?;
    check(
        response.status() == 200 && session_owner(database, &replay)? == "anonymous",
        "logout_cookie_replay_authenticated",
    )?;
    check(
        session_owner(database, &b)? == input.accounts[1].account_id,
        "logout_crossed_accounts",
    )?;
    check(
        sql(
            &game_database,
            "game_admissions",
            "SELECT count(*) FROM game_durability_fresh_admission_receipts;",
        )? == "0",
        "identity_admitted_to_game",
    )?;
    println!("NODE_BOOT_EVIDENCE flow=web_identity result=pass account_isolation=1");
    Ok(())
}
async fn csrf_for_logout(
    browser: &mut Browser,
    native: &NativeLoginClient,
    input: &Input,
) -> TestResult<String> {
    let url = native
        .authorization_url(
            &input.callback,
            &random()?,
            &URL_SAFE_NO_PAD.encode(Sha256::digest(random()?.as_bytes())),
        )
        .map_err(|_| "authorization_url_failed")?;
    browser
        .csrf(&format!(
            "/oauth/authorize?{}",
            url.query().ok_or("authorization_query_missing")?
        ))
        .await
}
#[test]
fn web_identity_pkce_isolation() -> TestResult {
    let Ok(input_file) = std::env::var("GAME_PLATFORM_INPUT") else {
        // Ordinary crate tests have no external topology and emit no E2E evidence.
        return Ok(());
    };
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> TestResult {
            let input: Input = serde_json::from_value(read_json(&input_file)?)
                .map_err(|_| "journey_input_invalid")?;
            let report = read_json(
                &std::env::var("GAME_PLATFORM_REPORT").map_err(|_| "report_not_configured")?,
            )?;
            let game = std::env::var("GAME_PLATFORM_GAME_DATABASE_URL")
                .map_err(|_| "game_database_not_configured")?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| "journey_runtime_failed")?
                .block_on(async {
                    tokio::time::timeout(Duration::from_secs(60), journey(input, report, game))
                        .await
                        .map_err(|_| "journey_budget_exceeded")?
                })
        })
        .map_err(|_| "journey_thread_failed")?
        .join()
        .map_err(|_| "journey_thread_panicked")?;
    if let Err(error) = &result
        && let Ok(file) = std::env::var("GAME_PLATFORM_RESULT_FILE")
        && !std::path::Path::new(&file).exists()
    {
        let reason = error.to_string();
        check(
            reason.len() <= 128
                && reason
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':'),
            "invalid_failure_reason",
        )?;
        fs::write(
            file,
            format!("GAME_PLATFORM_RESULT=FAIL flow=web_identity reason={reason}\n"),
        )
        .map_err(|_| "result_write_failed")?;
    }
    result
}
