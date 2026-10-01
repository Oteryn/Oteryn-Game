//! CHARM-5 server adapter for the proposed Bestiary and Charm wire
//! (`docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md` §5).
//!
//! A thin layer: decode the intent, derive the CHARM-3 command occurrence from the FND-02 command
//! identity, call the port, encode the outcome or the view. Every rule (credit, stage derivation,
//! costs, balances, assignment and slot limits) and every durable write belongs to the port's
//! implementations, CHARM-2 (Bestiary progress) and CHARM-3 (`commit_charm_command`), each one
//! fenced Character transaction. The port is bound to the one fenced character of the session,
//! so no method here names a character.
//!
//! The command types and state domains are registered under capability 1 `BESTIARY_CHARMS_V1`,
//! which the server does not offer before CHARM-6 (D170). Nothing routes to this module until
//! composition (CHARM-5-COMP) dispatches through [`dispatch_charm_command`].

// Composition is CHARM-5-COMP (proposal §8 step 3); until then only the tests reach this.
#![cfg_attr(not(test), allow(dead_code))]

use std::future::Future;

use oteryn_protocol_oteryn::bestiary::{BestiaryRaceProgress, encode_bestiary_view};
use oteryn_protocol_oteryn::charm::{
    COMMAND_TYPE_CHARM_ASSIGN_INTENT, COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT,
    CharmAssignDisposition, CharmAssignIntent, CharmUnlockDisposition, CharmUnlockStageIntent,
    CharmView, CyclopediaWireError, decode_charm_assign_intent, decode_charm_unlock_stage_intent,
    encode_charm_assign_result, encode_charm_unlock_stage_result, encode_charm_view,
};
use sha2::{Digest, Sha256};

use crate::durability::charm_state::CharmCommandOccurrence;
use crate::foundation::GameSessionId;

#[path = "charm_native.rs"]
pub(crate) mod native;

/// The port cannot produce a view now (no fenced character, storage unavailable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharmPortUnavailable;

/// The narrow boundary CHARM-2 and CHARM-3 implement for the one fenced character. Every method
/// is asynchronous because CHARM-3 commits and reads through the durability root.
pub(crate) trait CharmProgressionPort {
    /// The character's counted Bestiary races (CHARM-2), in any order.
    fn bestiary(
        &self,
    ) -> impl Future<Output = Result<Vec<BestiaryRaceProgress>, CharmPortUnavailable>> + Send;
    /// The character's charms, derived available balances and slot limit (CHARM-3), in any order.
    /// Effect availability is derived and never stored. A calculation-ready CHARM-4 definition
    /// still requires a qualified, connected production consumer before advertising activation.
    fn charms(&self) -> impl Future<Output = Result<CharmView, CharmPortUnavailable>> + Send;
    /// One CHARM-3 `commit_charm_command` of `UnlockNextStage` under `occurrence`. The port
    /// answers `STAGE_MISMATCH` when the charm's stored stage is not `intent.expected_stage`
    /// (proposal §3.3: a check before the commit, not under the root lock). An
    /// `AlreadyCommitted` outcome of a resent command reports its original disposition.
    fn unlock_next_stage(
        &mut self,
        occurrence: CharmCommandOccurrence,
        intent: CharmUnlockStageIntent,
    ) -> impl Future<Output = CharmUnlockDisposition> + Send;
    /// One CHARM-3 `commit_charm_command` of `Assign` under `occurrence`. Unassign is CHARM-6
    /// (D170).
    fn assign(
        &mut self,
        occurrence: CharmCommandOccurrence,
        intent: CharmAssignIntent,
    ) -> impl Future<Output = CharmAssignDisposition> + Send;
}

/// The FND-02 identity of one client command: `command_id` is strictly increasing within the
/// `GameSessionId` and survives an eligible reconnect of that session, so a resent command keeps
/// its identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharmCommandIdentity {
    pub(crate) game_session_id: GameSessionId,
    pub(crate) command_id: u64,
}

