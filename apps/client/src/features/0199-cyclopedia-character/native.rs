//! Native controls; parsing and intent selection are shared with Linux tests.
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::cyclopedia_character::{CyclopediaCharacterPanel, Intent, request};

pub(in crate::play) fn display(p: &CyclopediaCharacterPanel) -> String {
    let badges = oteryn_client_ui::cyclopedia_character::badge_text(p);
    format!(
        "{}\r\n{}\r\n{}",
        if p.pending || p.blessings.pending {
            "Waiting for the server"
        } else if !p.selected {
            "Feature unavailable in this session"
        } else {
            "Ready"
        },
        p.feedback.unwrap_or(""),
        if p.command_kind == 387 && p.badges.is_some() {
            badges.as_str()
        } else {
            p.result.as_deref().unwrap_or("")
        }
    )
}

pub(in crate::play) fn submit(
    p: &mut CyclopediaCharacterPanel,
    id: usize,
    values: [&str; 3],
) -> Option<FeatureCommand> {
    if p.pending || p.blessings.pending || !p.selected {
        return None;
    }
    if matches!(id, 180..=185) || (p.augments.open && matches!(id, 112 | 113)) {
        return augment_input::submit(p, id, values[2]);
    }
    if matches!(id, 114..=116) {
        use oteryn_client_runtime::cyclopedia_character::KillStatus;
        p.kill_filter = match id {
            115 => Some(KillStatus::Unjustified),
            116 => Some(KillStatus::GuildWar),
            _ => None,
        };
        return None;
    }
    if target::badge_filter(p, id) {
        return None;
    }
    if let Some(result) = target::submit_profile(p, id, values) {
        return result;
    }
    let tab = if matches!(id, 112 | 113) {
        Some(p.tab)
    } else {
        id.checked_sub(100)
            .and_then(|v| u8::try_from(v).ok())
            .and_then(oteryn_client_runtime::cyclopedia_character::CharacterTab::from_id)
    }?;
    let parsed = (|| {
        let offset = values[0].trim().parse::<u16>().ok()?;
        let limit = values[1].trim().parse::<u8>().ok()?;
        oteryn_client_runtime::character_info::PageRequest::new(offset, limit)?;
        let target = if values[2].trim().is_empty() {
            None
        } else {
            Some(parse_target(values[2].trim())?)
        };
        Some((offset, limit, target))
    })();
    let Some((offset, limit, target)) = parsed else {
        p.feedback = Some("Enter a valid offset, page size 1-50 and target UUID");
        return None;
    };
    p.tab = tab;
    p.offset = offset;
    p.limit = limit;
    p.change_target(target);
    request(
        p,
        match id {
            112 => Intent::Previous,
            113 => Intent::Next,
            104 => Intent::Achievements,
            _ => Intent::Tab,
        },
    )
}

#[path = "view/augment_input.rs"]
mod augment_input;
#[path = "view/target.rs"]
mod target;
use target::parse_target;

#[cfg(windows)]
pub(in crate::play) use window::open_window;
#[cfg(windows)]
#[path = "view/character_window.rs"]
mod window;

#[cfg(windows)]
#[path = "view/controls.rs"]
mod controls;

#[cfg(windows)]
#[path = "view/misc_layout.rs"]
mod misc_layout;

#[cfg(windows)]
#[path = "view/gallery_window.rs"]
mod gallery_window;

#[cfg(windows)]
#[path = "view/browsers.rs"]
mod browsers;
#[cfg(windows)]
#[path = "view/items_window.rs"]
mod items_window;

#[cfg(windows)]
#[path = "view/protection_window.rs"]
mod protection_window;
#[cfg(windows)]
#[path = "view/profile_table.rs"]
mod profile_table;

#[cfg(windows)]
#[path = "view/character_paint.rs"]
mod character_paint;
#[cfg(windows)]
#[path = "view/offence_table.rs"]
mod offence_table;
