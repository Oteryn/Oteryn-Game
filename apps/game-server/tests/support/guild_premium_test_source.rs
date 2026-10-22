// Test-only re-export: the real consumer and its existing producer source, mounted once.
pub use oteryn_gameplay_social::premium::*;
#[path = "../../../../crates/gameplay/social/src/premium/test_producer.rs"]
pub mod test_producer;
