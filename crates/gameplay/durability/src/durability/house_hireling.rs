//! E12 durable Hireling identities, owner entitlements, lamp custody and service admission.
//! Each mutation composes with the existing item or Store owner in one transaction.

mod appearance;
pub(crate) mod audit;
pub(crate) mod fence;
pub(crate) mod food;
mod ingress;
mod lamp;
mod model;
mod read;
pub(crate) use read::read_house_hirelings_in_transaction;
mod recall;
mod receipts;
mod return_lamp;
mod store_grants;

pub use food::{HirelingFoodError, HirelingFoodOutcome, HirelingFoodRequest};
pub use ingress::{HirelingLampInput, HirelingReturnInput};
pub use lamp::HirelingLampPlacement;
pub use model::{HIRELING_LAMP_KEY, HirelingSnapshot, MAX_HIRELINGS, npc_identity, npc_key};
pub use recall::HirelingRecallState;
pub use receipts::{HirelingReceiptRead, command_identity, return_binding};
pub use return_lamp::{HirelingLampReturn, HirelingRefusal, STORE_INBOX_CAPACITY};
pub(crate) use store_grants::{grant_store_hireling, grant_store_hireling_entitlement};

/// Reserved E12 native item-use cause; never accepted by the generic consumable writer.
pub const LAMP_USE_CAUSE: u32 = 470;
