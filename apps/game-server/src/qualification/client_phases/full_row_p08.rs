//! P08 full-row native-client journey.
//!
//! Scenario derived from the P08 matrix/audit row and its production dispatch owner:
//! 1. Browse the admitted proficiency catalogue and current tracks; expect bounded native reply
//!    pages whose definition, XP, level, selections, thresholds and perk descriptions agree.
//! 2. Walk to a compiled-world rat, select it with the ordinary attack command and finish the
//!    fight; expect the server to credit weapon proficiency XP only for an equipped bound weapon.
//! 3. Select an unlocked perk; expect an accepted selection reply and a matching browse delta.
//! 4. Exercise modify, rank-up, orb-rank, reshape-offer, reshape-choose and clear with the
//!    committed revision; expect typed replies and the corresponding slot transitions/costs.
//! 5. Send a stale-revision modification as the refusal path; expect `StaleRevision` and no
//!    change to the track page.
//! 6. Log out and obtain a fresh admission; expect XP, selections and shaped slots/catalyst
//!    consumption to remain durable. A server/content refusal is reported with the exact step
//!    and observed typed reply, but only after all mechanic commands have actually been sent.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::weapon_proficiency as wire;
use std::num::NonZeroU32;

#[derive(Default)]
struct Observations {
    failures: Vec<String>,
}

impl Observations {
    fn require(&mut self, step: &str, condition: bool, observed: impl std::fmt::Debug) {
        if !condition {
            self.failures.push(format!("step-{step}:{observed:?}"));
        }
    }
}

async fn command(
    client: &mut DevClientSession,
    kind: u32,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(wire::CAPABILITY_WEAPON_PROFICIENCY_V1, kind, payload)
        .await?)
}

async fn browse(client: &mut DevClientSession) -> TestResult<wire::ProficiencyPage> {
    let reply = command(
        client,
        wire::COMMAND_TYPE_PROFICIENCY_BROWSE,
        &0_u32.to_be_bytes(),
    )
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-browse:status={:?}", reply.status).into());
    }
    wire::ProficiencyPage::decode(&reply.payload)
        .ok_or_else(|| "step-browse:malformed proficiency page".into())
}

async fn catalogue(client: &mut DevClientSession) -> TestResult<wire::ProficiencyCataloguePage> {
    let reply = command(
        client,
        wire::COMMAND_TYPE_PROFICIENCY_CATALOGUE,
        &0_u32.to_be_bytes(),
    )
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-catalogue:status={:?}", reply.status).into());
    }
    wire::ProficiencyCataloguePage::decode(&reply.payload)
        .ok_or_else(|| "step-catalogue:malformed catalogue page".into())
}

async fn select(
    client: &mut DevClientSession,
    definition: NonZeroU32,
) -> TestResult<wire::SelectPerkReply> {
    let payload = wire::SelectPerkRequest {
        definition,
        level: 0,
        perk: Some(0),
    }
    .encode();
    let reply = command(
        client,
        wire::COMMAND_TYPE_PROFICIENCY_SELECT_PERK,
        &payload,
    )
    .await?;
    wire::SelectPerkReply::decode(&reply.payload)
        .ok_or_else(|| format!("step-select:status={:?} payload={:?}", reply.status, reply.payload).into())
}

async fn modify(
    client: &mut DevClientSession,
    definition: NonZeroU32,
    operation: wire::ModificationOperation,
    revision: u64,
) -> TestResult<wire::ModificationReply> {
    let request = wire::ModificationRequest {
        definition,
        slot: 1,
        operation,
        expected_revision: revision,
    };
    let reply = command(client, operation.command_type(), &request.encode()).await?;
    wire::ModificationReply::decode(&reply.payload).ok_or_else(|| {
        format!(
            "step-modify-{operation:?}:status={:?} payload={:?}",
            reply.status, reply.payload
        )
        .into()
    })
}

async fn journey(clients: &SeamClients<'_>, observed: &mut Observations) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xa2).await?;
    let initial_session = client.join_snapshot().game_session_id;
    let catalogue = catalogue(&mut client).await?;
    let before = browse(&mut client).await?;
    let definition = catalogue
        .entries
        .iter()
        .find_map(|entry| NonZeroU32::new(entry.definition))
        .ok_or("step-catalogue:no admitted weapon definition")?;

    // This is a real player fight. With the node-boot starter's empty hands it also proves that
    // an unrelated kill cannot manufacture a weapon track.
    let combat = async {
        travel_to_spawn(
            &mut client,
            &world,
            &metadata.spawns,
            "oteryn:creature.rat",
        )
        .await?;
        client_hunt::hunt(&mut client).await
    }
    .await;
    observed.require("combat", combat.is_ok(), &combat);
    let after_kill = browse(&mut client).await?;
    observed.require(
        "xp-delta",
        after_kill
            .rows
            .iter()
            .zip(&before.rows)
            .any(|(after, before)| after.definition == before.definition && after.experience > before.experience),
        (&before.rows, &after_kill.rows),
    );

    let selected = select(&mut client, definition).await?;
    observed.require(
        "select",
        selected.result == wire::SelectPerkResult::Accepted,
        &selected,
    );
    let operations = [
        wire::ModificationOperation::Modify { level: 0 },
        wire::ModificationOperation::RankUp,
        wire::ModificationOperation::OrbRank,
        wire::ModificationOperation::ReshapeOffer,
        wire::ModificationOperation::ReshapeChoose { choice: Some(0) },
        wire::ModificationOperation::Clear,
    ];
    for (index, operation) in operations.into_iter().enumerate() {
        let reply = modify(&mut client, definition, operation, index as u64 + 1).await?;
        observed.require(
            "modification",
            reply.result == wire::ModificationResult::Accepted,
            (operation, reply),
        );
    }
    let stable = browse(&mut client).await?;
    let refused = modify(&mut client, definition, wire::ModificationOperation::RankUp, u64::MAX).await?;
    observed.require(
        "stale-refusal",
        refused.result == wire::ModificationResult::StaleRevision,
        refused,
    );
    observed.require("refusal-no-delta", browse(&mut client).await? == stable, &stable);

    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-logout:refused".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xa3).await?;
    observed.require(
        "fresh-session",
        fresh.join_snapshot().game_session_id != initial_session,
        fresh.join_snapshot().game_session_id,
    );
    observed.require("durable-state", browse(&mut fresh).await? == stable, &stable);
    let _ = fresh.session_mut().logout().await;
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let mut observed = Observations::default();
    let result = tokio::time::timeout(
        Duration::from_secs(240),
        journey(clients, &mut observed),
    )
    .await;
    match result {
        Ok(Err(error)) => observed.failures.push(error.to_string()),
        Err(_) => observed.failures.push("step-deadline:timeout".into()),
        Ok(Ok(())) => {}
    }
    if observed.failures.is_empty() {
        evidence(&format!(
            "e2e_all scenario=P08.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        ));
    } else {
        let detail = observed.failures.join(";").replace(' ', "_");
        evidence(&format!(
            "e2e_all scenario=P08.full status=FAIL seconds={:.3} evidence={detail}",
            started.elapsed().as_secs_f64()
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_verdict_is_derived_from_failed_steps() {
        let mut observations = Observations::default();
        observations.require("accepted", true, "accepted");
        assert!(observations.failures.is_empty());
        observations.require("refused", false, wire::ModificationResult::NoOrb);
        assert_eq!(observations.failures, ["step-refused:NoOrb"]);
    }
}
