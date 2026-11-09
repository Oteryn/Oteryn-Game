//! Native controls; parsing and intent selection are shared with Linux tests.
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::weapon_proficiency::{Intent, WeaponProficiencyPanel, request};

pub(in crate::play) fn action_clicked(id: usize, notification: usize) -> bool {
    (matches!(id, 101..=114) || id == 116) && notification == 0 // Win32 BN_CLICKED
}

pub(in crate::play) fn display(p: &WeaponProficiencyPanel) -> String {
    format!(
        "{}\r\n{}\r\n{}",
        if p.pending {
            tr!("Waiting for the server")
        } else if !p.selected {
            tr!("Feature unavailable in this session")
        } else {
            tr!("Ready")
        },
        p.feedback.unwrap_or(""),
        oteryn_client_ui::weapon_proficiency::panel_text(p).replace('\n', "\r\n")
    )
}

#[path = "native_localized_6.rs"]
mod native_localized_6;

pub(in crate::play) use native_localized_6::submit;

#[path = "native_localized_8.rs"]
mod native_localized_8;

pub(in crate::play) use native_localized_8::choose;
#[path = "native_localized_7.rs"]
mod native_localized_7;

pub(in crate::play) use native_localized_7::visual_submit;

#[cfg(windows)]
pub(in crate::play) use window::open_window;
#[path = "native_localized_1.rs"]
#[cfg(windows)]
mod window;
