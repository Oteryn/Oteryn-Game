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
use oteryn_protocol_oteryn::{atelier, wheel, wheel_presentation};

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

fn newly_revealed(before: &atelier::AtelierReply, after: &atelier::AtelierReply, quality: u8) -> Option<[u8; 16]> {
    let mut added = after.gems.iter().filter(|gem| !before.gems.iter().any(|old| old.id == gem.id));
    let gem = added.next()?;
    (added.next().is_none() && gem.quality == quality && !gem.locked && !gem.vessel
        && before.gems.iter().all(|old| after.gems.contains(old))).then_some(gem.id)
}

type AtelierState = (atelier::AtelierReply, atelier::resources::ResourceBalances);

fn stale_unchanged(refused: bool, before: &AtelierState, after: &AtelierState) -> bool {
    refused && before == after
}

fn gold_debit(before: &atelier::resources::ResourceBalances, after: &atelier::resources::ResourceBalances) -> Option<u64> {
    before.bank_gold.checked_add(before.carried_gold)?.checked_sub(after.bank_gold.checked_add(after.carried_gold)?)
}

fn fragment_total(resources: &atelier::resources::ResourceBalances, kind: u8) -> u64 {
    if kind == 0 { resources.basic_fragments } else { resources.supreme_fragments }
}

fn reveal_delta(before: &AtelierState, after: &AtelierState, quote: &atelier::QualityQuote) -> Option<[u8; 16]> {
    let gem = newly_revealed(&before.0, &after.0, quote.quality)?;
    let prior = &before.1;
    let current = &after.1;
    (after.0.revision > before.0.revision
        && gold_debit(prior, current) == Some(quote.reveal_gold)
        && current.lesser_gems.checked_add(1) == Some(prior.lesser_gems)
        && current.regular_gems == prior.regular_gems
        && current.greater_gems == prior.greater_gems
        && current.basic_fragments == prior.basic_fragments
        && current.supreme_fragments == prior.supreme_fragments)
        .then_some(gem)
}

fn dismantle_preserves_other_assets(before: &AtelierState, after: &AtelierState, gem: [u8; 16], kind: u8) -> bool {
    let mut expected = before.0.gems.clone();
    expected.retain(|entry| entry.id != gem);
    expected.len().checked_add(1) == Some(before.0.gems.len())
        && expected == after.0.gems
        && before.0.grades == after.0.grades
        && unchanged_resources(before.1, after.1, kind)
}

fn unchanged_resources(
    mut before: atelier::resources::ResourceBalances, after: atelier::resources::ResourceBalances, kind: u8,
) -> bool {
    match kind {
        0 => before.basic_fragments = after.basic_fragments,
        1 => before.supreme_fragments = after.supreme_fragments,
        _ => return false,
    }
    before.bank_gold = after.bank_gold;
    before.carried_gold = after.carried_gold;
    before == after
}

fn grade_preserves_other_assets(before: &AtelierState, after: &AtelierState, kind: u8, modifier: u16, grade: u8) -> bool {
    let mut expected = before.0.grades.clone();
    let Some(target) = expected.iter_mut().find(|row| (row.kind, row.mod_id) == (kind, modifier)) else {
        return false;
    };
    if target.grade.checked_add(1) != Some(grade) {
        return false;
    }
    target.grade = grade;
    expected == after.0.grades && before.0.gems == after.0.gems && unchanged_resources(before.1, after.1, kind)
}

fn socket_committed(reply: &wheel::WheelAllocationReply, after: &AtelierState, gem: [u8; 16]) -> bool {
    !reply.refused && !after.0.refused && reply.vessels.is_some_and(|v| v[0] == Some(gem))
        && after.0.gems.iter().any(|entry| entry.id == gem && entry.vessel && entry.domain == 0)
        && after.0.gems.iter().filter(|entry| entry.vessel).all(|entry|
            reply.vessels.and_then(|slots| slots.get(usize::from(entry.domain)).copied().flatten()) == Some(entry.id))
}

