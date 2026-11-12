//! W12 full-row player scenario, derived from the matrix row, the Crystal reaudit,
//! and the production Cyclopedia owner (`gameplay_transport/cyclopedia.rs`):
//! 1. Admit the node-boot Character and send a Cyclopedia query; expect an accepted,
//!    decodable state frame identifying revision, discovery, POIs, donation pool,
//!    reset epoch and improved-respawn state.
//! 2. Send FIND before starting exploration; expect an accepted result frame with
//!    refusal code 4 and no discovery, POI, pool, epoch or respawn-state mutation.
//! 3. Send START.  A premium Character must receive code 0 and seven server-selected
//!    POIs; a non-premium Character must receive code 2 with unchanged durable state.
//! 4. When START succeeds, discover the POIs only by walking the Character through
//!    the compiled world and sending FIND at each server-revealed coordinate; expect
//!    found-count deltas culminating in discovered=true, found=7 and 100 percent.
//! 5. Send a one-gold DONATE through the ordinary physical trade path; expect either
//!    code 0 with an exact +1 pool delta or code 6 with no delta when player-earned
//!    funds are insufficient.  Then really attempt the 10,000,000-gold goal; expect
//!    code 0, pool/epoch/improved deltas and the improved respawn selection, or record
//!    code 6 as the observed refusal (the test never mints money).
//! 6. Send a malformed/unknown-area intent; expect the command-status refusal with no
//!    result payload, then query again to prove that the connection and state survived.
//! 7. Logout normally, make a fresh admission, query again, and require the discovery,
//!    donation pool, epoch and improved-respawn state from the last accepted action to
//!    be durable.  Movement-speed parity is not inferred from travel time: W12 needs a
//!    server result/delta exposing the donated-area respawn speed before it can PASS.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::cyclopedia as wire;

const SOURCE_GAP: &str = "source:crates/gameplay/domain/src/domain/cyclopedia/basic.rs:6";

#[derive(Debug)]
struct Failure {
    step: u8,
    observed: String,
}

impl Failure {
    fn new(step: u8, observed: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            step,
            observed: observed.into(),
        })
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "step-{}:{}", self.step, self.observed)
    }
}

impl std::error::Error for Failure {}

fn intent(operation: u8, area: u32, amount: u32) -> [u8; wire::MAX_REQUEST_BYTES] {
    let mut bytes = [0; wire::MAX_REQUEST_BYTES];
    bytes[0] = operation;
    bytes[1..5].copy_from_slice(&area.to_le_bytes());
    bytes[5..9].copy_from_slice(&amount.to_le_bytes());
    bytes
}

async fn exchange(
    client: &mut DevClientSession,
    kind: u32,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(wire::CAPABILITY_CYCLOPEDIA_MAP_V1, kind, payload)
        .await?)
}

async fn query(client: &mut DevClientSession, step: u8) -> Result<wire::MapResult, Box<Failure>> {
    let reply = exchange(client, wire::COMMAND_TYPE_CYCLOPEDIA_QUERY, &[])
        .await
        .map_err(|error| Failure::new(step, format!("query-error={error}")))?;
    if reply.status != CommandStatus::Accepted {
        return Err(Failure::new(step, format!("query-reply={reply:?}")));
    }
    wire::decode_result(&reply.payload)
        .map_err(|error| Failure::new(step, format!("result-decode={error:?};reply={reply:?}")))
}

async fn action(
    client: &mut DevClientSession,
    step: u8,
    operation: u8,
    amount: u32,
) -> Result<wire::MapResult, Box<Failure>> {
    let reply = exchange(
        client,
        wire::COMMAND_TYPE_CYCLOPEDIA_INTENT,
        &intent(operation, 1, amount),
    )
    .await
    .map_err(|error| Failure::new(step, format!("command-error={error}")))?;
    if reply.status != CommandStatus::Accepted {
        return Err(Failure::new(step, format!("command-reply={reply:?}")));
    }
    wire::decode_result(&reply.payload)
        .map_err(|error| Failure::new(step, format!("result-decode={error:?};reply={reply:?}")))
}

fn stable(before: &wire::MapResult, after: &wire::MapResult) -> bool {
    before.discovered == after.discovered
        && before.found == after.found
        && before.area_percent == after.area_percent
        && before.pool == after.pool
        && before.epoch == after.epoch
        && before.improved == after.improved
}

