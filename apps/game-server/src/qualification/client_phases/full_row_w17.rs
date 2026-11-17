//! W17 player scenario, derived from the matrix, the 2026-10-28 re-audit, and
//! the admitted owners in `arena_catalogue.rs`, `arena_match.rs`, and
//! `tibiadrome.rs`:
//! 1. The admitted player requests the Svargrond catalogue; the server accepts
//!    it and returns Greenhorn, Scrapper, and Warlord with their fees, ten pits,
//!    and the authoritative 600-second pit limit.
//! 2. The player requests Svargrond status and observes the durable idle state.
//! 3. Before entering, the player sends Advance; the server refuses it with
//!    WrongState and leaves the complete arena state unchanged.
//! 4. The player sends Enter. The server must either apply the entry (charging
//!    player-earned carried money, moving the player into pit one, and spawning
//!    its opponent) or return an explicit refusal whose unchanged state is
//!    asserted. An environmental refusal is reported as the failed step, not
//!    converted into a pass.
//! 5. After an applied entry the player fights each server-spawned opponent with
//!    ordinary combat commands, observes the lethal/result deltas, and sends
//!    Advance after each kill; ten accepted advances end in the reward room.
//! 6. The player claims a reward, observes its custody delta, repeats the claim
//!    and expects AlreadyClaimed without another item, then Leaves successfully.
//! 7. The player requests Tibiadrome status, attempts Enter, and asserts either
//!    an applied arena state or the native eligibility refusal without mutation.
//! 8. The player logs out, obtains a fresh admission, requests both statuses,
//!    and re-checks completed rank, claim masks, and Tibiadrome progress.
//!
//! The provisioned character is never edited or repositioned out of band. This
//! phase intentionally stops at the first failed player step and prints that
//! observed native reply; later steps therefore cannot be mistaken for proven.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::arena::{
    self as wire, ArenaAction, ArenaDisposition, ArenaState, DromeAction, DromeResult,
};

struct Journey {
    started: std::time::Instant,
    completed: usize,
}

impl Journey {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            completed: 0,
        }
    }

    fn pass(&mut self, step: usize, observed: impl std::fmt::Debug) -> TestResult {
        if step != self.completed + 1 {
            return Err(format!("W17 step order invalid: expected {}, got {step}", self.completed + 1).into());
        }
        self.completed = step;
        evidence(&format!("W17_CHECK step={step} status=PASS observed={observed:?}"));
        Ok(())
    }

    fn finish(self, result: TestResult) {
        let status = if result.is_ok() && self.completed == 8 {
            "PASS"
        } else {
            "FAIL"
        };
        let detail = match result {
            Ok(()) => "all-eight-player-steps-observed".to_owned(),
            Err(error) => compact(&error.to_string()),
        };
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W17.full status={status} seconds={:.3} evidence={detail}",
            self.started.elapsed().as_secs_f64()
        ));
    }
}

fn compact(detail: &str) -> String {
    detail
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:/=".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut journey = Journey::new();
    let result = verdict::indexed_timeout(Duration::from_secs(240), run(&mut journey, clients))
        .await
        .unwrap_or_else(|_| Err(format!("step-{}-timeout waiting for native arena reply", journey.completed + 1).into()));
    journey.finish(result);
}

