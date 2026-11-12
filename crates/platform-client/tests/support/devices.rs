//! Real portal process controls and mTLS publication; never a response producer.
use super::support::*;
use oteryn_platform_client::{
    PlatformClientConfig, device_sessions::DeviceSession, native_login::NativeLoginClient,
};
use reqwest::{Client, Url, redirect::Policy};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    process::{Child, Command, Stdio},
    time::Duration,
};

pub const BASE: &str = "/api/v1/game-auth/device-sessions";
pub fn enrollment_gap(status: u16, body: &Value) -> bool {
    status == 400 && *body == json!({"error":"invalid_request"})
}
pub fn rotation_gap(status: u16, body: &Value) -> bool {
    status == 401 && *body == json!({"error":"device_authorization_unavailable"})
}
pub fn gaps(enrollment: bool) -> TestResult {
    if enrollment {
        println!(
            "GAME_PLATFORM_EVIDENCE blocker=portal-rs/crates/gameauth/src/devices_http.rs:224 pending=contract_two_member_enrollment"
        );
    }
    println!(
        "GAME_PLATFORM_EVIDENCE blocker=portal-rs/crates/gameauth/src/devices_http.rs:199 pending=contract_rotation_without_proof"
    );
    record_blocker(if enrollment {
        "portal-rs/crates/gameauth/src/devices_http.rs:224"
    } else {
        "portal-rs/crates/gameauth/src/devices_http.rs:199"
    })
}
pub fn ca(t: &Topology) -> TestResult<Vec<u8>> {
    fs::read(text(&t.report, "caFile")?).map_err(|_| "ca_read_failed".into())
}
pub fn http(t: &Topology) -> TestResult<Client> {
    Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(5))
        .add_root_certificate(reqwest::Certificate::from_pem(&ca(t)?).map_err(|_| "ca_invalid")?)
        .build()
        .map_err(|_| "device_http_failed".into())
}
pub fn native(t: &Topology, origin: &str, client_id: &str) -> TestResult<NativeLoginClient> {
    let mut child = Command::new("openssl")
        .args(["x509", "-outform", "DER"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "ca_conversion_failed")?;
    child
        .stdin
        .take()
        .ok_or("ca_input_missing")?
        .write_all(&ca(t)?)
        .map_err(|_| "ca_conversion_failed")?;
    let output = child
        .wait_with_output()
        .map_err(|_| "ca_conversion_failed")?;
    check(output.status.success(), "ca_conversion_failed")?;
    NativeLoginClient::new(
        &PlatformClientConfig::new(origin).map_err(|_| "native_origin_invalid")?,
        &PlatformClientConfig::new(text(&t.report["baseUrls"], "gateway")?)
            .map_err(|_| "gateway_origin_invalid")?,
        client_id,
    )
    .map_err(|_| "device_client_failed")?
    .with_additional_tls_root(&output.stdout)
    .map_err(|_| "device_ca_failed".into())
}
pub async fn post(
    client: &Client,
    origin: &str,
    suffix: &str,
    authorization: &str,
    body: Value,
) -> TestResult<(u16, Value)> {
    let mut authorization = reqwest::header::HeaderValue::from_str(authorization)
        .map_err(|_| "device_probe_authorization_invalid")?;
    authorization.set_sensitive(true);
    let response = client
        .post(format!("{origin}{BASE}{suffix}"))
        .header("authorization", authorization)
        .json(&body)
        .send()
        .await
        .map_err(|_| "device_probe_transport_failed")?;
    let status = response.status().as_u16();
    if status == 404 {
        record_blocker("portal-rs/crates/gameauth/src/devices_http.rs:53")?;
        return Err("device_handler_missing".into());
    }
    check(
        response.content_length().is_none_or(|n| n <= 16384),
        "device_probe_oversize",
    )?;
    let bytes = response
        .bytes()
        .await
        .map_err(|_| "device_probe_read_failed")?;
    check(bytes.len() <= 16384, "device_probe_oversize")?;
    let body = serde_json::from_slice(&bytes).map_err(|_| "device_probe_json_invalid")?;
    Ok((status, body))
}
pub fn persisted(t: &Topology, session: &DeviceSession, account: &str) -> TestResult {
    check(
        oteryn_platform_client::device_sessions::canonical_uuid(&session.family_id),
        "family_id_invalid",
    )?;
    check(
        sql(
            &t.database,
            "device_owner",
            &format!(
                "SELECT count(*) FROM device_sessions WHERE id='{}' AND account_id='{account}' AND client_id='{}' AND revoked_at IS NULL;",
                session.family_id, t.input.client
            ),
        )? == "1",
        "device_wrong_owner_or_client",
    )
}
pub fn revoked(t: &Topology, session: &DeviceSession) -> TestResult {
    check(
        sql(
            &t.database,
            "family_revoked",
            &format!(
                "SELECT count(*) FROM device_sessions WHERE id='{}' AND revoked_at IS NOT NULL;",
                session.family_id
            ),
        )? == "1",
        "family_not_revoked",
    )
}
pub fn generation_change(t: &Topology, account: &str) -> TestResult {
    // Only the current generation changes. No expiry, credential, projection or
    // successful authority row is manufactured. The normal security trigger runs.
    check(
        oteryn_platform_client::device_sessions::canonical_uuid(account),
        "account_id_invalid",
    )?;
    let mut child = Command::new("psql")
        .args(["-X", "-qAt", "-v", "ON_ERROR_STOP=1"])
        .envs(postgres_env(&t.database)?)
        .env("PGCONNECT_TIMEOUT", "5")
        .env("PGOPTIONS", "-c statement_timeout=3000")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "security_change_failed")?;
    child.stdin.take().ok_or("security_change_input_missing")?.write_all(format!("UPDATE accounts SET native_security_generation=native_security_generation+1 WHERE id='{account}';").as_bytes()).map_err(|_| "security_change_failed")?;
    check(
        child
            .wait()
            .map_err(|_| "security_change_failed")?
            .success(),
        "security_change_failed",
    )
}