async fn logout(client: &mut DevClientSession, step: u8) -> Result<(), Box<Failure>> {
    let reply = client
        .session_mut()
        .logout()
        .await
        .map_err(|error| Failure::new(step, format!("logout-error={error}")))?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(Failure::new(step, format!("logout-reply={reply:?}")));
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> Result<(), Box<Failure>> {
    let mut client = connect_world(clients, 0xb8)
        .await
        .map_err(|error| Failure::new(1, format!("admission={error}")))?;
    let first_session = client.join_snapshot().game_session_id;
    let initial = query(&mut client, 1).await?;

    let refused_find = action(&mut client, 2, 3, 0).await?;
    if refused_find.code != 4 || !stable(&initial, &refused_find) {
        return Err(Failure::new(
            2,
            format!("expected-code=4-and-no-delta;observed={refused_find:?}"),
        ));
    }

    let started = action(&mut client, 3, 1, 0).await?;
    if !matches!(started.code, 0 | 2) || (started.code == 2 && !stable(&initial, &started)) {
        return Err(Failure::new(
            3,
            format!("expected-start-or-premium-refusal;observed={started:?}"),
        ));
    }
    // START's private points are deliberately revealed only in the player's viewport.
    // Exercise every currently revealed point with real automatic-walk and FIND commands.
    let mut progress = started.clone();
    if started.code == 0 {
        for _ in 0..7 {
            let Some(&(x, y, floor)) = progress.visible.first() else {
                break;
            };
            let walked = client
                .session_mut()
                .target_walk(ActorPosition {
                    x: i32::from(x),
                    y: i32::from(y),
                    floor: i16::from(-floor),
                })
                .await
                .map_err(|error| Failure::new(4, format!("walk-to-poi={x},{y},{floor}:{error}")))?;
            if walked.status != CommandStatus::Accepted || walked.payload != [8, 1] {
                return Err(Failure::new(4, format!("walk-reply={walked:?}")));
            }
            let target = ActorPosition {
                x: i32::from(x),
                y: i32::from(y),
                floor: i16::from(-floor),
            };
            let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
            while client.world_spatial().actor_position != target {
                if tokio::time::Instant::now() >= deadline {
                    return Err(Failure::new(
                        4,
                        format!(
                            "walk-timeout target={target:?} observed={:?}",
                            client.world_spatial().actor_position
                        ),
                    ));
                }
                client
                    .service_liveness(Duration::from_millis(100))
                    .await
                    .map_err(|error| Failure::new(4, format!("walk-liveness={error}")))?;
            }
            let before = progress.found;
            progress = action(&mut client, 4, 3, 0).await?;
            if progress.code != 0 || (!progress.discovered && progress.found <= before) {
                return Err(Failure::new(4, format!("find-no-delta={progress:?}")));
            }
        }
    }

    let before_donation = query(&mut client, 5).await?;
    let donated = action(&mut client, 5, 4, 1).await?;
    if !((donated.code == 0 && donated.pool == before_donation.pool + 1)
        || (donated.code == 6 && stable(&before_donation, &donated)))
    {
        return Err(Failure::new(5, format!("one-gold-donation={donated:?}")));
    }
    let goal = action(&mut client, 5, 4, 10_000_000).await?;

    let malformed = exchange(
        &mut client,
        wire::COMMAND_TYPE_CYCLOPEDIA_INTENT,
        &intent(1, 2, 0),
    )
    .await
    .map_err(|error| Failure::new(6, format!("malformed-command-error={error}")))?;
    if malformed.status != CommandStatus::Rejected || !malformed.payload.is_empty() {
        return Err(Failure::new(6, format!("malformed-reply={malformed:?}")));
    }
    let retained = query(&mut client, 6).await?;
    logout(&mut client, 7).await?;
    drop(client);

    let mut fresh = connect_world(clients, 0xb9)
        .await
        .map_err(|error| Failure::new(7, format!("fresh-admission={error}")))?;
    if fresh.join_snapshot().game_session_id == first_session {
        return Err(Failure::new(7, "fresh-admission-reused-session"));
    }
    let after = query(&mut fresh, 7).await?;
    let cleanup = logout(&mut fresh, 7).await;
    if !stable(&retained, &after) {
        return Err(Failure::new(
            7,
            format!("before={retained:?};after={after:?}"),
        ));
    }
    cleanup?;

    if started.code == 2 {
        return Err(Failure::new(3, format!("premium-required;reply={started:?}")));
    }
    if !progress.discovered || progress.found != 7 || progress.area_percent != 100 {
        return Err(Failure::new(4, format!("discovery-incomplete={progress:?}")));
    }
    if goal.code != 0 {
        return Err(Failure::new(5, format!("goal-donation-refused={goal:?}")));
    }
    // Reaching this point proves the playable discovery/donation lifecycle.  The owner
    // still emits no speed result/delta, so the full row cannot honestly claim PASS.
    Err(Failure::new(
        7,
        format!("movement-speed-delta-unavailable;{SOURCE_GAP}"),
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=W12.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => {
            let missing = error.observed.contains(SOURCE_GAP);
            evidence(&format!(
                "W12_CHECK step={} FAIL observed={}",
                error.step, error.observed
            ));
            evidence(&format!(
                "e2e_all scenario=W12.full status={} seconds={:.3}{}",
                if missing { "MISSING-SERVER" } else { "FAIL" },
                started.elapsed().as_secs_f64(),
                if missing {
                    format!(" evidence={SOURCE_GAP}")
                } else {
                    String::new()
                }
            ));
        }
        Err(_) => evidence(&format!(
            "e2e_all scenario=W12.full status=FAIL seconds={:.3} evidence=step-timeout",
            started.elapsed().as_secs_f64()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_encoder_and_durable_comparison_cover_the_wire_facts() {
        assert_eq!(
            wire::decode_request(wire::COMMAND_TYPE_CYCLOPEDIA_INTENT, &intent(4, 1, 7)),
            Ok(wire::MapIntent {
                operation: 4,
                amount: 7
            })
        );
        assert!(wire::decode_request(
            wire::COMMAND_TYPE_CYCLOPEDIA_INTENT,
            &intent(1, 2, 0)
        )
        .is_err());
        let state = wire::MapResult {
            code: 0,
            revision: 1,
            discovered: false,
            found: 0,
            area_percent: 0,
            pool: 0,
            epoch: 0,
            improved: false,
            visible: Vec::new(),
        };
        let mut refusal = state.clone();
        refusal.code = 4;
        refusal.revision = 2;
        assert!(stable(&state, &refusal));
        refusal.pool = 1;
        assert!(!stable(&state, &refusal));
    }
}
