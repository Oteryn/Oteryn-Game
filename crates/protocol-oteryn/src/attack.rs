//! Attack target, fight mode and actor combat state typed payloads (ATTACK-WIRE-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/attack_v1.proto`, from ATTACK-0 §3. Capability 17
//! `ATTACK_V1` (requires 6 `WORLD_SPATIAL_ENTITIES`), command types 11 `ATTACK_TARGET_INTENT` and
//! 12 `FIGHT_MODES_INTENT`, and state domain 10 `ACTOR_COMBAT_STATE` are registered in
//! `PROTOCOL_OTERYN_V1_REGISTRY.json`, leased by the #1622 control plane (D486). The server does
//! not offer the capability before ATTACK-1b composes it.
//!
//! Decoding is strict, and encoding refuses the same values as a fault before any byte is
//! emitted: a target whose identity is not 16 bytes or whose generation is zero, a zero or
//! unknown enum, a bool above 1 on the wire, a payload over its byte bound, and unknown or
//! repeated fields all fail closed. A command with no target stops attacking.

use crate::charm_wire::{
    WireResult, push_message_field, push_varint_field, read_bytes, read_result_enum, read_uint32,
    read_varint, set_once,
};
use crate::world_spatial_entities::{ENTITY_IDENTITY_BYTES, EntityRef};

pub use crate::charm_wire::CyclopediaWireError as AttackWireError;

/// Registered capability `ATTACK_V1`: command types 11 and 12, state domain 10.
pub const CAPABILITY_ATTACK_V1: u32 = 17;
/// The capability `ATTACK_V1` requires: 6 `WORLD_SPATIAL_ENTITIES`, which names the targets.
pub const CAPABILITY_ATTACK_V1_REQUIRES: [u32; 1] =
    [crate::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES];
/// Registered command type `ATTACK_TARGET_INTENT` (capability 17).
pub const COMMAND_TYPE_ATTACK_TARGET_INTENT: u32 = 11;
/// Registered command type `FIGHT_MODES_INTENT` (capability 17).
pub const COMMAND_TYPE_FIGHT_MODES_INTENT: u32 = 12;
/// Registered state domain `ACTOR_COMBAT_STATE` (capability 17).
pub const STATE_DOMAIN_ACTOR_COMBAT_STATE: u32 = 10;
/// Snapshot type 1: the whole [`ActorCombatState`].
pub const SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1: u32 = 1;
/// Delta type 1: the whole [`ActorCombatState`], replaced.
pub const DELTA_TYPE_ACTOR_COMBAT_STATE_V1: u32 = 1;

/// `ATTACK0-RL-01`: target changes (command type 11) of one GameSession per second.
pub const MAX_ATTACK_TARGET_CHANGES_PER_SECOND: u32 = 25;
/// `ATTACK0-RL-02`: fight-mode changes (command type 12) of one GameSession per second.
pub const MAX_FIGHT_MODE_CHANGES_PER_SECOND: u32 = 25;

/// An `EntityRefV1` with a target: identity 1 + 1 + 16, generation 1 + 10.
const MAX_TARGET_REF_BYTES: usize = 29;
/// `AttackTargetIntentV1`: the target 1 + 1 + 29.
pub const MAX_ATTACK_TARGET_INTENT_BYTES: usize = 31;
/// `FightModesIntentV1`: three fields of 1 + 1.
pub const MAX_FIGHT_MODES_INTENT_BYTES: usize = 6;
/// `AttackIntentResultV1`: the disposition 1 + 1.
pub const MAX_ATTACK_INTENT_RESULT_BYTES: usize = 2;
/// `ActorCombatStateV1`: the target 31 and four fields of 1 + 1.
pub const MAX_ACTOR_COMBAT_STATE_BYTES: usize = 39;

/// `FightMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightMode {
    Offensive = 1,
    Balanced = 2,
    Defensive = 3,
}

impl FightMode {
    fn from_wire(value: u32) -> WireResult<Self> {
        match value {
            1 => Ok(Self::Offensive),
            2 => Ok(Self::Balanced),
            3 => Ok(Self::Defensive),
            _ => Err(AttackWireError::Malformed),
        }
    }
}

/// `ChaseMode`. `Chase` is carried now; the server treats it as `Stand` until CHASE-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseMode {
    Stand = 1,
    Chase = 2,
}

impl ChaseMode {
    fn from_wire(value: u32) -> WireResult<Self> {
        match value {
            1 => Ok(Self::Stand),
            2 => Ok(Self::Chase),
            _ => Err(AttackWireError::Malformed),
        }
    }
}