pub async fn empty_projection(t: &Topology, account: &str) -> TestResult {
    let identity = &t.report["clients"]["lcfa"];
    let mut pem = fs::read(text(identity, "cert")?).map_err(|_| "lcfa_certificate_missing")?;
    pem.extend(fs::read(text(identity, "key")?).map_err(|_| "lcfa_key_missing")?);
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .add_root_certificate(reqwest::Certificate::from_pem(&ca(t)?).map_err(|_| "ca_invalid")?)
        .identity(reqwest::Identity::from_pem(&pem).map_err(|_| "lcfa_identity_invalid")?)
        .build()
        .map_err(|_| "lcfa_client_failed")?;
    let now = oteryn_platform_client::device_sessions::now()
        .map_err(|_| "clock_failed")?
        .to_string();
    for (suffix, body) in [
        (
            "",
            json!({"contract_version":1,"operation":"PublishAccountCharactersV1","source_authority":"oteryn-game-e2e","account_id":account,"projection_epoch":"1","projection_revision":"1","source_observed_at":now,"characters":[]}),
        ),
        (
            "/watermark",
            json!({"contract_version":1,"operation":"PublishProjectionWatermarkV1","source_authority":"oteryn-game-e2e","projection_epoch":"1","complete_through":now,"observed_at":now}),
        ),
    ] {
        let response = client
            .post(format!(
                "{}/internal/v1/game-auth/native-account-characters{suffix}",
                text(&t.report["baseUrls"], "internal")?
            ))
            .json(&body)
            .send()
            .await
            .map_err(|_| "lcfa_publication_transport_failed")?;
        check(response.status().is_success(), "lcfa_publication_refused")?;
    }
    Ok(())
}

pub struct Portal {
    child: Child,
    pub origin: String,
}
fn free_port() -> TestResult<u16> {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .map_err(|_| "port_allocation_failed")?
        .local_addr()
        .map(|a| a.port())
        .map_err(|_| "port_allocation_failed".into())
}
impl Portal {
    pub async fn start(t: &Topology, disabled_oauth: bool) -> TestResult<Self> {
        let env = read_json(&format!(
            "{}/secrets/env.json",
            text(&t.report, "outputDirectory")?
        ))?;
        let mut command = Command::new(&t.input.portal_binary);
        for (key, value) in env.as_object().ok_or("portal_environment_invalid")? {
            command.env(key, value.as_str().ok_or("portal_environment_invalid")?);
        }
        let port = free_port()?;
        let origin = format!("https://127.0.0.1:{port}");
        let oauth = json!({"enabled":!disabled_oauth,"clients":[{"client_id":t.input.client,"redirect_uris":[t.input.callback]}]});
        command
            .arg("serve")
            .env("PORTAL_OAUTH_CONFIG", oauth.to_string())
            .env("OTERYN_ACCOUNT_LOGIN", "true")
            .env("OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED", "true")
            .env(
                "OTERYN_NATIVE_DEVICE_PROJECTION_AUTHORITY",
                "oteryn-game-e2e",
            )
            .env("APP_ENV", "testing")
            .env("OTERYN_BIND", format!("127.0.0.1:{}", free_port()?))
            .env("OTERYN_NATIVE_TLS_BIND", format!("127.0.0.1:{port}"))
            .env(
                "OTERYN_NATIVE_TLS_CERT_FILE",
                text(&env, "OTERYN_INTERNAL_TLS_CERT_FILE")?,
            )
            .env(
                "OTERYN_NATIVE_TLS_KEY_FILE",
                text(&env, "OTERYN_INTERNAL_TLS_KEY_FILE")?,
            )
            .env_remove("OTERYN_INTERNAL_TLS_BIND")
            .env_remove("OTERYN_INTERNAL_TLS_CERT_FILE")
            .env_remove("OTERYN_INTERNAL_TLS_KEY_FILE")
            .env_remove("OTERYN_INTERNAL_TLS_CLIENT_CA_FILE")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut portal = Self {
            child: command.spawn().map_err(|_| "portal_spawn_failed")?,
            origin,
        };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        let client = http(t)?;
        loop {
            check(
                portal
                    .child
                    .try_wait()
                    .map_err(|_| "portal_poll_failed")?
                    .is_none(),
                "portal_exited",
            )?;
            if client
                .get(format!("{}/health", portal.origin))
                .send()
                .await
                .is_ok()
            {
                break;
            }
            check(
                tokio::time::Instant::now() < deadline,
                "portal_readiness_timeout",
            )?;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Ok(portal)
    }
    pub async fn stop(&mut self) -> TestResult {
        self.child.kill().map_err(|_| "portal_stop_failed")?;
        self.child.wait().map_err(|_| "portal_wait_failed")?;
        let url = Url::parse(&self.origin).map_err(|_| "portal_origin_invalid")?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while tokio::net::TcpStream::connect((
            "127.0.0.1",
            url.port().ok_or("portal_port_missing")?,
        ))
        .await
        .is_ok()
        {
            check(
                tokio::time::Instant::now() < deadline,
                "portal_still_listening",
            )?;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Ok(())
    }
}
impl Drop for Portal {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
