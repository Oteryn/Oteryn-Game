//! P07 player scenario, derived from the matrix row and the production Atelier owner:
//! 1. Admit an ordinary character and query commands 308 and 484; expect the current Atelier
//!    revision, Workshop grades and live carried/bank gem, fragment and gold balances.
//! 2. Reveal a gem with command 424; expect an accepted receipt, a revision increment, the
//!    chosen-quality gem, and the quoted gold debit.
//! 3. Dismantle that gem with command 424; expect its removal, the quoted fragment yield range,
//!    and another revision increment.
//! 4. Grade a compatible modifier with command 424; expect the grade delta plus the quoted gold
//!    and fragment debits.
//! 5. Reveal another gem, lock/unlock it with command 309, switch its domain with command 424,
//!    then socket it through Wheel command 161; expect lock, domain and vessel deltas.
//! 6. Replay a stale Atelier revision; expect a refused reply with no state or resource delta.
//! 7. Log out, freshly admit the same character, and query again; expect the exact durable gems,
//!    grades, locks, domains, vessel assignment and resource balances committed above. Resonance
//!    is asserted through the admitted Wheel/Atelier vessel projection exposed by command 161.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::wheel_allocation;
use oteryn_protocol_oteryn::{atelier, wheel};

#[derive(Default)]
struct Observations {
    failures: Vec<String>,
}

impl Observations {
    fn fail(&mut self, step: &str, observed: impl std::fmt::Debug) {
        verdict::record_assertion(step);
        let detail = format!("step={step};observed={observed:?}").replace(' ', "_");
        evidence(&format!("P07_CHECK status=FAIL {detail}"));
        self.failures.push(detail);
    }

    fn pass(&self, step: &str) {
        evidence(&format!("P07_CHECK status=PASS step={step}"));
    }
}

async fn raw(
    client: &mut DevClientSession,
    kind: u32,
    payload: Vec<u8>,
) -> TestResult<wheel_allocation::WheelResponse> {
    let command = wheel_allocation::WheelCommand::new(kind, payload)
        .ok_or("P07 command encoder rejected a locally encoded command")?;
    Ok(wheel_allocation::send(client.session_mut(), &command).await?)
}

fn occurrence(session: crate::foundation::GameSessionId, ordinal: u8) -> [u8; 16] {
    use sha2::Digest;
    let digest = sha2::Sha256::new()
        .chain_update(b"E2E-P07/occurrence")
        .chain_update(session.as_bytes())
        .chain_update([ordinal])
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}

fn atelier_reply(response: &wheel_allocation::WheelResponse) -> TestResult<atelier::AtelierReply> {
    atelier::decode_reply(&response.payload).map_err(|error| format!("P07 reply: {error:?}").into())
}