async fn socket_effect_is_live(
    client: &mut DevClientSession,
    wheel: &wheel::WheelAllocationReply,
    atelier_revision: u64,
    gem: [u8; 16],
) -> TestResult<bool> {
    let request = wheel_presentation::Request {
        nonce: 1,
        wheel_revision: wheel.revision,
        atelier_revision,
        points: wheel.points,
        vessels: wheel.vessels.ok_or("step=socket observed=no-vessel-receipt")?,
    };
    let payload = request.encode().ok_or("step=socket observed=invalid-presentation-request")?;
    let response = client.session_mut().gated_command(
        wheel_presentation::CAPABILITY_WHEEL_PRESENTATION_V1,
        wheel_presentation::COMMAND_TYPE_WHEEL_PRESENTATION,
        &payload,
    ).await?;
    let prefix = format!("Socketed gem {:032X}: ", u128::from_be_bytes(gem));
    Ok(response.status == CommandStatus::Accepted
        && wheel_presentation::Reply::decode(&response.payload).is_some_and(|view| {
            view.request == request && view.eligible && view.current.iter().any(|line| {
                line.strip_prefix(&prefix).and_then(|value| value.split_once(" active positions, effective grades "))
                    .is_some_and(|(positions, grades)| {
                        let grades = grades.split(", ").map(str::parse::<u8>).collect::<Result<Vec<_>, _>>();
                        grades.is_ok_and(|grades| !grades.is_empty()
                            && positions.parse::<usize>().ok() == Some(grades.len())
                            && grades.into_iter().all(|grade| (1..=4).contains(&grade)))
                    })
            })
        }))
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

    let quote = initial.quality_quotes.iter().find(|quote| quote.quality == 0)
        .ok_or("step=reveal observed=no-lesser-gem-quote")?;
    let baseline = (initial.clone(), initial_resources);
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
    let revealed = query(&mut client).await?;
    let new_gem = reveal_delta(&baseline, &revealed, quote);
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.revision > revision
                && new_gem.is_some_and(|id| reply.gems.iter().any(|entry| entry.id == id))
                && reply.revision == revealed.0.revision =>
        {
            revision = reply.revision;
            gem = new_gem.ok_or("step=reveal observed=no-new-gem")?;
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
    let dismantled = query(&mut client).await?;
    let fragments = fragment_total(&dismantled.1, quote.fragment_kind)
        .checked_sub(fragment_total(&revealed.1, quote.fragment_kind));
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.revision > revision
                && reply.revision == dismantled.0.revision
                && !reply.gems.iter().any(|entry| entry.id == gem)
                && dismantle_preserves_other_assets(&revealed, &dismantled, gem, quote.fragment_kind)
                && fragments.is_some_and(|count| (u64::from(quote.fragment_min)..=u64::from(quote.fragment_max)).contains(&count))
                && (revealed.1.bank_gold, revealed.1.carried_gold) == (dismantled.1.bank_gold, dismantled.1.carried_gold) =>
        {
            revision = reply.revision;
            seen.pass("3-dismantle-fragments");
        }
        observed => seen.fail("3-dismantle-fragments", (&response.status, observed)),
    }

    let target = dismantled.0.grades.iter().find(|row| row.grade < 3)
        .map(|row| (row.kind, row.mod_id, row.grade))
        .or_else(|| revealed.0.gems.iter().find(|entry| entry.id == gem)
            .and_then(|entry| entry.modifiers.first()).map(|id| (0, *id, 0)))
        .ok_or("step=grade needed=compatible-modifier-below-grade-cap")?;
    let grade = target.2.checked_add(1).ok_or("step=grade observed=invalid-grade")?;
    let grade_quote = dismantled.0.grade_quotes.iter().find(|quote| (quote.kind, quote.grade) == (target.0, grade))
        .ok_or("step=grade observed=no-next-grade-quote")?;
    let request = atelier::MutateRequest {
        occurrence: occurrence(session, 3),
        expected_revision: revision,
        mutation: atelier::Mutation::GradeUp {
            kind: target.0,
            mod_id: target.1,
        },
    };
    let response = raw(
        &mut client,
        atelier::COMMAND_TYPE_ATELIER_MUTATE,
        atelier::encode_mutate(&request).map_err(|e| format!("grade encode: {e:?}"))?,
    )
    .await?;
    let graded = query(&mut client).await?;
    let selected_grade = |state: &atelier::AtelierReply| state.grades.iter().any(|row| (row.kind, row.mod_id, row.grade) == (target.0, target.1, grade));
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.revision > revision
                && reply.revision == graded.0.revision
                && selected_grade(&reply) && grade_preserves_other_assets(&dismantled, &graded, target.0, target.1, grade)
                && gold_debit(&dismantled.1, &graded.1) == Some(grade_quote.gold)
                && graded.1.bank_gold <= dismantled.1.bank_gold && graded.1.carried_gold <= dismantled.1.carried_gold
                && fragment_total(&dismantled.1, target.0).checked_sub(fragment_total(&graded.1, target.0)) == Some(u64::from(grade_quote.fragments)) =>
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
    let second_reveal = query(&mut client).await?;
    let socket_gem = reveal_delta(&graded, &second_reveal, quote);
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.revision > revision
                && socket_gem.is_some_and(|id| reply.gems.iter().any(|entry| entry.id == id))
                && reply.revision == second_reveal.0.revision =>
        {
            revision = reply.revision;
            gem = socket_gem.ok_or("step=reveal observed=no-new-socket-gem")?;
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

    let before_switch = query(&mut client).await?;
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
    let switched = query(&mut client).await?;
    let prior_domain = before_switch.0.gems.iter().find(|entry| entry.id == gem).map(|entry| entry.domain);
    let next_domain = switched.0.gems.iter().find(|entry| entry.id == gem).map(|entry| entry.domain);
    match atelier_reply(&response) {
        Ok(reply)
            if response.status == CommandStatus::Accepted
                && !reply.refused
                && reply.revision > revision
                && reply.revision == switched.0.revision
                && prior_domain.is_some() && next_domain.is_some() && prior_domain != next_domain
                && reply.gems.iter().any(|entry| entry.id == gem && Some(entry.domain) == next_domain)
                && gold_debit(&before_switch.1, &switched.1) == Some(quote.switch_gold) =>
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
    let before_socket = query(&mut client).await?;
    if !before_socket.0.gems.iter().any(|entry| entry.id == gem && !entry.vessel) {
        return Err("step=socket needed=unsocketed-original-gem".into());
    }
    let mut vessels = [None; wheel::WHEEL_VESSELS];
    for entry in before_socket.0.gems.iter().filter(|entry| entry.vessel) {
        let Some(cell) = vessels.get_mut(usize::from(entry.domain)) else {
            return Err("step=socket observed=invalid-vessel-cell".into());
        };
        *cell = Some(entry.id);
    }
    vessels[0] = Some(gem);
    let socket = wheel::WheelAllocationRequest {
        action: 1,
        occurrence: occurrence(session, 8),
        expected_revision: wheel_state.revision,
        points: wheel_state.points,
        vessels: Some(vessels),
    };
    let response = raw(
        &mut client,
        wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
        wheel::encode_wheel_request(&socket).map_err(|e| format!("socket encode: {e:?}"))?,
    )
    .await?;
    let before_stale = query(&mut client).await?;
    if response.status == CommandStatus::Accepted
        && let Ok(reply) = wheel::decode_wheel_reply(&response.payload)
        && reply.revision > wheel_state.revision && reply.points == wheel_state.points
        && reply.vessels == Some(vessels)
        && socket_committed(&reply, &before_stale, gem)
        && socket_effect_is_live(&mut client, &reply, before_stale.0.revision, gem).await?
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
    let durable = verdict::observe(query(&mut client)).await;
    if response.status == CommandStatus::Accepted
        && initial.revision < before_stale.0.revision
        && durable.as_ref().is_ok_and(|after| stale_unchanged(
            atelier_reply(&response).is_ok_and(|reply| reply.refused),
            &before_stale, after,
        ))
    {
        seen.pass("6-stale-refusal");
    } else {
        seen.fail("6-stale-refusal", &response);
    }

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

#[cfg(test)]
mod proof_tests {
    use super::*;

    fn state(ids: &[u8]) -> atelier::AtelierReply {
        atelier::AtelierReply {
            refused: false, revision: 1,
            gems: ids.iter().map(|&id| atelier::GemView { id: [id; 16], quality: 0, domain: 0,
                locked: false, vessel: false, modifiers: vec![1] }).collect(),
            grades: Vec::new(), quality_quotes: Vec::new(), grade_quotes: Vec::new(),
        }
    }

    #[test]
    fn reveal_identifies_the_new_gem_when_old_gems_precede_it() {
        assert_eq!(newly_revealed(&state(&[1]), &state(&[1, 2]), 0), Some([2; 16]));
        assert_eq!(newly_revealed(&state(&[1]), &state(&[1]), 0), None);
    }

    #[test]
    fn stale_atelier_refusal_requires_unchanged_state_and_assets() {
        let before = (state(&[1]), atelier::resources::ResourceBalances::default());
        let after = (state(&[1, 2]), atelier::resources::ResourceBalances::default());
        assert!(!stale_unchanged(true, &before, &after));
        assert!(stale_unchanged(true, &before, &before));
    }
}

#[cfg(test)]
mod delta_tests {
    use super::*;

    fn assets() -> AtelierState {
        let gem = |id| atelier::GemView { id: [id; 16], quality: 0, domain: 0, locked: false, vessel: false, modifiers: vec![1] };
        let grade = |mod_id| atelier::GradeView { kind: 0, mod_id, grade: 1 };
        (atelier::AtelierReply { refused: false, revision: 1, gems: vec![gem(1), gem(2)],
            grades: vec![grade(1), grade(2)], quality_quotes: vec![], grade_quotes: vec![] },
         atelier::resources::ResourceBalances::default())
    }

    #[test]
    fn dismantle_and_grade_reject_collateral_mutations() {
        let before = assets();
        let mut dismantled = before.clone();
        dismantled.0.gems.clear();
        dismantled.1.supreme_fragments = 1;
        let mut graded = before.clone();
        graded.0.grades[0].grade = 2;
        graded.0.grades[1].grade = 2;
        assert_eq!((dismantle_preserves_other_assets(&before, &dismantled, [1; 16], 0),
            grade_preserves_other_assets(&before, &graded, 0, 1, 2)), (false, false));
    }

    #[test]
    fn socket_receipt_cannot_replace_live_socket_observation() {
        let receipt = wheel::WheelAllocationReply { refused: false, revision: 2,
            points: [0; wheel::WHEEL_SLOTS], vessels: Some([Some([1; 16]), None, None, None]) };
        let mut live = assets();
        assert!(!socket_committed(&receipt, &live, [1; 16]));
        live.0.gems[0].vessel = true;
        assert!(socket_committed(&receipt, &live, [1; 16]));
    }
}
