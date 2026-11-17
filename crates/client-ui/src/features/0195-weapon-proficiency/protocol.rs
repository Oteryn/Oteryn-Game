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
    State,
    Browse,
    NextPage,
    PreviousPage,
    Catalogue,
    SelectPerk,
    Modify,
    RankUp,
    OrbRank,
    ReshapeOffer,
    ReshapeChoose,
    DeclineOffer,
    Clear,
    ConfirmClear,
    CancelClear,
}
pub fn request(p: &mut WeaponProficiencyPanel, intent: Intent) -> Option<FeatureCommand> {
    if !p.selected {
        p.feedback = Some(tr!("Feature unavailable in this session"));
        return None;
    }
    if p.pending {
        p.feedback = Some(tr!("Waiting for the server"));
        return None;
    }
    if p.mutation_uncertain
        && matches!(
            intent,
            Intent::SelectPerk
                | Intent::Modify
                | Intent::RankUp
                | Intent::OrbRank
                | Intent::ReshapeOffer
                | Intent::ReshapeChoose
                | Intent::DeclineOffer
                | Intent::Clear
                | Intent::ConfirmClear
        )
    {
        p.feedback = Some(tr!(
            "The previous modification result was not decoded. Reconnect before changing proficiency again"
        ));
        return None;
    }
    let intent = match intent {
        Intent::CancelClear => {
            p.clear_confirmation = None;
            p.feedback = None;
            return None;
        }
        Intent::Clear => {
            let state = p
                .shaping
                .as_ref()
                .filter(|state| state.definition == p.weapon && state.revision != 0)?;
            p.clear_confirmation = Some((p.weapon, p.slot, state.revision));
            p.feedback = Some(tr!(
                "Confirm Clear to remove this slot modification and spend the displayed dust cost"
            ));
            return None;
        }
        Intent::ConfirmClear => {
            let confirmed = p.clear_confirmation.take()?;
            if p.shaping
                .as_ref()
                .is_none_or(|state| confirmed != (p.weapon, p.slot, state.revision))
            {
                p.feedback = Some(tr!("Slot changed. Review it before confirming Clear"));
                return None;
            }
            p.revision = confirmed.2;
            Intent::Clear
        }
        other => {
            p.clear_confirmation = None;
            other
        }
    };
    if intent == Intent::State {
        if !p.state_selected || p.weapon == 0 {
            p.feedback = Some(tr!("Shaping state is unavailable in this session"));
            return None;
        }
        p.shaping = None;
        p.revision = 0;
        p.pending = true;
        p.command_kind = 732;
        p.state_refresh_needed = false;
        p.feedback = None;
        return Some(FeatureCommand {
            capability: 448,
            kind: 732,
            payload: p.weapon.to_be_bytes().to_vec(),
        });
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
    if intent == Intent::Catalogue {
        p.next_poll = Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
        p.catalogue_next = None;
        p.pending = true;
        p.command_kind = 660;
        p.feedback = None;
        return Some(FeatureCommand {
            capability: 195,
            kind: 660,
            payload: p.catalogue_offset.to_be_bytes().to_vec(),
        });
    }
    let Some(definition) = NonZeroU32::new(p.weapon) else {
        p.feedback = Some(tr!("Enter a nonzero weapon id"));
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
            Intent::ConfirmClear
            | Intent::CancelClear
            | Intent::State
            | Intent::SelectPerk
            | Intent::Catalogue
            | Intent::Browse
            | Intent::NextPage
            | Intent::PreviousPage => return None,
        };
        let request = ModificationRequest {
            definition,
            slot: p.slot,
            operation,
            expected_revision: p.revision,
        };
        let payload = request.encode();
        if ModificationRequest::decode(operation.command_type(), &payload).is_none() {
            p.feedback = Some(tr!(
                "Enter slot 1-2, a valid level/choice, and nonzero revision"
            ));
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
    p.clear_confirmation = None;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        if matches!(p.command_kind,378..=380|392..=395) {
            p.invalidate_observation();
        }
        p.feedback = Some(tr!("The server refused the proficiency action"));
        return false;
    }
    if p.command_kind == 732 {
        let Some(state) =
            oteryn_client_runtime::weapon_proficiency_state::State::decode(&result.payload)
                .filter(|state| state.definition == p.weapon)
        else {
            p.feedback = Some(tr!("The server returned invalid shaping state"));
            return false;
        };
        p.revision = state.revision;
        p.shaping = Some(state);
        p.feedback = if p.mutation_uncertain {
            Some(tr!(
                "The previous modification result was not decoded. Reconnect before changing proficiency again"
            ))
        } else {
            None
        };
        return true;
    }
    if p.command_kind == 641 {
        match oteryn_client_runtime::weapon_proficiency::ProficiencyPage::decode(&result.payload) {
            Some(page) if page.offset == p.requested_offset => {
                for row in &page.rows {
                    p.observe_progress(row.definition.get(), row.experience);
                }
                let next = page.offset.saturating_add(page.rows.len() as u32);
                p.progress_next = (!page.rows.is_empty() && next < page.total).then_some(next);
                p.page = Some(page);
                p.feedback = None;
                return true;
            }
            _ => {
                p.feedback = Some(tr!("The server returned an invalid proficiency page"));
                return false;
            }
        }
    }
    if p.command_kind == 660 {
        let Some(page) =
            oteryn_client_runtime::weapon_proficiency::ProficiencyCataloguePage::decode(
                &result.payload,
            )
        else {
            p.feedback = Some(tr!("The server returned an invalid proficiency catalogue"));
            return false;
        };
        if page.offset != p.catalogue_offset {
            return false;
        }
        let next = page.offset.saturating_add(page.entries.len() as u32);
        p.catalogue_next = (!page.entries.is_empty() && next < page.total).then_some(next);
        if page.offset == 0 {
            for entry in &page.entries {
                p.observe_progress(entry.definition, entry.experience);
            }
            p.catalogue = Some(page);
        } else if let Some(catalogue) = &mut p.catalogue {
            if catalogue.entries.len() as u32 != page.offset || catalogue.total != page.total {
                p.catalogue_next = None;
                p.feedback = Some(tr!("Catalogue changed. Refresh the catalogue"));
                return false;
            }
            catalogue.entries.extend(page.entries);
        }
        if p.catalogue_next.is_none() && p.weapon != 0 && p.weapon_entry().is_none() {
            p.weapon = 0;
            p.shaping = None;
            p.revision = 0;
            p.state_refresh_needed = false;
            p.clear_confirmation = None;
        }
        p.feedback = None;
        return true;
    }
    let text = if p.command_kind == 378 {
        SelectPerkReply::decode(&result.payload).map(|v| {
            if let Some(entry) = p.catalogue.as_mut().and_then(|page| {
                page.entries
                    .iter_mut()
                    .find(|entry| entry.definition == p.weapon)
            }) {
                for (level, selection) in entry.levels.iter_mut().zip(&v.selections) {
                    level.selected = *selection;
                }
            }
            lfmt!("Selection: {:?}; levels: {:?}", v.result, v.selections)
        })
    } else {
        ModificationReply::decode(&result.payload)
            .map(|v| lfmt!("Modification: {:?}; slot: {:?}", v.result, v.slot))
    };
    match text {
        Some(v) => {
            p.result = Some(v);
            p.shaping = None;
            p.revision = 0;
            p.state_refresh_needed = p.state_selected;
            p.feedback = None;
            true
        }
        None => {
            if matches!(p.command_kind, 379 | 380 | 392..=395) {
                p.mutation_uncertain = true;
                p.invalidate_observation();
                p.feedback = Some(tr!(
                    "The modification result was not decoded. Reconnect before making another change"
                ));
            } else {
                p.feedback = Some(tr!("The server returned an invalid proficiency reply"));
            }
            false
        }
    }
}

/// Run by the HUD and native panel; uses the same bounded play queue as manual requests.
pub fn tick(p: &mut WeaponProficiencyPanel, now: std::time::Instant) -> Option<FeatureCommand> {
    if !p.selected || p.pending || p.clear_confirmation.is_some() {
        return None;
    }
    if let Some(offset) = p.catalogue_next.take() {
        p.catalogue_offset = offset;
        return request(p, Intent::Catalogue);
    }
    if p.state_refresh_needed && p.state_selected && p.weapon != 0 {
        return request(p, Intent::State);
    }
    if p.progress_next.take().is_some() {
        return request(p, Intent::NextPage);
    }
    if p.next_poll.is_some_and(|deadline| now < deadline) {
        return None;
    }
    p.next_poll = Some(now + std::time::Duration::from_secs(5));
    if p.catalogue.is_none() {
        p.catalogue_offset = 0;
        request(p, Intent::Catalogue)
    } else {
        request(p, Intent::Browse)
    }
}
