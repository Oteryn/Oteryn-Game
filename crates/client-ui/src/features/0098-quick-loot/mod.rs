pub use oteryn_client_runtime::discovery::quick_loot::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Action, apply_result, request};
pub use state::{Discovery, Panel};
pub use view::panel_text;
