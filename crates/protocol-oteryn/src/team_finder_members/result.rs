use super::{
    Applicant, BrowseListing, MAX_NAME_BYTES, MAX_PAGE_ENTRIES, MAX_RESULT_BYTES, MemberStatus,
    TeamFinderResult, TeamFinderWireError as Error,
};
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};
use crate::team_finder::{decode_upsert, encode_upsert};
use std::collections::BTreeSet;

fn validate_page(handles: impl Iterator<Item = u64>, next: Option<u64>) -> Result<(), Error> {
    let mut seen = BTreeSet::new();
    let mut last = None;
    for handle in handles {
        if handle == 0 || !seen.insert(handle) {
            return Err(Error::Malformed);
        }
        if seen.len() > MAX_PAGE_ENTRIES {
            return Err(Error::LimitExceeded);
        }
        last = Some(handle);
    }
    if next.is_some() && next != last {
        return Err(Error::Malformed);
    }
    Ok(())
}

fn validate_name(value: &str) -> Result<(), Error> {
    if value.is_empty() {
        return Err(Error::Malformed);
    }
    if value.len() > MAX_NAME_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(())
}

pub fn encode_result(value: &TeamFinderResult) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let next = match value {
        TeamFinderResult::Browse { listings, next } => {
            validate_page(listings.iter().map(|entry| entry.leader_handle), *next)?;
            push_varint_field(&mut out, 1, 1);
            for entry in listings {
                validate_name(&entry.name)?;
                let mut inner = Vec::new();
                push_varint_field(&mut inner, 1, entry.leader_handle);
                push_message_field(&mut inner, 2, entry.name.as_bytes());
                push_message_field(&mut inner, 3, &encode_upsert(entry.listing)?);
                push_varint_field(&mut inner, 4, u64::from(entry.occupied_slots));
                push_varint_field(&mut inner, 5, entry.status as u64);
                push_message_field(&mut out, 2, &inner);
            }
            *next
        }
        TeamFinderResult::Applicants {
            listing,
            occupied_slots,
            members,
            next,
        } => {
            validate_page(members.iter().map(|entry| entry.handle), *next)?;
            if listing.is_none() && (*occupied_slots != 0 || !members.is_empty() || next.is_some())
            {
                return Err(Error::Malformed);
            }
            push_varint_field(&mut out, 1, 2);
            if let Some(listing) = listing {
                push_message_field(&mut out, 3, &encode_upsert(*listing)?);
            }
            push_varint_field(&mut out, 4, u64::from(*occupied_slots));
            for member in members {
                validate_name(&member.name)?;
                if member.status == MemberStatus::None {
                    return Err(Error::Malformed);
                }
                let mut inner = Vec::new();
                push_varint_field(&mut inner, 1, member.handle);
                push_message_field(&mut inner, 2, member.name.as_bytes());
                push_varint_field(&mut inner, 3, u64::from(member.level));
                push_varint_field(&mut inner, 4, u64::from(member.vocation));
                push_varint_field(&mut inner, 5, member.status as u64);
                push_message_field(&mut out, 5, &inner);
            }
            *next
        }
    };
    if let Some(next) = next {
        push_varint_field(&mut out, 6, next);
    }
    if out.len() > MAX_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

pub fn decode_result(input: &[u8]) -> Result<TeamFinderResult, Error> {
    if input.len() > MAX_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut kind, mut listing, mut occupied_slots, mut next) = (None, None, None, None);
    let (mut listings, mut members) = (Vec::new(), Vec::new());
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            8 => set_once(&mut kind, read_varint(input, &mut cursor)?)?,
            18 => {
                if listings.len() >= MAX_PAGE_ENTRIES {
                    return Err(Error::LimitExceeded);
                }
                listings.push(decode_listing(read_bytes(input, &mut cursor)?)?);
            }
            26 => set_once(
                &mut listing,
                decode_upsert(read_bytes(input, &mut cursor)?)?,
            )?,
            32 => set_once(&mut occupied_slots, read_u16(input, &mut cursor)?)?,
            42 => {
                if members.len() >= MAX_PAGE_ENTRIES {
                    return Err(Error::LimitExceeded);
                }
                members.push(decode_applicant(read_bytes(input, &mut cursor)?)?);
            }
            48 => set_once(&mut next, read_varint(input, &mut cursor)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let result = match (kind, listing, occupied_slots) {
        (Some(1), None, None) if members.is_empty() => TeamFinderResult::Browse { listings, next },
        (Some(2), listing, Some(occupied_slots)) if listings.is_empty() => {
            TeamFinderResult::Applicants {
                listing,
                occupied_slots,
                members,
                next,
            }
        }
        _ => return Err(Error::Malformed),
    };
    // Shared validation also rejects repeated/zero handles and inconsistent page cursors.
    encode_result(&result)?;
    Ok(result)
}

fn read_u16(input: &[u8], cursor: &mut usize) -> Result<u16, Error> {
    u16::try_from(read_varint(input, cursor)?).map_err(|_| Error::LimitExceeded)
}

fn read_u8(input: &[u8], cursor: &mut usize) -> Result<u8, Error> {
    u8::try_from(read_varint(input, cursor)?).map_err(|_| Error::LimitExceeded)
}

fn read_name(input: &[u8], cursor: &mut usize) -> Result<String, Error> {
    let name = std::str::from_utf8(read_bytes(input, cursor)?).map_err(|_| Error::Malformed)?;
    validate_name(name)?;
    Ok(name.to_owned())
}

fn decode_listing(input: &[u8]) -> Result<BrowseListing, Error> {
    let (mut leader_handle, mut name, mut listing, mut occupied_slots, mut status) =
        (None, None, None, None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            8 => set_once(&mut leader_handle, read_varint(input, &mut cursor)?)?,
            18 => set_once(&mut name, read_name(input, &mut cursor)?)?,
            26 => set_once(
                &mut listing,
                decode_upsert(read_bytes(input, &mut cursor)?)?,
            )?,
            32 => set_once(&mut occupied_slots, read_u16(input, &mut cursor)?)?,
            40 => set_once(
                &mut status,
                MemberStatus::try_from(read_u8(input, &mut cursor)?)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(BrowseListing {
        leader_handle: leader_handle.ok_or(Error::Malformed)?,
        name: name.ok_or(Error::Malformed)?,
        listing: listing.ok_or(Error::Malformed)?,
        occupied_slots: occupied_slots.ok_or(Error::Malformed)?,
        status: status.ok_or(Error::Malformed)?,
    })
}

fn decode_applicant(input: &[u8]) -> Result<Applicant, Error> {
    let (mut handle, mut name, mut level, mut vocation, mut status) =
        (None, None, None, None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            8 => set_once(&mut handle, read_varint(input, &mut cursor)?)?,
            18 => set_once(&mut name, read_name(input, &mut cursor)?)?,
            24 => set_once(&mut level, read_u16(input, &mut cursor)?)?,
            32 => set_once(&mut vocation, read_u8(input, &mut cursor)?)?,
            40 => set_once(
                &mut status,
                MemberStatus::try_from(read_u8(input, &mut cursor)?)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(Applicant {
        handle: handle.ok_or(Error::Malformed)?,
        name: name.ok_or(Error::Malformed)?,
        level: level.ok_or(Error::Malformed)?,
        vocation: vocation.ok_or(Error::Malformed)?,
        status: status.ok_or(Error::Malformed)?,
    })
}
