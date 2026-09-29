//! Charm view, unlock and assign typed payloads (CHARM-5; **proposed**, not registered).
//!
//! Wire proposal: `docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md` §3.2 to
//! §3.4, following CHARM-0 §4.2 and the owner answers of CHARM-0 §7: only unlocks and assignments
//! are stored and the balances are derived (answer 2a); there is **no unassign command** in this
//! version (answer 3c). The command-type, state-domain and payload-type IDs below are proposals
//! for the protocol owner. They are not in `PROTOCOL_OTERYN_V1_REGISTRY.json`, and no session or
//! server sends or accepts them until that owner registers them.
//!
//! One `CharmViewV1` serves both the snapshot and the delta: the view is small (at most
//! [`MAX_CHARM_VIEW_BYTES`]), so every delta replaces it whole. Command results report an outcome,
//! never state; the new state arrives through the view (FIRST-CONTROL-WIRE-V1 pattern).
//!
//! Decoding is strict, and encoding refuses the same values as a server fault before any byte is
//! emitted: zero or unknown enum values, a charm or race index of zero or above its bound, charms
//! out of ascending order or repeated, a stage above 3, an assignment on a locked charm, a next
//! stage cost that is zero before the final stage or non-zero at it, a cost or balance above its
//! bound, and an unlock that expects the final stage all fail closed.

use std::num::NonZeroU32;

use crate::bestiary::MAX_BESTIARY_RACE;
pub use crate::charm_wire::CyclopediaWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes,
    read_result_enum, read_uint32, read_uint32_fields, read_varint, set_once,
};

/// Proposed command type `CHARM_UNLOCK_STAGE_INTENT` (proposal §2).
pub const COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT: u32 = 4;
/// Proposed command type `CHARM_ASSIGN_INTENT` (proposal §2).
pub const COMMAND_TYPE_CHARM_ASSIGN_INTENT: u32 = 5;
/// Proposed state domain `CHARACTER_CHARMS` (proposal §2).
pub const STATE_DOMAIN_CHARACTER_CHARMS: u32 = 5;
pub const DELTA_TYPE_CHARACTER_CHARMS_V1: u32 = 1;
pub const SNAPSHOT_TYPE_CHARACTER_CHARMS_V1: u32 = 1;

/// Proposed `CHARM5-RL-03`: charms in one view. The candidate catalogue has 25 (CHARM-0 §2).
pub const MAX_CHARMS: usize = 32;
/// A charm index is 1-based into a charm list of at most [`MAX_CHARMS`] charms.
pub const MAX_CHARM: u32 = 32;
/// Charm stages are 0 (locked) to 3 (fully unlocked).
pub const MAX_CHARM_STAGE: u8 = 3;
/// Proposed `CHARM5-RL-04`: the largest single stage cost, in Charm Points or Minor Charm Echoes.
pub const MAX_CHARM_STAGE_COST: u32 = 100_000;
/// Proposed `CHARM5-RL-05`: the largest available balance of either currency.
pub const MAX_CHARM_BALANCE: u32 = 1_000_000;
/// Worst case per entry: charm 1 + 1, kind 1 + 1, stage 1 + 1, race 1 + 2, cost 1 + 3, plus the
/// entry's own tag and length = 15 bytes; 32 entries = 480, and two balances of 1 + 3 = 488 bytes.
pub const MAX_CHARM_VIEW_BYTES: usize = 488;
/// Canonical worst case 4 bytes (charm 1 + 1, expected stage 1 + 1), with slack for explicit
/// defaults.
pub const MAX_CHARM_UNLOCK_STAGE_INTENT_BYTES: usize = 8;
/// Canonical worst case 5 bytes (charm 1 + 1, race 1 + 2), with slack.
pub const MAX_CHARM_ASSIGN_INTENT_BYTES: usize = 8;
/// A single small enum field (1 tag + 1 value byte = 2), with the same slack as the spell result.
pub const MAX_CHARM_RESULT_BYTES: usize = 4;

/// Which currency a charm's stages cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharmKind {
    /// `CHARM_KIND_MAJOR = 1`: paid in Charm Points.
    Major = 1,
    /// `CHARM_KIND_MINOR = 2`: paid in Minor Charm Echoes.
    Minor = 2,
}

/// `CharmStateV1`: one charm of the character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharmState {
    /// 1-based index into the charm list of the loaded content generation (proposal §2).
    pub charm: NonZeroU32,
    pub kind: CharmKind,
    /// 0 (locked) to [`MAX_CHARM_STAGE`].
    pub unlocked_stage: u8,
    /// The Bestiary race (as in `bestiary`) the charm is assigned to; only on an unlocked charm.
    pub assigned_race: Option<NonZeroU32>,
    /// The cost of the next stage in the kind's currency; 0 exactly at the final stage.
    pub next_stage_cost: u32,
}

