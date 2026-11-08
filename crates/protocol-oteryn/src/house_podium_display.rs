//! Negotiated podium visibility and preview metadata. Legacy capability 150 is unchanged.
use crate::house_podium::{Projection, SetRefusal, SetRequest, SetResult};
pub const CAPABILITY_HOUSE_PODIUM_DISPLAY_V1: u32 = 449;
pub const COMMAND_TYPE_HOUSE_PODIUM_DISPLAY: u32 = 733;
pub const MAX_BYTES: usize = 512;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visibility {
    pub display: bool,
    pub platform: bool,
}
impl Default for Visibility {
    fn default() -> Self {
        Self {
            display: true,
            platform: true,
        }
    }
}
impl Visibility {
    pub fn bits(self) -> u8 {
        u8::from(self.display) | u8::from(self.platform) << 1
    }
    pub fn decode(value: u8) -> Option<Self> {
        (value <= 3).then_some(Self {
            display: value & 1 != 0,
            platform: value & 2 != 0,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Query([u8; 16]),
    QueryHandle(u64),
    SetHandle {
        handle: u64,
        value: SetRequest,
        visibility: Visibility,
    },
    PreviewHandle {
        handle: u64,
        value: SetRequest,
        visibility: Visibility,
    },
    Set {
        value: SetRequest,
        visibility: Visibility,
    },
    Preview {
        value: SetRequest,
        visibility: Visibility,
    },
}
impl Request {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        match self {
            Self::Query(item) => {
                out.push(0);
                out.extend(item);
            }
            Self::QueryHandle(handle) => {
                if *handle == 0 {
                    return None;
                }
                out.push(3);
                out.extend(handle.to_be_bytes());
            }
            Self::SetHandle {
                handle,
                value,
                visibility,
            }
            | Self::PreviewHandle {
                handle,
                value,
                visibility,
            } => {
                if *handle == 0 || value.item != [0; 16] {
                    return None;
                }
                out.extend([
                    if matches!(self, Self::SetHandle { .. }) {
                        4
                    } else {
                        5
                    },
                    visibility.bits(),
                ]);
                out.extend(handle.to_be_bytes());
                out.extend(value.encode()?);
            }
            Self::Set { value, visibility } | Self::Preview { value, visibility } => {
                out.extend([
                    if matches!(self, Self::Set { .. }) {
                        1
                    } else {
                        2
                    },
                    visibility.bits(),
                ]);
                out.extend(value.encode()?);
            }
        }
        Self::decode(&out)?;
        Some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_BYTES {
            return None;
        }
        match bytes {
            [0, item @ ..] => Some(Self::Query(item.try_into().ok()?)),
            [3, handle @ ..] => {
                let handle = u64::from_be_bytes(handle.try_into().ok()?);
                (handle != 0).then_some(Self::QueryHandle(handle))
            }
            [op @ (4 | 5), visibility, rest @ ..] if rest.len() >= 8 => {
                let handle = u64::from_be_bytes(rest[..8].try_into().ok()?);
                let value = SetRequest::decode(&rest[8..])?;
                if handle == 0 || value.item != [0; 16] {
                    return None;
                }
                let visibility = Visibility::decode(*visibility)?;
                Some(if *op == 4 {
                    Self::SetHandle {
                        handle,
                        value,
                        visibility,
                    }
                } else {
                    Self::PreviewHandle {
                        handle,
                        value,
                        visibility,
                    }
                })
            }
            [op @ (1 | 2), visibility, value @ ..] => {
                let value = SetRequest::decode(value)?;
                let visibility = Visibility::decode(*visibility)?;
                Some(if *op == 1 {
                    Self::Set { value, visibility }
                } else {
                    Self::Preview { value, visibility }
                })
            }
            _ => None,
        }
    }
}
/// Exact active-catalogue render references; zero means metadata is unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub committed: bool,
    pub podium_kind: u8,
    pub item_definition_key: String,
    pub item_definition_revision: String,
    pub platform_appearance: u32,
    /// 0 unavailable, 1 Outfit appearance, 2 Object appearance.
    pub appearance_kind: u8,
    pub appearance: u32,
    pub visibility: Visibility,
    pub projection: Option<Projection>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    State(State),
    Refused(SetRefusal),
}
impl Reply {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        match self {
            Self::Refused(reason) => out.extend([0, *reason as u8]),
            Self::State(state) => {
                out.extend([
                    1,
                    u8::from(state.committed),
                    state.podium_kind,
                    state.visibility.bits(),
                    state.appearance_kind,
                ]);
                out.extend(state.platform_appearance.to_be_bytes());
                out.extend(state.appearance.to_be_bytes());
                let key = state.item_definition_key.as_bytes();
                let revision = state.item_definition_revision.as_bytes();
                if key.is_empty() || key.len() > 160 || revision.is_empty() || revision.len() > 128
                {
                    return None;
                }
                out.push(key.len() as u8);
                out.extend(key);
                out.push(revision.len() as u8);
                out.extend(revision);
                out.extend(Projection::encode(state.projection.as_ref())?);
            }
        }
        Self::decode(&out)?;
        Some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_BYTES {
            return None;
        }
        match bytes {
            [0, reason] => match SetResult::decode(&[*reason])? {
                SetResult::Refused(r) => Some(Self::Refused(r)),
                _ => None,
            },
            [
                1,
                committed @ 0..=1,
                kind @ 1..=3,
                visibility,
                appearance_kind @ 0..=2,
                rest @ ..,
            ] if rest.len() >= 11 => {
                let platform_appearance = u32::from_be_bytes(rest[..4].try_into().ok()?);
                let appearance = u32::from_be_bytes(rest[4..8].try_into().ok()?);
                if (*appearance_kind == 0) != (appearance == 0)
                    || platform_appearance > 65535
                    || appearance > 65535
                {
                    return None;
                }
                let key_len = usize::from(*rest.get(8)?);
                if !(1..=160).contains(&key_len) {
                    return None;
                }
                let item_definition_key = std::str::from_utf8(rest.get(9..9 + key_len)?)
                    .ok()?
                    .to_owned();
                let revision_at = 9 + key_len;
                let revision_len = usize::from(*rest.get(revision_at)?);
                if !(1..=128).contains(&revision_len) {
                    return None;
                }
                let end = revision_at + 1 + revision_len;
                let item_definition_revision = std::str::from_utf8(rest.get(revision_at + 1..end)?)
                    .ok()?
                    .to_owned();
                let projection = Projection::decode(rest.get(end..)?)?;
                if projection.is_none() && *appearance_kind != 0 {
                    return None;
                }
                if projection.as_ref().is_some_and(|p| {
                    p.podium_kind != *kind || !(1..=3).contains(&p.display_kind) || p.direction > 3
                }) {
                    return None;
                }
                Some(Self::State(State {
                    committed: *committed != 0,
                    podium_kind: *kind,
                    item_definition_key,
                    item_definition_revision,
                    platform_appearance,
                    appearance_kind: *appearance_kind,
                    appearance,
                    visibility: Visibility::decode(*visibility)?,
                    projection,
                }))
            }
            _ => None,
        }
    }
}
#[cfg(test)]
#[path = "house_podium_display_tests.rs"]
mod tests;
