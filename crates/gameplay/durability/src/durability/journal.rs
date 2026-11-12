//! Private items exports. The terminal release owns the atomic source cut.
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionAuthoritySnapshot, WorldId};
use oteryn_gameplay_native_admission_source::journal::{
    ItemRow, ItemsBegin, ItemsEnd, ItemsPage, ItemsPayload, PublishGameJournalRecordV1,
    ReadGameJournalCursorV1,
};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
pub(in crate::durability) mod capture;
mod delivery;
mod ownership;
mod scheduling;
pub use delivery::{DeliveryClaim, ProducerStore};
type Result<T> = std::result::Result<T, DurabilityError>;
fn invalid<T>(_: T) -> DurabilityError {
    DurabilityError::InvalidStoredState
}
/// Complete depot IDs of the admitted World, including empty depots. Callers use
/// BundleFacts rather than the player's current item rows to build this catalogue.
#[derive(Debug, Clone)]
pub struct LogoutCatalogue {
    pub(crate) world: WorldId,
    pub(crate) depots: Vec<u16>,
}
impl LogoutCatalogue {
    pub fn new(world: WorldId, depots: Vec<u16>) -> Result<Self> {
        if depots.len() > 256 || !depots.windows(2).all(|v| v[0] < v[1]) {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Self { world, depots })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    Disabled,
    Unavailable,
    Queued,
}
