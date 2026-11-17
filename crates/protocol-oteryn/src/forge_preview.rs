//! Optional bounded read-only candidates and quotes for cap152's unchanged mutations.
use crate::item_forge::{Operation, Request as ForgeRequest};
use crate::item_view::ItemHandle;
/// Shares the existing Forge capability; this projection adds no capability ID.
pub const CAPABILITY_FORGE_PREVIEW_V1: u32 = crate::item_forge::CAPABILITY_ITEM_FORGE_V1;
pub const COMMAND_TYPE_FORGE_PREVIEW: u32 = 6116;
pub const MAX_REQUEST_BYTES: usize = 60;
pub const MAX_REPLY_BYTES: usize = 32768;
pub const PAGE_SIZE: usize = 32;
pub const MAX_NAME_BYTES: usize = 128;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Effects(ItemHandle),
    Resource(Operation),
    Browse {
        after: u64,
    },
    Pair {
        operation: Operation,
        first: ItemHandle,
        second: ItemHandle,
        boost: bool,
        protect: bool,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub nonce: u64,
    pub selection: Selection,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub handle: ItemHandle,
    pub definition_ref: u32,
    pub appearance: u32,
    pub name: String,
    pub classification: u8,
    pub tier: u8,
    pub maximum_tier: u8,
    pub container: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Resources {
    pub carried_gold: u64,
    pub bank_gold: u64,
    pub bank_spendable: bool,
    pub dust: u32,
    pub dust_limit: u32,
    pub cores: u64,
    pub slivers: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cost {
    pub gold: u64,
    pub dust: u32,
    pub cores: u8,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub after: u64,
    pub next: Option<u64>,
    pub items: Vec<Item>,
    pub resources: Resources,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quote {
    pub selection: Selection,
    pub first: Item,
    pub second: Item,
    pub resources: Resources,
    pub cost: Cost,
    pub target_tier: u8,
    pub success_percent: u8,
    pub protection_percent: u8,
}
/// Advisory exact resource transition calculated by the same pure rules as command181.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceQuote {
    pub operation: Operation,
    /// Dust-limit reviews may omit unrelated inventory counts; never display their zero placeholders.
    pub full_resources: bool,
    pub before: Resources,
    pub after_dust: u32,
    pub after_limit: u32,
    pub after_slivers: u64,
    pub after_cores: u64,
}
pub fn resource_operation(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::DustToSlivers | Operation::SliversToCores | Operation::DustLimit
    )
}
impl ResourceQuote {
    fn valid(self) -> bool {
        (self.full_resources
            || (self.operation == Operation::DustLimit
                && self.before.cores == 0
                && self.before.slivers == 0
                && self.after_cores == 0
                && self.after_slivers == 0))
            && self.before.dust <= self.before.dust_limit
            && self.after_dust <= self.after_limit
            && match self.operation {
                Operation::DustToSlivers => {
                    self.after_dust < self.before.dust
                        && self.after_limit == self.before.dust_limit
                        && self.after_slivers > self.before.slivers
                        && self.after_cores == self.before.cores
                }
                Operation::SliversToCores => {
                    self.after_dust == self.before.dust
                        && self.after_limit == self.before.dust_limit
                        && self.after_slivers < self.before.slivers
                        && self.after_cores > self.before.cores
                }
                Operation::DustLimit => {
                    self.after_dust < self.before.dust
                        && self.after_limit > self.before.dust_limit
                        && self.after_slivers == self.before.slivers
                        && self.after_cores == self.before.cores
                }
                _ => false,
            }
    }
}
/// Reference descriptions from the server's existing inspection owner, indexed by tier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effects {
    pub item: Item,
    pub tiers: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    Effects(Effects),
    Resource(ResourceQuote),
    Page(Page),
    Quote(Quote),
    Refused,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub nonce: u64,
    pub payload: Payload,
}
impl Selection {
    pub fn canonical(self, occurrence: [u8; 16]) -> Option<ForgeRequest> {
        let Self::Pair {
            operation,
            first,
            second,
            boost,
            protect,
        } = self
        else {
            return None;
        };
        if first == second || !operation.requires_pair() {
            return None;
        }
        let request = ForgeRequest {
            operation,
            occurrence,
            first_item: Some(first),
            second_item: Some(second),
            boost,
            protect,
            history_limit: 0,
        };
        request.is_canonical().then_some(request)
    }
}
impl Request {
    pub fn encode(self) -> Option<Vec<u8>> {
        if self.nonce == 0 {
            return None;
        }
        let mut out = self.nonce.to_le_bytes().to_vec();
        match self.selection {
            Selection::Effects(handle) => {
                out.push(3);
                out.extend(handle.get().to_le_bytes());
            }
            Selection::Resource(operation) if resource_operation(operation) => {
                out.extend([2, operation as u8]);
            }
            Selection::Resource(_) => return None,
            Selection::Browse { after } => {
                out.push(0);
                out.extend(after.to_le_bytes());
            }
            pair => {
                out.push(1);
                out.extend(pair.canonical([0; 16])?.encode()?);
            }
        }
        Some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_REQUEST_BYTES || bytes.len() < 9 {
            return None;
        }
        let nonce = u64::from_le_bytes(bytes[..8].try_into().ok()?);
        if nonce == 0 {
            return None;
        }
        let selection = match bytes[8] {
            3 if bytes.len() == 17 => Selection::Effects(ItemHandle::new(u64::from_le_bytes(
                bytes[9..].try_into().ok()?,
            ))?),
            2 if bytes.len() == 10 => Selection::Resource(match bytes[9] {
                5 => Operation::DustToSlivers,
                6 => Operation::SliversToCores,
                7 => Operation::DustLimit,
                _ => return None,
            }),
            0 if bytes.len() == 17 => Selection::Browse {
                after: u64::from_le_bytes(bytes[9..].try_into().ok()?),
            },
            1 if bytes.len() == 60 => {
                let r = ForgeRequest::decode(&bytes[9..])?;
                if r.occurrence != [0; 16] || !r.operation.requires_pair() {
                    return None;
                }
                Selection::Pair {
                    operation: r.operation,
                    first: r.first_item?,
                    second: r.second_item?,
                    boost: r.boost,
                    protect: r.protect,
                }
            }
            _ => return None,
        };
        if matches!(selection, Selection::Pair { .. }) && selection.canonical([0; 16]).is_none() {
            return None;
        }
        Some(Self { nonce, selection })
    }
}
#[path = "forge_preview/reply.rs"]
mod reply;
#[cfg(test)]
#[path = "forge_preview/tests.rs"]
mod tests;
