//! INSPECT-1: bounded native inspection, independent of the reference opcodes.
use crate::charm_wire::{
    CyclopediaWireError, push_message_field, push_varint_field, read_bytes, read_uint32,
    read_varint, set_once,
};
use crate::item_look::{
    ItemLookIntent, ItemLookSelection, ItemLookTarget, decode_item_look_intent_for,
    encode_item_look_intent_for,
};
use crate::item_view::ItemHandle;
use crate::world_spatial_entities::EntityRef;

pub const CAPABILITY_INSPECTION_V1: u32 = 132;
pub const COMMAND_TYPE_INSPECTION: u32 = 153;
pub const STATE_DOMAIN_INSPECTION: u32 = 97;
pub const MAX_REQUEST_BYTES: usize = 128;
pub const MAX_RESULT_BYTES: usize = 65_536;
pub const MAX_ITEMS: usize = 10;
pub const MAX_ATTRIBUTES: usize = 64;
pub const MAX_TEXT_BYTES: usize = 1024;
pub const MAX_PERMISSION_PEERS: usize = 32;
pub type WireError = CyclopediaWireError;
type Result<T> = std::result::Result<T, WireError>;
const EXTENDED: ItemLookSelection = ItemLookSelection { extended: true };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Item(ItemHandle),
    Player(EntityRef),
    Trade {
        counter: bool,
        index: u8,
    },
    Catalogue(u32),
    /// 1 ask, 2 grant, 3 revoke, 4 allow all, 5 disable allow all, 6 poll requests.
    Permission {
        action: u8,
        target: Option<EntityRef>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectedItem {
    /// Zero for a single item, native equipment slots 1–9, main backpack 10.
    pub slot: u8,
    pub definition_ref: u32,
    pub count: u32,
    pub tier: u8,
    pub name: String,
    pub attributes: Vec<(String, String)>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// 1 success, 2 unavailable, 3 not visible, 4 too far, 5 rate limited,
    /// 6 permission denied, 7 request queued, 8 permission acknowledgement.
    pub status: u8,
    pub items: Vec<InspectedItem>,
    pub requests: Vec<EntityRef>,
}
impl Reply {
    pub fn refused(status: u8) -> Self {
        Self {
            status,
            items: Vec::new(),
            requests: Vec::new(),
        }
    }
}
fn bad<T>() -> Result<T> {
    Err(WireError::Malformed)
}
fn bounded(bytes: &[u8], max: usize) -> Result<()> {
    if bytes.len() > max {
        Err(WireError::LimitExceeded)
    } else {
        Ok(())
    }
}
fn text(bytes: &[u8]) -> Result<String> {
    bounded(bytes, MAX_TEXT_BYTES)?;
    let s = std::str::from_utf8(bytes).map_err(|_| WireError::Malformed)?;
    if s.is_empty() || s.contains('\0') {
        return bad();
    }
    Ok(s.to_owned())
}
fn fields(input: &[u8], mut f: impl FnMut(u64, &[u8], &mut usize) -> Result<()>) -> Result<()> {
    let mut cursor = 0;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        f(key, input, &mut cursor)?;
    }
    Ok(())
}
pub fn encode_request(request: Request) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    match request {
        Request::Item(handle) => push_message_field(
            &mut out,
            1,
            &encode_item_look_intent_for(
                ItemLookIntent {
                    target: ItemLookTarget::Item(handle),
                },
                EXTENDED,
            )
            .map_err(|_| WireError::Malformed)?,
        ),
        Request::Player(entity) if entity.generation != 0 => push_message_field(
            &mut out,
            2,
            &encode_item_look_intent_for(
                ItemLookIntent {
                    target: ItemLookTarget::Creature(entity),
                },
                EXTENDED,
            )
            .map_err(|_| WireError::Malformed)?,
        ),
        Request::Trade { counter, index } => {
            push_varint_field(&mut out, 3, (u64::from(index) << 1) | u64::from(counter))
        }
        Request::Catalogue(reference) if reference != 0 => {
            push_varint_field(&mut out, 4, u64::from(reference))
        }
        Request::Permission { action, target } => {
            if !(1..=6).contains(&action) || target.is_some() != (action <= 3) {
                return bad();
            }
            let mut bytes = Vec::new();
            push_varint_field(&mut bytes, 1, u64::from(action));
            if let Some(entity) = target {
                if entity.generation == 0 {
                    return bad();
                }
                push_message_field(
                    &mut bytes,
                    2,
                    &encode_item_look_intent_for(
                        ItemLookIntent {
                            target: ItemLookTarget::Creature(entity),
                        },
                        EXTENDED,
                    )
                    .map_err(|_| WireError::Malformed)?,
                );
            }
            push_message_field(&mut out, 5, &bytes);
        }
        _ => return bad(),
    }
    Ok(out)
}
pub fn decode_request(input: &[u8]) -> Result<Request> {
    bounded(input, MAX_REQUEST_BYTES)?;
    let mut result = None;
    fields(input, |key, input, cursor| {
        let request = match key {
            10 | 18 => {
                let intent = decode_item_look_intent_for(read_bytes(input, cursor)?, EXTENDED)
                    .map_err(|_| WireError::Malformed)?;
                match (key, intent.target) {
                    (10, ItemLookTarget::Item(handle)) => Request::Item(handle),
                    (18, ItemLookTarget::Creature(entity)) if entity.generation != 0 => {
                        Request::Player(entity)
                    }
                    _ => return bad(),
                }
            }
            24 => {
                let n = read_uint32(input, cursor)?;
                if n > 511 {
                    return bad();
                }
                Request::Trade {
                    counter: n & 1 != 0,
                    index: (n >> 1) as u8,
                }
            }
            32 => {
                let n = read_uint32(input, cursor)?;
                if n == 0 {
                    return bad();
                }
                Request::Catalogue(n)
            }
            42 => {
                let mut action = None;
                let mut target = None;
                fields(read_bytes(input, cursor)?, |key, input, cursor| {
                    match key {
                        8 => set_once(&mut action, read_uint32(input, cursor)?)?,
                        18 => {
                            let intent =
                                decode_item_look_intent_for(read_bytes(input, cursor)?, EXTENDED)
                                    .map_err(|_| WireError::Malformed)?;
                            let ItemLookTarget::Creature(entity) = intent.target else {
                                return bad();
                            };
                            if entity.generation == 0 {
                                return bad();
                            }
                            set_once(&mut target, entity)?;
                        }
                        _ => return bad(),
                    }
                    Ok(())
                })?;
                let action = action.ok_or(WireError::Malformed)?;
                if !(1..=6).contains(&action) || target.is_some() != (action <= 3) {
                    return bad();
                }
                Request::Permission {
                    action: action as u8,
                    target,
                }
            }
            _ => return bad(),
        };
        set_once(&mut result, request)
    })?;
    result.ok_or(WireError::Malformed)
}
pub fn encode_reply(reply: &Reply) -> Result<Vec<u8>> {
    if !(1..=8).contains(&reply.status) || (reply.status != 1 && !reply.items.is_empty()) {
        return bad();
    }
    if !reply.requests.is_empty() && reply.status != 8 {
        return bad();
    }
    if reply.requests.len() > MAX_PERMISSION_PEERS {
        return Err(WireError::LimitExceeded);
    }
    if reply.items.len() > MAX_ITEMS {
        return Err(WireError::LimitExceeded);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(reply.status));
    for item in &reply.items {
        if item.definition_ref == 0 || item.count == 0 || item.slot > 10 || item.tier > 10 {
            return bad();
        }
        if item.attributes.len() > MAX_ATTRIBUTES {
            return Err(WireError::LimitExceeded);
        }
        let mut bytes = Vec::new();
        for (field, value) in [
            (1, item.slot as u64),
            (2, item.definition_ref as u64),
            (3, item.count as u64),
            (4, item.tier as u64),
        ] {
            push_varint_field(&mut bytes, field, value);
        }
        text(item.name.as_bytes())?;
        push_message_field(&mut bytes, 5, item.name.as_bytes());
        for (label, value) in &item.attributes {
            text(label.as_bytes())?;
            text(value.as_bytes())?;
            let mut pair = Vec::new();
            push_message_field(&mut pair, 1, label.as_bytes());
            push_message_field(&mut pair, 2, value.as_bytes());
            push_message_field(&mut bytes, 6, &pair);
        }
        push_message_field(&mut out, 2, &bytes);
        bounded(&out, MAX_RESULT_BYTES)?;
    }
    for (index, entity) in reply.requests.iter().enumerate() {
        if entity.generation == 0 || reply.requests[..index].contains(entity) {
            return bad();
        }
        push_message_field(
            &mut out,
            3,
            &encode_item_look_intent_for(
                ItemLookIntent {
                    target: ItemLookTarget::Creature(*entity),
                },
                EXTENDED,
            )
            .map_err(|_| WireError::Malformed)?,
        );
    }
    bounded(&out, MAX_RESULT_BYTES)?;
    Ok(out)
}
pub fn decode_reply(input: &[u8]) -> Result<Reply> {
    bounded(input, MAX_RESULT_BYTES)?;
    let mut status = None;
    let mut items = Vec::new();
    let mut requests = Vec::new();
    fields(input, |key, input, cursor| {
        match key {
            8 => set_once(
                &mut status,
                u8::try_from(read_uint32(input, cursor)?).map_err(|_| WireError::Malformed)?,
            )?,
            18 => {
                if items.len() == MAX_ITEMS {
                    return Err(WireError::LimitExceeded);
                }
                let mut numbers = [None; 4];
                let mut name = None;
                let mut attributes = Vec::new();
                fields(read_bytes(input, cursor)?, |key, input, cursor| {
                    match key {
                        8 | 16 | 24 | 32 => set_once(
                            &mut numbers[(key / 8 - 1) as usize],
                            read_uint32(input, cursor)?,
                        )?,
                        42 => set_once(&mut name, text(read_bytes(input, cursor)?)?)?,
                        50 => {
                            if attributes.len() == MAX_ATTRIBUTES {
                                return Err(WireError::LimitExceeded);
                            }
                            let mut label = None;
                            let mut value = None;
                            fields(read_bytes(input, cursor)?, |key, input, cursor| match key {
                                10 => set_once(&mut label, text(read_bytes(input, cursor)?)?),
                                18 => set_once(&mut value, text(read_bytes(input, cursor)?)?),
                                _ => bad(),
                            })?;
                            attributes.push((
                                label.ok_or(WireError::Malformed)?,
                                value.ok_or(WireError::Malformed)?,
                            ));
                        }
                        _ => return bad(),
                    }
                    Ok(())
                })?;
                items.push(InspectedItem {
                    slot: u8::try_from(numbers[0].unwrap_or(0))
                        .map_err(|_| WireError::Malformed)?,
                    definition_ref: numbers[1].unwrap_or(0),
                    count: numbers[2].unwrap_or(0),
                    tier: u8::try_from(numbers[3].unwrap_or(0))
                        .map_err(|_| WireError::Malformed)?,
                    name: name.ok_or(WireError::Malformed)?,
                    attributes,
                });
            }
            26 => {
                if requests.len() == MAX_PERMISSION_PEERS {
                    return Err(WireError::LimitExceeded);
                }
                let intent = decode_item_look_intent_for(read_bytes(input, cursor)?, EXTENDED)
                    .map_err(|_| WireError::Malformed)?;
                let ItemLookTarget::Creature(entity) = intent.target else {
                    return bad();
                };
                if entity.generation == 0 || requests.contains(&entity) {
                    return bad();
                }
                requests.push(entity);
            }
            _ => return bad(),
        }
        Ok(())
    })?;
    let reply = Reply {
        status: status.ok_or(WireError::Malformed)?,
        items,
        requests,
    };
    encode_reply(&reply)?;
    Ok(reply)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inspection_wire_independent_golden() -> Result<()> {
        // Hand-written schema bytes provide an oracle independent of the paired codec.
        assert_eq!(encode_request(Request::Catalogue(1))?, [0x20, 0x01]);
        assert_eq!(
            encode_request(Request::Trade {
                counter: true,
                index: 255
            })?,
            [0x18, 0xff, 0x03]
        );
        assert_eq!(
            encode_request(Request::Permission {
                action: 6,
                target: None
            })?,
            [0x2a, 0x02, 0x08, 0x06]
        );
        assert_eq!(decode_request(&[0x20, 0x01])?, Request::Catalogue(1));
        assert_eq!(encode_reply(&Reply::refused(6))?, [0x08, 0x06]);
        let bytes = [
            0x08, 0x01, 0x12, 0x13, 0x08, 0x00, 0x10, 0x01, 0x18, 0x01, 0x20, 0x00, 0x2a, 0x01,
            b'X', 0x32, 0x06, 0x0a, 0x01, b'W', 0x12, 0x01, b'Z',
        ];
        let reply = Reply {
            status: 1,
            requests: Vec::new(),
            items: vec![InspectedItem {
                slot: 0,
                definition_ref: 1,
                count: 1,
                tier: 0,
                name: "X".into(),
                attributes: vec![("W".into(), "Z".into())],
            }],
        };
        assert_eq!(encode_reply(&reply)?, bytes);
        assert_eq!(decode_reply(&bytes)?, reply);
        Ok(())
    }
    #[test]
    fn inspection_wire_roundtrip_and_bounds() -> Result<()> {
        for request in [
            Request::Permission {
                action: 1,
                target: Some(EntityRef {
                    identity: [4; 16],
                    generation: 2,
                }),
            },
            Request::Permission {
                action: 6,
                target: None,
            },
            Request::Catalogue(1),
            Request::Trade {
                counter: true,
                index: 255,
            },
            Request::Player(EntityRef {
                identity: [3; 16],
                generation: 1,
            }),
        ] {
            let bytes = encode_request(request)?;
            assert_eq!(decode_request(&bytes)?, request);
            let mut duplicate = bytes.clone();
            duplicate.extend_from_slice(&bytes);
            assert!(decode_request(&duplicate).is_err());
        }
        assert!(
            encode_request(Request::Permission {
                action: 1,
                target: None
            })
            .is_err()
        );
        assert!(
            encode_request(Request::Permission {
                action: 6,
                target: Some(EntityRef {
                    identity: [4; 16],
                    generation: 2
                })
            })
            .is_err()
        );
        assert!(decode_request(&[32, 0]).is_err());
        assert!(decode_request(&[0; MAX_REQUEST_BYTES + 1]).is_err());
        let reply = Reply {
            status: 1,
            requests: Vec::new(),
            items: vec![InspectedItem {
                slot: 0,
                definition_ref: 1,
                count: 3,
                tier: 2,
                name: "sword".into(),
                attributes: vec![("Weight".into(), "30.00 oz".into())],
            }],
        };
        assert_eq!(decode_reply(&encode_reply(&reply)?)?, reply);
        let mut large = reply.clone();
        large.items[0].name = "x".repeat(MAX_TEXT_BYTES + 1);
        assert!(encode_reply(&large).is_err());
        let mut attributes = reply.clone();
        attributes.items[0].attributes = vec![("A".into(), "B".into()); MAX_ATTRIBUTES];
        assert!(encode_reply(&attributes).is_ok());
        attributes.items[0]
            .attributes
            .push(("C".into(), "D".into()));
        assert!(encode_reply(&attributes).is_err());
        let mut permissions = Reply::refused(8);
        permissions.requests = vec![EntityRef {
            identity: [1; 16],
            generation: 1,
        }];
        assert_eq!(decode_reply(&encode_reply(&permissions)?)?, permissions);
        permissions.requests = vec![
            EntityRef {
                identity: [1; 16],
                generation: 1
            };
            MAX_PERMISSION_PEERS + 1
        ];
        assert!(encode_reply(&permissions).is_err());
        let mut large = reply.clone();
        large.items = vec![reply.items[0].clone(); MAX_ITEMS + 1];
        assert!(encode_reply(&large).is_err());
        assert_eq!(
            decode_reply(&encode_reply(&Reply::refused(6))?)?,
            Reply::refused(6)
        );
        Ok(())
    }
}
