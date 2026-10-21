// Keep the moved Premium witness in the same crate as the standalone durable latch.
pub use production_server::domain::*;
#[path = "../../../../crates/gameplay/domain/src/domain/premium.rs"]
pub mod premium;
