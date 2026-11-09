#[path = "protocol/blessings.rs"]
mod blessings;
pub use blessings::{BlessingsPanel, request_blessings};
#[path = "protocol/decode.rs"]
mod decode;
use decode::{decode_badges, decode_general, decode_history, decode_store, decode_tab};
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
    Protection,
    Items,
    ItemsNext,
    ItemsPrevious,
    Appearances,
    GalleryNext,
    GalleryPrevious,
    Achievements,
    Badges,
    ProfileBadges,
    ProfileInspection,
    SpellAugments(super::AugmentIntent),
    SetBadgeVisibility { badge: u8, public: bool },
    MiscResources,
    Tab,
    SelectTab(cyclopedia_character::CharacterTab),
    Next,
    Previous,
}
pub fn request(p: &mut CyclopediaCharacterPanel, intent: Intent) -> Option<FeatureCommand> {
    if !p.selected {
        p.feedback = Some(tr!("Feature unavailable in this session"));
        return None;
    }
    if p.pending || p.blessings.pending {
        p.feedback = Some(tr!("Waiting for the server"));
        return None;
    }
    if let Intent::SpellAugments(intent) = intent {
        return super::augment_request(p, intent);
    }
    p.augments.open = false;
    p.augments.requested = None;
    p.profile_pending = false;
    if matches!(intent, Intent::Protection) {
        return super::protection::request(p);
    }
    if matches!(
        intent,
        Intent::Items | Intent::ItemsNext | Intent::ItemsPrevious
    ) {
        return super::items::request(p, intent);
    }
    if matches!(
        intent,
        Intent::Appearances | Intent::GalleryNext | Intent::GalleryPrevious
    ) {
        return super::gallery::request(p, intent);
    }
    if matches!(
        intent,
        Intent::Achievements | Intent::SelectTab(cyclopedia_character::CharacterTab::Achievements)
    ) {
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
        Intent::ProfileBadges
        | Intent::ProfileInspection
        | Intent::SpellAugments(_)
        | Intent::Protection
        | Intent::Items
        | Intent::ItemsNext
        | Intent::ItemsPrevious
        | Intent::Appearances
        | Intent::GalleryNext
        | Intent::GalleryPrevious
        | Intent::Achievements => return None,
        Intent::Badges => (
            387,
            cyclopedia_character::badge_visibility::Request::Query(p.target).encode(),
        ),
        Intent::SetBadgeVisibility { badge, public } => {
            if p.target.is_some()
                || !p.badge_visibility_known
                || !p.badges.as_ref().is_some_and(|ids| ids.contains(&badge))
            {
                p.feedback =
                    Some("Only your earned badges with confirmed visibility can be changed");
                return None;
            }
            let badge = cyclopedia_character::Badge::from_id(badge)?;
            (
                387,
                cyclopedia_character::badge_visibility::Request::Set { badge, public }.encode(),
            )
        }
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
                p.feedback = Some(tr!("Enter a valid page and target"));
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
    if p.protection.awaiting {
        return super::protection::apply(p, r);
    }
    if p.items.pending_offset.is_some() {
        return super::items::apply(p, r);
    }
    if p.gallery.pending_offset.is_some() {
        return super::gallery::apply(p, r);
    }
    if p.augments.requested.is_some() {
        return super::augments::apply(p, r);
    }
    if p.blessings.pending {
        return blessings::apply(&mut p.blessings, r);
    }
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
        p.feedback = Some(tr!("The server refused the character query"));
        return false;
    }
    if auxiliary {
        let Some(stats) = decode_general(&r.payload) else {
            p.feedback = Some(tr!("The server returned invalid resource statistics"));
            return false;
        };
        p.misc_general = Some(stats);
        p.feedback = None;
        return true;
    }
    let text = if p.command_kind == 387 {
        if r.payload.first() == Some(&cyclopedia_character::badge_visibility::PREFIX) {
            match cyclopedia_character::badge_visibility::Reply::decode(&r.payload) {
                Ok(reply) => {
                    p.badges = Some(reply.earned.iter().map(|badge| badge.id()).collect());
                    p.hidden_badges = reply.hidden.iter().map(|badge| badge.id()).collect();
                    p.badge_visibility_known = true;
                    Some("Server-confirmed badge visibility".into())
                }
                Err(_) => None,
            }
        } else {
            let decoded = decode_badges(&r.payload);
            if decoded.is_some() {
                p.badges = Some(r.payload[1..].to_vec());
                p.hidden_badges.clear();
                p.badge_visibility_known = false;
            }
            decoded
        }
    } else {
        tab.and_then(|tab| decode_tab(&r.payload, tab, p.limit))
    };
    match text {
        Some(v) => {
            let general = if tab == Some(cyclopedia_character::CharacterTab::GeneralStats) {
                let Some(stats) = decode_general(&r.payload) else {
                    p.feedback = Some(tr!("The server returned invalid general statistics"));
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
                    p.feedback = Some(tr!("The server returned invalid character history"));
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
            p.feedback = Some(tr!("The server returned an invalid character view"));
            false
        }
    }
}
