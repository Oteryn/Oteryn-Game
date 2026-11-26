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

// The authored Rat approach is 208 wire steps. The client packet now runs a
// first credited hunt to establish its baseline, then the captured hunt.
const DEADLINE: Duration = Duration::from_secs(600);

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    let root = required("E2E_ALL_CAPTURE_DIR")?;
    let mut file = std::fs::File::create_new(Path::new(&root).join(name))?;
    file.write_all(bytes)?;
    Ok(())
}

async fn send(
    client: &mut DevClientSession,
    action: CharmProgressionAction,
    capture_name: Option<&str>,
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
    if let Some(name) = capture_name {
        capture(name, &reply.payload)?;
    }
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
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = full_row_p01::prepare_hunter(clients, "P01.client", 0x21).await?;
    let rat_index = full_row_p05::canonical_races()?
        .iter()
        .position(|(definition, _)| definition.key() == "oteryn:creature.rat")
        .ok_or("Rat is absent from the canonical Bestiary")?;
    let race = NonZeroU32::new(u32::try_from(rat_index)? + 1).ok_or("Rat race index")?;
    let initial = send(&mut client, CharmProgressionAction::Query, None).await?;
    if initial.outcome != CharmProgressionOutcome::Query || kills(&initial, race) != 0 {
        return Err("P01 client requires a zero-kill Rat baseline".into());
    }
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    verdict::Harness::global().bounded_wait(
        tokio::time::Instant::now() + Duration::from_secs(125),
        "baseline-rat-combat",
    );
    client_hunt::hunt(&mut client).await?;
    let before = send(
        &mut client,
        CharmProgressionAction::Query,
        Some("p01-before.bin"),
    )
    .await?;
    if before.outcome != CharmProgressionOutcome::Query || before.charms.charms.is_empty() {
        return Err(format!("QUERY outcome {:?}", before.outcome).into());
    }
    if kills(&before, race) != 1 {
        return Err("first Rat hunt did not establish exactly one Bestiary kill".into());
    }
    let (index, locked) = before
        .charms
        .charms
        .iter()
        .enumerate()
        .find(|(_, charm)| charm.unlocked_stage == 0 && charm.next_stage_cost > 0)
        .ok_or("no locked charm to refuse")?;
    let charm = NonZeroU32::new(u32::try_from(index)? + 1).ok_or("charm index")?;
    let refused = send(
        &mut client,
        CharmProgressionAction::Assign { charm, race },
        Some("p01-refused.bin"),
    )
    .await?;
    if refused.outcome != CharmProgressionOutcome::Assign(CharmAssignDisposition::CharmLocked)
        || refused.charms != before.charms
        || refused.bestiary != before.bestiary
    {
        return Err(format!("locked ASSIGN reply {refused:?}").into());
    }
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    verdict::Harness::global().bounded_wait(
        tokio::time::Instant::now() + Duration::from_secs(125),
        "captured-rat-combat",
    );
    client_hunt::hunt(&mut client).await?;
    let after = send(
        &mut client,
        CharmProgressionAction::Query,
        Some("p01-after.bin"),
    )
    .await?;
    if after.outcome != CharmProgressionOutcome::Query {
        return Err(format!("post-hunt QUERY outcome {:?}", after.outcome).into());
    }
    let (hunted, count_before, count_after) = after
        .bestiary
        .iter()
        .map(|entry| (entry.race, kills(&before, entry.race), entry.kill_count))
        .find(|(_, was, now)| now > was)
        .ok_or("hunt credited no Bestiary kill")?;
    if hunted != race || count_after != count_before + 1 {
        return Err(format!("kill delta {count_before}->{count_after}").into());
    }
    let expected = format!(
        "race={hunted}\nkills_before={count_before}\nkills_after={count_after}\npoints_before={}\npoints_after={}\nlocked_charm={charm}\nlocked_stage={}\nrefusal=CharmLocked\n",
        before.charms.charm_points_available,
        after.charms.charm_points_available,
        locked.unlocked_stage
    );
    capture("p01-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(DEADLINE, verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P01.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P01.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P01.client status=FAIL evidence=timeout"),
    }
}
