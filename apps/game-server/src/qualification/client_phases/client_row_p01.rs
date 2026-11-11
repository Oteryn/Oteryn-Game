//! P01 client replay capture: real CHARM_PROGRESSION replies for the initial QUERY, a refused
//! ASSIGN of a locked charm, and the QUERY after a credited hunt kill. The consumer is `apply` in
//! `apps/client/src/features/0067-bestiary-charm-progression/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::charm::CharmAssignDisposition;
use oteryn_protocol_oteryn::charm_progression::{
    self as wire, CharmProgressionAction, CharmProgressionOutcome, CharmProgressionReply,
};
use std::num::NonZeroU32;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn send(
    client: &mut DevClientSession,
    action: CharmProgressionAction,
    capture_name: &str,
) -> TestResult<CharmProgressionReply> {
    let payload = wire::encode_charm_progression_request(&action)
        .map_err(|error| format!("encode {action:?}: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
            wire::COMMAND_TYPE_CHARM_PROGRESSION,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted || reply.payload.is_empty() {
        return Err(format!("{action:?} status={:?}", reply.status).into());
    }
    capture(capture_name, &reply.payload)?;
    wire::decode_charm_progression_reply(&reply.payload)
        .map_err(|error| format!("decode {action:?}: {error:?}").into())
}

fn kills(reply: &CharmProgressionReply, race: NonZeroU32) -> u32 {
    reply
        .bestiary
        .iter()
        .find(|entry| entry.race == race)
        .map_or(0, |entry| entry.kill_count)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x21).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let before = send(&mut client, CharmProgressionAction::Query, "p01-before.bin").await?;
    if before.outcome != CharmProgressionOutcome::Query || before.charms.charms.is_empty() {
        return Err(format!("QUERY outcome {:?}", before.outcome).into());
    }
    let (index, locked) = before
        .charms
        .charms
        .iter()
        .enumerate()
        .find(|(_, charm)| charm.unlocked_stage == 0 && charm.next_stage_cost > 0)
        .ok_or("no locked charm to refuse")?;
    let charm = NonZeroU32::new(u32::try_from(index)? + 1).ok_or("charm index")?;
    let race = before.bestiary.first().ok_or("no Bestiary race")?.race;
    let refused = send(
        &mut client,
        CharmProgressionAction::Assign { charm, race },
        "p01-refused.bin",
    )
    .await?;
    if refused.outcome != CharmProgressionOutcome::Assign(CharmAssignDisposition::CharmLocked)
        || refused.charms != before.charms
    {
        return Err(format!("locked ASSIGN reply {refused:?}").into());
    }
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    client_hunt::hunt(&mut client).await?;
    let after = send(&mut client, CharmProgressionAction::Query, "p01-after.bin").await?;
    let (hunted, count_before, count_after) = after
        .bestiary
        .iter()
        .map(|entry| (entry.race, kills(&before, entry.race), entry.kill_count))
        .find(|(_, was, now)| now > was)
        .ok_or("hunt credited no Bestiary kill")?;
    if count_after != count_before + 1 {
        return Err(format!("kill delta {count_before}->{count_after}").into());
    }
    let expected = format!(
        "race={hunted}\nkills_before={count_before}\nkills_after={count_after}\npoints_before={}\npoints_after={}\nlocked_charm={charm}\nlocked_stage={}\nrefusal=CharmLocked\n",
        before.charms.charm_points_available, after.charms.charm_points_available, locked.unlocked_stage
    );
    capture("p01-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P01.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P01.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P01.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kill_lookup_does_not_credit_a_different_race() {
        let mut reply = CharmProgressionReply {
            outcome: CharmProgressionOutcome::Query,
            fee_gold: 0,
            bestiary: vec![],
            charms: oteryn_protocol_oteryn::charm::CharmView {
                charms: vec![],
                charm_points_available: 0,
                minor_charm_echoes_available: 0,
                assignment_slot_limit: None,
            },
        };
        assert_eq!(kills(&reply, NonZeroU32::MIN), 0);
        reply
            .bestiary
            .push(oteryn_protocol_oteryn::bestiary::BestiaryRaceProgress {
                race: NonZeroU32::MIN,
                kill_count: u32::MAX,
                kill_thresholds: [1, 2, 3],
            });
        assert_eq!(kills(&reply, NonZeroU32::MIN), u32::MAX);
        assert_eq!(kills(&reply, NonZeroU32::MAX), 0);
    }
}