impl CharmState {
    fn check(&self) -> WireResult<()> {
        if self.charm.get() > MAX_CHARM
            || self
                .assigned_race
                .is_some_and(|race| race.get() > MAX_BESTIARY_RACE)
            || self.next_stage_cost > MAX_CHARM_STAGE_COST
        {
            return Err(CyclopediaWireError::LimitExceeded);
        }
        if self.unlocked_stage > MAX_CHARM_STAGE
            || (self.assigned_race.is_some() && self.unlocked_stage == 0)
            || ((self.next_stage_cost == 0) != (self.unlocked_stage == MAX_CHARM_STAGE))
        {
            return Err(CyclopediaWireError::Malformed);
        }
        Ok(())
    }
}

/// `CharmViewV1`: every charm of the character and the derived available balances.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharmView {
    /// Strictly ascending by `charm`.
    pub charms: Vec<CharmState>,
    /// Derived: Charm Points earned minus spent (CHARM-0 §4.2), never stored.
    pub charm_points_available: u32,
    /// Derived: Minor Charm Echoes earned minus spent (CHARM-0 §4.2), never stored.
    pub minor_charm_echoes_available: u32,
}

fn check_view(view: &CharmView) -> WireResult<()> {
    if view.charms.len() > MAX_CHARMS
        || view.charm_points_available > MAX_CHARM_BALANCE
        || view.minor_charm_echoes_available > MAX_CHARM_BALANCE
    {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    for charm in &view.charms {
        charm.check()?;
    }
    if view
        .charms
        .windows(2)
        .any(|pair| pair[0].charm >= pair[1].charm)
    {
        return Err(CyclopediaWireError::Malformed);
    }
    Ok(())
}

/// Encodes the domain-5 snapshot or delta payload. A view outside the bounds is a server fault
/// and is refused before any byte is emitted. Fields at their proto3 default are omitted.
pub fn encode_charm_view(view: &CharmView) -> WireResult<Vec<u8>> {
    check_view(view)?;
    let mut output = Vec::with_capacity(MAX_CHARM_VIEW_BYTES);
    let mut entry = Vec::with_capacity(13);
    for charm in &view.charms {
        entry.clear();
        push_varint_field(&mut entry, 1, u64::from(charm.charm.get()));
        push_varint_field(&mut entry, 2, charm.kind as u64);
        push_nonzero_varint_field(&mut entry, 3, u64::from(charm.unlocked_stage));
        push_nonzero_varint_field(
            &mut entry,
            4,
            u64::from(charm.assigned_race.map_or(0, NonZeroU32::get)),
        );
        push_nonzero_varint_field(&mut entry, 5, u64::from(charm.next_stage_cost));
        push_message_field(&mut output, 1, &entry);
    }
    push_nonzero_varint_field(&mut output, 2, u64::from(view.charm_points_available));
    push_nonzero_varint_field(&mut output, 3, u64::from(view.minor_charm_echoes_available));
    Ok(output)
}

fn decode_charm_state(input: &[u8]) -> WireResult<CharmState> {
    let [charm, kind, unlocked_stage, assigned_race, next_stage_cost] =
        read_uint32_fields::<5>(input)?;
    let kind = match kind {
        1 => CharmKind::Major,
        2 => CharmKind::Minor,
        _ => return Err(CyclopediaWireError::Malformed),
    };
    Ok(CharmState {
        charm: NonZeroU32::new(charm).ok_or(CyclopediaWireError::Malformed)?,
        kind,
        unlocked_stage: u8::try_from(unlocked_stage).map_err(|_| CyclopediaWireError::Malformed)?,
        assigned_race: NonZeroU32::new(assigned_race),
        next_stage_cost,
    })
}

pub fn decode_charm_view(payload: &[u8]) -> WireResult<CharmView> {
    if payload.len() > MAX_CHARM_VIEW_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut charms = Vec::new();
    let (mut points, mut echoes) = (None, None);
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x0a => {
                if charms.len() == MAX_CHARMS {
                    return Err(CyclopediaWireError::LimitExceeded);
                }
                charms.push(decode_charm_state(read_bytes(payload, &mut cursor)?)?);
            }
            0x10 => set_once(&mut points, read_uint32(payload, &mut cursor)?)?,
            0x18 => set_once(&mut echoes, read_uint32(payload, &mut cursor)?)?,
            _ => return Err(CyclopediaWireError::Malformed),
        }
    }
    let view = CharmView {
        charms,
        charm_points_available: points.unwrap_or(0),
        minor_charm_echoes_available: echoes.unwrap_or(0),
    };
    check_view(&view)?;
    Ok(view)
}

