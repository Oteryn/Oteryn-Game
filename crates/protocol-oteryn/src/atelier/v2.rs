//! Gem Atelier v2 (Winter Update 2026): gems without a domain, the twelve ordered Gem Vessel
//! slots and Replace Vessel. Lock and paid requests keep the v1 bytes; Switch Domain is gone.
use super::{
    GradeQuote, GradeView, SetLockRequest, decode_grade, decode_quote_fields, encode_quote_fields,
    read_bool, read_id,
};
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
    set_once,
};

pub const CAPABILITY_GEM_ATELIER_V2: u32 = 483;
pub const COMMAND_TYPE_ATELIER_V2_QUERY: u32 = 6210;
pub const COMMAND_TYPE_ATELIER_V2_SET_LOCK: u32 = 6211;
pub const COMMAND_TYPE_ATELIER_V2_MUTATE: u32 = 6212;
pub const COMMAND_TYPE_ATELIER_V2_REPLACE_VESSEL: u32 = 6213;
pub const MAX_REQUEST_BYTES: usize = super::MAX_REQUEST_BYTES;
pub const MAX_REPLY_BYTES: usize = 16_384;
pub const GEM_VESSEL_SLOTS: usize = 12;

/// Paid Atelier intent. Action tags are the v1 tags; tag 2 (Switch Domain) is malformed and tag
/// 4 is reserved for Mod Reshaping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mutation {
    Reveal { quality: u8 },
    Dismantle { gem: [u8; 16] },
    GradeUp { kind: u8, mod_id: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutateRequest {
    pub occurrence: [u8; 16],
    pub expected_revision: u64,
    pub mutation: Mutation,
}

/// Replace Vessel: puts `gem` into `slot` in one Wheel write, moving it out of any other slot
/// and returning the slot's occupant to the gem list; `None` empties the slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceVesselRequest {
    pub occurrence: [u8; 16],
    pub expected_wheel_revision: u64,
    pub slot: u8,
    pub gem: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemView {
    pub id: [u8; 16],
    pub quality: u8,
    pub locked: bool,
    /// The Gem Vessel slot (0..=11) holding this gem.
    pub slot: Option<u8>,
    pub modifiers: Vec<u16>,
}

/// One Gem Vessel slot, in slot order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VesselSlotView {
    pub quality: u8,
    pub open: bool,
    pub gem: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityQuote {
    pub quality: u8,
    pub reveal_gold: u64,
    pub fragment_kind: u8,
    pub fragment_min: u32,
    pub fragment_max: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierReply {
    pub refused: bool,
    pub revision: u64,
    pub wheel_revision: u64,
    /// The Character stands where the Atelier's location-bound actions are permitted.
    pub in_temple: bool,
    pub premium: bool,
    pub gems: Vec<GemView>,
    pub grades: Vec<GradeView>,
    pub quality_quotes: Vec<QualityQuote>,
    pub grade_quotes: Vec<GradeQuote>,
    /// Empty, or all twelve slots in slot order.
    pub vessels: Vec<VesselSlotView>,
    /// The Wheel slots (1..=36) of the vocation's Gem Vessel nodes, in slot order.
    pub vessel_nodes: Vec<u8>,
    /// Flat damage and healing each fully allocated Gem Vessel node grants.
    pub vessel_node_bonus: u32,
    /// The socketed gems' summed resistance against at least one element exceeds the 30%
    /// cap; the excess has no effect.
    pub resistance_capped: bool,
}

pub fn encode_query() -> Vec<u8> {
    Vec::new()
}

pub fn decode_query(bytes: &[u8]) -> Result<(), Error> {
    super::decode_query(bytes)
}

pub fn encode_set_lock(value: &SetLockRequest) -> Result<Vec<u8>, Error> {
    super::encode_set_lock(value)
}

pub fn decode_set_lock(bytes: &[u8]) -> Result<SetLockRequest, Error> {
    super::decode_set_lock(bytes)
}

fn v1_mutation(mutation: &Mutation) -> super::Mutation {
    match *mutation {
        Mutation::Reveal { quality } => super::Mutation::Reveal { quality },
        Mutation::Dismantle { gem } => super::Mutation::Dismantle { gem },
        Mutation::GradeUp { kind, mod_id } => super::Mutation::GradeUp { kind, mod_id },
    }
}

pub fn encode_mutate(value: &MutateRequest) -> Result<Vec<u8>, Error> {
    super::encode_mutate(&super::MutateRequest {
        occurrence: value.occurrence,
        expected_revision: value.expected_revision,
        mutation: v1_mutation(&value.mutation),
    })
}

pub fn decode_mutate(bytes: &[u8]) -> Result<MutateRequest, Error> {
    let request = super::decode_mutate(bytes)?;
    let mutation = match request.mutation {
        super::Mutation::Reveal { quality } => Mutation::Reveal { quality },
        super::Mutation::Dismantle { gem } => Mutation::Dismantle { gem },
        super::Mutation::GradeUp { kind, mod_id } => Mutation::GradeUp { kind, mod_id },
        super::Mutation::SwitchDomain { .. } => return Err(Error::Malformed),
    };
    Ok(MutateRequest {
        occurrence: request.occurrence,
        expected_revision: request.expected_revision,
        mutation,
    })
}

pub fn encode_replace_vessel(value: &ReplaceVesselRequest) -> Result<Vec<u8>, Error> {
    if usize::from(value.slot) >= GEM_VESSEL_SLOTS {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, &value.occurrence);
    push_varint_field(&mut out, 2, value.expected_wheel_revision);
    push_varint_field(&mut out, 3, u64::from(value.slot));
    if let Some(gem) = &value.gem {
        push_message_field(&mut out, 4, gem);
    }
    if out.len() > MAX_REQUEST_BYTES {
        Err(Error::LimitExceeded)
    } else {
        Ok(out)
    }
}

pub fn decode_replace_vessel(bytes: &[u8]) -> Result<ReplaceVesselRequest, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut occurrence, mut revision, mut slot, mut gem) = (None, None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => read_id(bytes, &mut at, &mut occurrence)?,
            16 => set_once(&mut revision, read_varint(bytes, &mut at)?)?,
            24 => set_once(
                &mut slot,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            34 => read_id(bytes, &mut at, &mut gem)?,
            _ => return Err(Error::Malformed),
        }
    }
    let value = ReplaceVesselRequest {
        occurrence: occurrence.ok_or(Error::Malformed)?,
        expected_wheel_revision: revision.ok_or(Error::Malformed)?,
        slot: slot.ok_or(Error::Malformed)?,
        gem,
    };
    encode_replace_vessel(&value)?;
    Ok(value)
}

