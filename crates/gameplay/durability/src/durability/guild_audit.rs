//! Closed Guild activity payloads; one immutable same-transaction event per operation.
use crate::durability::item_mint_audit::check_uuid_v7;
use serde::{Deserialize, Serialize};
mod membership;
pub use membership::*;
mod text;
pub use text::*;
mod rank;
pub use rank::*;
mod resign;
pub use resign::*;
mod edit_ranks;
pub use edit_ranks::*;
mod decline;
pub use decline::*;
mod deadlines;
pub use deadlines::*;
mod disband;
pub use disband::*;