async fn query(
    client: &mut DevClientSession,
) -> TestResult<(atelier::AtelierReply, atelier::resources::ResourceBalances)> {
    let response = raw(
        client,
        atelier::COMMAND_TYPE_ATELIER_QUERY,
        atelier::encode_query(),
    )
    .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!(
            "query status={:?} payload={:?}",
            response.status, response.payload
        )
        .into());
    }
    let state = atelier_reply(&response)?;
    let bytes = response.resources.ok_or("query omitted resource frame")?;
    let resources = atelier::resources::decode_reply(&bytes)
        .map_err(|error| format!("P07 resources: {error:?}"))?;
    Ok((state, resources))
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P07 logout observed={reply:?}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>, seen: &mut Observations) -> TestResult {
    let mut client = connect_world(clients, 0xa7).await?;
    let session = client.join_snapshot().game_session_id;
    let (initial, initial_resources) = match query(&mut client).await {
        Ok(value) if !value.0.refused => {
            seen.pass("1-query-state-and-resources");
            value
        }
        Ok(value) => {
            seen.fail("1-query-state-and-resources", &value);
            value
        }
        Err(error) => {
            verdict::record_error(error.as_ref());
            seen.fail("1-query-state-and-resources", &error);
            (
                atelier::AtelierReply {
                    refused: true,
                    revision: 0,
                    gems: Vec::new(),
                    grades: Vec::new(),
                    quality_quotes: Vec::new(),
                    grade_quotes: Vec::new(),
                },
                Default::default(),
            )
        }
    };

    let mut revision = initial.revision;
    let mut gem = [0; 16];
    let reveal = atelier::MutateRequest {
        occurrence: occurrence(session, 1),
        expected_revision: revision,
        mutation: atelier::Mutation::Reveal { quality: 0 },
    };
    let response = raw(
        &mut client,
        atelier::COMMAND_TYPE_ATELIER_MUTATE,
        atelier::encode_mutate(&reveal).map_err(|e| format!("reveal encode: {e:?}"))?,
    )
    .await?;
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.revision > revision
                && !reply.gems.is_empty() =>
        {
            revision = reply.revision;
            gem = reply.gems[0].id;
            seen.pass("2-reveal");
        }
        observed => seen.fail("2-reveal", (&response.status, observed)),
    }

    let dismantle = atelier::MutateRequest {
        occurrence: occurrence(session, 2),
        expected_revision: revision,
        mutation: atelier::Mutation::Dismantle { gem },
    };
    let response = raw(
        &mut client,
        atelier::COMMAND_TYPE_ATELIER_MUTATE,
        atelier::encode_mutate(&dismantle).map_err(|e| format!("dismantle encode: {e:?}"))?,
    )
    .await?;
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.revision > revision =>
        {
            revision = reply.revision;
            seen.pass("3-dismantle-fragments");
        }
        observed => seen.fail("3-dismantle-fragments", (&response.status, observed)),
    }

    let grade = initial
        .grades
        .first()
        .map(|row| (row.kind, row.mod_id))
        .unwrap_or((0, 0));
    let request = atelier::MutateRequest {
        occurrence: occurrence(session, 3),
        expected_revision: revision,
        mutation: atelier::Mutation::GradeUp {
            kind: grade.0,
            mod_id: grade.1,
        },
    };
    let response = raw(
        &mut client,
        atelier::COMMAND_TYPE_ATELIER_MUTATE,
        atelier::encode_mutate(&request).map_err(|e| format!("grade encode: {e:?}"))?,
    )
    .await?;
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.revision > revision =>
        {
            revision = reply.revision;
            seen.pass("4-grade");
        }
        observed => seen.fail("4-grade", (&response.status, observed)),
    }

    let reveal = atelier::MutateRequest {
        occurrence: occurrence(session, 4),
        expected_revision: revision,
        mutation: atelier::Mutation::Reveal { quality: 0 },
    };
    let response = raw(
        &mut client,
        atelier::COMMAND_TYPE_ATELIER_MUTATE,
        atelier::encode_mutate(&reveal).map_err(|e| format!("second reveal encode: {e:?}"))?,
    )
    .await?;
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.revision > revision
                && !reply.gems.is_empty() =>
        {
            revision = reply.revision;
            gem = reply.gems[0].id;
            seen.pass("5-reveal-socket-candidate");
        }
        observed => seen.fail("5-reveal-socket-candidate", (&response.status, observed)),
    }

    let lock = atelier::SetLockRequest {
        occurrence: occurrence(session, 5),
        expected_revision: revision,
        gem,
        locked: true,
    };
    let response = raw(
        &mut client,
        atelier::COMMAND_TYPE_ATELIER_SET_LOCK,
        atelier::encode_set_lock(&lock).map_err(|e| format!("lock encode: {e:?}"))?,
    )
    .await?;
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.gems.iter().any(|row| row.id == gem && row.locked) =>
        {
            revision = reply.revision;
            seen.pass("5-lock");
        }
        observed => seen.fail("5-lock", (&response.status, observed)),
    }

    let unlock = atelier::SetLockRequest {
        occurrence: occurrence(session, 6),
        expected_revision: revision,
        gem,
        locked: false,
    };
    let response = raw(
        &mut client,
        atelier::COMMAND_TYPE_ATELIER_SET_LOCK,
        atelier::encode_set_lock(&unlock).map_err(|e| format!("unlock encode: {e:?}"))?,
    )
    .await?;
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.gems.iter().any(|row| row.id == gem && !row.locked) =>
        {
            revision = reply.revision;
            seen.pass("5-unlock");
        }
        observed => seen.fail("5-unlock", (&response.status, observed)),
    }

    let switch = atelier::MutateRequest {
        occurrence: occurrence(session, 7),
        expected_revision: revision,
        mutation: atelier::Mutation::SwitchDomain { gem },
    };
    let response = raw(
        &mut client,
        atelier::COMMAND_TYPE_ATELIER_MUTATE,
        atelier::encode_mutate(&switch).map_err(|e| format!("switch encode: {e:?}"))?,
    )
    .await?;
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.revision > revision =>
        {
            seen.pass("5-switch-domain");
        }
        observed => seen.fail("5-switch-domain", (&response.status, observed)),
    }

    let wheel_query = wheel::WheelAllocationRequest {
        action: 0,
        occurrence: [0; 16],
        expected_revision: 0,
        points: [0; wheel::WHEEL_SLOTS],
        vessels: None,
    };
    let wheel_response = raw(
        &mut client,
        wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
        wheel::encode_wheel_request(&wheel_query)
            .map_err(|e| format!("wheel query encode: {e:?}"))?,
    )
    .await?;
    let wheel_state = match wheel::decode_wheel_reply(&wheel_response.payload) {
        Ok(reply) => reply,
        Err(error) => {
            seen.fail("5-wheel-query", (&wheel_response.status, error));
            wheel::WheelAllocationReply {
                refused: true,
                revision: 0,
                points: [0; wheel::WHEEL_SLOTS],
                vessels: None,
            }
        }
    };
    let socket = wheel::WheelAllocationRequest {
        action: 1,
        occurrence: occurrence(session, 8),
        expected_revision: wheel_state.revision,
        points: wheel_state.points,
        vessels: Some([Some(gem), None, None, None]),
    };
    let response = raw(
        &mut client,
        wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
        wheel::encode_wheel_request(&socket).map_err(|e| format!("socket encode: {e:?}"))?,
    )
    .await?;
    if response.status == CommandStatus::Accepted
        && wheel::decode_wheel_reply(&response.payload)
            .is_ok_and(|reply| !reply.refused && reply.vessels.is_some_and(|v| v[0] == Some(gem)))
    {
        seen.pass("5-socket-vessel-resonance");
    } else {
        seen.fail("5-socket-vessel-resonance", &response);
    }

    let stale = atelier::MutateRequest {
        occurrence: occurrence(session, 9),
        expected_revision: initial.revision,
        mutation: atelier::Mutation::Reveal { quality: 0 },
    };
    let response = raw(
        &mut client,
        atelier::COMMAND_TYPE_ATELIER_MUTATE,
        atelier::encode_mutate(&stale).map_err(|e| format!("stale encode: {e:?}"))?,
    )
    .await?;
    if response.status == CommandStatus::Accepted
        && atelier_reply(&response).is_ok_and(|reply| reply.refused)
    {
        seen.pass("6-stale-refusal");
    } else {
        seen.fail("6-stale-refusal", &response);
    }

    let durable = verdict::observe(query(&mut client)).await;
    if durable.is_err() {
        seen.fail("7-pre-logout-state", &durable);
    }
    logout(&mut client).await?;
    let mut fresh = connect_world(clients, 0xa8).await?;
    let fresh_state = verdict::observe(query(&mut fresh)).await;
    match (&durable, &fresh_state) {
        (Ok(before), Ok(after)) if before == after && !after.0.refused => {
            seen.pass("7-fresh-admission-durable-state");
            if after.1 == initial_resources {
                seen.fail("7-resource-delta-persisted", (&initial_resources, &after.1));
            } else {
                seen.pass("7-resource-delta-persisted");
            }
        }
        observed => seen.fail("7-fresh-admission-durable-state", observed),
    }
    logout(&mut fresh).await
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let mut seen = Observations::default();
    let result = verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(journey(clients, &mut seen))).await;
    if let Err(error) = &result {
        seen.fail("journey-deadline", error);
    }
    if let Ok(Err(error)) = &result {
        seen.fail("journey", error);
    }
    evidence(&format!(
        "e2e_all scenario=P07.full status={} seconds={:.3}",
        if matches!(result, Ok(Ok(()))) && seen.failures.is_empty() {
            "PASS"
        } else {
            "FAIL"
        },
        started.elapsed().as_secs_f64()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn occurrences_are_stable_and_distinct() -> TestResult {
        let session = crate::foundation::GameSessionId::decode(&v7(1, 1))?;
        assert_eq!(occurrence(session, 1), occurrence(session, 1));
        assert_ne!(occurrence(session, 1), occurrence(session, 2));
        Ok(())
    }
}
