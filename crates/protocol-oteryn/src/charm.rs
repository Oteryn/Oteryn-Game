//! Charm view, unlock and assign typed payloads (CHARM-5).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/charm_bestiary_v1.proto`, from
//! `docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md` §3.2 to §3.4,
//! following CHARM-0 §4.2 and the owner answers of CHARM-0 §7: only unlocks and assignments are
//! stored and the balances are derived (answer 2a). There is **no unassign command**: D170 plans
//! unassign and reset as CHARM-6, which proposes its own command types and gates the player-facing
//! charm release. The command types, state domain and resource limits below are registered in
//! `PROTOCOL_OTERYN_V1_REGISTRY.json` and `RESOURCE_LIMITS_REGISTRY.json` under capability 1
//! `BESTIARY_CHARMS_V1` (Sol ruling, #162 comment 5907282001). The server does not offer that
//! capability before CHARM-6 ships (D170).
//!
//! One `CharmViewV1` serves both the snapshot and the delta: the view is small (at most
//! [`MAX_CHARM_VIEW_BYTES`]), so every delta replaces it whole. Command results report an outcome,
//! never state; the new state arrives through the view (FIRST-CONTROL-WIRE-V1 pattern).
//!
//! Decoding is strict, and encoding refuses the same values as a server fault before any byte is
//! emitted: zero or unknown enum values, a charm or race index of zero or above its bound, charms
//! out of ascending order or repeated, a stage above 3, an assignment on a locked charm, a next
//! stage cost that is zero before the final stage or non-zero at it, an `effect_active` other than
//! 0 or 1, a cost, balance or slot limit above its bound, and an unlock that expects the final
//! stage all fail closed.

use std::num::NonZeroU32;

use crate::bestiary::MAX_BESTIARY_RACE;
pub use crate::charm_wire::CyclopediaWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes,
    read_result_enum, read_uint32, read_uint32_fields, read_varint, set_once,
};

/// Registered capability `BESTIARY_CHARMS_V1`: command types 4 and 5, state domains 4 and 5. Not
/// offered before CHARM-6 ships (D170).
pub const CAPABILITY_BESTIARY_CHARMS_V1: u32 = 1;
/// Registered command type `CHARM_UNLOCK_STAGE_INTENT` (capability 1).
pub const COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT: u32 = 4;
/// Registered command type `CHARM_ASSIGN_INTENT` (capability 1).
pub const COMMAND_TYPE_CHARM_ASSIGN_INTENT: u32 = 5;
/// Registered state domain `CHARACTER_CHARMS` (capability 1).
pub const STATE_DOMAIN_CHARACTER_CHARMS: u32 = 5;
pub const DELTA_TYPE_CHARACTER_CHARMS_V1: u32 = 1;
pub const SNAPSHOT_TYPE_CHARACTER_CHARMS_V1: u32 = 1;

/// `CHARM5-RL-03`: charms in one view. The candidate catalogue has 25 (CHARM-0 §2).
pub const MAX_CHARMS: usize = 32;
/// A charm index is 1-based into a charm list of at most [`MAX_CHARMS`] charms.
pub const MAX_CHARM: u32 = 32;
/// Charm stages are 0 (locked) to 3 (fully unlocked).
pub const MAX_CHARM_STAGE: u8 = 3;
/// `CHARM5-RL-04`: the largest single stage cost, in Charm Points or Minor Charm Echoes.
pub const MAX_CHARM_STAGE_COST: u32 = 100_000;
/// `CHARM5-RL-05`: the largest available balance of either currency.
pub const MAX_CHARM_BALANCE: u32 = 1_000_000;
/// Worst case per entry: charm 1 + 1, kind 1 + 1, stage 1 + 1, race 1 + 2, cost 1 + 3,
/// `effect_active` 1 + 1, plus the entry's own tag and length = 17 bytes; 32 entries = 544, two
/// balances of 1 + 3 and the slot limit of 1 + 1 = 554 bytes. Sol acknowledged 490 without
/// `effect_active` and ruled the field in (#162 comment 5913269950); its 2 bytes per entry make 554.
pub const MAX_CHARM_VIEW_BYTES: usize = 554;
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
    /// Whether the server applies this charm's effect today: `false` while the runtime system the
    /// effect needs does not exist (CHARM-4 `CharmMissingSystem`), shown as not yet active. The
    /// server derives it from the charm's definition (Sol ruling, #162 comment 5913269950).
    pub effect_active: bool,
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
    /// How many charms the character may hold assigned (D169: 2 free, 6 Premium), at most
    /// [`MAX_CHARMS`]; `None` (0 on the wire) is no limit (the Charm Expansion). The slots in use
    /// are the charms with an assigned race and are not sent. A display hint; CHARM-3 enforces
    /// the limit.
    pub assignment_slot_limit: Option<NonZeroU32>,
}