impl CharmCommandIdentity {
    /// The CHARM-3 idempotency occurrence (proposal §5), following the spell-cast occurrence:
    /// the first 16 bytes of `SHA-256("oteryn:charm-command-occurrence:v1" || GameSessionId ||
    /// command_id as big-endian u64)`, with the UUID version nibble set to 7 and the variant bits
    /// to `10`. A resent command therefore replays in CHARM-3 instead of committing twice.
    /// `None` only if CHARM-3 refuses the form, which the bit setting rules out.
    pub(crate) fn occurrence(self) -> Option<CharmCommandOccurrence> {
        let digest = Sha256::new()
            .chain_update(b"oteryn:charm-command-occurrence:v1")
            .chain_update(self.game_session_id.as_bytes())
            .chain_update(self.command_id.to_be_bytes())
            .finalize();
        let mut bytes = [0_u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        bytes[6] = (bytes[6] & 0x0f) | 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        CharmCommandOccurrence::from_bytes(bytes).ok()
    }
}

/// The `CommandResult` of one charm command: the FND-02 status is `Rejected` exactly for the
/// `REJECTED` disposition, as for the other command types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CharmCommandReply {
    pub(crate) rejected: bool,
    pub(crate) result_payload: Vec<u8>,
}

/// A view the port produced cannot be sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharmViewFault {
    Unavailable,
    /// A server fault: the port's view breaks a wire invariant or bound. Never truncated.
    OutOfBounds(CyclopediaWireError),
}

/// Command type 4. A payload that does not decode is `REJECTED` without reaching the port.
pub(crate) async fn charm_unlock_stage(
    port: &mut impl CharmProgressionPort,
    identity: CharmCommandIdentity,
    payload: &[u8],
) -> CharmCommandReply {
    let disposition = match (
        decode_charm_unlock_stage_intent(payload),
        identity.occurrence(),
    ) {
        (Ok(intent), Some(occurrence)) => port.unlock_next_stage(occurrence, intent).await,
        _ => CharmUnlockDisposition::Rejected,
    };
    CharmCommandReply {
        rejected: disposition == CharmUnlockDisposition::Rejected,
        result_payload: encode_charm_unlock_stage_result(disposition),
    }
}

/// Command type 5. A payload that does not decode is `REJECTED` without reaching the port.
pub(crate) async fn charm_assign(
    port: &mut impl CharmProgressionPort,
    identity: CharmCommandIdentity,
    payload: &[u8],
) -> CharmCommandReply {
    let disposition = match (decode_charm_assign_intent(payload), identity.occurrence()) {
        (Ok(intent), Some(occurrence)) => port.assign(occurrence, intent).await,
        _ => CharmAssignDisposition::Rejected,
    };
    CharmCommandReply {
        rejected: disposition == CharmAssignDisposition::Rejected,
        result_payload: encode_charm_assign_result(disposition),
    }
}

/// Routes a charm command type; `None` for every other command type.
pub(crate) async fn dispatch_charm_command(
    port: &mut impl CharmProgressionPort,
    identity: CharmCommandIdentity,
    command_type: u32,
    payload: &[u8],
) -> Option<CharmCommandReply> {
    match command_type {
        COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT => {
            Some(charm_unlock_stage(port, identity, payload).await)
        }
        COMMAND_TYPE_CHARM_ASSIGN_INTENT => Some(charm_assign(port, identity, payload).await),
        _ => None,
    }
}

/// The domain-4 snapshot payload (also a delta that upserts every counted race).
pub(crate) async fn bestiary_view_payload(
    port: &impl CharmProgressionPort,
) -> Result<Vec<u8>, CharmViewFault> {
    let mut races = port
        .bestiary()
        .await
        .map_err(|CharmPortUnavailable| CharmViewFault::Unavailable)?;
    races.sort_unstable_by_key(|race| race.race);
    encode_bestiary_view(&races).map_err(CharmViewFault::OutOfBounds)
}

