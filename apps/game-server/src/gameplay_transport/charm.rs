//! CHARM-5 server adapter for the proposed Bestiary and Charm wire
//! (`docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md` §5).
//!
//! A thin layer: decode the intent, call the port, encode the outcome or the view. Every rule
//! (credit, stage derivation, costs, balances, assignment limits) and every durable write
//! belongs to the port's implementations, CHARM-2 (Bestiary progress) and CHARM-3 (Charm state),
//! each one fenced Character transaction. The port is bound to the one fenced character of the
//! session, so no method here names a character.
//!
//! Nothing routes to this module until the protocol owner registers the proposed command types
//! and state domains; composition then dispatches through [`dispatch_charm_command`].

// Composition waits for ID registration (proposal §8); until then only the tests reach this.
#![cfg_attr(not(test), allow(dead_code))]

use oteryn_protocol_oteryn::bestiary::{BestiaryRaceProgress, encode_bestiary_view};
use oteryn_protocol_oteryn::charm::{
    COMMAND_TYPE_CHARM_ASSIGN_INTENT, COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT,
    CharmAssignDisposition, CharmAssignIntent, CharmUnlockDisposition, CharmUnlockStageIntent,
    CharmView, CyclopediaWireError, decode_charm_assign_intent, decode_charm_unlock_stage_intent,
    encode_charm_assign_result, encode_charm_unlock_stage_result, encode_charm_view,
};

/// The port cannot produce a view now (no fenced character, storage unavailable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharmPortUnavailable;