fn check_view(view: &CharmView) -> WireResult<()> {
    if view.charms.len() > MAX_CHARMS
        || view.charm_points_available > MAX_CHARM_BALANCE
        || view.minor_charm_echoes_available > MAX_CHARM_BALANCE
        || view
            .assignment_slot_limit
            .is_some_and(|limit| limit.get() > MAX_CHARM)
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
    let mut entry = Vec::with_capacity(15);
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
        push_nonzero_varint_field(&mut entry, 6, u64::from(charm.effect_active));
        push_message_field(&mut output, 1, &entry);
    }
    push_nonzero_varint_field(&mut output, 2, u64::from(view.charm_points_available));
    push_nonzero_varint_field(&mut output, 3, u64::from(view.minor_charm_echoes_available));
    push_nonzero_varint_field(
        &mut output,
        4,
        u64::from(view.assignment_slot_limit.map_or(0, NonZeroU32::get)),
    );
    Ok(output)
}

fn decode_charm_state(input: &[u8]) -> WireResult<CharmState> {
    let [
        charm,
        kind,
        unlocked_stage,
        assigned_race,
        next_stage_cost,
        effect_active,
    ] = read_uint32_fields::<6>(input)?;
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
        effect_active: match effect_active {
            0 => false,
            1 => true,
            _ => return Err(CyclopediaWireError::Malformed),
        },
    })
}