pub fn encode_reply(value: &AtelierReply) -> Result<Vec<u8>, Error> {
    if value.gems.len() > super::MAX_GEMS
        || value.grades.len() > super::MAX_GRADE_ENTRIES
        || value.quality_quotes.len() > 3
        || value.grade_quotes.len() > 6
        || !(value.vessels.is_empty() || value.vessels.len() == GEM_VESSEL_SLOTS)
        || value.vessel_nodes.len() > GEM_VESSEL_SLOTS
    {
        return Err(Error::LimitExceeded);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(value.refused));
    push_varint_field(&mut out, 2, value.revision);
    for gem in &value.gems {
        if gem.quality > 2
            || gem
                .slot
                .is_some_and(|slot| usize::from(slot) >= GEM_VESSEL_SLOTS)
            || (!gem.modifiers.is_empty() && gem.modifiers.len() != usize::from(gem.quality) + 1)
        {
            return Err(Error::Malformed);
        }
        let mut row = Vec::new();
        push_message_field(&mut row, 1, &gem.id);
        push_varint_field(&mut row, 2, u64::from(gem.quality));
        push_varint_field(&mut row, 3, u64::from(gem.locked));
        if let Some(slot) = gem.slot {
            push_varint_field(&mut row, 4, u64::from(slot));
        }
        for modifier in &gem.modifiers {
            push_varint_field(&mut row, 5, u64::from(*modifier));
        }
        push_message_field(&mut out, 3, &row);
    }
    for grade in &value.grades {
        if grade.kind > 1 || grade.grade > 3 {
            return Err(Error::Malformed);
        }
        let row = encode_quote_fields([
            u64::from(grade.kind),
            u64::from(grade.mod_id),
            u64::from(grade.grade),
        ]);
        push_message_field(&mut out, 4, &row);
    }
    for quote in &value.quality_quotes {
        if quote.quality > 2
            || quote.fragment_kind > 1
            || quote.fragment_min == 0
            || quote.fragment_min > quote.fragment_max
        {
            return Err(Error::Malformed);
        }
        let row = encode_quote_fields([
            u64::from(quote.quality),
            quote.reveal_gold,
            u64::from(quote.fragment_kind),
            u64::from(quote.fragment_min),
            u64::from(quote.fragment_max),
        ]);
        push_message_field(&mut out, 5, &row);
    }
    for quote in &value.grade_quotes {
        if quote.kind > 1 || !(1..=3).contains(&quote.grade) || quote.fragments == 0 {
            return Err(Error::Malformed);
        }
        let row = encode_quote_fields([
            u64::from(quote.kind),
            u64::from(quote.grade),
            quote.gold,
            u64::from(quote.fragments),
        ]);
        push_message_field(&mut out, 6, &row);
    }
    for vessel in &value.vessels {
        if vessel.quality > 2 {
            return Err(Error::Malformed);
        }
        let mut row = Vec::new();
        push_varint_field(&mut row, 1, u64::from(vessel.quality));
        push_varint_field(&mut row, 2, u64::from(vessel.open));
        if let Some(gem) = &vessel.gem {
            push_message_field(&mut row, 3, gem);
        }
        push_message_field(&mut out, 7, &row);
    }
    push_varint_field(&mut out, 8, value.wheel_revision);
    push_varint_field(&mut out, 9, u64::from(value.in_temple));
    push_varint_field(&mut out, 10, u64::from(value.premium));
    let mut previous = 0;
    for node in &value.vessel_nodes {
        if !(1..=36).contains(node) || *node <= previous {
            return Err(Error::Malformed);
        }
        previous = *node;
        push_varint_field(&mut out, 11, u64::from(*node));
    }
    push_varint_field(&mut out, 12, u64::from(value.vessel_node_bonus));
    push_varint_field(&mut out, 13, u64::from(value.resistance_capped));
    if out.len() > MAX_REPLY_BYTES {
        Err(Error::LimitExceeded)
    } else {
        Ok(out)
    }
}

