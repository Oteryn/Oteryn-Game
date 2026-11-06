use super::WeaponProficiencyPanel;
use oteryn_client_runtime::{
    CommandStatus,
    raw_feature::{FeatureCommand, GatedCommandResult},
    weapon_proficiency::{
        ModificationOperation, ModificationReply, ModificationRequest, SelectPerkReply,
        SelectPerkRequest,
    },
};
use std::num::NonZeroU32;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Browse,
    NextPage,
    PreviousPage,
    SelectPerk,
    Modify,
    RankUp,
    OrbRank,
    ReshapeOffer,
    ReshapeChoose,
    DeclineOffer,
    Clear,
}
pub fn request(p: &mut WeaponProficiencyPanel, intent: Intent) -> Option<FeatureCommand> {
    if !p.selected {
        p.feedback = Some("Feature unavailable in this session");
        return None;
    }
    if p.pending {
        p.feedback = Some("Waiting for the server");
        return None;
    }
    if matches!(
        intent,
        Intent::Browse | Intent::NextPage | Intent::PreviousPage
    ) {
        let offset = match intent {
            Intent::NextPage => {
                let page = p.page.as_ref()?;
                let next = page
                    .offset
                    .checked_add(u32::try_from(page.rows.len()).ok()?)?;
                if next >= page.total || page.rows.is_empty() {
                    return None;
                }
                next
            }
            Intent::PreviousPage => p.page.as_ref()?.offset.saturating_sub(20),
            _ => 0,
        };
        p.pending = true;
        p.feedback = None;
        p.command_kind = 641;
        p.requested_offset = offset;
        return Some(FeatureCommand {
            capability: 195,
            kind: 641,
            payload: offset.to_be_bytes().to_vec(),
        });
    }
    let Some(definition) = NonZeroU32::new(p.weapon) else {
        p.feedback = Some("Enter a nonzero weapon id");
        return None;
    };
    let (kind, payload) = if intent == Intent::SelectPerk {
        (
            378,
            SelectPerkRequest {
                definition,
                level: p.level,
                perk: (!p.clear_perk).then_some(p.perk),
            }
            .encode()
            .to_vec(),
        )
    } else {
        let operation = match intent {
            Intent::Modify => ModificationOperation::Modify { level: p.level },
            Intent::RankUp => ModificationOperation::RankUp,
            Intent::OrbRank => ModificationOperation::OrbRank,
            Intent::ReshapeOffer => ModificationOperation::ReshapeOffer,
            Intent::ReshapeChoose => ModificationOperation::ReshapeChoose {
                choice: Some(p.shaping_choice),
            },
            Intent::DeclineOffer => ModificationOperation::ReshapeChoose { choice: None },
            Intent::Clear => ModificationOperation::Clear,
            Intent::SelectPerk | Intent::Browse | Intent::NextPage | Intent::PreviousPage => {
                return None;
            }
        };
        let request = ModificationRequest {
            definition,
            slot: p.slot,
            operation,
            expected_revision: p.revision,
        };
        let payload = request.encode();
        if ModificationRequest::decode(operation.command_type(), &payload).is_none() {
            p.feedback = Some("Enter slot 1-2, a valid level/choice, and nonzero revision");
            return None;
        }
        (operation.command_type(), payload)
    };
    p.pending = true;
    p.command_kind = kind;
    p.feedback = None;
    Some(FeatureCommand {
        capability: 195,
        kind,
        payload,
    })
}
pub fn apply_result(p: &mut WeaponProficiencyPanel, result: &GatedCommandResult) -> bool {
    p.pending = false;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = Some("The server refused the proficiency action");
        return false;
    }
    if p.command_kind == 641 {
        match oteryn_client_runtime::weapon_proficiency::ProficiencyPage::decode(&result.payload) {
            Some(page) if page.offset == p.requested_offset => {
                p.page = Some(page);
                p.feedback = None;
                return true;
            }
            _ => {
                p.feedback = Some("The server returned an invalid proficiency page");
                return false;
            }
        }
    }
    let text = if p.command_kind == 378 {
        SelectPerkReply::decode(&result.payload)
            .map(|v| format!("Selection: {:?}; levels: {:?}", v.result, v.selections))
    } else {
        ModificationReply::decode(&result.payload)
            .map(|v| format!("Modification: {:?}; slot: {:?}", v.result, v.slot))
    };
    match text {
        Some(v) => {
            p.result = Some(v);
            p.feedback = None;
            true
        }
        None => {
            p.feedback = Some("The server returned an invalid proficiency reply");
            false
        }
    }
}