/// The domain-5 snapshot or delta payload (a delta replaces the whole view).
pub(crate) async fn charm_view_payload(
    port: &impl CharmProgressionPort,
) -> Result<Vec<u8>, CharmViewFault> {
    let mut view = port
        .charms()
        .await
        .map_err(|CharmPortUnavailable| CharmViewFault::Unavailable)?;
    view.charms.sort_unstable_by_key(|charm| charm.charm);
    encode_charm_view(&view).map_err(CharmViewFault::OutOfBounds)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::bestiary::decode_bestiary_view;
    use oteryn_protocol_oteryn::charm::{
        CharmKind, CharmState, decode_charm_assign_result, decode_charm_unlock_stage_result,
        decode_charm_view,
    };
    use std::num::NonZeroU32;

    fn index(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).expect("non-zero index")
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime")
            .block_on(future)
    }

    const fn uuid_v7(tag: u8) -> [u8; 16] {
        [
            0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
        ]
    }

    fn identity(session: u8, command_id: u64) -> CharmCommandIdentity {
        CharmCommandIdentity {
            game_session_id: GameSessionId::decode(&uuid_v7(session)).expect("session"),
            command_id,
        }
    }

    fn occurrence(session: u8, command_id: u64) -> CharmCommandOccurrence {
        identity(session, command_id)
            .occurrence()
            .expect("occurrence")
    }

    /// Test double for CHARM-2 and CHARM-3: scripted views and dispositions, recorded calls.
    #[derive(Default)]
    struct FakePort {
        bestiary: Option<Vec<BestiaryRaceProgress>>,
        charms: Option<CharmView>,
        unlock_reply: Option<CharmUnlockDisposition>,
        assign_reply: Option<CharmAssignDisposition>,
        unlocks: Vec<(CharmCommandOccurrence, CharmUnlockStageIntent)>,
        assigns: Vec<(CharmCommandOccurrence, CharmAssignIntent)>,
    }

    impl CharmProgressionPort for FakePort {
        async fn bestiary(&self) -> Result<Vec<BestiaryRaceProgress>, CharmPortUnavailable> {
            self.bestiary.clone().ok_or(CharmPortUnavailable)
        }

        async fn charms(&self) -> Result<CharmView, CharmPortUnavailable> {
            self.charms.clone().ok_or(CharmPortUnavailable)
        }

        async fn unlock_next_stage(
            &mut self,
            occurrence: CharmCommandOccurrence,
            intent: CharmUnlockStageIntent,
        ) -> CharmUnlockDisposition {
            self.unlocks.push((occurrence, intent));
            self.unlock_reply.expect("scripted unlock reply")
        }

        async fn assign(
            &mut self,
            occurrence: CharmCommandOccurrence,
            intent: CharmAssignIntent,
        ) -> CharmAssignDisposition {
            self.assigns.push((occurrence, intent));
            self.assign_reply.expect("scripted assign reply")
        }
    }

    #[test]
    fn unlock_decodes_calls_the_port_once_and_encodes_its_outcome() {
        for (reply, rejected) in [
            (CharmUnlockDisposition::Unlocked, false),
            (CharmUnlockDisposition::NotEnoughCharmPoints, false),
            (CharmUnlockDisposition::StageMismatch, false),
            (CharmUnlockDisposition::Rejected, true),
        ] {
            let mut port = FakePort {
                unlock_reply: Some(reply),
                ..FakePort::default()
            };
            let reply_bytes = block_on(charm_unlock_stage(
                &mut port,
                identity(1, 7),
                &[0x08, 0x03, 0x10, 0x01],
            ));
            assert_eq!(reply_bytes.rejected, rejected, "{reply:?}");
            assert_eq!(
                decode_charm_unlock_stage_result(&reply_bytes.result_payload),
                Ok(reply)
            );
            assert_eq!(
                port.unlocks,
                [(
                    occurrence(1, 7),
                    CharmUnlockStageIntent {
                        charm: index(3),
                        expected_stage: 1
                    }
                )]
            );
        }
    }

    #[test]
    fn a_malformed_unlock_is_rejected_without_reaching_the_port() {
        // Charm zero, the final stage expected, an unknown field, over the byte bound.
        for payload in [
            &[0x08, 0x00][..],
            &[0x08, 0x01, 0x10, 0x03][..],
            &[0x08, 0x01, 0x18, 0x01][..],
            &[0; 9][..],
        ] {
            let mut port = FakePort::default();
            let reply = block_on(charm_unlock_stage(&mut port, identity(1, 7), payload));
            assert!(reply.rejected);
            assert_eq!(reply.result_payload, [0x08, 0x06]);
            assert!(port.unlocks.is_empty());
        }
    }

    #[test]
    fn assign_decodes_calls_the_port_once_and_encodes_its_outcome() {
        for (reply, rejected) in [
            (CharmAssignDisposition::Assigned, false),
            (CharmAssignDisposition::AlreadyAssigned, false),
            (CharmAssignDisposition::RaceStageTooLow, false),
            (CharmAssignDisposition::AssignmentSlotsFull, false),
            (CharmAssignDisposition::Rejected, true),
        ] {
            let mut port = FakePort {
                assign_reply: Some(reply),
                ..FakePort::default()
            };
            let reply_bytes = block_on(charm_assign(
                &mut port,
                identity(1, 8),
                &[0x08, 0x02, 0x10, 0xac, 0x02],
            ));
            assert_eq!(reply_bytes.rejected, rejected, "{reply:?}");
            assert_eq!(
                decode_charm_assign_result(&reply_bytes.result_payload),
                Ok(reply)
            );
            assert_eq!(
                port.assigns,
                [(
                    occurrence(1, 8),
                    CharmAssignIntent {
                        charm: index(2),
                        race: index(300)
                    }
                )]
            );
        }
    }

    #[test]
    fn a_malformed_assign_is_rejected_without_reaching_the_port() {
        // Race absent, race above its bound, an unassign-shaped extra field.
        for payload in [
            &[0x08, 0x01][..],
            &[0x08, 0x01, 0x10, 0x81, 0x08][..],
            &[0x08, 0x01, 0x10, 0x01, 0x18, 0x01][..],
        ] {
            let mut port = FakePort::default();
            let reply = block_on(charm_assign(&mut port, identity(1, 8), payload));
            assert!(reply.rejected);
            assert_eq!(reply.result_payload, [0x08, 0x08]);
            assert!(port.assigns.is_empty());
        }
    }

    #[test]
    fn the_occurrence_is_bound_to_the_fnd02_command_identity() {
        // Independent fixture: SHA-256("oteryn:charm-command-occurrence:v1" || session 0x11 ||
        // command 7 as u64 BE) = a10273b887895ffcc0a11d07a3756b1e.., version 7, variant 10.
        assert_eq!(
            occurrence(0x11, 7).as_bytes(),
            &[
                0xa1, 0x02, 0x73, 0xb8, 0x87, 0x89, 0x7f, 0xfc, 0x80, 0xa1, 0x1d, 0x07, 0xa3, 0x75,
                0x6b, 0x1e
            ]
        );
        // A resent command (same session, same CommandId) replays; anything else is new.
        assert_eq!(occurrence(0x11, 7), occurrence(0x11, 7));
        assert_ne!(occurrence(0x11, 7), occurrence(0x11, 8));
        assert_ne!(occurrence(0x11, 7), occurrence(0x12, 7));
        for command_id in [1, 2, u64::MAX] {
            let bytes = *occurrence(0x11, command_id).as_bytes();
            assert_eq!(bytes[6] >> 4, 7);
            assert_eq!(bytes[8] & 0xc0, 0x80);
        }
    }

    #[test]
    fn dispatch_routes_only_the_charm_command_types() {
        let mut port = FakePort {
            unlock_reply: Some(CharmUnlockDisposition::Unlocked),
            assign_reply: Some(CharmAssignDisposition::Assigned),
            ..FakePort::default()
        };
        let unlock = block_on(dispatch_charm_command(
            &mut port,
            identity(1, 1),
            COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT,
            &[0x08, 0x01],
        ));
        assert_eq!(
            unlock.map(|reply| reply.result_payload),
            Some(vec![0x08, 0x01])
        );
        let assign = block_on(dispatch_charm_command(
            &mut port,
            identity(1, 2),
            COMMAND_TYPE_CHARM_ASSIGN_INTENT,
            &[0x08, 0x01, 0x10, 0x01],
        ));
        assert_eq!(
            assign.map(|reply| reply.result_payload),
            Some(vec![0x08, 0x01])
        );
        // Step, use and spell cast (1, 2, 3) and anything unknown are not charm commands.
        for other in [0, 1, 2, 3, 6, u32::MAX] {
            assert_eq!(
                block_on(dispatch_charm_command(
                    &mut port,
                    identity(1, 3),
                    other,
                    &[0x08, 0x01]
                )),
                None
            );
        }
        assert_eq!(port.unlocks.len(), 1);
        assert_eq!(port.assigns.len(), 1);
    }

    fn race(race: u32, kill_count: u32) -> BestiaryRaceProgress {
        BestiaryRaceProgress {
            race: index(race),
            kill_count,
            kill_thresholds: [5, 10, 25],
        }
    }

    fn charm(charm: u32, unlocked_stage: u8, race: Option<u32>) -> CharmState {
        CharmState {
            charm: index(charm),
            kind: CharmKind::Major,
            unlocked_stage,
            assigned_race: race.map(index),
            next_stage_cost: if unlocked_stage == 3 { 0 } else { 100 },
            effect_active: charm.is_multiple_of(2),
        }
    }

    #[test]
    fn views_are_ordered_and_encoded_from_the_port() {
        let port = FakePort {
            bestiary: Some(vec![race(9, 25), race(2, 1)]),
            charms: Some(CharmView {
                charms: vec![charm(4, 3, Some(9)), charm(1, 0, None)],
                charm_points_available: 480,
                minor_charm_echoes_available: 100,
                assignment_slot_limit: Some(index(2)),
            }),
            ..FakePort::default()
        };
        let bestiary = block_on(bestiary_view_payload(&port)).expect("bestiary payload");
        assert_eq!(
            decode_bestiary_view(&bestiary),
            Ok(vec![race(2, 1), race(9, 25)])
        );
        let charms = block_on(charm_view_payload(&port)).expect("charm payload");
        assert_eq!(
            decode_charm_view(&charms),
            Ok(CharmView {
                charms: vec![charm(1, 0, None), charm(4, 3, Some(9))],
                charm_points_available: 480,
                minor_charm_echoes_available: 100,
                assignment_slot_limit: Some(index(2)),
            })
        );
    }

    #[test]
    fn a_view_the_port_cannot_give_or_that_breaks_a_bound_is_a_fault() {
        let unavailable = FakePort::default();
        assert_eq!(
            block_on(bestiary_view_payload(&unavailable)),
            Err(CharmViewFault::Unavailable)
        );
        assert_eq!(
            block_on(charm_view_payload(&unavailable)),
            Err(CharmViewFault::Unavailable)
        );
        // A repeated race and an assignment on a locked charm are never sent.
        let broken = FakePort {
            bestiary: Some(vec![race(2, 1), race(2, 3)]),
            charms: Some(CharmView {
                charms: vec![charm(1, 0, Some(2))],
                ..CharmView::default()
            }),
            ..FakePort::default()
        };
        assert_eq!(
            block_on(bestiary_view_payload(&broken)),
            Err(CharmViewFault::OutOfBounds(CyclopediaWireError::Malformed))
        );
        assert_eq!(
            block_on(charm_view_payload(&broken)),
            Err(CharmViewFault::OutOfBounds(CyclopediaWireError::Malformed))
        );
    }
}
