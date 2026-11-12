//! W17 mounts this adapter in the existing native form and raw command exchange.
use oteryn_client_runtime::{house::Action, raw_feature::FeatureCommand};
use oteryn_client_ui::house::{Panel, request};
pub const BUTTONS: &[&str] = &[
    "Query",
    "Next",
    "Bid",
    "Guests",
    "Subowners",
    "Door",
    "Kick",
    "Leave",
    "Notice",
    "Cancel notice",
];
pub fn submit(
    panel: &mut Panel,
    selection: usize,
    action: Option<Action>,
) -> Result<FeatureCommand, &'static str> {
    match (selection, action) {
        (0..=1, None) => request(panel, None, selection == 1),
        (2, Some(a @ Action::Bid(_)))
        | (3, Some(a @ Action::Guests(_)))
        | (4, Some(a @ Action::Subowners(_)))
        | (5, Some(a @ Action::Door { .. }))
        | (6, Some(a @ Action::Kick(_)))
        | (8, Some(a @ Action::Notice(_))) => request(panel, Some(&a), false),
        (7, None) => request(panel, Some(&Action::Leave), false),
        (9, None) => request(panel, Some(&Action::Cancel), false),
        _ => Err(oteryn_client_ui::tr!("Invalid house action selection")),
    }
}
