//! W18 explicit House furniture rotation. Ordinary USE remains container opening.
use crate::world_map::MAP_TARGET_BYTES;
pub use crate::world_object::UseDisposition;
use crate::world_object::{WorldObjectTarget, decode_use_result, encode_use_result};

pub const CAPABILITY_HOUSE_FURNITURE_ROTATION_V1: u32 = 445;
pub const COMMAND: u32 = 730;
pub const PAGE_SIZE: usize = 16;
pub const MAX_PAGE: u16 = 255;
pub const MAX_REQUEST_BYTES: usize = 2 + MAP_TARGET_BYTES + 8;
pub const MAX_REPLY_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Item {
        item: [u8; 16],
        expected_revision: u64,
    },
    Base(WorldObjectTarget),
}
impl Target {
    fn encode_into(&self, out: &mut Vec<u8>) -> Option<()> {
        match self {
            Self::Item {
                item,
                expected_revision,
            } => {
                if *item == [0; 16] || *expected_revision == 0 {
                    return None;
                }
                out.push(0);
                out.extend(item);
                out.extend(expected_revision.to_be_bytes());
            }
            Self::Base(target) => {
                if target.placement.len() != MAP_TARGET_BYTES {
                    return None;
                }
                out.push(1);
                out.extend(&target.placement);
                out.extend(target.expected_revision.to_be_bytes());
            }
        }
        Some(())
    }
    fn decode_at(bytes: &[u8], cursor: &mut usize) -> Option<Self> {
        let tag = take(bytes, cursor, 1)?[0];
        match tag {
            0 => {
                let item = take(bytes, cursor, 16)?.try_into().ok()?;
                let expected_revision =
                    u64::from_be_bytes(take(bytes, cursor, 8)?.try_into().ok()?);
                if item == [0; 16] || expected_revision == 0 {
                    return None;
                }
                Some(Self::Item {
                    item,
                    expected_revision,
                })
            }
            1 => Some(Self::Base(WorldObjectTarget {
                placement: take(bytes, cursor, MAP_TARGET_BYTES)?.to_vec(),
                expected_revision: u64::from_be_bytes(take(bytes, cursor, 8)?.try_into().ok()?),
            })),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Query(u16),
    Rotate(Target),
}
impl Request {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        match self {
            Self::Query(page) => {
                if *page > MAX_PAGE {
                    return None;
                }
                out.push(0);
                out.extend(page.to_be_bytes());
            }
            Self::Rotate(target) => {
                out.push(1);
                target.encode_into(&mut out)?;
            }
        }
        (out.len() <= MAX_REQUEST_BYTES).then_some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_REQUEST_BYTES {
            return None;
        }
        let mut cursor = 0;
        let request = match take(bytes, &mut cursor, 1)?[0] {
            0 => {
                let page = u16::from_be_bytes(take(bytes, &mut cursor, 2)?.try_into().ok()?);
                if page > MAX_PAGE {
                    return None;
                }
                Self::Query(page)
            }
            1 => Self::Rotate(Target::decode_at(bytes, &mut cursor)?),
            _ => return None,
        };
        (cursor == bytes.len()).then_some(request)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub label: String,
    pub target: Target,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalogue {
    pub page: u16,
    pub more: bool,
    pub entries: Vec<Entry>,
}
impl Catalogue {
    pub fn rotate(&self, index: usize) -> Option<Request> {
        Some(Request::Rotate(self.entries.get(index)?.target.clone()))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Catalogue(Catalogue),
    Applied(UseDisposition),
}
impl Reply {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        match self {
            Self::Applied(disposition) => {
                out.push(1);
                out.extend(encode_use_result(*disposition).ok()?);
            }
            Self::Catalogue(catalogue) => {
                if catalogue.page > MAX_PAGE || catalogue.entries.len() > PAGE_SIZE {
                    return None;
                }
                out.push(0);
                out.extend(catalogue.page.to_be_bytes());
                out.push(u8::from(catalogue.more));
                out.push(u8::try_from(catalogue.entries.len()).ok()?);
                for entry in &catalogue.entries {
                    if !valid_label(&entry.label) {
                        return None;
                    }
                    entry.target.encode_into(&mut out)?;
                    out.push(u8::try_from(entry.label.len()).ok()?);
                    out.extend(entry.label.as_bytes());
                }
            }
        }
        (out.len() <= MAX_REPLY_BYTES).then_some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_REPLY_BYTES {
            return None;
        }
        let mut cursor = 0;
        match take(bytes, &mut cursor, 1)?[0] {
            1 => Some(Self::Applied(decode_use_result(bytes.get(cursor..)?).ok()?)),
            0 => {
                let page = u16::from_be_bytes(take(bytes, &mut cursor, 2)?.try_into().ok()?);
                if page > MAX_PAGE {
                    return None;
                }
                let more = match take(bytes, &mut cursor, 1)?[0] {
                    0 => false,
                    1 => true,
                    _ => return None,
                };
                let count = usize::from(take(bytes, &mut cursor, 1)?[0]);
                if count > PAGE_SIZE {
                    return None;
                }
                let mut entries = Vec::with_capacity(count);
                for _ in 0..count {
                    let target = Target::decode_at(bytes, &mut cursor)?;
                    let length = usize::from(take(bytes, &mut cursor, 1)?[0]);
                    let label = std::str::from_utf8(take(bytes, &mut cursor, length)?)
                        .ok()?
                        .to_owned();
                    if !valid_label(&label) {
                        return None;
                    }
                    entries.push(Entry { label, target });
                }
                (cursor == bytes.len()).then_some(Self::Catalogue(Catalogue {
                    page,
                    more,
                    entries,
                }))
            }
            _ => None,
        }
    }
}
fn valid_label(label: &str) -> bool {
    !label.is_empty() && label.len() <= 120 && !label.chars().any(char::is_control)
}
fn take<'a>(bytes: &'a [u8], cursor: &mut usize, length: usize) -> Option<&'a [u8]> {
    let end = cursor.checked_add(length)?;
    let value = bytes.get(*cursor..end)?;
    *cursor = end;
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotation_native_targets_and_catalogue_roundtrip_and_selection()
    -> Result<(), Box<dyn std::error::Error>> {
        let item = Target::Item {
            item: [1; 16],
            expected_revision: 7,
        };
        let base = Target::Base(WorldObjectTarget {
            placement: vec![3; MAP_TARGET_BYTES],
            expected_revision: 0,
        });
        for request in [
            Request::Query(0),
            Request::Rotate(item.clone()),
            Request::Rotate(base.clone()),
        ] {
            let bytes = request.encode().ok_or("request encode")?;
            assert_eq!(Request::decode(&bytes), Some(request));
            let mut trailing = bytes.clone();
            trailing.push(0);
            assert_eq!(Request::decode(&trailing), None);
            assert_eq!(Request::decode(&bytes[..bytes.len() - 1]), None);
        }
        let catalogue = Catalogue {
            page: 0,
            more: true,
            entries: vec![
                Entry {
                    label: "Wooden chair".into(),
                    target: item,
                },
                Entry {
                    label: "Zaoan cabinet".into(),
                    target: base.clone(),
                },
            ],
        };
        assert_eq!(catalogue.rotate(1), Some(Request::Rotate(base)));
        assert_eq!(catalogue.rotate(2), None);
        let reply = Reply::Catalogue(catalogue);
        assert_eq!(
            Reply::decode(&reply.encode().ok_or("catalogue encode")?),
            Some(reply)
        );
        let reply = Reply::Applied(UseDisposition::Committed);
        assert_eq!(
            Reply::decode(&reply.encode().ok_or("applied encode")?),
            Some(reply)
        );
        assert_eq!(Request::Query(MAX_PAGE + 1).encode(), None);
        assert_eq!(
            Request::Rotate(Target::Item {
                item: [0; 16],
                expected_revision: 1
            })
            .encode(),
            None
        );
        Ok(())
    }
}
