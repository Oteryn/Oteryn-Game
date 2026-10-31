//! Gem Atelier query, lock and paid mutation wire contract.
use crate::charm_wire::{CyclopediaWireError as Error, push_varint_field, read_varint, set_once};

pub const CAPABILITY_WHEEL_ALLOCATION_V1: u32 = 148;
pub const COMMAND_TYPE_ATELIER_QUERY: u32 = 308;
pub const COMMAND_TYPE_ATELIER_SET_LOCK: u32 = 309;
pub const COMMAND_TYPE_ATELIER_MUTATE: u32 = 424;
pub const MAX_REQUEST_BYTES: usize = 64;
pub const MAX_REPLY_BYTES: usize = 16_384;
pub const MAX_GEMS: usize = 225;
pub const MAX_GRADE_ENTRIES: usize = 140;

/// Only player intent crosses the wire; draws, prices and resource facts are server-owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mutation {
    Reveal { quality: u8 },
    Dismantle { gem: [u8; 16] },
    SwitchDomain { gem: [u8; 16] },
    GradeUp { kind: u8, mod_id: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutateRequest {
    pub occurrence: [u8; 16],
    pub expected_revision: u64,
    pub mutation: Mutation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetLockRequest {
    pub occurrence: [u8; 16],
    pub expected_revision: u64,
    pub gem: [u8; 16],
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemView {
    pub id: [u8; 16],
    pub quality: u8,
    pub domain: u8,
    pub locked: bool,
    pub vessel: bool,
    pub modifiers: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradeView {
    pub kind: u8,
    pub mod_id: u16,
    pub grade: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityQuote {
    pub quality: u8,
    pub reveal_gold: u64,
    pub switch_gold: u64,
    pub fragment_kind: u8,
    pub fragment_min: u32,
    pub fragment_max: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradeQuote {
    pub kind: u8,
    /// The target grade, from 1 through 3.
    pub grade: u8,
    pub gold: u64,
    pub fragments: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierReply {
    pub refused: bool,
    pub revision: u64,
    pub gems: Vec<GemView>,
    pub grades: Vec<GradeView>,
    /// Current server prices are supplied by query replies; never accepted as mutation input.
    pub quality_quotes: Vec<QualityQuote>,
    pub grade_quotes: Vec<GradeQuote>,
}

pub fn encode_query() -> Vec<u8> {
    Vec::new()
}

pub fn decode_query(bytes: &[u8]) -> Result<(), Error> {
    if bytes.is_empty() {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}

pub fn encode_set_lock(value: &SetLockRequest) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    crate::charm_wire::push_message_field(&mut out, 1, &value.occurrence);
    push_varint_field(&mut out, 2, value.expected_revision);
    crate::charm_wire::push_message_field(&mut out, 3, &value.gem);
    push_varint_field(&mut out, 4, u64::from(value.locked));
    if out.len() > MAX_REQUEST_BYTES {
        Err(Error::LimitExceeded)
    } else {
        Ok(out)
    }
}

pub fn decode_set_lock(bytes: &[u8]) -> Result<SetLockRequest, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut occurrence, mut revision, mut gem, mut locked) = (None, None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => read_id(bytes, &mut at, &mut occurrence)?,
            16 => set_once(&mut revision, read_varint(bytes, &mut at)?)?,
            26 => read_id(bytes, &mut at, &mut gem)?,
            32 => set_once(
                &mut locked,
                match read_varint(bytes, &mut at)? {
                    0 => false,
                    1 => true,
                    _ => return Err(Error::Malformed),
                },
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let value = SetLockRequest {
        occurrence: occurrence.ok_or(Error::Malformed)?,
        expected_revision: revision.ok_or(Error::Malformed)?,
        gem: gem.ok_or(Error::Malformed)?,
        locked: locked.ok_or(Error::Malformed)?,
    };
    encode_set_lock(&value)?;
    Ok(value)
}

pub fn encode_mutate(value: &MutateRequest) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    crate::charm_wire::push_message_field(&mut out, 1, &value.occurrence);
    push_varint_field(&mut out, 2, value.expected_revision);
    match value.mutation {
        Mutation::Reveal { quality } if quality <= 2 => {
            push_varint_field(&mut out, 3, 0);
            push_varint_field(&mut out, 5, u64::from(quality));
        }
        Mutation::Dismantle { gem } | Mutation::SwitchDomain { gem } => {
            push_varint_field(
                &mut out,
                3,
                if matches!(value.mutation, Mutation::Dismantle { .. }) {
                    1
                } else {
                    2
                },
            );
            crate::charm_wire::push_message_field(&mut out, 4, &gem);
        }
        Mutation::GradeUp { kind, mod_id } if kind <= 1 => {
            push_varint_field(&mut out, 3, 3);
            push_varint_field(&mut out, 6, u64::from(kind));
            push_varint_field(&mut out, 7, u64::from(mod_id));
        }
        _ => return Err(Error::Malformed),
    }
    if out.len() > MAX_REQUEST_BYTES {
        Err(Error::LimitExceeded)
    } else {
        Ok(out)
    }
}

pub fn decode_mutate(bytes: &[u8]) -> Result<MutateRequest, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut occurrence, mut revision, mut action, mut gem, mut quality, mut kind, mut mod_id) =
        (None, None, None, None, None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => read_id(bytes, &mut at, &mut occurrence)?,
            16 => set_once(&mut revision, read_varint(bytes, &mut at)?)?,
            24 => set_once(&mut action, read_varint(bytes, &mut at)?)?,
            34 => read_id(bytes, &mut at, &mut gem)?,
            40 => set_once(&mut quality, read_varint(bytes, &mut at)?)?,
            48 => set_once(&mut kind, read_varint(bytes, &mut at)?)?,
            56 => set_once(&mut mod_id, read_varint(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let mutation = match (action, gem, quality, kind, mod_id) {
        (Some(0), None, Some(quality @ 0..=2), None, None) => Mutation::Reveal {
            quality: quality as u8,
        },
        (Some(1), Some(gem), None, None, None) => Mutation::Dismantle { gem },
        (Some(2), Some(gem), None, None, None) => Mutation::SwitchDomain { gem },
        (Some(3), None, None, Some(kind @ 0..=1), Some(mod_id)) => Mutation::GradeUp {
            kind: kind as u8,
            mod_id: u16::try_from(mod_id).map_err(|_| Error::Malformed)?,
        },
        _ => return Err(Error::Malformed),
    };
    Ok(MutateRequest {
        occurrence: occurrence.ok_or(Error::Malformed)?,
        expected_revision: revision.ok_or(Error::Malformed)?,
        mutation,
    })
}

fn read_id(bytes: &[u8], at: &mut usize, slot: &mut Option<[u8; 16]>) -> Result<(), Error> {
    let value = crate::charm_wire::read_bytes(bytes, at)?;
    if value.len() != 16 || slot.is_some() {
        return Err(Error::Malformed);
    }
    let mut id = [0; 16];
    id.copy_from_slice(value);
    *slot = Some(id);
    Ok(())
}

pub fn encode_reply(value: &AtelierReply) -> Result<Vec<u8>, Error> {
    if value.gems.len() > MAX_GEMS
        || value.grades.len() > MAX_GRADE_ENTRIES
        || value.quality_quotes.len() > 3
        || value.grade_quotes.len() > 6
    {
        return Err(Error::LimitExceeded);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(value.refused));
    push_varint_field(&mut out, 2, value.revision);
    for gem in &value.gems {
        if gem.quality > 2
            || gem.domain > 3
            || (!gem.modifiers.is_empty() && gem.modifiers.len() != usize::from(gem.quality) + 1)
        {
            return Err(Error::Malformed);
        }
        let mut row = Vec::new();
        crate::charm_wire::push_message_field(&mut row, 1, &gem.id);
        push_varint_field(&mut row, 2, u64::from(gem.quality));
        push_varint_field(&mut row, 3, u64::from(gem.domain));
        push_varint_field(&mut row, 4, u64::from(gem.locked));
        push_varint_field(&mut row, 5, u64::from(gem.vessel));
        for modifier in &gem.modifiers {
            push_varint_field(&mut row, 6, u64::from(*modifier));
        }
        crate::charm_wire::push_message_field(&mut out, 3, &row);
    }
    for grade in &value.grades {
        if grade.kind > 1 || grade.grade > 3 {
            return Err(Error::Malformed);
        }
        let mut row = Vec::new();
        push_varint_field(&mut row, 1, u64::from(grade.kind));
        push_varint_field(&mut row, 2, u64::from(grade.mod_id));
        push_varint_field(&mut row, 3, u64::from(grade.grade));
        crate::charm_wire::push_message_field(&mut out, 4, &row);
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
            quote.switch_gold,
            u64::from(quote.fragment_kind),
            u64::from(quote.fragment_min),
            u64::from(quote.fragment_max),
        ]);
        crate::charm_wire::push_message_field(&mut out, 5, &row);
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
        crate::charm_wire::push_message_field(&mut out, 6, &row);
    }
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
    let (mut refused, mut revision) = (None, None);
    let (mut gems, mut grades) = (Vec::new(), Vec::new());
    let (mut quality_quotes, mut grade_quotes) = (Vec::new(), Vec::new());
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut refused, read_bool(bytes, &mut at)?)?,
            16 => set_once(&mut revision, read_varint(bytes, &mut at)?)?,
            26 if gems.len() < MAX_GEMS => {
                gems.push(decode_gem(crate::charm_wire::read_bytes(bytes, &mut at)?)?);
            }
            34 if grades.len() < MAX_GRADE_ENTRIES => {
                grades.push(decode_grade(crate::charm_wire::read_bytes(
                    bytes, &mut at,
                )?)?);
            }
            42 if quality_quotes.len() < 3 => {
                let [
                    quality,
                    reveal_gold,
                    switch_gold,
                    fragment_kind,
                    fragment_min,
                    fragment_max,
                ] = decode_quote_fields(crate::charm_wire::read_bytes(bytes, &mut at)?)?;
                quality_quotes.push(QualityQuote {
                    quality: u8::try_from(quality).map_err(|_| Error::Malformed)?,
                    reveal_gold,
                    switch_gold,
                    fragment_kind: u8::try_from(fragment_kind).map_err(|_| Error::Malformed)?,
                    fragment_min: u32::try_from(fragment_min).map_err(|_| Error::Malformed)?,
                    fragment_max: u32::try_from(fragment_max).map_err(|_| Error::Malformed)?,
                });
            }
            50 if grade_quotes.len() < 6 => {
                let [kind, grade, gold, fragments] =
                    decode_quote_fields(crate::charm_wire::read_bytes(bytes, &mut at)?)?;
                grade_quotes.push(GradeQuote {
                    kind: u8::try_from(kind).map_err(|_| Error::Malformed)?,
                    grade: u8::try_from(grade).map_err(|_| Error::Malformed)?,
                    gold,
                    fragments: u32::try_from(fragments).map_err(|_| Error::Malformed)?,
                });
            }
            _ => return Err(Error::Malformed),
        }
    }
    let value = AtelierReply {
        refused: refused.ok_or(Error::Malformed)?,
        revision: revision.ok_or(Error::Malformed)?,
        gems,
        grades,
        quality_quotes,
        grade_quotes,
    };
    encode_reply(&value)?;
    Ok(value)
}

fn encode_quote_fields<const N: usize>(fields: [u64; N]) -> Vec<u8> {
    let mut row = Vec::new();
    for (index, value) in fields.into_iter().enumerate() {
        push_varint_field(&mut row, index as u64 + 1, value);
    }
    row
}

fn decode_quote_fields<const N: usize>(bytes: &[u8]) -> Result<[u64; N], Error> {
    let mut fields = [None; N];
    let mut at = 0;
    while at < bytes.len() {
        let tag = read_varint(bytes, &mut at)?;
        if tag & 7 != 0 {
            return Err(Error::Malformed);
        }
        let index = usize::try_from(tag >> 3)
            .map_err(|_| Error::Malformed)?
            .checked_sub(1)
            .ok_or(Error::Malformed)?;
        set_once(
            fields.get_mut(index).ok_or(Error::Malformed)?,
            read_varint(bytes, &mut at)?,
        )?;
    }
    let mut values = [0; N];
    for (value, field) in values.iter_mut().zip(fields) {
        *value = field.ok_or(Error::Malformed)?;
    }
    Ok(values)
}

fn read_bool(bytes: &[u8], at: &mut usize) -> Result<bool, Error> {
    match read_varint(bytes, at)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Malformed),
    }
}

