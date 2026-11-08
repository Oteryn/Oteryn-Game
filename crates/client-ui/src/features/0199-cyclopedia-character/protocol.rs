#[path = "protocol/tabs.rs"]
mod tabs;
use tabs::{decode_badges, decode_general, decode_history, decode_store, decode_tab};
#[path = "protocol/stamina.rs"]
mod stamina;
use super::CyclopediaCharacterPanel;
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{self, TabRequest},
    raw_feature::{FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Achievements,
    Badges,
    ProfileBadges,
    ProfileInspection,
    MiscResources,
    Tab,
    SelectTab(cyclopedia_character::CharacterTab),
    Next,
    Previous,
}
pub fn request(p: &mut CyclopediaCharacterPanel, intent: Intent) -> Option<FeatureCommand> {
    if !p.selected {
        p.feedback = Some("Feature unavailable in this session");
        return None;
    }
    if p.pending {
        p.feedback = Some("Waiting for the server");
        return None;
    }
    p.profile_pending = false;
    if matches!(intent, Intent::Achievements) {
        p.tab = cyclopedia_character::CharacterTab::Achievements;
        return super::achievements::request(p, crate::account_achievements::Intent::Refresh);
    }
    if matches!(intent, Intent::ProfileBadges | Intent::ProfileInspection) {
        p.badges_open = matches!(intent, Intent::ProfileBadges);
        p.tab = cyclopedia_character::CharacterTab::Inspection;
        p.profile_pending = true;
        p.pending = true;
        p.command_kind = 388;
        p.feedback = None;
        return Some(FeatureCommand {
            capability: 199,
            kind: 388,
            payload: cyclopedia_character::profile::request(p.target),
        });
    }
    p.badges_open = false;
    let auxiliary = matches!(intent, Intent::MiscResources);
    if auxiliary && (p.tab != cyclopedia_character::CharacterTab::Misc || !p.misc_resources_due) {
        return None;
    }
    if !auxiliary {
        p.misc_resources_due = false;
        if matches!(intent, Intent::Tab | Intent::SelectTab(_) | Intent::Badges) {
            p.misc_general = None;
        }
    }
    match intent {
        Intent::SelectTab(tab) => {
            p.tab = tab;
            p.offset = 0;
        }
        Intent::Next => p.offset = p.offset.saturating_add(u16::from(p.limit)),
        Intent::Previous => p.offset = p.offset.saturating_sub(u16::from(p.limit)),
        _ => {}
    }
    let (kind, payload) = match intent {
        Intent::Achievements | Intent::ProfileBadges | Intent::ProfileInspection => return None,
        Intent::Badges => (387, Vec::new()),
        Intent::Tab
        | Intent::SelectTab(_)
        | Intent::Next
        | Intent::Previous
        | Intent::MiscResources => {
            let requested = if auxiliary {
                cyclopedia_character::CharacterTab::GeneralStats
            } else {
                p.tab
            };
            let mut v = vec![requested as u8];
            v.extend_from_slice(&p.offset.to_le_bytes());
            v.push(p.limit);
            if let Some(target) = p.target {
                v.extend_from_slice(&target)
            }
            if cyclopedia_character::decode_tab_request(&v)
                != Ok(TabRequest {
                    tab: requested,
                    page: oteryn_client_runtime::character_info::PageRequest::new(
                        p.offset, p.limit,
                    )?,
                    target: p.target,
                })
            {
                p.feedback = Some("Enter a valid page and target");
                return None;
            }
            (388, v)
        }
    };
    p.auxiliary_pending = auxiliary;
    if auxiliary {
        p.misc_resources_due = false;
    }
    p.requested_tab = (kind == 388).then_some(if auxiliary {
        cyclopedia_character::CharacterTab::GeneralStats
    } else {
        p.tab
    });
    p.pending = true;
    p.command_kind = kind;
    p.feedback = None;
    Some(FeatureCommand {
        capability: 199,
        kind,
        payload,
    })
}
pub fn apply_result(p: &mut CyclopediaCharacterPanel, r: &GatedCommandResult) -> bool {
    if p.achievement_pending {
        return super::achievements::apply(p, r);
    }
    if p.profile_pending {
        p.profile_pending = false;
        p.pending = false;
        p.profile = None;
        p.badges = None;
        p.result = None;
        if !matches!(
            r.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) {
            p.feedback = Some("The server refused the profile query");
            return false;
        }
        let Ok(profile) = cyclopedia_character::profile::Profile::decode(&r.payload) else {
            p.feedback = Some("The server returned an invalid profile");
            return false;
        };
        if p.target.is_some_and(|target| target != profile.character) {
            p.feedback = Some("The server returned a different profile");
            return false;
        }
        p.result = Some(format!("{} — confirmed profile", profile.name));
        p.badges = Some(profile.badges.iter().map(|badge| badge.id()).collect());
        p.profile = Some(profile);
        p.feedback = None;
        return true;
    }
    let pending = p.pending;
    p.pending = false;
    let tab = p.requested_tab.take();
    let auxiliary = std::mem::take(&mut p.auxiliary_pending);
    if !pending {
        return false;
    }
    if !matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = Some("The server refused the character query");
        return false;
    }
    if auxiliary {
        let Some(stats) = decode_general(&r.payload) else {
            p.feedback = Some("The server returned invalid resource statistics");
            return false;
        };
        p.misc_general = Some(stats);
        p.feedback = None;
        return true;
    }
    let text = if p.command_kind == 387 {
        let decoded = decode_badges(&r.payload);
        if decoded.is_some() {
            p.badges = Some(r.payload[1..].to_vec());
        }
        decoded
    } else {
        tab.and_then(|tab| decode_tab(&r.payload, tab, p.limit))
    };
    match text {
        Some(v) => {
            let general = if tab == Some(cyclopedia_character::CharacterTab::GeneralStats) {
                let Some(stats) = decode_general(&r.payload) else {
                    p.feedback = Some("The server returned invalid general statistics");
                    return false;
                };
                Some(stats)
            } else {
                None
            };
            p.combat = if matches!(
                tab,
                Some(
                    cyclopedia_character::CharacterTab::Offence
                        | cyclopedia_character::CharacterTab::Defence
                        | cyclopedia_character::CharacterTab::Misc
                )
            ) {
                cyclopedia_character::combat::CombatStats::decode(&r.payload).ok()
            } else {
                None
            };
            p.store = if tab == Some(cyclopedia_character::CharacterTab::StoreSummary) {
                decode_store(&r.payload)
            } else {
                None
            };
            p.general = general;
            if tab == Some(cyclopedia_character::CharacterTab::Misc) {
                p.misc_resources_due = true;
            }
            if let Some(tab) = tab.filter(|t| {
                matches!(
                    t,
                    cyclopedia_character::CharacterTab::RecentDeaths
                        | cyclopedia_character::CharacterTab::RecentPvpKills
                )
            }) {
                let Some(rows) = decode_history(&r.payload, tab, p.limit) else {
                    p.feedback = Some("The server returned invalid character history");
                    return false;
                };
                p.history = rows;
                p.history_tab = Some(tab);
            } else {
                p.history.clear();
                p.history_tab = None;
            }
            p.result = Some(v);
            p.feedback = None;
            true
        }
        None => {
            p.feedback = Some("The server returned an invalid character view");
            false
        }
    }
}
