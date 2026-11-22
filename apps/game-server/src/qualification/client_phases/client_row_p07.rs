//! P07 client replay capture: the real Atelier current-state frames (gems, quotes, resource
//! balances) and a stale-revision Reveal refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0148-wheel-allocation/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::wheel_allocation::{self, WheelCommand, WheelResponse};
use oteryn_protocol_oteryn::{atelier, wheel};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("P07_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CurrentAtelier {
    V1(atelier::AtelierReply),
    V2(atelier::v2::AtelierReply),
}

impl CurrentAtelier {
    fn refused(&self) -> bool {
        match self { Self::V1(value) => value.refused, Self::V2(value) => value.refused }
    }

    fn revision(&self) -> u64 {
        match self { Self::V1(value) => value.revision, Self::V2(value) => value.revision }
    }

    fn counts_and_quote(&self) -> (usize, usize, u64) {
        match self {
            Self::V1(value) => (value.gems.len(), value.grades.len(), value.quality_quotes.first().map_or(0, |q| q.reveal_gold)),
            Self::V2(value) => (value.gems.len(), value.grades.len(), value.quality_quotes.first().map_or(0, |q| q.reveal_gold)),
        }
    }
}

fn decode_current_atelier(bytes: &[u8], atelier_v2: bool) -> TestResult<CurrentAtelier> {
    if atelier_v2 {
        return atelier::v2::decode_reply(bytes)
            .map(CurrentAtelier::V2)
            .map_err(|error| format!("current Atelier v2: {error:?}").into());
    }
    atelier::decode_reply(bytes)
        .map(CurrentAtelier::V1)
        .map_err(|error| format!("current Atelier: {error:?}").into())
}

async fn send(
    client: &mut DevClientSession,
    kind: u32,
    payload: Vec<u8>,
    name: &str,
) -> TestResult<(WheelResponse, CurrentAtelier)> {
    let command = WheelCommand::new(kind, payload).ok_or("Wheel runtime rejected the command")?;
    let response = wheel_allocation::send(client.session_mut(), &command).await?;
    if !matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Err(format!("{name} status={:?}", response.status).into());
    }
    capture(&format!("p07-{name}-receipt.bin"), &response.payload);
    for (suffix, frame) in [
        ("wheel", &response.wheel),
        ("atelier", &response.atelier),
        ("resources", &response.resources),
    ] {
        if let Some(bytes) = frame {
            capture(&format!("p07-{name}-{suffix}.bin"), bytes);
        }
    }
    let current = response
        .atelier
        .as_deref()
        .ok_or("current Atelier frame absent")?;
    let current = decode_current_atelier(
        current,
        client.selected_capabilities().contains(&atelier::v2::CAPABILITY_GEM_ATELIER_V2),
    )
    .map_err(|error| format!("{name} {error}"))?;
    Ok((response, current))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x29).await?;
    if !client
        .selected_capabilities()
        .contains(&wheel::CAPABILITY_WHEEL_ALLOCATION_V1)
    {
        return Err("admission omitted Atelier capability 148".into());
    }
    let atelier_v2 = client.selected_capabilities().contains(&atelier::v2::CAPABILITY_GEM_ATELIER_V2);
    capture("p07-atelier-profile.txt", format!("atelier_v2={}\n", u8::from(atelier_v2)).as_bytes());
    let (query, initial) = send(
        &mut client,
        wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
        vec![8, 0],
        "query",
    )
    .await?;
    if initial.refused() {
        return Err(format!("Atelier query refused: {initial:?}").into());
    }
    let balances = query
        .resources
        .as_deref()
        .and_then(|bytes| atelier::resources::decode_reply(bytes).ok())
        .ok_or("resource balance frame absent or malformed")?;
    let mut occurrence = [0x29; 16];
    occurrence[6] = 0x70;
    occurrence[8] = 0x80;
    let (kind, payload) = if atelier_v2 {
        (
            atelier::v2::COMMAND_TYPE_ATELIER_V2_MUTATE,
            atelier::v2::encode_mutate(&atelier::v2::MutateRequest {
                occurrence,
                expected_revision: initial.revision() + 99,
                mutation: atelier::v2::Mutation::Reveal { quality: 0 },
            }),
        )
    } else {
        (
            atelier::COMMAND_TYPE_ATELIER_MUTATE,
            atelier::encode_mutate(&atelier::MutateRequest {
                occurrence,
                expected_revision: initial.revision() + 99,
                mutation: atelier::Mutation::Reveal { quality: 0 },
            }),
        )
    };
    let payload = payload.map_err(|error| format!("reveal encoder: {error:?}"))?;
    let (stale, after) = send(&mut client, kind, payload, "stale").await?;
    let receipt = decode_current_atelier(&stale.payload, atelier_v2)
        .map_err(|error| format!("stale receipt: {error}"))?;
    if !receipt.refused() || after != initial {
        return Err(format!("stale Reveal was not refused unchanged: {receipt:?}").into());
    }
    let (gems, grades, reveal_gold) = initial.counts_and_quote();
    let expected = format!(
        "revision={}\ngems={}\ngrades={}\nreveal_gold_q0={}\ncarried_gold={}\nstale_refused={}\nstale_current_revision={}\n",
        initial.revision(),
        gems,
        grades,
        reveal_gold,
        balances.carried_gold,
        receipt.refused(),
        after.revision()
    );
    capture("p07-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P07.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P07.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P07.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_v2() -> atelier::v2::AtelierReply {
        atelier::v2::AtelierReply {
            refused: false,
            revision: 3,
            wheel_revision: 9,
            in_temple: true,
            premium: true,
            gems: Vec::new(),
            grades: Vec::new(),
            quality_quotes: vec![atelier::v2::QualityQuote {
                quality: 0, reveal_gold: 50_000, fragment_kind: 0,
                fragment_min: 1, fragment_max: 2,
            }],
            grade_quotes: Vec::new(),
            vessels: vec![atelier::v2::VesselSlotView { quality: 0, open: true, gem: None }; 12],
            vessel_nodes: vec![1, 5, 9],
            vessel_node_bonus: 7,
            resistance_capped: true,
        }
    }

    #[test]
    fn captured_current_atelier_decodes_negotiated_v2_without_losing_fields() -> TestResult {
        let expected = current_v2();
        let payload = atelier::v2::encode_reply(&expected).map_err(|error| format!("{error:?}"))?;
        assert_eq!(decode_current_atelier(&payload, true)?, CurrentAtelier::V2(expected));
        assert!(decode_current_atelier(&payload, false).is_err());
        let mut malformed = payload;
        malformed.push(0xff);
        assert!(decode_current_atelier(&malformed, true).is_err());
        Ok(())
    }

    #[test]
    fn unchanged_atelier_compares_v2_only_state() {
        let before = current_v2();
        let mut after = before.clone();
        after.vessel_node_bonus += 1;
        assert_ne!(CurrentAtelier::V2(before), CurrentAtelier::V2(after));
    }
}