/// `FightModesIntentV1`, also the modes in `ActorCombatStateV1`. `secure` has no effect before
/// PvP (ATTACK-0 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FightModes {
    pub fight_mode: FightMode,
    pub chase: ChaseMode,
    pub secure: bool,
}

impl FightModes {
    /// The runtime defaults at admission (ATTACK-0 §3): balanced, stand, secure on.
    pub const DEFAULT: Self = Self {
        fight_mode: FightMode::Balanced,
        chase: ChaseMode::Stand,
        secure: true,
    };
}

/// `AttackIntentDisposition`, the result of command types 11 and 12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackIntentDisposition {
    Ok = 1,
    TargetNotVisible = 2,
    TargetNotACreature = 3,
    ProtectionZone = 4,
    ReentryProtected = 5,
    Rejected = 6,
}

impl AttackIntentDisposition {
    const ALL: [Self; 6] = [
        Self::Ok,
        Self::TargetNotVisible,
        Self::TargetNotACreature,
        Self::ProtectionZone,
        Self::ReentryProtected,
        Self::Rejected,
    ];
}

/// `ActorCombatStateV1`: the own actor's target (or none), modes and in-fight flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorCombatState {
    pub target: Option<EntityRef>,
    pub modes: FightModes,
    pub in_fight: bool,
}

fn check_target(target: &EntityRef) -> WireResult<()> {
    if target.generation == 0 {
        return Err(AttackWireError::Malformed);
    }
    Ok(())
}

fn encode_target(output: &mut Vec<u8>, field: u64, target: &EntityRef) -> WireResult<()> {
    check_target(target)?;
    let mut inner = Vec::with_capacity(MAX_TARGET_REF_BYTES);
    push_message_field(&mut inner, 1, &target.identity);
    push_varint_field(&mut inner, 2, target.generation);
    push_message_field(output, field, &inner);
    Ok(())
}

fn decode_target(input: &[u8]) -> WireResult<EntityRef> {
    let (mut identity, mut generation) = (None, None);
    read_fields(input, |number, wire, input, cursor| match (number, wire) {
        (1, 2) => {
            let bytes: [u8; ENTITY_IDENTITY_BYTES] = read_bytes(input, cursor)?
                .try_into()
                .map_err(|_| AttackWireError::Malformed)?;
            set_once(&mut identity, bytes)
        }
        (2, 0) => set_once(&mut generation, read_varint(input, cursor)?),
        _ => Err(AttackWireError::Malformed),
    })?;
    let target = EntityRef {
        identity: identity.ok_or(AttackWireError::Malformed)?,
        generation: generation.ok_or(AttackWireError::Malformed)?,
    };
    check_target(&target)?;
    Ok(target)
}

/// Reads the fields of a message: `field(number, wire_type, input, cursor)` consumes one value
/// or refuses the key.
fn read_fields(
    input: &[u8],
    mut field: impl FnMut(u64, u64, &[u8], &mut usize) -> WireResult<()>,
) -> WireResult<()> {
    let mut cursor = 0;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        field(key >> 3, key & 0x07, input, &mut cursor)?;
    }
    Ok(())
}

/// A proto3 bool: an encoder omits false, but one written explicitly as 0 is false too. Any value
/// above 1 fails closed.
fn read_bool(input: &[u8], cursor: &mut usize) -> WireResult<bool> {
    match read_varint(input, cursor)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(AttackWireError::Malformed),
    }
}

fn push_bool_field(output: &mut Vec<u8>, field: u64, value: bool) {
    if value {
        push_varint_field(output, field, 1);
    }
}

fn push_modes(output: &mut Vec<u8>, first: u64, modes: &FightModes) {
    push_varint_field(output, first, modes.fight_mode as u64);
    push_varint_field(output, first + 1, modes.chase as u64);
    push_bool_field(output, first + 2, modes.secure);
}

#[derive(Default)]
struct ModeFields {
    fight_mode: Option<u32>,
    chase: Option<u32>,
    secure: Option<bool>,
}

impl ModeFields {
    /// Reads field `number` if it is one of the three mode fields starting at `first`.
    fn read(
        &mut self,
        first: u64,
        number: u64,
        wire: u64,
        input: &[u8],
        cursor: &mut usize,
    ) -> WireResult<()> {
        match (number.checked_sub(first), wire) {
            (Some(0), 0) => set_once(&mut self.fight_mode, read_uint32(input, cursor)?),
            (Some(1), 0) => set_once(&mut self.chase, read_uint32(input, cursor)?),
            (Some(2), 0) => set_once(&mut self.secure, read_bool(input, cursor)?),
            _ => Err(AttackWireError::Malformed),
        }
    }

