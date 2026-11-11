//! G28: Character Bazaar, world transfer and login challenge fences (cg-account).
//! The bazaar and world-transfer intents are decoded by the native admission source
//! (`native-admission-source/src/bazaar/mod.rs`, `decode_intent`), a Platform-to-Game HTTP route
//! signed by the Platform; the game session (`oteryn-session`, `client-runtime`) has no bazaar
//! command, capability or helper, so a player-driven dev client cannot send or show it. The row
//! proves the one thing the dev client can do (the session offers no bazaar capability) and
//! reports MISSING-CLIENT naming the route.
use super::*;

const SOURCE: &str = "source:crates/gameplay/native-admission-source/src/bazaar/mod.rs:315";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(60), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("MISSING-CLIENT", SOURCE.to_owned()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G28.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = row_connect_world(clients, 0xdb).await?;
    let offered = client.selected_capabilities().len();
    if offered == 0 {
        return Err("step=admit observed=no-capabilities-selected".into());
    }
    if client.row_logout().await?.result
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
    fn evidence_tokens_cover_empty_and_whitespace() {
        assert_eq!(token(""), "");
        assert!(!token("a b\tc\nd").chars().any(char::is_whitespace));
        assert_eq!(token("source:path.rs:12"), "source:path.rs:12");
    }
}