pub fn decode_reply(bytes: &[u8]) -> Result<AtelierReply, Error> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut refused, mut revision, mut wheel_revision) = (None, None, None);
    let (mut in_temple, mut premium, mut bonus) = (None, None, None);
    let mut resistance_capped = None;
    let (mut gems, mut grades, mut vessels, mut vessel_nodes) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (mut quality_quotes, mut grade_quotes) = (Vec::new(), Vec::new());
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut refused, read_bool(bytes, &mut at)?)?,
            16 => set_once(&mut revision, read_varint(bytes, &mut at)?)?,
            26 if gems.len() < super::MAX_GEMS => {
                gems.push(decode_gem(read_bytes(bytes, &mut at)?)?);
            }
            34 if grades.len() < super::MAX_GRADE_ENTRIES => {
                grades.push(decode_grade(read_bytes(bytes, &mut at)?)?);
            }
            42 if quality_quotes.len() < 3 => {
                let [
                    quality,
                    reveal_gold,
                    fragment_kind,
                    fragment_min,
                    fragment_max,
                ] = decode_quote_fields(read_bytes(bytes, &mut at)?)?;
                quality_quotes.push(QualityQuote {
                    quality: u8::try_from(quality).map_err(|_| Error::Malformed)?,
                    reveal_gold,
                    fragment_kind: u8::try_from(fragment_kind).map_err(|_| Error::Malformed)?,
                    fragment_min: u32::try_from(fragment_min).map_err(|_| Error::Malformed)?,
                    fragment_max: u32::try_from(fragment_max).map_err(|_| Error::Malformed)?,
                });
            }
            50 if grade_quotes.len() < 6 => {
                let [kind, grade, gold, fragments] =
                    decode_quote_fields(read_bytes(bytes, &mut at)?)?;
                grade_quotes.push(GradeQuote {
                    kind: u8::try_from(kind).map_err(|_| Error::Malformed)?,
                    grade: u8::try_from(grade).map_err(|_| Error::Malformed)?,
                    gold,
                    fragments: u32::try_from(fragments).map_err(|_| Error::Malformed)?,
                });
            }
            58 if vessels.len() < GEM_VESSEL_SLOTS => {
                vessels.push(decode_vessel(read_bytes(bytes, &mut at)?)?);
            }
            64 => set_once(&mut wheel_revision, read_varint(bytes, &mut at)?)?,
            72 => set_once(&mut in_temple, read_bool(bytes, &mut at)?)?,
            80 => set_once(&mut premium, read_bool(bytes, &mut at)?)?,
            88 if vessel_nodes.len() < GEM_VESSEL_SLOTS => vessel_nodes
                .push(u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?),
            96 => set_once(
                &mut bonus,
                u32::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            104 => set_once(&mut resistance_capped, read_bool(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let value = AtelierReply {
        refused: refused.ok_or(Error::Malformed)?,
        revision: revision.ok_or(Error::Malformed)?,
        wheel_revision: wheel_revision.ok_or(Error::Malformed)?,
        in_temple: in_temple.ok_or(Error::Malformed)?,
        premium: premium.ok_or(Error::Malformed)?,
        gems,
        grades,
        quality_quotes,
        grade_quotes,
        vessels,
        vessel_nodes,
        vessel_node_bonus: bonus.ok_or(Error::Malformed)?,
        resistance_capped: resistance_capped.ok_or(Error::Malformed)?,
    };
    encode_reply(&value)?;
    Ok(value)
}

fn decode_gem(bytes: &[u8]) -> Result<GemView, Error> {
    let (mut id, mut quality, mut locked, mut slot) = (None, None, None, None);
    let mut modifiers = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => read_id(bytes, &mut at, &mut id)?,
            16 => set_once(
                &mut quality,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            24 => set_once(&mut locked, read_bool(bytes, &mut at)?)?,
            32 => set_once(
                &mut slot,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            40 if modifiers.len() < 3 => modifiers
                .push(u16::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?),
            _ => return Err(Error::Malformed),
        }
    }
    Ok(GemView {
        id: id.ok_or(Error::Malformed)?,
        quality: quality.ok_or(Error::Malformed)?,
        locked: locked.ok_or(Error::Malformed)?,
        slot,
        modifiers,
    })
}

fn decode_vessel(bytes: &[u8]) -> Result<VesselSlotView, Error> {
    let (mut quality, mut open, mut gem) = (None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(
                &mut quality,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            16 => set_once(&mut open, read_bool(bytes, &mut at)?)?,
            26 => read_id(bytes, &mut at, &mut gem)?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(VesselSlotView {
        quality: quality.ok_or(Error::Malformed)?,
        open: open.ok_or(Error::Malformed)?,
        gem,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply() -> AtelierReply {
        let mut vessels: Vec<VesselSlotView> = (0..GEM_VESSEL_SLOTS)
            .map(|slot| VesselSlotView {
                quality: (slot % 3) as u8,
                open: slot < 4,
                gem: None,
            })
            .collect();
        vessels[1].gem = Some([5; 16]);
        AtelierReply {
            refused: false,
            revision: 7,
            wheel_revision: 3,
            in_temple: true,
            premium: true,
            gems: vec![
                GemView {
                    id: [5; 16],
                    quality: 1,
                    locked: true,
                    slot: Some(1),
                    modifiers: vec![12, 40],
                },
                GemView {
                    id: [6; 16],
                    quality: 0,
                    locked: false,
                    slot: None,
                    modifiers: vec![3],
                },
            ],
            grades: vec![GradeView {
                kind: 0,
                mod_id: 12,
                grade: 2,
            }],
            quality_quotes: vec![QualityQuote {
                quality: 2,
                reveal_gold: 6_000_000,
                fragment_kind: 1,
                fragment_min: 1,
                fragment_max: 3,
            }],
            grade_quotes: vec![GradeQuote {
                kind: 1,
                grade: 3,
                gold: 75_000_000,
                fragments: 30,
            }],
            vessels,
            vessel_nodes: vec![3, 5, 7, 10, 15, 18, 19, 22, 27, 30, 32, 34],
            vessel_node_bonus: 1,
            resistance_capped: true,
        }
    }

    #[test]
    fn atelier_v2_reply_round_trips_vessel_slots_and_flags() -> Result<(), Error> {
        let value = reply();
        assert_eq!(decode_reply(&encode_reply(&value)?)?, value);
        let refusal = AtelierReply {
            refused: true,
            revision: 0,
            wheel_revision: 0,
            in_temple: false,
            premium: false,
            gems: Vec::new(),
            grades: Vec::new(),
            quality_quotes: Vec::new(),
            grade_quotes: Vec::new(),
            vessels: Vec::new(),
            vessel_nodes: Vec::new(),
            vessel_node_bonus: 1,
            resistance_capped: false,
        };
        assert_eq!(decode_reply(&encode_reply(&refusal)?)?, refusal);
        let mut partial = value.clone();
        partial.vessels.pop();
        assert!(encode_reply(&partial).is_err());
        let mut unordered = value;
        unordered.vessel_nodes.swap(0, 1);
        assert!(encode_reply(&unordered).is_err());
        Ok(())
    }

    #[test]
    fn atelier_v2_reply_has_no_domain_field() -> Result<(), Error> {
        let bytes = encode_reply(&reply())?;
        let mut at = 0;
        let mut gem_fields = Vec::new();
        while at < bytes.len() {
            let tag = read_varint(&bytes, &mut at)?;
            if tag & 7 == 2 {
                let row = read_bytes(&bytes, &mut at)?;
                if tag >> 3 == 3 {
                    let mut inner = 0;
                    while inner < row.len() {
                        let field = read_varint(row, &mut inner)?;
                        gem_fields.push(field >> 3);
                        if field & 7 == 2 {
                            read_bytes(row, &mut inner)?;
                        } else {
                            read_varint(row, &mut inner)?;
                        }
                    }
                }
            } else {
                read_varint(&bytes, &mut at)?;
            }
        }
        // id, quality, locked, slot, modifiers; the v1 domain (field 3, a value 0..=3) is gone.
        assert_eq!(gem_fields, [1, 2, 3, 4, 5, 5, 1, 2, 3, 5]);
        // A v1 reader cannot mistake a v2 reply for a v1 one.
        assert!(super::super::decode_reply(&bytes).is_err());
        Ok(())
    }

    #[test]
    fn atelier_v2_requests_round_trip_and_switch_domain_is_gone() -> Result<(), Error> {
        for mutation in [
            Mutation::Reveal { quality: 2 },
            Mutation::Dismantle { gem: [2; 16] },
            Mutation::GradeUp {
                kind: 1,
                mod_id: 93,
            },
        ] {
            let request = MutateRequest {
                occurrence: [7; 16],
                expected_revision: 4,
                mutation,
            };
            let bytes = encode_mutate(&request)?;
            assert_eq!(decode_mutate(&bytes)?, request);
            // The same bytes as the v1 request, so one Atelier writer serves both.
            assert!(super::super::decode_mutate(&bytes).is_ok());
        }
        let switch = super::super::encode_mutate(&super::super::MutateRequest {
            occurrence: [7; 16],
            expected_revision: 4,
            mutation: super::super::Mutation::SwitchDomain { gem: [2; 16] },
        })?;
        assert!(
            super::super::decode_mutate(&switch).is_ok(),
            "v1 still decodes"
        );
        assert_eq!(decode_mutate(&switch), Err(Error::Malformed));
        for request in [
            ReplaceVesselRequest {
                occurrence: [1; 16],
                expected_wheel_revision: u64::MAX,
                slot: 11,
                gem: Some([9; 16]),
            },
            ReplaceVesselRequest {
                occurrence: [1; 16],
                expected_wheel_revision: 0,
                slot: 0,
                gem: None,
            },
        ] {
            let bytes = encode_replace_vessel(&request)?;
            assert_eq!(decode_replace_vessel(&bytes)?, request);
            let mut repeated = bytes.clone();
            repeated.extend_from_slice(&[24, 1]);
            assert!(decode_replace_vessel(&repeated).is_err());
        }
        let mut beyond = vec![10, 16];
        beyond.extend([1; 16]);
        beyond.extend([16, 0, 24, 12]);
        assert!(
            decode_replace_vessel(&beyond).is_err(),
            "slot 12 does not exist"
        );
        assert!(decode_query(&[]).is_ok());
        Ok(())
    }
}
