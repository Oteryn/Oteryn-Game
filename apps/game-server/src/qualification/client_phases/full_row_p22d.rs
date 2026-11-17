//! P22d (live tracker refresh on kill progress):
//! 1. Admit the node-boot character, track a race and note its kill count.
//! 2. Walk to a compiled spawn of that race, kill one creature and wait for the live frames.
//! 3. Require a decoded Cyclopedia tracker update for that race and matching fresh-admission
//!    credit. A typed unsupported tracker domain with durable credit identifies a decoder gap.
use super::*;
use oteryn_dev_client::{DevClientError, DevClientSession, EntityKind, SessionError, SessionEvent};
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

const GAP: &str = "tracker-domain-98-unsupported-with-durable-credit";

fn kills(snapshot: &Option<tr::TrackerSnapshot>, key: &str) -> u32 {
    snapshot
        .as_ref()
        .and_then(|list| list.entries.iter().find(|entry| entry.creature_key == key))
        .map_or(0, |entry| entry.kill_count)
}

/// Whether a session failure is the refusal of the pushed tracker domain.
fn is_domain_gap(mut error: &DevClientError) -> bool {
    while let DevClientError::Context { source, .. } = error {
        error = source;
    }
    matches!(error, DevClientError::Session(SessionError::UnsupportedPushedDomain {
        domain_id: tr::STATE_DOMAIN_CYCLOPEDIA_TRACKER
    }))
}

fn classify_refresh(before: u32, durable: u32, pushed: Option<u32>, error: Option<&DevClientError>) -> TestResult<Verdict> {
    expect(before.checked_add(1) == Some(durable), "kill-credit", (before, durable))?;
    match error {
        Some(error) if is_domain_gap(error) => Ok(("MISSING-CLIENT", GAP.to_owned())),
        Some(error) => Err(format!("step=live-refresh;observed=session-error:{error}").into()),
        None if pushed == Some(durable) => Ok(("PASS", format!("live-and-durable-tracker-kill-count:{durable}"))),
        None => Err(format!("step=live-refresh;observed=pushed:{pushed:?};needed={durable}").into()),
    }
}

fn pushed_kills(events: Vec<SessionEvent>, after: u64) -> TestResult<Option<u32>> {
    let mut count = None;
    for event in events {
        if let SessionEvent::GatedSnapshotReplaced(delta) = event
            && delta.value.domain_id == tr::STATE_DOMAIN_CYCLOPEDIA_TRACKER
            && delta.server_sequence > after
        {
            let (bestiary, _) = tr::decode_domain_snapshot(&delta.value.payload)
                .ok_or("step=live-refresh;observed=undecodable-tracker-snapshot")?;
            count = bestiary.entries.iter().find(|entry| entry.creature_key == RAT).map(|entry| entry.kill_count);
        }
    }
    Ok(count)
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
    let was_tracked = has(&read(&mut client, best).await?, RAT);
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
                    && entry.display_name.is_some_and(|name| name.as_str().is_ok_and(|name| name == "Rat"))
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
        .ok_or("step=hunt;observed=no-adjacent-rat")?;
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
    verdict::Harness::global().bounded_wait(deadline, "tracker-push");
    let mut pushed = None;
    let mut push_error = None;
    while tokio::time::Instant::now() < deadline {
        if let Some(count) = pushed_kills(client.take_events(), attack.result_server_sequence)?
            && count != before
        {
            pushed = Some(count);
            break;
        }
        match tokio::time::timeout_at(deadline, client.service_liveness(Duration::from_millis(50))).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) if is_domain_gap(&error) => {
                push_error = Some(error);
                break;
            }
            Ok(Err(error)) => return Err(error.into()),
            Err(_) => return Err("step=live-refresh;observed=tracker-push-timeout".into()),
        }
    }
    if push_error.is_none() {
        logout(&mut client).await?;
    }
    drop(client);

    let mut fresh = connect_world(clients, 0xd7).await?;
    let list = read(&mut fresh, best).await?;
    let after = kills(&list, RAT);
    if !was_tracked {
        let (status, list) = call(&mut fresh, best, &change(tr::TrackerAction::Remove, RAT)).await?;
        expect(status == CommandStatus::Accepted && !has(&list, RAT), "cleanup", status)?;
    }
    logout(&mut fresh).await?;
    classify_refresh(before, after, pushed, push_error.as_ref())
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
    fn tracker_refresh_classifier_requires_domain_identity_and_durable_credit() {
        use oteryn_dev_client::{DevClientError, SessionError};
        let wrong = DevClientError::Session(SessionError::UnsupportedPushedDomain { domain_id: 97 });
        let tracker = DevClientError::Session(SessionError::UnsupportedPushedDomain { domain_id: tr::STATE_DOMAIN_CYCLOPEDIA_TRACKER });
        assert!(classify_refresh(3, 4, None, Some(&wrong)).is_err());
        assert!(matches!(classify_refresh(3, 4, None, Some(&tracker)), Ok(("MISSING-CLIENT", _))));
        assert!(classify_refresh(3, 3, Some(4), None).is_err());
    }

    #[test]
    fn decoded_tracker_refresh_and_matching_durable_credit_pass() {
        assert!(matches!(classify_refresh(3, 4, Some(4), None), Ok(("PASS", _))));
        assert!(classify_refresh(3, 4, Some(5), None).is_err());
    }

    #[test]
    fn token_has_no_whitespace_and_is_bounded() {
        let value = token(&format!("step=a b\tc\n{}", "x".repeat(400)));
        assert!(!value.contains(char::is_whitespace));
        assert!(value.len() <= 240);
        assert!(value.starts_with("step=a-b-c-"));
    }

    #[test]
    fn domain_gap_is_recognised_from_the_session_error() {
        assert!(is_domain_gap(&DevClientError::Session(SessionError::UnsupportedPushedDomain {
            domain_id: tr::STATE_DOMAIN_CYCLOPEDIA_TRACKER,
        })));
        assert!(!is_domain_gap(&DevClientError::Io(std::io::ErrorKind::ConnectionReset.into())));
    }
}
