//! Graph metadata and exact draft observations remain advisory to the original161 writer.
use super::*;
use oteryn_client_runtime::{
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
    wheel_presentation as wire,
};
pub mod graph;
pub mod view;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Presentation {
    pub selected: bool,
    pub open: bool,
    pub epoch: u64,
    pub nonce: u64,
    pub pending: Option<wire::Request>,
    pub reply: Option<wire::Reply>,
    pub last: Option<wire::Request>,
    pub reset_requested: bool,
    pub feedback: String,
}
impl Presentation {
    pub fn invalidate(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        self.nonce = self.nonce.saturating_add(1);
        self.open = false;
        self.pending = None;
        self.reply = None;
        self.last = None;
        self.reset_requested = false;
    }
}
fn desired(p: &WheelPanel, nonce: u64) -> Option<wire::Request> {
    Some(wire::Request {
        nonce,
        wheel_revision: p.confirmed_wheel.as_ref()?.revision,
        atelier_revision: p.confirmed_atelier.as_ref()?.revision,
        points: p.points,
        vessels: p.vessels,
    })
}
pub fn metadata(p: &WheelPanel) -> Option<&wire::Reply> {
    p.diagram.reply.as_ref().filter(|r| {
        desired(p, r.request.nonce).is_some_and(|q| {
            q.wheel_revision == r.request.wheel_revision
                && q.atelier_revision == r.request.atelier_revision
        })
    })
}
pub fn current(p: &WheelPanel) -> Option<&wire::Reply> {
    metadata(p).filter(|r| desired(p, r.request.nonce).as_ref() == Some(&r.request))
}
pub fn needs_query(p: &WheelPanel) -> bool {
    desired(p, 1).is_some_and(|q| {
        p.diagram.last.as_ref().is_none_or(|last| {
            let mut last = last.clone();
            last.nonce = 1;
            last != q
        })
    })
}
pub fn query(p: &mut WheelPanel) -> Option<FeatureCommand> {
    if !p.ready() || !p.diagram.selected || p.diagram.pending.is_some() {
        return None;
    }
    let nonce = p.diagram.nonce.checked_add(1)?;
    let request = desired(p, nonce)?;
    let payload = request.encode()?;
    p.diagram.nonce = nonce;
    p.diagram.pending = Some(request.clone());
    p.diagram.last = Some(request);
    Some(FeatureCommand {
        capability: 148,
        kind: 6119,
        payload,
    })
}
pub fn apply(p: &mut WheelPanel, c: &FeatureCommand, r: &GatedCommandResult) {
    if c.capability != 148 || c.kind != 6119 {
        return;
    }
    let Some(q) = wire::Request::decode(&c.payload) else {
        return;
    };
    if p.diagram.pending.as_ref() != Some(&q) {
        return;
    }
    p.diagram.pending = None;
    let reply = wire::Reply::decode(&r.payload).filter(|r| {
        r.request == q
            && desired(p, q.nonce).is_some_and(|d| {
                d.wheel_revision == q.wheel_revision && d.atelier_revision == q.atelier_revision
            })
    });
    if !matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) || reply.is_none()
    {
        p.diagram.reply = None;
        p.diagram.feedback =
            "Wheel preview unavailable for the current scope or state. Refresh current Wheel data."
                .into();
        return;
    }
    p.diagram.reply = reply;
    p.diagram.feedback.clear();
}
pub fn slot(p: &mut WheelPanel, nonce: u64, index: usize) -> bool {
    if index >= 36 || metadata(p).is_none_or(|r| r.request.nonce != nonce) {
        return false;
    }
    p.slot = index;
    true
}
pub fn points(p: &mut WheelPanel, value: u16) -> bool {
    if !p.ready()
        || p.diagram.pending.is_some()
        || metadata(p)
            .and_then(|r| r.nodes.get(p.slot))
            .is_none_or(|n| value > n.capacity)
    {
        return false;
    }
    p.points[p.slot] = value;
    p.diagram.reset_requested = false;
    true
}
pub fn reset_draft(p: &mut WheelPanel) -> bool {
    if !p.ready() || p.diagram.pending.is_some() || metadata(p).is_none() {
        return false;
    }
    p.vessels = p.confirmed_vessels().unwrap_or([None; 4]);
    p.points = [0; 36];
    p.diagram.reset_requested = true;
    true
}
pub fn submit(p: &mut WheelPanel, occurrence: [u8; 16]) -> Option<WheelCommand> {
    if current(p).is_none_or(|r| !r.draft_valid) || !p.dirty() || p.diagram.pending.is_some() {
        return None;
    }
    let intent = if p.diagram.reset_requested {
        Intent::Reset
    } else {
        Intent::Save
    };
    super::request(p, intent, occurrence)
}
#[cfg(test)]
mod tests;
