#[path = "view/map.rs"]
mod map;
mod protocol;
#[path = "state/ownership.rs"]
pub mod sale;
mod state;
mod view;
pub use map::{MapHouse, map_houses};
pub use protocol::{read_acl, request};
pub use state::Panel;
pub use view::{ownership_text, panel_text};

#[path = "view/ownership_date.rs"]
mod ownership_date;

pub use ownership_date::{format as format_ownership_date, parse as parse_ownership_date};
