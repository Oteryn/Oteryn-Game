//! Preserve exact native domain types while adding the uninstalled Imbuement modules
//! to the canonical PostgreSQL test crate. No replacement types or authority methods.

pub use production_server::domain::*;

#[expect(
    dead_code,
    reason = "structural and access checks run in the library; PG uses stored-row data"
)]
#[path = "../../src/domain/imbuement.rs"]
pub(crate) mod imbuement;
#[path = "../../src/domain/imbuement_checkpoint.rs"]
pub(crate) mod imbuement_checkpoint;
