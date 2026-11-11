//! Borrowed native and production session callbacks for the existing Atelier writer.
use crate::{
    combat_input::PlayCommand,
    play::{
        Outbox,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
use tokio::sync::mpsc::UnboundedSender;
#[cfg_attr(not(windows), allow(dead_code, reason = "Windows Gem controls dispatch these borrowed callbacks"))]
pub(in crate::play) fn query(m: &mut Mailbox, s: &UnboundedSender<PlayCommand>) {
    if let Some(c) = oteryn_client_ui::wheel_allocation::gem_presentation_query(&mut m.wheel)
        && s.send(PlayCommand::Feature(c)).is_err()
    {
        m.wheel.gems.pending = None;
        m.wheel.gems.feedback = Some("The session has ended".into());
    }
}
#[cfg_attr(not(windows), allow(dead_code, reason = "Windows Gem controls dispatch these borrowed callbacks"))]
pub(in crate::play) fn confirm(m: &mut Mailbox, s: &UnboundedSender<PlayCommand>) {
    if let Some(c) = oteryn_client_ui::wheel_allocation::gems::confirm(&mut m.wheel)
        && s.send(PlayCommand::Wheel(c)).is_err()
    {
        m.wheel.pending = None;
        m.wheel.gems.feedback=Some("The session ended before confirming the outcome; check current state before retrying the same occurrence".into());
    }
}
#[cfg_attr(not(windows), allow(dead_code, reason = "Windows Gem controls dispatch these borrowed callbacks"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::play) enum Action {
    Gem { revision: u64, id: [u8; 16] },
    Modifier { nonce: u64, kind: u8, id: u16 },
    Quality(u8),
    Vessel(u8),
    Stage(oteryn_client_ui::wheel_allocation::Intent),
    Confirm,
    Retry,
    Cancel,
    Socket,
    Clear,
    Save,
    Lock,
    Refresh,
}
#[cfg_attr(not(windows), allow(dead_code, reason = "Windows Gem controls dispatch these borrowed callbacks"))]
pub(in crate::play) fn activate(m: &mut Mailbox, s: &UnboundedSender<PlayCommand>, a: Action) {
    use oteryn_client_ui::wheel_allocation::{Intent, gems};
    if !m.wheel.selected || m.wheel_shutdown {
        return;
    }
    if a == Action::Retry {
        if m.wheel.pending.is_none() && m.wheel.gems.pending.is_none() && m.wheel.retry.is_some() {
            super::enqueue(m, s, Intent::Retry);
        }
        return;
    }
    if a == Action::Cancel {
        m.wheel.gems.review = None;
        return;
    }
    if a == Action::Confirm {
        confirm(m, s);
        return;
    }
    if m.wheel.gems.review.is_some()
        || m.wheel.pending.is_some()
        || m.wheel.gems.pending.is_some()
        || m.wheel.retry.is_some()
    {
        return;
    }
    match a {
        Action::Gem { revision, id } => {
            if m.wheel
                .confirmed_atelier
                .as_ref()
                .is_some_and(|v| v.revision == revision && v.gems.iter().any(|g| g.id == id))
            {
                m.wheel.select_gem(Some(id));
            }
        }
        Action::Modifier { nonce, kind, id } => {
            if gems::current(&m.wheel).is_some_and(|r| {
                r.request.nonce == nonce && r.modifiers.iter().any(|v| v.kind == kind && v.id == id)
            }) {
                m.wheel.modifier = Some((kind, id));
            }
        }
        Action::Quality(q) if q < 3 => m.wheel.quality = q,
        Action::Vessel(d) if d < 4 => m.wheel.vessel_slot = d,
        Action::Stage(i) => super::enqueue(m, s, i),
        Action::Socket => {
            m.wheel.socket_selected();
        }
        Action::Clear => {
            m.wheel.clear_vessel();
        }
        Action::Save => super::enqueue(m, s, Intent::Save),
        Action::Lock => super::enqueue(m, s, Intent::ToggleLock),
        Action::Refresh => {
            m.wheel.gems.last_revision = None;
            super::enqueue(m, s, Intent::Refresh);
        }
        _ => {}
    }
}
impl crate::play::PlayLink {
    pub fn gem_confirm(&self) {
        confirm(&mut lock(&self.mailbox), &self.commands);
    }
}
impl Outbox {
    pub(in crate::play) fn gem_presentation_initialize(&self, selected: bool) {
        lock(&self.0).wheel.gems.selected = selected;
    }
    pub(in crate::play) fn gem_presentation_result(
        &self,
        c: &FeatureCommand,
        r: &GatedCommandResult,
    ) {
        oteryn_client_ui::wheel_allocation::gems::apply(&mut lock(&self.0).wheel, c, r);
    }
}

impl Mailbox {
    pub(in crate::play) fn end_wheel_visuals(&mut self) {
        self.wheel.gems.invalidate();
        self.wheel.diagram.invalidate();
    }
}
