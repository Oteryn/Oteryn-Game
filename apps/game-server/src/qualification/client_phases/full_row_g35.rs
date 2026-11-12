//! G35: Soul War, Rotten Blood and Primal Hazard selection, private boss loot, Svargrond arena.
//! The entry owner is `endgame_boss_entry` (`crates/gameplay/dispatch/src/gameplay_transport/
//! endgame_entry.rs`): BOSS_LEVER (capability 117, command 178) checks quest access, boss chain
//! and level from durable character state.
//! 1. Admit the node-boot character, send BOSS_LEVER for a Soul War boss and a Rotten Blood
//!    boss, and judge the real server reply for each.
//! 2. Private boss loot and the Svargrond arena need an accepted entry, a boss kill and the
//!    arena run; none is reachable from a fresh character, so the row ends FAIL naming the
//!    precondition (a correct refusal is not MISSING-SERVER).
//! 3. Log out and require Accepted.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::boss_encounter as wire;

const BOSSES: [&str; 2] = [
    "oteryn:creature.goshnar_s_cruelty",
    "oteryn:creature.the_primal_menace",
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let detail = match result {
        Ok(Ok(observed)) => format!(
            "needed:quest-access-boss-chain-and-lever-position-then-boss-kill-private-loot-and-svargrond-arena;observed={observed}"
        ),
        Ok(Err(error)) => token(&error.to_string()),
        Err(_) => "step=journey;observed=timeout".to_owned(),
    };
    evidence(&format!(
        "e2e_all scenario=G35.full status=FAIL seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

/// One word describing the lever reply.
fn classify(status: CommandStatus, reply: Option<&wire::BossActivityReply>) -> String {
    match (status, reply) {
        (CommandStatus::Accepted, Some(wire::BossActivityReply::Accepted { .. })) => {
            "entered".to_owned()
        }
        (CommandStatus::Accepted, Some(wire::BossActivityReply::Refused(reason))) => {
            format!("refused-{reason:?}")
        }
        (status, _) => format!("status-{status:?}"),
    }
}

async fn lever(client: &mut DevClientSession, key: &str) -> TestResult<String> {
    let payload = wire::encode_boss_key(key).map_err(|error| format!("encode:{error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BOSS_ENCOUNTER_V1,
            wire::COMMAND_TYPE_BOSS_LEVER,
            &payload,
        )
        .await?;
    let reply = wire::decode_boss_activity_reply(&response.payload).ok();
    Ok(classify(response.status, reply.as_ref()))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xe5).await?;
    let mut observed = Vec::new();
    for key in BOSSES {
        let outcome = lever(&mut client, key).await?;
        observed.push(format!(
            "{}={outcome}",
            key.rsplit('.').next().unwrap_or(key)
        ));
    }
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout;observed=refused".into());
    }
    Ok(observed.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_names_each_reply_shape() {
        let refused = wire::BossActivityReply::Refused(wire::BossActivityRefusal::Level);
        assert_eq!(
            classify(CommandStatus::Accepted, Some(&refused)),
            "refused-Level"
        );
        assert_eq!(classify(CommandStatus::Rejected, None), "status-Rejected");
        let entered = wire::BossActivityReply::Accepted {
            handoff: [0; 16],
            deadline_unix_ms: None,
        };
        assert_eq!(classify(CommandStatus::Accepted, Some(&entered)), "entered");
    }
}