async fn command(
    client: &mut DevClientSession,
    action: ArenaAction,
) -> TestResult<wire::ArenaResult> {
    let payload = wire::encode_match_request(action)
        .map_err(|error| format!("arena request encoding: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_ARENA_V1, wire::COMMAND_TYPE_ARENA_MATCH, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("native command {action:?} status={:?} payload={:?}", reply.status, reply.payload).into());
    }
    wire::decode_match_result(&reply.payload)
        .map_err(|error| format!("native command {action:?} malformed reply={:?}: {error:?}", reply.payload).into())
}

async fn drome(
    client: &mut DevClientSession,
    action: DromeAction,
) -> TestResult<DromeResult> {
    let payload = wire::encode_match_request(ArenaAction::Drome(action))
        .map_err(|error| format!("Tibiadrome request encoding: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_ARENA_V1, wire::COMMAND_TYPE_ARENA_MATCH, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("native Tibiadrome {action:?} status={:?} payload={:?}", reply.status, reply.payload).into());
    }
    DromeResult::decode(&reply.payload)
        .map_err(|error| format!("native Tibiadrome {action:?} malformed reply={:?}: {error:?}", reply.payload).into())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step-8 logout observed={reply:?}").into());
    }
    Ok(())
}

async fn run(journey: &mut Journey, clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xb7).await?;
    if !client.selected_capabilities().contains(&wire::CAPABILITY_ARENA_V1) {
        return Err("step-1 selected-capabilities missing arena-v1".into());
    }

    let catalogue = client
        .session_mut()
        .gated_command(wire::CAPABILITY_ARENA_V1, wire::COMMAND_TYPE_ARENA_CATALOGUE, &[])
        .await?;
    if catalogue.status != CommandStatus::Accepted {
        return Err(format!("step-1 catalogue status={:?} payload={:?}", catalogue.status, catalogue.payload).into());
    }
    let challenges = wire::decode_catalogue(&catalogue.payload)
        .map_err(|error| format!("step-1 catalogue payload={:?} error={error:?}", catalogue.payload))?;
    if challenges != wire::SVARGROND_CHALLENGES {
        return Err(format!("step-1 catalogue delta mismatch observed={challenges:?}").into());
    }
    journey.pass(1, challenges)?;

    let initial = command(&mut client, ArenaAction::Status).await?;
    if initial.disposition != ArenaDisposition::Applied {
        return Err(format!("step-2 status observed={initial:?}").into());
    }
    journey.pass(2, initial)?;

    let refused = command(&mut client, ArenaAction::Advance).await?;
    if refused.disposition != ArenaDisposition::WrongState || refused.state != initial.state {
        return Err(format!("step-3 Advance expected WrongState with no delta observed={refused:?} before={initial:?}").into());
    }
    journey.pass(3, refused)?;

    let drome_before = drome(&mut client, DromeAction::Status).await?;
    let drome_enter = drome(&mut client, DromeAction::Enter).await?;
    if drome_enter.disposition == 1 {
        let left = drome(&mut client, DromeAction::Leave).await?;
        if left.disposition != 1 {
            return Err(format!("step-7 Tibiadrome Leave observed={left:?}").into());
        }
    } else if drome_enter.disposition != 3 || drome_enter != drome_before {
        return Err(format!(
            "step-7 Tibiadrome eligibility refusal mutated state before={drome_before:?} observed={drome_enter:?}"
        )
        .into());
    }

    logout(&mut client).await?;
    drop(client);
    let mut client = connect_world(clients, 0xb8).await?;
    let after_relog = command(&mut client, ArenaAction::Status).await?;
    if after_relog.state != initial.state {
        return Err(format!(
            "step-8 fresh-admission arena state mismatch before={initial:?} observed={after_relog:?}"
        )
        .into());
    }
    let drome_after = drome(&mut client, DromeAction::Status).await?;
    if drome_enter.disposition != 1 && drome_after != drome_before {
        return Err(format!(
            "step-8 fresh-admission Tibiadrome state mismatch before={drome_before:?} observed={drome_after:?}"
        )
        .into());
    }

    let entered = command(&mut client, ArenaAction::Enter).await?;
    if entered.disposition != ArenaDisposition::Applied
        || entered.state.active_rank == 0
        || entered.state.pit != 1
        || entered.state.phase != 1
    {
        return Err(format!(
            "step-4 Enter observed={entered:?}; server-needs=reachable-Svargrond-entrance-and-player-earned-1000-gold"
        ).into());
    }
    journey.pass(4, entered)?;

    // Advance is also the protocol's negative proof that a client cannot forge
    // a kill. Sending it here is a real first-pit attempt; do not infer victory
    // merely because entry succeeded.
    let premature = command(&mut client, ArenaAction::Advance).await?;
    if premature.disposition == ArenaDisposition::WrongState && premature.state == entered.state {
        return Err(format!(
            "step-5 Advance-after-Enter observed={premature:?}; server-needs=arena-opponent-exposed-to-native-client-combat-journey"
        ).into());
    }
    Err(format!("step-5 unexpected Advance-after-Enter observed={premature:?}").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_evidence_is_one_token() {
        assert_eq!(compact("step 4: WrongState [2]"), "step_4:_WrongState__2_");
    }

    #[test]
    fn completion_requires_every_numbered_step() -> TestResult {
        let mut journey = Journey::new();
        journey.pass(1, "catalogue")?;
        assert!(journey.pass(3, "advance").is_err());
        assert_eq!(journey.completed, 1);
        Ok(())
    }

    #[test]
    fn idle_state_is_not_an_enter_delta() {
        let state = ArenaState::default();
        assert_eq!(state.active_rank, 0);
        assert_ne!(state.phase, 1);
    }
}
