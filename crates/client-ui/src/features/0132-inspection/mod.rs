pub use oteryn_client_runtime::discovery::inspection::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{apply_result, request};
pub use state::Panel;
pub use view::panel_text;
#[cfg(test)]
#[path = "tests/e2e_i16.rs"]
mod e2e_i16;
