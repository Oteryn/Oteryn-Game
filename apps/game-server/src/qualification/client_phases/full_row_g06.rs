//! G06 exercises Platform OAuth challenge proof before Game admission.
use super::*;

const SOURCE: &str = "source:docs/contracts/OTERYN_NATIVE_LOGIN_CHALLENGE_V1.md:4";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var("NODE_BOOT_LOGIN_CHALLENGE").as_deref() != Ok("1") {
        evidence(&format!("e2e_all scenario=G06.full status=MISSING-SERVER seconds=0.000 evidence={SOURCE}"));
        return;
    }
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(60), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "platform-challenge-ticket-grant-and-logout".to_owned()),
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
    wait_character_released(clients, 1).await?;
    let token = platform_row_grant(clients, 1, true).await?;
    let mut client = admit_character(clients, 1, &token).await?;
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
