//! PLAYER-TRADE0: whole direct-backpack leaf offers; no durable identity is exposed.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
    set_once,
};
use crate::world_spatial_entities::EntityRef;

pub const CAPABILITY_PLAYER_TRADE_V1: u32 = 60;
pub const COMMAND_TYPE_PLAYER_TRADE_INTENT: u32 = 78;
pub const STATE_DOMAIN_PLAYER_TRADE: u32 = 26;
pub const MAX_INTENT_BYTES: usize = 64;
pub const MAX_STATE_BYTES: usize = 4608;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Offer {
        partner: EntityRef,
        item: u64,
    },
    Accept {
        trade_id: [u8; 16],
    },
    Cancel,
    /// A read of one of the two offered leaves, never a transferable item handle.
    Look {
        trade_id: [u8; 16],
        counter_offer: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResultCode {
    Offered = 1,
    Accepted,
    Completed,
    Cancelled,
    TooFar,
    NotTradeable,
    PartnerBusy,
    NoRoom,
    NoBackpack,
    Stale,
    NotSupported,
    Rejected,
}
impl ResultCode {
    pub fn payload(self) -> Vec<u8> {
        vec![8, self as u8]
    }
    pub fn message(self) -> &'static str {
        match self {
            Self::Cancelled | Self::Stale => "Trade cancelled.",
            Self::TooFar => "Move closer to your trade partner.",
            Self::NotTradeable => "This item cannot be traded.",
            Self::PartnerBusy => "This player is already trading.",
            Self::NoRoom => "You do not have enough room to carry this object.",
            Self::NoBackpack => "You need a backpack to trade.",
            Self::NotSupported | Self::Rejected => "Sorry, not possible.",
            Self::Offered | Self::Accepted | Self::Completed => "",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offer {
    pub definition: u32,
    pub count: u32,
    pub sub_type: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub trade_id: Option<[u8; 16]>,
    pub partner: Option<EntityRef>,
    pub own: Option<Offer>,
    pub counter: Option<Offer>,
    pub accepted: [bool; 2],
    /// 1 OFFERED, 2 READY, 3 TRANSFERRING, 4 CLOSED.
    pub phase: u8,
    /// 0 open, 1 COMPLETED, 2 CANCELLED, 3 NO_ROOM, 4 STALE, 5 REJECTED.
    pub reason: u8,
    pub message: String,
    pub look: String,
}
impl Default for State {
    fn default() -> Self {
        Self {
            trade_id: None,
            partner: None,
            own: None,
            counter: None,
            accepted: [false; 2],
            phase: 4,
            reason: 2,
            message: String::new(),
            look: String::new(),
        }
    }
}
fn id(bytes: &[u8]) -> Result<[u8; 16], Error> {
    let id: [u8; 16] = bytes.try_into().map_err(|_| Error::Malformed)?;
    if id == [0; 16] {
        return Err(Error::Malformed);
    }
    Ok(id)
}
fn entity(e: EntityRef) -> Result<Vec<u8>, Error> {
    id(&e.identity)?;
    if e.generation == 0 {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, &e.identity);
    push_varint_field(&mut out, 2, e.generation);
    Ok(out)
}
fn read_entity(bytes: &[u8]) -> Result<EntityRef, Error> {
    let (mut identity, mut generation) = (None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => set_once(&mut identity, id(read_bytes(bytes, &mut at)?)?)?,
            16 => set_once(&mut generation, read_varint(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let e = EntityRef {
        identity: identity.ok_or(Error::Malformed)?,
        generation: generation.ok_or(Error::Malformed)?,
    };
    entity(e)?;
    Ok(e)
}
pub fn encode_intent(intent: Intent) -> Result<Vec<u8>, Error> {
    let mut inner = Vec::new();
    let field = match intent {
        Intent::Offer { partner, item } => {
            if item == 0 {
                return Err(Error::Malformed);
            }
            push_message_field(&mut inner, 1, &entity(partner)?);
            push_varint_field(&mut inner, 2, item);
            1
        }
        Intent::Accept { trade_id } => {
            id(&trade_id)?;
            inner.extend_from_slice(&trade_id);
            2
        }
        Intent::Cancel => 3,
        Intent::Look {
            trade_id,
            counter_offer,
        } => {
            id(&trade_id)?;
            push_message_field(&mut inner, 1, &trade_id);
            push_varint_field(&mut inner, 2, u64::from(counter_offer));
            4
        }
    };
    let mut out = Vec::new();
    push_message_field(&mut out, field, &inner);
    Ok(out)
}
pub fn decode_intent(bytes: &[u8]) -> Result<Intent, Error> {
    if bytes.len() > MAX_INTENT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut at = 0;
    let tag = read_varint(bytes, &mut at)?;
    let inner = read_bytes(bytes, &mut at)?;
    if at != bytes.len() {
        return Err(Error::Malformed);
    }
    let intent = match tag {
        18 => Intent::Accept {
            trade_id: id(inner)?,
        },
        26 if inner.is_empty() => Intent::Cancel,
        10 | 34 => {
            let (mut body, mut number) = (None, None);
            let mut at = 0;
            while at < inner.len() {
                match read_varint(inner, &mut at)? {
                    10 => set_once(&mut body, read_bytes(inner, &mut at)?)?,
                    16 => set_once(&mut number, read_varint(inner, &mut at)?)?,
                    _ => return Err(Error::Malformed),
                }
            }
            let body = body.ok_or(Error::Malformed)?;
            let number = number.ok_or(Error::Malformed)?;
            if tag == 10 {
                Intent::Offer {
                    partner: read_entity(body)?,
                    item: number,
                }
            } else if number <= 1 {
                Intent::Look {
                    trade_id: id(body)?,
                    counter_offer: number == 1,
                }
            } else {
                return Err(Error::Malformed);
            }
        }
        _ => return Err(Error::Malformed),
    };
    encode_intent(intent)?;
    Ok(intent)
}
fn offer(value: Offer) -> Result<Vec<u8>, Error> {
    if value.definition == 0 || value.count == 0 || value.count > 100 {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    for (field, n) in [(1, value.definition), (2, value.count), (3, value.sub_type)] {
        push_varint_field(&mut out, field, n.into());
    }
    Ok(out)
}
pub fn encode_state(state: &State) -> Result<Vec<u8>, Error> {
    if !(1..=4).contains(&state.phase)
        || state.reason > 5
        || (state.phase == 4) != (state.reason != 0)
        || (state.phase != 4 && state.partner.is_none())
        || (matches!(state.phase, 2 | 3)
            && (state.trade_id.is_none() || state.own.is_none() || state.counter.is_none()))
        || (state.counter.is_some() && state.own.is_none())
        || state.message.len() > 256
        || state.look.len() > 4096
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    if let Some(value) = state.trade_id {
        id(&value)?;
        push_message_field(&mut out, 1, &value);
    }
    if let Some(value) = state.partner {
        push_message_field(&mut out, 2, &entity(value)?);
    }
    if let Some(value) = state.own {
        push_message_field(&mut out, 3, &offer(value)?);
    }
    if let Some(value) = state.counter {
        push_message_field(&mut out, 4, &offer(value)?);
    }
    for (field, value) in [
        (5, u64::from(state.accepted[0])),
        (6, u64::from(state.accepted[1])),
        (7, state.phase.into()),
        (8, state.reason.into()),
    ] {
        push_varint_field(&mut out, field, value);
    }
    push_message_field(&mut out, 9, state.message.as_bytes());
    push_message_field(&mut out, 10, state.look.as_bytes());
    if out.len() > MAX_STATE_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}
#[cfg(test)]
#[path = "player_trade_tests.rs"]
mod tests;