    fn modes(self) -> WireResult<FightModes> {
        Ok(FightModes {
            fight_mode: FightMode::from_wire(self.fight_mode.unwrap_or(0))?,
            chase: ChaseMode::from_wire(self.chase.unwrap_or(0))?,
            secure: self.secure.unwrap_or(false),
        })
    }
}

fn check_size(payload: &[u8], maximum: usize) -> WireResult<()> {
    if payload.len() > maximum {
        return Err(AttackWireError::LimitExceeded);
    }
    Ok(())
}

/// Encodes an `AttackTargetIntentV1` (the client side); `None` stops attacking.
pub fn encode_attack_target_intent(target: Option<&EntityRef>) -> WireResult<Vec<u8>> {
    let mut output = Vec::with_capacity(MAX_ATTACK_TARGET_INTENT_BYTES);
    if let Some(target) = target {
        encode_target(&mut output, 1, target)?;
    }
    Ok(output)
}

/// Decodes an `AttackTargetIntentV1`; `None` stops attacking. The server answers `REJECTED` to
/// any error.
pub fn decode_attack_target_intent(payload: &[u8]) -> WireResult<Option<EntityRef>> {
    check_size(payload, MAX_ATTACK_TARGET_INTENT_BYTES)?;
    let mut target = None;
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 2) => set_once(&mut target, decode_target(read_bytes(input, cursor)?)?),
            _ => Err(AttackWireError::Malformed),
        }
    })?;
    Ok(target)
}

/// Encodes a `FightModesIntentV1` (the client side).
pub fn encode_fight_modes_intent(modes: &FightModes) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_FIGHT_MODES_INTENT_BYTES);
    push_modes(&mut output, 1, modes);
    output
}

/// Decodes a `FightModesIntentV1`; the server answers `REJECTED` to any error.
pub fn decode_fight_modes_intent(payload: &[u8]) -> WireResult<FightModes> {
    check_size(payload, MAX_FIGHT_MODES_INTENT_BYTES)?;
    let mut fields = ModeFields::default();
    read_fields(payload, |number, wire, input, cursor| {
        fields.read(1, number, wire, input, cursor)
    })?;
    fields.modes()
}

/// Encodes an `AttackIntentResultV1` (the server side).
pub fn encode_attack_intent_result(disposition: AttackIntentDisposition) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_ATTACK_INTENT_RESULT_BYTES);
    push_varint_field(&mut output, 1, disposition as u64);
    output
}

pub fn decode_attack_intent_result(payload: &[u8]) -> WireResult<AttackIntentDisposition> {
    let value = read_result_enum(payload, MAX_ATTACK_INTENT_RESULT_BYTES)?;
    AttackIntentDisposition::ALL
        .into_iter()
        .find(|disposition| *disposition as u32 == value)
        .ok_or(AttackWireError::Malformed)
}

/// Encodes the snapshot (type 1) or delta (type 1) payload. A target with a zero generation is
/// a server fault.
pub fn encode_actor_combat_state(state: &ActorCombatState) -> WireResult<Vec<u8>> {
    let mut output = Vec::with_capacity(MAX_ACTOR_COMBAT_STATE_BYTES);
    if let Some(target) = &state.target {
        encode_target(&mut output, 1, target)?;
    }
    push_modes(&mut output, 2, &state.modes);
    push_bool_field(&mut output, 5, state.in_fight);
    Ok(output)
}

pub fn decode_actor_combat_state(payload: &[u8]) -> WireResult<ActorCombatState> {
    check_size(payload, MAX_ACTOR_COMBAT_STATE_BYTES)?;
    let (mut target, mut modes, mut in_fight) = (None, ModeFields::default(), None);
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 2) => set_once(&mut target, decode_target(read_bytes(input, cursor)?)?),
            (5, 0) => set_once(&mut in_fight, read_bool(input, cursor)?),
            _ => modes.read(2, number, wire, input, cursor),
        }
    })?;
    Ok(ActorCombatState {
        target,
        modes: modes.modes()?,
        in_fight: in_fight.unwrap_or(false),
    })
}

#[cfg(test)]
#[path = "attack_tests.rs"]
mod tests;
