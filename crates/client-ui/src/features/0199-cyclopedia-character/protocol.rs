use super::CyclopediaCharacterPanel;
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{self, TabRequest},
    raw_feature::{FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Badges,
    Tab,
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
    let (kind, payload) = match intent {
        Intent::Badges => (387, Vec::new()),
        Intent::Tab => {
            let mut v = vec![p.tab as u8];
            v.extend_from_slice(&p.offset.to_le_bytes());
            v.push(p.limit);
            if let Some(target) = p.target {
                v.extend_from_slice(&target)
            }
            if cyclopedia_character::decode_tab_request(&v)
                != Ok(TabRequest {
                    tab: p.tab,
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
    p.pending = false;
    if !matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = Some("The server refused the character query");
        return false;
    }
    let text = if p.command_kind == 387 {
        decode_badges(&r.payload)
    } else {
        decode_tab(&r.payload)
    };
    match text {
        Some(v) => {
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
fn decode_badges(b: &[u8]) -> Option<String> {
    let (&count, ids) = b.split_first()?;
    (usize::from(count) == ids.len()
        && ids
            .iter()
            .all(|id| cyclopedia_character::Badge::from_id(*id).is_some()))
    .then(|| {
        format!(
            "Badges: {}",
            ids.iter()
                .filter_map(|id| cyclopedia_character::Badge::from_id(*id))
                .map(|v| v.name())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}
fn decode_tab(b: &[u8]) -> Option<String> {
    let tab = cyclopedia_character::CharacterTab::from_id(*b.first()?)?;
    Some(format!("{tab:?} server page: {} bytes", b.len() - 1))
}