fn decode_gem(bytes: &[u8]) -> Result<GemView, Error> {
    let (mut id, mut quality, mut domain, mut locked, mut vessel) = (None, None, None, None, None);
    let mut modifiers = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => read_id(bytes, &mut at, &mut id)?,
            16 => set_once(
                &mut quality,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            24 => set_once(
                &mut domain,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            32 => set_once(&mut locked, read_bool(bytes, &mut at)?)?,
            40 => set_once(&mut vessel, read_bool(bytes, &mut at)?)?,
            48 if modifiers.len() < 3 => modifiers
                .push(u16::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?),
            _ => return Err(Error::Malformed),
        }
    }
    Ok(GemView {
        id: id.ok_or(Error::Malformed)?,
        quality: quality.ok_or(Error::Malformed)?,
        domain: domain.ok_or(Error::Malformed)?,
        locked: locked.ok_or(Error::Malformed)?,
        vessel: vessel.ok_or(Error::Malformed)?,
        modifiers,
    })
}

fn decode_grade(bytes: &[u8]) -> Result<GradeView, Error> {
    let (mut kind, mut mod_id, mut grade) = (None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(
                &mut kind,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            16 => set_once(
                &mut mod_id,
                u16::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            24 => set_once(
                &mut grade,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(GradeView {
        kind: kind.ok_or(Error::Malformed)?,
        mod_id: mod_id.ok_or(Error::Malformed)?,
        grade: grade.ok_or(Error::Malformed)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lock_round_trip_and_query_shape() -> Result<(), Error> {
        let value = SetLockRequest {
            occurrence: [7; 16],
            expected_revision: 9,
            gem: [3; 16],
            locked: true,
        };
        assert_eq!(decode_set_lock(&encode_set_lock(&value)?)?, value);
        assert!(decode_query(&[]).is_ok());
        assert!(decode_query(&[0]).is_err());
        assert!(decode_set_lock(&[32, 2]).is_err());
        Ok(())
    }

    #[test]
    fn paid_actions_round_trip_without_client_economy_or_draw_facts() -> Result<(), Error> {
        for mutation in [
            Mutation::Reveal { quality: 0 },
            Mutation::Reveal { quality: 1 },
            Mutation::Reveal { quality: 2 },
            Mutation::Dismantle { gem: [2; 16] },
            Mutation::SwitchDomain { gem: [3; 16] },
            Mutation::GradeUp {
                kind: 0,
                mod_id: 45,
            },
            Mutation::GradeUp {
                kind: 1,
                mod_id: 93,
            },
        ] {
            let request = MutateRequest {
                occurrence: [7; 16],
                expected_revision: u64::MAX,
                mutation,
            };
            let bytes = encode_mutate(&request)?;
            assert_eq!(decode_mutate(&bytes)?, request);
            for suffix in [&[24, 0][..], &[40, 0][..], &[64, 1][..]] {
                let mut invalid = bytes.clone();
                invalid.extend_from_slice(suffix);
                assert!(decode_mutate(&invalid).is_err());
            }
        }
        Ok(())
    }

    #[test]
    fn paid_actions_reject_invalid_shape_enums_and_bounds() -> Result<(), Error> {
        let mut request = MutateRequest {
            occurrence: [7; 16],
            expected_revision: 1,
            mutation: Mutation::Reveal { quality: 3 },
        };
        assert!(encode_mutate(&request).is_err());
        request.mutation = Mutation::GradeUp { kind: 2, mod_id: 0 };
        assert!(encode_mutate(&request).is_err());
        assert!(decode_mutate(&[]).is_err());
        assert!(decode_mutate(&[0; MAX_REQUEST_BYTES + 1]).is_err());
        request.mutation = Mutation::Reveal { quality: 0 };
        let bytes = encode_mutate(&request)?;
        for end in 0..bytes.len() {
            assert!(decode_mutate(&bytes[..end]).is_err());
        }
        Ok(())
    }

    #[test]
    fn atelier_reply_carries_modifiers_and_character_grades() -> Result<(), Error> {
        let mut value = AtelierReply {
            refused: false,
            revision: 8,
            gems: vec![GemView {
                id: [4; 16],
                quality: 2,
                domain: 1,
                locked: false,
                vessel: true,
                modifiers: vec![3, 1, 0],
            }],
            grades: vec![GradeView {
                kind: 1,
                mod_id: 0,
                grade: 3,
            }],
            quality_quotes: vec![QualityQuote {
                quality: 1,
                reveal_gold: 1_000_000,
                switch_gold: 250_000,
                fragment_kind: 0,
                fragment_min: 2,
                fragment_max: 10,
            }],
            grade_quotes: vec![GradeQuote {
                kind: 1,
                grade: 2,
                gold: 12_000_000,
                fragments: 15,
            }],
        };
        assert_eq!(decode_reply(&encode_reply(&value)?)?, value);
        value.gems[0].modifiers.pop();
        assert!(encode_reply(&value).is_err());
        value.gems[0].modifiers.clear();
        value.grades.clear();
        value.quality_quotes.clear();
        value.grade_quotes.clear();
        // Absence of the new additive fields is a valid legacy query result.
        assert_eq!(decode_reply(&encode_reply(&value)?)?, value);
        assert!(decode_reply(&[8, 2, 16, 1]).is_err());
        Ok(())
    }

    #[test]
    fn atelier_quote_fields_reject_duplicates_unknown_fields_and_missing_values()
    -> Result<(), Error> {
        let row = encode_quote_fields([1, 2, 3, 4]);
        assert_eq!(decode_quote_fields::<4>(&row)?, [1, 2, 3, 4]);
        for field in [8, 40, 9] {
            let mut bad = row.clone();
            bad.extend_from_slice(&[field, 1]);
            assert!(decode_quote_fields::<4>(&bad).is_err());
        }
        for end in 0..row.len() {
            assert!(decode_quote_fields::<4>(&row[..end]).is_err());
        }
        Ok(())
    }
}
