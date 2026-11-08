//! Native controls; parsing and intent selection are shared with Linux tests.
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::cyclopedia_character::{CyclopediaCharacterPanel, Intent, request};

pub(in crate::play) fn display(p: &CyclopediaCharacterPanel) -> String {
    let badges = oteryn_client_ui::cyclopedia_character::badge_text(p);
    format!(
        "{}\r\n{}\r\n{}",
        if p.pending {
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
    if p.pending || !p.selected {
        return None;
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
    if matches!(id, 145..=148) {
        use oteryn_client_ui::cyclopedia_character::BadgeFilter;
        p.badge_filter = match id {
            146 => BadgeFilter::Earned,
            147 => BadgeFilter::Locked,
            148 => BadgeFilter::Hidden,
            _ => BadgeFilter::All,
        };
        return None;
    }
    if id == 111 {
        return request(p, Intent::Badges);
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
            _ => Intent::Tab,
        },
    )
}

fn parse_target(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32
        && (text.len() != 36 || ![8, 13, 18, 23].iter().all(|i| text.as_bytes()[*i] == b'-'))
    {
        return None;
    }
    let compact: String = text.chars().filter(|c| *c != '-').collect();
    if compact.len() != 32 || !compact.is_ascii() {
        return None;
    }
    let mut target = [0; 16];
    for (i, byte) in target.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&compact[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(target)
}

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
