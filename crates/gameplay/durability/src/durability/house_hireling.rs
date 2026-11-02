//! E12 durable Hireling identities, owner entitlements, lamp custody and service admission.
//! Each mutation composes with the existing item or Store owner in one transaction.

mod appearance;
pub(crate) mod audit;
pub(crate) mod fence;
mod ingress;
mod lamp;
pub(crate) mod food;
mod model;
mod read;
pub(crate) use read::read_house_hirelings_in_transaction;
mod recall;
mod receipts;
mod return_lamp;
mod store_grants;

pub use model::{HirelingSnapshot, HIRELING_LAMP_KEY, MAX_HIRELINGS, npc_identity, npc_key};
pub use lamp::HirelingLampPlacement;
pub use ingress::{HirelingLampInput,HirelingReturnInput};
pub use receipts::{HirelingReceiptRead,command_identity,return_binding};
pub use return_lamp::{HirelingLampReturn, HirelingRefusal, STORE_INBOX_CAPACITY};
pub use recall::HirelingRecallState;
pub use food::{HirelingFoodRequest, HirelingFoodError, HirelingFoodOutcome};
pub(crate) use store_grants::{grant_store_hireling, grant_store_hireling_entitlement};

/// Reserved E12 native item-use cause; never accepted by the generic consumable writer.
pub const LAMP_USE_CAUSE: u32 = 470;
