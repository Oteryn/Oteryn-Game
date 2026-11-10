//! Own active condition icons and server-owned remaining durations.
pub use oteryn_client_runtime::condition_status::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::ConditionStatusPanel;
pub use view::{icon_key, icon_label, panel_text, status_text};

#[cfg(test)]
#[path = "tests/e2e_c16b.rs"]
mod e2e_c16b;
#[cfg(test)]
#[path = "tests/e2e_c17a.rs"]
mod e2e_c17a;
