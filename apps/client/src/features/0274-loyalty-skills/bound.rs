//! Shared presentation admission; layout preferences retain the native client's semantics.
use super::{SkillsLayoutState, SkillsNumbers, SkillsWidget};
use crate::play::{PlayLink, mailbox::{Mailbox, lock}};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsConfigSnapshot {
    session: usize,
    pub open: bool,
    pub layout: SkillsLayoutState,
    pub widget: SkillsWidget,
    loyalty: oteryn_client_ui::loyalty_skills::LoyaltySkillsPanel,
    progress: oteryn_client_ui::skill_progress::SkillProgressPanel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsConfigControl {
    Family(u8), Visible(bool), Up, Down, Numbers(SkillsNumbers), Progress(bool), Refresh, Close,
}
impl PlayLink {
    fn skills_config_locked(&self, mailbox: &Mailbox) -> SkillsConfigSnapshot {
        SkillsConfigSnapshot {
            session: std::sync::Arc::as_ptr(&self.mailbox) as usize,
            open: mailbox.loyalty_skills.open,
            layout: mailbox.skills_layout.clone(),
            widget: mailbox.skills_layout.preferences.project(&mailbox.skill_progress, &mailbox.loyalty_skills),
            loyalty: mailbox.loyalty_skills.clone(),
            progress: mailbox.skill_progress.clone(),
        }
    }
    pub fn skills_config_snapshot(&self) -> SkillsConfigSnapshot {
        self.skills_config_locked(&lock(&self.mailbox))
    }
    pub fn skills_config_open(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.quest_shutdown || mailbox.server_modal.is_active() || self.commands.is_closed() { return false; }
        mailbox.loyalty_skills.open = !mailbox.loyalty_skills.open;
        if mailbox.loyalty_skills.open { self.skills_config_refresh(&mut mailbox); }
        true
    }
    fn skills_config_refresh(&self, mailbox: &mut Mailbox) {
        use crate::combat_input::PlayCommand;
        if let Some(payload) = oteryn_client_ui::loyalty_skills::request(&mut mailbox.loyalty_skills)
            && self.commands.send(PlayCommand::LoyaltySkills(payload)).is_err() {
            mailbox.loyalty_skills.pending = false;
            mailbox.loyalty_skills.error = Some(tr!("Session ended"));
        }
        if let Some(payload) = oteryn_client_ui::skill_progress::request(&mut mailbox.skill_progress)
            && self.commands.send(PlayCommand::Feature(oteryn_client_runtime::raw_feature::FeatureCommand {
                capability: if mailbox.skill_progress.complete_selected {370} else {279},
                kind: if mailbox.skill_progress.complete_selected {640} else {495}, payload,
            })).is_err() {
            mailbox.skill_progress.pending = false;
            mailbox.skill_progress.error = Some(tr!("Session ended"));
        }
    }
    pub fn skills_config_dispatch_bound(&self, expected: &SkillsConfigSnapshot, control: SkillsConfigControl) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.quest_shutdown || mailbox.server_modal.is_active() || self.commands.is_closed()
            || !mailbox.loyalty_skills.open || self.skills_config_locked(&mailbox) != *expected { return false; }
        if control == SkillsConfigControl::Close { mailbox.loyalty_skills.open = false; return true; }
        if control == SkillsConfigControl::Refresh {
            if mailbox.loyalty_skills.pending || mailbox.skill_progress.pending
                || !(mailbox.loyalty_skills.selected || mailbox.skill_progress.selected) { return false; }
            self.skills_config_refresh(&mut mailbox);
            return true;
        }
        let state = &mut mailbox.skills_layout;
        if let SkillsConfigControl::Family(family) = control {
            if family >= 8 || family == state.active { return false; }
            state.active = family;
            return true;
        }
        let mut candidate = state.preferences.clone();
        match control {
            SkillsConfigControl::Visible(show) => {
                if show && !candidate.order.contains(&state.active) { candidate.order.push(state.active); }
                if !show { candidate.order.retain(|id| *id != state.active); }
            }
            SkillsConfigControl::Up | SkillsConfigControl::Down => {
                if let Some(index) = candidate.order.iter().position(|id| *id == state.active) {
                    if control == SkillsConfigControl::Up && index > 0 { candidate.order.swap(index, index - 1); }
                    if control == SkillsConfigControl::Down && index + 1 < candidate.order.len() { candidate.order.swap(index, index + 1); }
                }
            }
            SkillsConfigControl::Numbers(numbers) => candidate.numbers = numbers,
            SkillsConfigControl::Progress(progress) => candidate.progress = progress,
            _ => return false,
        }
        if !candidate.valid() || candidate == state.preferences { return false; }
        state.preferences = candidate;
        state.dirty = true;
        state.edit = crate::play::feature_0017_attack::view::next_edit();
        true
    }
}
