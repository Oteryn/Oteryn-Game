//! P22d (live tracker refresh on kill progress):
//! 1. Admit the node-boot character, track a race and note its kill count.
//! 2. Walk to a compiled spawn of that race, kill one creature and wait for the live frames.
//! 3. The server pushes a Cyclopedia tracker StateDelta (domain 98); the native session has no
//!    decoder for that domain and fails with `UnsupportedPushedDomain`. The row therefore
//!    confirms the credit through a fresh admission (kill count +1) and reports the client gap.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::{CommandStatus, cyclopedia_tracker as tr};

const RAT: &str = "oteryn:creature.rat";

fn selected(client: &DevClientSession) -> TestResult {
    expect(
        client
            .selected_capabilities()
            .contains(&tr::CAPABILITY_CYCLOPEDIA_TRACKER_V1),
        "capability",
        "needed=capability-197-selected-at-join",
    )
}

/// One tracker command; the reply snapshot is `None` for an empty refusal payload.
async fn call(
    client: &mut DevClientSession,
    command: u32,
    request: &tr::TrackerRequest,
) -> TestResult<(CommandStatus, Option<tr::TrackerSnapshot>)> {
    let payload =
        tr::encode_request(request).ok_or("step=encode;observed=request-not-encodable")?;
    let reply = client
        .session_mut()
        .gated_command(tr::CAPABILITY_CYCLOPEDIA_TRACKER_V1, command, &payload)
        .await?;
    let snapshot = if reply.payload.is_empty() {
        None
    } else {
        Some(
            tr::decode_snapshot(&reply.payload)
                .ok_or("step=decode;observed=undecodable-tracker-snapshot")?,
        )
    };
    Ok((reply.status, snapshot))
}

fn change(action: tr::TrackerAction, key: &str) -> tr::TrackerRequest {
    tr::TrackerRequest::Change {
        action,
        creature_key: key.to_owned(),
    }
}

fn has(snapshot: &Option<tr::TrackerSnapshot>, key: &str) -> bool {
    snapshot
        .as_ref()
        .is_some_and(|list| list.entries.iter().any(|entry| entry.creature_key == key))
}

async fn read(
    client: &mut DevClientSession,
    command: u32,
) -> TestResult<Option<tr::TrackerSnapshot>> {
    let (status, list) = call(client, command, &tr::TrackerRequest::Read).await?;
    expect(status == CommandStatus::Accepted, "read", status)?;
    Ok(list)
}

const GAP: &str = "source:crates/session/src/domain_dispatch.rs:346";

fn kills(snapshot: &Option<tr::TrackerSnapshot>, key: &str) -> u32 {
    snapshot
        .as_ref()
        .and_then(|list| list.entries.iter().find(|entry| entry.creature_key == key))
        .map_or(0, |entry| entry.kill_count)
}

/// Whether a session failure is the refusal of the pushed tracker domain.
fn is_domain_gap(error: &str) -> bool {
    let lowered = error.to_ascii_lowercase();
    lowered.contains("unsupportedpusheddomain")
        || (lowered.contains("unsupported") && lowered.contains("98"))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let best = tr::COMMAND_TYPE_BESTIARY_TRACKER;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xd6).await?;
    selected(&client)?;
    let (status, list) = call(&mut client, best, &change(tr::TrackerAction::Add, RAT)).await?;
    expect(
        status == CommandStatus::Accepted && has(&list, RAT),
        "add",
        status,
    )?;
    let before = kills(&list, RAT);

    travel_to_spawn(&mut client, world, &metadata.spawns, RAT).await?;
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry.position.floor == here.floor
                    && entry
                        .position
                        .x
                        .abs_diff(here.x)
                        .max(entry.position.y.abs_diff(here.y))
                        <= 1
            })
        })
        .copied()
        .ok_or("step=hunt;observed=no-adjacent-creature")?;
    client.take_events();
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    expect(
        attack.disposition == AttackIntentDisposition::Ok,
        "attack",
        attack.disposition,
    )?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut push_error: Option<String> = None;
    let mut killed = false;
    while tokio::time::Instant::now() < deadline {
        if let Err(error) = client.service_liveness(Duration::from_millis(50)).await {
            push_error = Some(error.to_string());
            break;
        }
        client.take_events();
        if client
            .world_entities()
            .is_some_and(|entities| entities.get(&target.entity).is_none())
        {
            killed = true;
            break;
        }
    }
    drop(client);

    let mut fresh = connect_world(clients, 0xd7).await?;
    let list = read(&mut fresh, best).await?;
    let after = kills(&list, RAT);
    // Leave the tracker as found.
    call(&mut fresh, best, &change(tr::TrackerAction::Remove, RAT)).await?;
    logout(&mut fresh).await?;
    expect(
        after == before + 1,
        "kill-credit",
        format!("before:{before},after:{after},killed:{killed},error:{push_error:?}"),
    )?;
    match push_error {
        Some(error) if is_domain_gap(&error) => Ok(("MISSING-CLIENT", GAP.to_owned())),
        Some(error) => Err(format!("step=live-refresh;observed=session-error:{error}").into()),
        None => Err("step=live-refresh;observed=no-domain-98-push-and-no-client-refusal".into()),
    }
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=P22d.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

/// One evidence token: no whitespace, bounded length.
fn token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:/=<>,;".contains(character) {
                character
            } else {
                '-'
            }
        })
        .take(240)
        .collect()
}

fn expect(ok: bool, step: &str, observed: impl std::fmt::Debug) -> TestResult {
    if ok {
        Ok(())
    } else {
        Err(format!("step={step};observed={observed:?}").into())
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    expect(
        reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
        "logout",
        reply.result,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_no_whitespace_and_is_bounded() {
        let value = token(&format!("step=a b\tc\n{}", "x".repeat(400)));
        assert!(!value.contains(char::is_whitespace));
        assert!(value.len() <= 240);
        assert!(value.starts_with("step=a-b-c-"));
    }

    #[test]
    fn domain_gap_is_recognised_from_the_session_error() {
        assert!(is_domain_gap("UnsupportedPushedDomain { domain_id: 98 }"));
        assert!(!is_domain_gap("connection reset"));
    }
}
