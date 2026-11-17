//! Native and production-loop callbacks share the same borrowed mailbox methods.
use crate::{
    combat_input::PlayCommand,
    play::{
        Outbox,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
use oteryn_client_ui::wheel_allocation::{Intent, diagram};
use tokio::sync::mpsc::UnboundedSender;
#[cfg_attr(not(windows), allow(dead_code, reason = "Windows Wheel controls construct and dispatch these actions"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::play) enum Action {
    Slot { nonce: u64, index: usize },
    Points(u16),
    ResetDraft,
    Apply,
    Refresh,
    Retry,
}
#[cfg_attr(not(windows), allow(dead_code, reason = "Windows Wheel controls construct and dispatch these actions"))]
pub(in crate::play) fn query(m: &mut Mailbox, s: &UnboundedSender<PlayCommand>) {
    if let Some(c) = oteryn_client_ui::wheel_allocation::wheel_presentation_query(&mut m.wheel)
        && s.send(PlayCommand::Feature(c)).is_err()
    {
        m.wheel.diagram.pending = None;
        m.wheel.diagram.feedback = "The session has ended".into();
    }
}
#[cfg_attr(not(windows), allow(dead_code, reason = "Windows Wheel controls construct and dispatch these actions"))]
pub(in crate::play) fn activate(m: &mut Mailbox, s: &UnboundedSender<PlayCommand>, a: Action) {
    if !m.wheel.selected || m.wheel_shutdown {
        return;
    }
    match a {
        Action::Slot { nonce, index } => {
            diagram::slot(&mut m.wheel, nonce, index);
        }
        Action::Points(v) => {
            diagram::points(&mut m.wheel, v);
        }
        Action::ResetDraft => {
            diagram::reset_draft(&mut m.wheel);
        }
        Action::Apply => {
            let Some(id) = super::occurrence() else {
                return;
            };
            if let Some(c) = diagram::submit(&mut m.wheel, id)
                && s.send(PlayCommand::Wheel(c)).is_err()
            {
                m.wheel.pending = None;
                m.wheel.diagram.feedback = "The session ended before confirming the outcome".into();
            }
        }
        Action::Refresh => {
            m.wheel.diagram.last = None;
            m.wheel.diagram.reset_requested = false;
            super::enqueue(m, s, Intent::Refresh);
        }
        Action::Retry => {
            if m.wheel.diagram.pending.is_none() {
                super::enqueue(m, s, Intent::Retry);
            }
        }
    }
}
impl Outbox {
    pub(in crate::play) fn wheel_presentation_initialize(&self, selected: bool) {
        lock(&self.0).wheel.diagram.selected = selected;
    }
    pub(in crate::play) fn wheel_presentation_result(
        &self,
        c: &FeatureCommand,
        r: &GatedCommandResult,
    ) {
        diagram::apply(&mut lock(&self.0).wheel, c, r);
    }
}
