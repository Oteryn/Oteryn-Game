//! Compile the production fence selector and its unchanged unit test independently
//! of the large in-crate PostgreSQL scenario harness. The in-crate test is retained.
#[path = "../src/gameplay_transport/quest_callbacks/admission.rs"]
mod admission;
