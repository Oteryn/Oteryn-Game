//! G06: native login challenge before admission (apps/client `native_entry/challenge`,
//! `platform-client` `native_login/challenge.rs`).
//! The challenge is an OAuth extension of the Platform token endpoint: Platform answers
//! `POST /oauth/token` with `oteryn_challenge_required`, serves `/oauth/challenge` and verifies
//! the proof. The Game server owns no challenge route (the contract assigns issuance and
//! verification to Platform: `docs/contracts/OTERYN_NATIVE_LOGIN_CHALLENGE_V1.md`), so nothing
//! in the production-session path can issue one.
//! 1. Admit the node-boot character through the dev client with a signed grant; the Game
//!    admission accepts it without any challenge exchange (no Game challenge step exists).
//! 2. Log out and require `Accepted`.
//!
//! The row then reports MISSING-SERVER naming the contract line that leaves issuance to Platform.
use super::*;

const SOURCE: &str = "source:docs/contracts/OTERYN_NATIVE_LOGIN_CHALLENGE_V1.md:4";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(60), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("MISSING-SERVER", SOURCE.to_owned()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G06.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xc7).await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_token_has_no_spaces() {
        assert!(SOURCE.starts_with("source:") && !SOURCE.contains(' '));
        assert_eq!(token("a\nb"), "a-b");
    }
}