/// The narrow boundary CHARM-2 and CHARM-3 implement for the one fenced character.
pub(crate) trait CharmProgressionPort {
    /// The character's counted Bestiary races (CHARM-2), in any order.
    fn bestiary(&self) -> Result<Vec<BestiaryRaceProgress>, CharmPortUnavailable>;
    /// The character's charms and derived available balances (CHARM-3), in any order.
    fn charms(&self) -> Result<CharmView, CharmPortUnavailable>;
    /// One fenced transaction: unlock `expected_stage + 1`, or say why not.
    fn unlock_next_stage(&mut self, intent: CharmUnlockStageIntent) -> CharmUnlockDisposition;
    /// One fenced transaction: assign the charm to the race, or say why not. There is no
    /// unassign in this version (CHARM-0 §7 answer 3c).
    fn assign(&mut self, intent: CharmAssignIntent) -> CharmAssignDisposition;
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
pub(crate) fn charm_unlock_stage(
    port: &mut impl CharmProgressionPort,
    payload: &[u8],
) -> CharmCommandReply {
    let disposition = match decode_charm_unlock_stage_intent(payload) {
        Ok(intent) => port.unlock_next_stage(intent),
        Err(_) => CharmUnlockDisposition::Rejected,
    };
    CharmCommandReply {
        rejected: disposition == CharmUnlockDisposition::Rejected,
        result_payload: encode_charm_unlock_stage_result(disposition),
    }
}

/// Command type 5. A payload that does not decode is `REJECTED` without reaching the port.
pub(crate) fn charm_assign(
    port: &mut impl CharmProgressionPort,
    payload: &[u8],
) -> CharmCommandReply {
    let disposition = match decode_charm_assign_intent(payload) {
        Ok(intent) => port.assign(intent),
        Err(_) => CharmAssignDisposition::Rejected,
    };
    CharmCommandReply {
        rejected: disposition == CharmAssignDisposition::Rejected,
        result_payload: encode_charm_assign_result(disposition),
    }
}

/// Routes a charm command type; `None` for every other command type.
pub(crate) fn dispatch_charm_command(
    port: &mut impl CharmProgressionPort,
    command_type: u32,
    payload: &[u8],
) -> Option<CharmCommandReply> {
    match command_type {
        COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT => Some(charm_unlock_stage(port, payload)),
        COMMAND_TYPE_CHARM_ASSIGN_INTENT => Some(charm_assign(port, payload)),
        _ => None,
    }
}

/// The domain-4 snapshot payload (also a delta that upserts every counted race).
pub(crate) fn bestiary_view_payload(
    port: &impl CharmProgressionPort,
) -> Result<Vec<u8>, CharmViewFault> {
    let mut races = port
        .bestiary()
        .map_err(|CharmPortUnavailable| CharmViewFault::Unavailable)?;
    races.sort_unstable_by_key(|race| race.race);
    encode_bestiary_view(&races).map_err(CharmViewFault::OutOfBounds)
}

/// The domain-5 snapshot or delta payload (a delta replaces the whole view).
pub(crate) fn charm_view_payload(
    port: &impl CharmProgressionPort,
) -> Result<Vec<u8>, CharmViewFault> {
    let mut view = port
        .charms()
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

    /// Test double for CHARM-2 and CHARM-3: scripted views and dispositions, recorded calls.
    #[derive(Default)]
    struct FakePort {
        bestiary: Option<Vec<BestiaryRaceProgress>>,
        charms: Option<CharmView>,
        unlock_reply: Option<CharmUnlockDisposition>,
        assign_reply: Option<CharmAssignDisposition>,
        unlocks: Vec<CharmUnlockStageIntent>,
        assigns: Vec<CharmAssignIntent>,
    }

    impl CharmProgressionPort for FakePort {
        fn bestiary(&self) -> Result<Vec<BestiaryRaceProgress>, CharmPortUnavailable> {
            self.bestiary.clone().ok_or(CharmPortUnavailable)
        }

        fn charms(&self) -> Result<CharmView, CharmPortUnavailable> {
            self.charms.clone().ok_or(CharmPortUnavailable)
        }

        fn unlock_next_stage(&mut self, intent: CharmUnlockStageIntent) -> CharmUnlockDisposition {
            self.unlocks.push(intent);
            self.unlock_reply.expect("scripted unlock reply")
        }

        fn assign(&mut self, intent: CharmAssignIntent) -> CharmAssignDisposition {
            self.assigns.push(intent);
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
            let reply_bytes = charm_unlock_stage(&mut port, &[0x08, 0x03, 0x10, 0x01]);
            assert_eq!(reply_bytes.rejected, rejected, "{reply:?}");
            assert_eq!(
                decode_charm_unlock_stage_result(&reply_bytes.result_payload),
                Ok(reply)
            );
            assert_eq!(
                port.unlocks,
                [CharmUnlockStageIntent {
                    charm: index(3),
                    expected_stage: 1
                }]
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
            let reply = charm_unlock_stage(&mut port, payload);
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
            (CharmAssignDisposition::Rejected, true),
        ] {
            let mut port = FakePort {
                assign_reply: Some(reply),
                ..FakePort::default()
            };
            let reply_bytes = charm_assign(&mut port, &[0x08, 0x02, 0x10, 0xac, 0x02]);
            assert_eq!(reply_bytes.rejected, rejected, "{reply:?}");
            assert_eq!(
                decode_charm_assign_result(&reply_bytes.result_payload),
                Ok(reply)
            );
            assert_eq!(
                port.assigns,
                [CharmAssignIntent {
                    charm: index(2),
                    race: index(300)
                }]
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
            let reply = charm_assign(&mut port, payload);
            assert!(reply.rejected);
            assert_eq!(reply.result_payload, [0x08, 0x08]);
            assert!(port.assigns.is_empty());
        }
    }

    #[test]
    fn dispatch_routes_only_the_charm_command_types() {
        let mut port = FakePort {
            unlock_reply: Some(CharmUnlockDisposition::Unlocked),
            assign_reply: Some(CharmAssignDisposition::Assigned),
            ..FakePort::default()
        };
        let unlock = dispatch_charm_command(
            &mut port,
            COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT,
            &[0x08, 0x01],
        );
        assert_eq!(
            unlock.map(|reply| reply.result_payload),
            Some(vec![0x08, 0x01])
        );
        let assign = dispatch_charm_command(
            &mut port,
            COMMAND_TYPE_CHARM_ASSIGN_INTENT,
            &[0x08, 0x01, 0x10, 0x01],
        );
        assert_eq!(
            assign.map(|reply| reply.result_payload),
            Some(vec![0x08, 0x01])
        );
        // Step, use and spell cast (1, 2, 3) and anything unknown are not charm commands.
        for other in [0, 1, 2, 3, 6, u32::MAX] {
            assert_eq!(
                dispatch_charm_command(&mut port, other, &[0x08, 0x01]),
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
            }),
            ..FakePort::default()
        };
        let bestiary = bestiary_view_payload(&port).expect("bestiary payload");
        assert_eq!(
            decode_bestiary_view(&bestiary),
            Ok(vec![race(2, 1), race(9, 25)])
        );
        let charms = charm_view_payload(&port).expect("charm payload");
        assert_eq!(
            decode_charm_view(&charms),
            Ok(CharmView {
                charms: vec![charm(1, 0, None), charm(4, 3, Some(9))],
                charm_points_available: 480,
                minor_charm_echoes_available: 100,
            })
        );
    }

    #[test]
    fn a_view_the_port_cannot_give_or_that_breaks_a_bound_is_a_fault() {
        let unavailable = FakePort::default();
        assert_eq!(
            bestiary_view_payload(&unavailable),
            Err(CharmViewFault::Unavailable)
        );
        assert_eq!(
            charm_view_payload(&unavailable),
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
            bestiary_view_payload(&broken),
            Err(CharmViewFault::OutOfBounds(CyclopediaWireError::Malformed))
        );
        assert_eq!(
            charm_view_payload(&broken),
            Err(CharmViewFault::OutOfBounds(CyclopediaWireError::Malformed))
        );
    }
}