/// `CharmUnlockStageIntentV1`: unlock stage `expected_stage + 1` of `charm`. The expected stage
/// makes a stale view explicit instead of unlocking a stage the player did not see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharmUnlockStageIntent {
    pub charm: NonZeroU32,
    /// The stage the client saw, 0 to `MAX_CHARM_STAGE - 1`.
    pub expected_stage: u8,
}

/// `CharmAssignIntentV1`: assign `charm` to the Bestiary `race`. There is no unassign command in
/// this version (CHARM-0 §7 answer 3c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharmAssignIntent {
    pub charm: NonZeroU32,
    pub race: NonZeroU32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharmUnlockDisposition {
    Unlocked = 1,
    NotEnoughCharmPoints = 2,
    NotEnoughMinorCharmEchoes = 3,
    /// The charm is not at `expected_stage` (a stale view, or already fully unlocked).
    StageMismatch = 4,
    /// No charm has this index in the loaded content generation.
    UnknownCharm = 5,
    /// Ineligible actor, lost Character fence, or the charm system is not available.
    Rejected = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharmAssignDisposition {
    Assigned = 1,
    /// The charm has no unlocked stage.
    CharmLocked = 2,
    /// The charm already holds a race; without an unassign command it cannot be moved.
    AlreadyAssigned = 3,
    /// The race's Bestiary stage is below what the charm kind needs (major: complete; minor:
    /// stage 2; CHARM-0 §4.2).
    RaceStageTooLow = 4,
    /// The race already holds as many charms as the rule allows (CHARM-0 §4.2, rule pending the
    /// owner's live verification).
    RaceCharmLimit = 5,
    UnknownCharm = 6,
    UnknownRace = 7,
    /// Ineligible actor, lost Character fence, or the charm system is not available.
    Rejected = 8,
}

impl CharmUnlockStageIntent {
    fn check(&self) -> WireResult<()> {
        if self.charm.get() > MAX_CHARM {
            return Err(CyclopediaWireError::LimitExceeded);
        }
        if self.expected_stage >= MAX_CHARM_STAGE {
            return Err(CyclopediaWireError::Malformed);
        }
        Ok(())
    }
}

impl CharmAssignIntent {
    fn check(&self) -> WireResult<()> {
        if self.charm.get() > MAX_CHARM || self.race.get() > MAX_BESTIARY_RACE {
            return Err(CyclopediaWireError::LimitExceeded);
        }
        Ok(())
    }
}

/// `ClientCommand.payload` of the proposed command type 4. An intent the decoder would refuse
/// (a charm above its bound, or expecting the final stage) is refused before any byte is emitted.
pub fn encode_charm_unlock_stage_intent(intent: &CharmUnlockStageIntent) -> WireResult<Vec<u8>> {
    intent.check()?;
    let mut output = Vec::with_capacity(4);
    push_varint_field(&mut output, 1, u64::from(intent.charm.get()));
    push_nonzero_varint_field(&mut output, 2, u64::from(intent.expected_stage));
    Ok(output)
}

pub fn decode_charm_unlock_stage_intent(payload: &[u8]) -> WireResult<CharmUnlockStageIntent> {
    if payload.len() > MAX_CHARM_UNLOCK_STAGE_INTENT_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let [charm, expected_stage] = read_uint32_fields::<2>(payload)?;
    let intent = CharmUnlockStageIntent {
        charm: NonZeroU32::new(charm).ok_or(CyclopediaWireError::Malformed)?,
        expected_stage: u8::try_from(expected_stage).map_err(|_| CyclopediaWireError::Malformed)?,
    };
    intent.check()?;
    Ok(intent)
}

/// `ClientCommand.payload` of the proposed command type 5. An index above its bound is refused
/// before any byte is emitted.
pub fn encode_charm_assign_intent(intent: &CharmAssignIntent) -> WireResult<Vec<u8>> {
    intent.check()?;
    let mut output = Vec::with_capacity(5);
    push_varint_field(&mut output, 1, u64::from(intent.charm.get()));
    push_varint_field(&mut output, 2, u64::from(intent.race.get()));
    Ok(output)
}

pub fn decode_charm_assign_intent(payload: &[u8]) -> WireResult<CharmAssignIntent> {
    if payload.len() > MAX_CHARM_ASSIGN_INTENT_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let [charm, race] = read_uint32_fields::<2>(payload)?;
    let intent = CharmAssignIntent {
        charm: NonZeroU32::new(charm).ok_or(CyclopediaWireError::Malformed)?,
        race: NonZeroU32::new(race).ok_or(CyclopediaWireError::Malformed)?,
    };
    intent.check()?;
    Ok(intent)
}

pub fn encode_charm_unlock_stage_result(disposition: CharmUnlockDisposition) -> Vec<u8> {
    let mut output = Vec::with_capacity(2);
    push_varint_field(&mut output, 1, disposition as u64);
    output
}

pub fn decode_charm_unlock_stage_result(payload: &[u8]) -> WireResult<CharmUnlockDisposition> {
    Ok(match read_result_enum(payload, MAX_CHARM_RESULT_BYTES)? {
        1 => CharmUnlockDisposition::Unlocked,
        2 => CharmUnlockDisposition::NotEnoughCharmPoints,
        3 => CharmUnlockDisposition::NotEnoughMinorCharmEchoes,
        4 => CharmUnlockDisposition::StageMismatch,
        5 => CharmUnlockDisposition::UnknownCharm,
        6 => CharmUnlockDisposition::Rejected,
        _ => return Err(CyclopediaWireError::Malformed),
    })
}

pub fn encode_charm_assign_result(disposition: CharmAssignDisposition) -> Vec<u8> {
    let mut output = Vec::with_capacity(2);
    push_varint_field(&mut output, 1, disposition as u64);
    output
}

pub fn decode_charm_assign_result(payload: &[u8]) -> WireResult<CharmAssignDisposition> {
    Ok(match read_result_enum(payload, MAX_CHARM_RESULT_BYTES)? {
        1 => CharmAssignDisposition::Assigned,
        2 => CharmAssignDisposition::CharmLocked,
        3 => CharmAssignDisposition::AlreadyAssigned,
        4 => CharmAssignDisposition::RaceStageTooLow,
        5 => CharmAssignDisposition::RaceCharmLimit,
        6 => CharmAssignDisposition::UnknownCharm,
        7 => CharmAssignDisposition::UnknownRace,
        8 => CharmAssignDisposition::Rejected,
        _ => return Err(CyclopediaWireError::Malformed),
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::bestiary::{
        MAX_BESTIARY_VIEW_BYTES, SNAPSHOT_TYPE_CHARACTER_BESTIARY_V1,
        STATE_DOMAIN_CHARACTER_BESTIARY,
    };
    use serde_json::Value;

    const PROTOCOL_REGISTRY: &str =
        include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
    const PROPOSAL: &str = include_str!(
        "../../../docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md"
    );

    fn index(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).expect("non-zero index")
    }

    fn state(
        charm: u32,
        kind: CharmKind,
        unlocked_stage: u8,
        race: Option<u32>,
        next_stage_cost: u32,
    ) -> CharmState {
        CharmState {
            charm: index(charm),
            kind,
            unlocked_stage,
            assigned_race: race.map(index),
            next_stage_cost,
        }
    }

    fn view(charms: Vec<CharmState>, points: u32, echoes: u32) -> CharmView {
        CharmView {
            charms,
            charm_points_available: points,
            minor_charm_echoes_available: echoes,
        }
    }

    /// One `charms` entry around hand-written inner bytes: 0a <len> <inner>.
    fn entry(inner: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0x0a, u8::try_from(inner.len()).expect("short entry")];
        bytes.extend_from_slice(inner);
        bytes
    }

    /// Hand-computed canonical proto3 bytes (not produced by the encoder under test).
    fn view_fixtures() -> Vec<(CharmView, Vec<u8>)> {
        vec![
            (CharmView::default(), vec![]),
            // Echoes only: 18 64.
            (view(vec![], 0, 100), vec![0x18, 0x64]),
            // A locked major charm costing 100 (stage and race omitted); a minor charm at stage 1
            // on race 5, next cost 150 = 96 01; charm 25 = 0x19 complete on race 300 = AC 02
            // (cost omitted); 480 points = E0 03, no echoes.
            (
                view(
                    vec![
                        state(1, CharmKind::Major, 0, None, 100),
                        state(2, CharmKind::Minor, 1, Some(5), 150),
                        state(25, CharmKind::Major, 3, Some(300), 0),
                    ],
                    480,
                    0,
                ),
                [
                    entry(&[0x08, 0x01, 0x10, 0x01, 0x28, 0x64]),
                    entry(&[
                        0x08, 0x02, 0x10, 0x02, 0x18, 0x01, 0x20, 0x05, 0x28, 0x96, 0x01,
                    ]),
                    entry(&[0x08, 0x19, 0x10, 0x01, 0x18, 0x03, 0x20, 0xac, 0x02]),
                    vec![0x10, 0xe0, 0x03],
                ]
                .concat(),
            ),
        ]
    }

    #[test]
    fn charm_view_matches_independent_fixtures() {
        for (value, bytes) in view_fixtures() {
            assert_eq!(encode_charm_view(&value), Ok(bytes.clone()), "{value:?}");
            assert_eq!(decode_charm_view(&bytes), Ok(value), "{bytes:02x?}");
        }
        // Balances before entries and explicit defaults decode as the canonical form.
        assert_eq!(
            decode_charm_view(&[
                0x18, 0x00, 0x10, 0x05, 0x0a, 0x08, 0x28, 0x64, 0x18, 0x00, 0x10, 0x01, 0x08, 0x01
            ]),
            Ok(view(vec![state(1, CharmKind::Major, 0, None, 100)], 5, 0))
        );
    }

    #[test]
    fn the_full_view_at_its_bounds_is_exactly_the_byte_bound() {
        let charms: Vec<_> = (1..=MAX_CHARM)
            .map(|charm| state(charm, CharmKind::Minor, 2, Some(1024), MAX_CHARM_STAGE_COST))
            .collect();
        assert_eq!(charms.len(), MAX_CHARMS);
        let value = view(charms, MAX_CHARM_BALANCE, MAX_CHARM_BALANCE);
        let bytes = encode_charm_view(&value).expect("view at its bounds");
        assert_eq!(bytes.len(), MAX_CHARM_VIEW_BYTES);
        // The last entry and the balances by hand: race 1024 = 80 08, 100000 = A0 8D 06,
        // 1000000 = C0 84 3D.
        assert_eq!(
            bytes[bytes.len() - 23..],
            [
                0x0a, 0x0d, 0x08, 0x20, 0x10, 0x02, 0x18, 0x02, 0x20, 0x80, 0x08, 0x28, 0xa0, 0x8d,
                0x06, 0x10, 0xc0, 0x84, 0x3d, 0x18, 0xc0, 0x84, 0x3d
            ]
        );
        assert_eq!(decode_charm_view(&bytes), Ok(value));
    }

    #[test]
    fn each_view_invariant_fails_closed_in_both_directions() {
        let one = |charm: CharmState| view(vec![charm], 0, 0);
        let valid = state(1, CharmKind::Major, 0, None, 100);
        let valid_bytes = [0x08, 0x01, 0x10, 0x01, 0x28, 0x64];
        let cases: [(&str, CharmView, Vec<u8>, CyclopediaWireError); 11] = [
            (
                "charm above its bound",
                one(state(MAX_CHARM + 1, CharmKind::Major, 0, None, 100)),
                entry(&[0x08, 0x21, 0x10, 0x01, 0x28, 0x64]),
                CyclopediaWireError::LimitExceeded,
            ),
            (
                "charms out of order",
                view(vec![state(2, CharmKind::Major, 0, None, 100), valid], 0, 0),
                [
                    entry(&[0x08, 0x02, 0x10, 0x01, 0x28, 0x64]),
                    entry(&valid_bytes),
                ]
                .concat(),
                CyclopediaWireError::Malformed,
            ),
            (
                "charm repeated",
                view(vec![valid, valid], 0, 0),
                [entry(&valid_bytes), entry(&valid_bytes)].concat(),
                CyclopediaWireError::Malformed,
            ),
            (
                "stage above 3",
                one(state(1, CharmKind::Major, 4, None, 100)),
                entry(&[0x08, 0x01, 0x10, 0x01, 0x18, 0x04, 0x28, 0x64]),
                CyclopediaWireError::Malformed,
            ),
            (
                "assignment on a locked charm",
                one(state(1, CharmKind::Major, 0, Some(5), 100)),
                entry(&[0x08, 0x01, 0x10, 0x01, 0x20, 0x05, 0x28, 0x64]),
                CyclopediaWireError::Malformed,
            ),
            (
                "race above its bound",
                one(state(1, CharmKind::Major, 1, Some(1025), 100)),
                entry(&[
                    0x08, 0x01, 0x10, 0x01, 0x18, 0x01, 0x20, 0x81, 0x08, 0x28, 0x64,
                ]),
                CyclopediaWireError::LimitExceeded,
            ),
            (
                "no cost before the final stage",
                one(state(1, CharmKind::Major, 2, None, 0)),
                entry(&[0x08, 0x01, 0x10, 0x01, 0x18, 0x02]),
                CyclopediaWireError::Malformed,
            ),
            (
                "a cost at the final stage",
                one(state(1, CharmKind::Major, 3, None, 5)),
                entry(&[0x08, 0x01, 0x10, 0x01, 0x18, 0x03, 0x28, 0x05]),
                CyclopediaWireError::Malformed,
            ),
            (
                "cost above its bound",
                // 100001 = A1 8D 06.
                one(state(
                    1,
                    CharmKind::Major,
                    1,
                    None,
                    MAX_CHARM_STAGE_COST + 1,
                )),
                entry(&[0x08, 0x01, 0x10, 0x01, 0x18, 0x01, 0x28, 0xa1, 0x8d, 0x06]),
                CyclopediaWireError::LimitExceeded,
            ),
            (
                "points above their bound",
                // 1000001 = C1 84 3D.
                view(vec![], MAX_CHARM_BALANCE + 1, 0),
                vec![0x10, 0xc1, 0x84, 0x3d],
                CyclopediaWireError::LimitExceeded,
            ),
            (
                "echoes above their bound",
                view(vec![], 0, MAX_CHARM_BALANCE + 1),
                vec![0x18, 0xc1, 0x84, 0x3d],
                CyclopediaWireError::LimitExceeded,
            ),
        ];
        for (case, value, bytes, error) in cases {
            assert_eq!(encode_charm_view(&value), Err(error), "encode {case}");
            assert_eq!(decode_charm_view(&bytes), Err(error), "decode {case}");
        }
        let too_many: Vec<_> = (1..=MAX_CHARM + 1)
            .map(|charm| state(charm, CharmKind::Major, 0, None, 100))
            .collect();
        assert_eq!(
            encode_charm_view(&view(too_many, 0, 0)),
            Err(CyclopediaWireError::LimitExceeded)
        );
    }

    #[test]
    fn malformed_view_encodings_fail_closed() {
        let malformed: &[(&str, &[u8])] = &[
            ("charm absent", &[0x0a, 0x04, 0x10, 0x01, 0x28, 0x64]),
            (
                "charm zero",
                &[0x0a, 0x06, 0x08, 0x00, 0x10, 0x01, 0x28, 0x64],
            ),
            ("kind absent", &[0x0a, 0x04, 0x08, 0x01, 0x28, 0x64]),
            (
                "kind unknown",
                &[0x0a, 0x06, 0x08, 0x01, 0x10, 0x03, 0x28, 0x64],
            ),
            (
                "stage above u8",
                &[
                    0x0a, 0x09, 0x08, 0x01, 0x10, 0x01, 0x18, 0x80, 0x02, 0x28, 0x64,
                ],
            ),
            (
                "entry field 6",
                &[0x0a, 0x08, 0x08, 0x01, 0x10, 0x01, 0x28, 0x64, 0x30, 0x01],
            ),
            (
                "entry field repeated",
                &[0x0a, 0x08, 0x08, 0x01, 0x08, 0x01, 0x10, 0x01, 0x28, 0x64],
            ),
            ("points repeated", &[0x10, 0x01, 0x10, 0x01]),
            ("echoes repeated", &[0x18, 0x01, 0x18, 0x01]),
            ("unknown top-level field 4", &[0x20, 0x01]),
            ("points wrong wire type", &[0x12, 0x00]),
            ("entry length past end", &[0x0a, 0x05, 0x08, 0x01]),
            ("truncated varint", &[0x10, 0x81]),
        ];
        for (case, bytes) in malformed {
            assert_eq!(
                decode_charm_view(bytes),
                Err(CyclopediaWireError::Malformed),
                "{case}"
            );
        }
        // 33 small entries fit the byte bound; the count alone refuses them.
        let bytes: Vec<u8> = (0..=MAX_CHARMS)
            .flat_map(|_| entry(&[0x08, 0x01, 0x10, 0x01, 0x28, 0x64]))
            .collect();
        assert!(bytes.len() <= MAX_CHARM_VIEW_BYTES);
        assert_eq!(
            decode_charm_view(&bytes),
            Err(CyclopediaWireError::LimitExceeded)
        );
        assert_eq!(
            decode_charm_view(&[0; MAX_CHARM_VIEW_BYTES + 1]),
            Err(CyclopediaWireError::LimitExceeded)
        );
    }

    #[test]
    fn unlock_stage_intent_round_trips_and_fails_closed() {
        let unlock = |charm, expected_stage| CharmUnlockStageIntent {
            charm: index(charm),
            expected_stage,
        };
        for (value, bytes) in [
            (unlock(1, 0), vec![0x08, 0x01]),
            (unlock(MAX_CHARM, 2), vec![0x08, 0x20, 0x10, 0x02]),
        ] {
            assert_eq!(encode_charm_unlock_stage_intent(&value), Ok(bytes.clone()));
            assert!(bytes.len() <= MAX_CHARM_UNLOCK_STAGE_INTENT_BYTES);
            assert_eq!(decode_charm_unlock_stage_intent(&bytes), Ok(value));
        }
        assert_eq!(
            decode_charm_unlock_stage_intent(&[0x10, 0x00, 0x08, 0x01]),
            Ok(unlock(1, 0))
        );
        assert_eq!(
            encode_charm_unlock_stage_intent(&unlock(MAX_CHARM + 1, 0)),
            Err(CyclopediaWireError::LimitExceeded)
        );
        assert_eq!(
            encode_charm_unlock_stage_intent(&unlock(1, MAX_CHARM_STAGE)),
            Err(CyclopediaWireError::Malformed)
        );
        let rejected: &[(&str, &[u8], CyclopediaWireError)] = &[
            ("charm absent", &[], CyclopediaWireError::Malformed),
            ("charm zero", &[0x08, 0x00], CyclopediaWireError::Malformed),
            (
                "charm above its bound",
                &[0x08, 0x21],
                CyclopediaWireError::LimitExceeded,
            ),
            (
                "expects the final stage",
                &[0x08, 0x01, 0x10, 0x03],
                CyclopediaWireError::Malformed,
            ),
            (
                "stage above u8",
                &[0x08, 0x01, 0x10, 0x80, 0x02],
                CyclopediaWireError::Malformed,
            ),
            (
                "charm repeated",
                &[0x08, 0x01, 0x08, 0x01],
                CyclopediaWireError::Malformed,
            ),
            (
                "unknown field 3",
                &[0x08, 0x01, 0x18, 0x01],
                CyclopediaWireError::Malformed,
            ),
            (
                "wrong wire type",
                &[0x0a, 0x00],
                CyclopediaWireError::Malformed,
            ),
            ("truncated", &[0x08], CyclopediaWireError::Malformed),
            (
                "over the byte bound",
                &[0x08, 0x81, 0x80, 0x80, 0x80, 0x00, 0x10, 0x00, 0x00],
                CyclopediaWireError::LimitExceeded,
            ),
        ];
        for (case, bytes, error) in rejected {
            assert_eq!(
                decode_charm_unlock_stage_intent(bytes),
                Err(*error),
                "{case}"
            );
        }
    }

    #[test]
    fn assign_intent_round_trips_and_fails_closed() {
        let assign = |charm, race| CharmAssignIntent {
            charm: index(charm),
            race: index(race),
        };
        for (value, bytes) in [
            (assign(1, 1), vec![0x08, 0x01, 0x10, 0x01]),
            // 1024 = 80 08.
            (assign(MAX_CHARM, 1024), vec![0x08, 0x20, 0x10, 0x80, 0x08]),
        ] {
            assert_eq!(encode_charm_assign_intent(&value), Ok(bytes.clone()));
            assert!(bytes.len() <= MAX_CHARM_ASSIGN_INTENT_BYTES);
            assert_eq!(decode_charm_assign_intent(&bytes), Ok(value));
        }
        assert_eq!(
            encode_charm_assign_intent(&assign(MAX_CHARM + 1, 1)),
            Err(CyclopediaWireError::LimitExceeded)
        );
        assert_eq!(
            encode_charm_assign_intent(&assign(1, 1025)),
            Err(CyclopediaWireError::LimitExceeded)
        );
        let rejected: &[(&str, &[u8], CyclopediaWireError)] = &[
            (
                "charm absent",
                &[0x10, 0x01],
                CyclopediaWireError::Malformed,
            ),
            ("race absent", &[0x08, 0x01], CyclopediaWireError::Malformed),
            (
                "race zero",
                &[0x08, 0x01, 0x10, 0x00],
                CyclopediaWireError::Malformed,
            ),
            (
                "charm above its bound",
                &[0x08, 0x21, 0x10, 0x01],
                CyclopediaWireError::LimitExceeded,
            ),
            (
                "race above its bound",
                &[0x08, 0x01, 0x10, 0x81, 0x08],
                CyclopediaWireError::LimitExceeded,
            ),
            (
                "race repeated",
                &[0x08, 0x01, 0x10, 0x01, 0x10, 0x01],
                CyclopediaWireError::Malformed,
            ),
            (
                "unassign-shaped extra field 3",
                &[0x08, 0x01, 0x10, 0x01, 0x18, 0x01],
                CyclopediaWireError::Malformed,
            ),
            (
                "over the byte bound",
                &[0x08, 0x81, 0x80, 0x80, 0x80, 0x00, 0x10, 0x01, 0x00],
                CyclopediaWireError::LimitExceeded,
            ),
        ];
        for (case, bytes, error) in rejected {
            assert_eq!(decode_charm_assign_intent(bytes), Err(*error), "{case}");
        }
    }

    #[test]
    fn results_match_independent_fixtures_and_refuse_unknown() {
        for (disposition, value) in [
            (CharmUnlockDisposition::Unlocked, 1),
            (CharmUnlockDisposition::NotEnoughCharmPoints, 2),
            (CharmUnlockDisposition::NotEnoughMinorCharmEchoes, 3),
            (CharmUnlockDisposition::StageMismatch, 4),
            (CharmUnlockDisposition::UnknownCharm, 5),
            (CharmUnlockDisposition::Rejected, 6),
        ] {
            let bytes = [0x08, value];
            assert_eq!(encode_charm_unlock_stage_result(disposition), bytes);
            assert_eq!(decode_charm_unlock_stage_result(&bytes), Ok(disposition));
        }
        for (disposition, value) in [
            (CharmAssignDisposition::Assigned, 1),
            (CharmAssignDisposition::CharmLocked, 2),
            (CharmAssignDisposition::AlreadyAssigned, 3),
            (CharmAssignDisposition::RaceStageTooLow, 4),
            (CharmAssignDisposition::RaceCharmLimit, 5),
            (CharmAssignDisposition::UnknownCharm, 6),
            (CharmAssignDisposition::UnknownRace, 7),
            (CharmAssignDisposition::Rejected, 8),
        ] {
            let bytes = [0x08, value];
            assert_eq!(encode_charm_assign_result(disposition), bytes);
            assert_eq!(decode_charm_assign_result(&bytes), Ok(disposition));
        }
        for bad in [
            &[][..],                       // missing disposition
            &[0x08, 0x00][..],             // zero enum
            &[0x08, 0x09][..],             // unknown to both
            &[0x08, 0x01, 0x08, 0x01][..], // repeated field
            &[0x10, 0x01][..],             // unknown field
            &[0x0a, 0x00][..],             // wrong wire type
            &[0x08][..],                   // truncated
        ] {
            assert_eq!(
                decode_charm_unlock_stage_result(bad),
                Err(CyclopediaWireError::Malformed),
                "{bad:02x?}"
            );
            assert_eq!(
                decode_charm_assign_result(bad),
                Err(CyclopediaWireError::Malformed),
                "{bad:02x?}"
            );
        }
        // 7 is an assign disposition but not an unlock one.
        assert_eq!(
            decode_charm_unlock_stage_result(&[0x08, 0x07]),
            Err(CyclopediaWireError::Malformed)
        );
        let oversized = [0x08, 0x81, 0x80, 0x80, 0x00];
        assert_eq!(
            decode_charm_unlock_stage_result(&oversized),
            Err(CyclopediaWireError::LimitExceeded)
        );
        assert_eq!(
            decode_charm_assign_result(&oversized),
            Err(CyclopediaWireError::LimitExceeded)
        );
    }

    #[test]
    fn proposed_ids_are_free_or_registered_as_proposed() {
        let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
        let commands = protocol["command_types"].as_array().expect("command_types");
        for (id, name) in [
            (
                COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT,
                "CHARM_UNLOCK_STAGE_INTENT",
            ),
            (COMMAND_TYPE_CHARM_ASSIGN_INTENT, "CHARM_ASSIGN_INTENT"),
        ] {
            if let Some(command) = commands.iter().find(|command| command["id"] == id) {
                assert_eq!(
                    command["name"], name,
                    "command type {id} taken by another writer"
                );
            }
            assert!(PROPOSAL.contains(&format!("| {id} | `{name}` |")), "{name}");
        }
        let domains = protocol["state_domains"].as_array().expect("state_domains");
        for (id, name) in [
            (STATE_DOMAIN_CHARACTER_BESTIARY, "CHARACTER_BESTIARY"),
            (STATE_DOMAIN_CHARACTER_CHARMS, "CHARACTER_CHARMS"),
        ] {
            if let Some(domain) = domains.iter().find(|domain| domain["id"] == id) {
                assert_eq!(
                    domain["name"], name,
                    "state domain {id} taken by another writer"
                );
            }
            assert!(PROPOSAL.contains(&format!("| {id} | `{name}` |")), "{name}");
        }
        assert_eq!(SNAPSHOT_TYPE_CHARACTER_BESTIARY_V1, 1);
        assert_eq!(SNAPSHOT_TYPE_CHARACTER_CHARMS_V1, 1);
        assert_eq!(DELTA_TYPE_CHARACTER_CHARMS_V1, 1);
        for message in [
            "BestiaryRaceProgressV1",
            "BestiaryViewV1",
            "CharmStateV1",
            "CharmViewV1",
            "CharmUnlockStageIntentV1",
            "CharmUnlockStageResultV1",
            "CharmAssignIntentV1",
            "CharmAssignResultV1",
        ] {
            assert!(
                PROPOSAL.contains(&format!("message {message} {{")),
                "{message}"
            );
        }
        for bound in [
            MAX_BESTIARY_VIEW_BYTES,
            MAX_CHARM_VIEW_BYTES,
            MAX_CHARM_UNLOCK_STAGE_INTENT_BYTES,
            MAX_CHARM_ASSIGN_INTENT_BYTES,
            MAX_CHARM_RESULT_BYTES,
        ] {
            assert!(
                PROPOSAL.contains(&format!("At most {bound} bytes")),
                "{bound}"
            );
        }
        // No unassign command is proposed in this version (CHARM-0 §7 answer 3c).
        assert!(!PROPOSAL.contains("message CharmUnassign"));
    }
}