pub fn decode_charm_view(payload: &[u8]) -> WireResult<CharmView> {
    if payload.len() > MAX_CHARM_VIEW_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut charms = Vec::new();
    let (mut points, mut echoes, mut slot_limit) = (None, None, None);
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
            0x20 => set_once(&mut slot_limit, read_uint32(payload, &mut cursor)?)?,
            _ => return Err(CyclopediaWireError::Malformed),
        }
    }
    let view = CharmView {
        charms,
        charm_points_available: points.unwrap_or(0),
        minor_charm_echoes_available: echoes.unwrap_or(0),
        assignment_slot_limit: slot_limit.and_then(NonZeroU32::new),
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

/// `CharmAssignIntentV1`: assign `charm` to the Bestiary `race`. Unassign is not part of this
/// proposal; CHARM-6 proposes it (D170).
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
    /// The charm already holds a race; it moves only through CHARM-6 unassign (D170).
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
    /// Every assignment slot of the character is in use (D169).
    AssignmentSlotsFull = 9,
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

/// `ClientCommand.payload` of the registered command type 4. An intent the decoder would refuse
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

/// `ClientCommand.payload` of the registered command type 5. An index above its bound is refused
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
        9 => CharmAssignDisposition::AssignmentSlotsFull,
        _ => return Err(CyclopediaWireError::Malformed),
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::bestiary::{
        DELTA_TYPE_CHARACTER_BESTIARY_V1, MAX_BESTIARY_KILL_THRESHOLD, MAX_BESTIARY_VIEW_BYTES,
        MAX_BESTIARY_VIEW_ENTRIES, SNAPSHOT_TYPE_CHARACTER_BESTIARY_V1,
        STATE_DOMAIN_CHARACTER_BESTIARY,
    };
    use serde_json::Value;

    const PROTOCOL_REGISTRY: &str =
        include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
    const RESOURCE_REGISTRY: &str =
        include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");
    const SCHEMA_PATH: &str = "docs/contracts/protocol-oteryn/v1/charm_bestiary_v1.proto";
    const SCHEMA: &str =
        include_str!("../../../docs/contracts/protocol-oteryn/v1/charm_bestiary_v1.proto");

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
            effect_active: false,
        }
    }

    fn active(mut value: CharmState) -> CharmState {
        value.effect_active = true;
        value
    }

    fn view(charms: Vec<CharmState>, points: u32, echoes: u32) -> CharmView {
        CharmView {
            charms,
            charm_points_available: points,
            minor_charm_echoes_available: echoes,
            assignment_slot_limit: None,
        }
    }

    fn limited(mut value: CharmView, limit: u32) -> CharmView {
        value.assignment_slot_limit = Some(index(limit));
        value
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
            // A locked major charm costing 100 (stage and race omitted) whose effect is active
            // (30 01); a minor charm at stage 1 on race 5, next cost 150 = 96 01, effect not
            // active (omitted); charm 25 = 0x19 complete on race 300 = AC 02 (cost omitted),
            // active; 480 points = E0 03, no echoes; 6 slots (Premium) = 20 06.
            (
                limited(
                    view(
                        vec![
                            active(state(1, CharmKind::Major, 0, None, 100)),
                            state(2, CharmKind::Minor, 1, Some(5), 150),
                            active(state(25, CharmKind::Major, 3, Some(300), 0)),
                        ],
                        480,
                        0,
                    ),
                    6,
                ),
                [
                    entry(&[0x08, 0x01, 0x10, 0x01, 0x28, 0x64, 0x30, 0x01]),
                    entry(&[
                        0x08, 0x02, 0x10, 0x02, 0x18, 0x01, 0x20, 0x05, 0x28, 0x96, 0x01,
                    ]),
                    entry(&[
                        0x08, 0x19, 0x10, 0x01, 0x18, 0x03, 0x20, 0xac, 0x02, 0x30, 0x01,
                    ]),
                    vec![0x10, 0xe0, 0x03, 0x20, 0x06],
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
        // Balances before entries and explicit defaults (an explicit 0 slot limit is no limit,
        // an explicit false effect_active is inactive) decode as the canonical form.
        assert_eq!(
            decode_charm_view(&[
                0x20, 0x00, 0x18, 0x00, 0x10, 0x05, 0x0a, 0x0a, 0x30, 0x00, 0x28, 0x64, 0x18, 0x00,
                0x10, 0x01, 0x08, 0x01
            ]),
            Ok(view(vec![state(1, CharmKind::Major, 0, None, 100)], 5, 0))
        );
    }

    #[test]
    fn the_full_view_at_its_bounds_is_exactly_the_byte_bound() {
        let charms: Vec<_> = (1..=MAX_CHARM)
            .map(|charm| {
                active(state(
                    charm,
                    CharmKind::Minor,
                    2,
                    Some(1024),
                    MAX_CHARM_STAGE_COST,
                ))
            })
            .collect();
        assert_eq!(charms.len(), MAX_CHARMS);
        let value = limited(
            view(charms, MAX_CHARM_BALANCE, MAX_CHARM_BALANCE),
            MAX_CHARM,
        );
        let bytes = encode_charm_view(&value).expect("view at its bounds");
        assert_eq!(bytes.len(), MAX_CHARM_VIEW_BYTES);
        // The last entry, the balances and the slot limit by hand: race 1024 = 80 08,
        // 100000 = A0 8D 06, effect active = 30 01, 1000000 = C0 84 3D, 32 slots = 20 20.
        assert_eq!(
            bytes[bytes.len() - 27..],
            [
                0x0a, 0x0f, 0x08, 0x20, 0x10, 0x02, 0x18, 0x02, 0x20, 0x80, 0x08, 0x28, 0xa0, 0x8d,
                0x06, 0x30, 0x01, 0x10, 0xc0, 0x84, 0x3d, 0x18, 0xc0, 0x84, 0x3d, 0x20, 0x20
            ]
        );
        assert_eq!(decode_charm_view(&bytes), Ok(value));
    }

    #[test]
    fn each_view_invariant_fails_closed_in_both_directions() {
        let one = |charm: CharmState| view(vec![charm], 0, 0);
        let valid = state(1, CharmKind::Major, 0, None, 100);
        let valid_bytes = [0x08, 0x01, 0x10, 0x01, 0x28, 0x64];
        let cases: [(&str, CharmView, Vec<u8>, CyclopediaWireError); 12] = [
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
            (
                "slot limit above its bound",
                limited(view(vec![], 0, 0), MAX_CHARM + 1),
                vec![0x20, 0x21],
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
                "effect_active 2",
                &[0x0a, 0x08, 0x08, 0x01, 0x10, 0x01, 0x28, 0x64, 0x30, 0x02],
            ),
            (
                "effect_active repeated",
                &[
                    0x0a, 0x0a, 0x08, 0x01, 0x10, 0x01, 0x28, 0x64, 0x30, 0x01, 0x30, 0x01,
                ],
            ),
            (
                "entry field 7",
                &[0x0a, 0x08, 0x08, 0x01, 0x10, 0x01, 0x28, 0x64, 0x38, 0x01],
            ),
            (
                "entry field repeated",
                &[0x0a, 0x08, 0x08, 0x01, 0x08, 0x01, 0x10, 0x01, 0x28, 0x64],
            ),
            ("points repeated", &[0x10, 0x01, 0x10, 0x01]),
            ("echoes repeated", &[0x18, 0x01, 0x18, 0x01]),
            ("slot limit repeated", &[0x20, 0x02, 0x20, 0x02]),
            ("unknown top-level field 5", &[0x28, 0x01]),
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
            (CharmAssignDisposition::AssignmentSlotsFull, 9),
        ] {
            let bytes = [0x08, value];
            assert_eq!(encode_charm_assign_result(disposition), bytes);
            assert_eq!(decode_charm_assign_result(&bytes), Ok(disposition));
        }
        for bad in [
            &[][..],                       // missing disposition
            &[0x08, 0x00][..],             // zero enum
            &[0x08, 0x0a][..],             // unknown to both
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
        // 7 and 9 are assign dispositions but not unlock ones.
        for assign_only in [0x07, 0x09] {
            assert_eq!(
                decode_charm_unlock_stage_result(&[0x08, assign_only]),
                Err(CyclopediaWireError::Malformed)
            );
        }
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

    /// The IDs are registered exactly once, under these names, with the codecs' bounds: an ID
    /// taken by another entry or a bound that drifts fails here (proposal §8 step 1).
    #[test]
    fn registries_bind_the_charm_wire_ids_and_limits() {
        let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
        let only = |section: &str, id: u32| {
            let matches: Vec<&Value> = protocol[section]
                .as_array()
                .expect(section)
                .iter()
                .filter(|entry| entry["id"] == id)
                .collect();
            assert_eq!(matches.len(), 1, "{section} {id} registered once");
            matches[0].clone()
        };

        let capability = only("capabilities", CAPABILITY_BESTIARY_CHARMS_V1);
        assert_eq!(capability["name"], "BESTIARY_CHARMS_V1");
        assert_eq!(capability["owner"], "Character Authority");
        assert_eq!(
            capability["offered"], false,
            "not offered before CHARM-6 (D170)"
        );
        assert_eq!(
            capability["command_types"],
            serde_json::json!([
                COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT,
                COMMAND_TYPE_CHARM_ASSIGN_INTENT
            ])
        );
        assert_eq!(
            capability["state_domains"],
            serde_json::json!([
                STATE_DOMAIN_CHARACTER_BESTIARY,
                STATE_DOMAIN_CHARACTER_CHARMS
            ])
        );

        for (id, name, payload, result, max_payload) in [
            (
                COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT,
                "CHARM_UNLOCK_STAGE_INTENT",
                "CharmUnlockStageIntentV1",
                "CharmUnlockStageResultV1",
                MAX_CHARM_UNLOCK_STAGE_INTENT_BYTES,
            ),
            (
                COMMAND_TYPE_CHARM_ASSIGN_INTENT,
                "CHARM_ASSIGN_INTENT",
                "CharmAssignIntentV1",
                "CharmAssignResultV1",
                MAX_CHARM_ASSIGN_INTENT_BYTES,
            ),
        ] {
            let command = only("command_types", id);
            assert_eq!(command["name"], name);
            assert_eq!(command["capability"], CAPABILITY_BESTIARY_CHARMS_V1);
            assert_eq!(
                command["payload_schema"],
                format!("{SCHEMA_PATH}#{payload}")
            );
            assert_eq!(command["result_schema"], format!("{SCHEMA_PATH}#{result}"));
            assert_eq!(command["max_payload_bytes"], max_payload as u64);
            assert_eq!(
                command["max_result_payload_bytes"],
                MAX_CHARM_RESULT_BYTES as u64
            );
        }

        for (id, name, message, bound, delta, snapshot) in [
            (
                STATE_DOMAIN_CHARACTER_BESTIARY,
                "CHARACTER_BESTIARY",
                "BestiaryViewV1",
                MAX_BESTIARY_VIEW_BYTES,
                DELTA_TYPE_CHARACTER_BESTIARY_V1,
                SNAPSHOT_TYPE_CHARACTER_BESTIARY_V1,
            ),
            (
                STATE_DOMAIN_CHARACTER_CHARMS,
                "CHARACTER_CHARMS",
                "CharmViewV1",
                MAX_CHARM_VIEW_BYTES,
                DELTA_TYPE_CHARACTER_CHARMS_V1,
                SNAPSHOT_TYPE_CHARACTER_CHARMS_V1,
            ),
        ] {
            let domain = only("state_domains", id);
            assert_eq!(domain["name"], name);
            assert_eq!(domain["owner"], "Character Authority");
            assert_eq!(domain["capability"], CAPABILITY_BESTIARY_CHARMS_V1);
            for (types, type_id, suffix) in [
                ("delta_types", delta, "DELTA_V1"),
                ("snapshot_types", snapshot, "SNAPSHOT_V1"),
            ] {
                let entries = domain[types].as_array().expect(types);
                assert_eq!(entries.len(), 1, "{name} {types}");
                assert_eq!(entries[0]["id"], type_id);
                assert_eq!(entries[0]["name"], format!("{name}_{suffix}"));
                assert_eq!(
                    entries[0]["payload_schema"],
                    format!("{SCHEMA_PATH}#{message}")
                );
                assert_eq!(entries[0]["max_payload_bytes"], bound as u64);
            }
        }

        // Every registered schema anchor names a message of the proto file, which states the
        // same byte bounds; unassign is CHARM-6's own proposal (D170).
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
                SCHEMA.contains(&format!("message {message} {{")),
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
                SCHEMA.contains(&format!("At most {bound} bytes")),
                "{bound}"
            );
        }
        assert!(SCHEMA.contains("bool effect_active = 6;"));
        assert!(!SCHEMA.contains("message CharmUnassign"));

        let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
        let entries = resources["entries"].as_array().expect("entries");
        let limit = |id: &str| {
            let matches: Vec<&Value> = entries.iter().filter(|entry| entry["id"] == id).collect();
            assert_eq!(matches.len(), 1, "{id} registered once");
            matches[0]["hard_maximum"].as_u64()
        };
        assert_eq!(
            limit("CHARM5-RL-01"),
            Some(MAX_BESTIARY_VIEW_ENTRIES as u64)
        );
        assert_eq!(limit("CHARM5-RL-01"), Some(u64::from(MAX_BESTIARY_RACE)));
        assert_eq!(
            limit("CHARM5-RL-02"),
            Some(u64::from(MAX_BESTIARY_KILL_THRESHOLD))
        );
        assert_eq!(limit("CHARM5-RL-03"), Some(MAX_CHARMS as u64));
        assert_eq!(limit("CHARM5-RL-03"), Some(u64::from(MAX_CHARM)));
        assert_eq!(limit("CHARM5-RL-04"), Some(u64::from(MAX_CHARM_STAGE_COST)));
        assert_eq!(limit("CHARM5-RL-05"), Some(u64::from(MAX_CHARM_BALANCE)));
    }
}
