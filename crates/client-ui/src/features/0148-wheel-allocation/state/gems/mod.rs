//! Visual Gem Atelier consumes old current state and a separately negotiated executed catalogue.
use super::*;
use oteryn_client_runtime::{
    gem_presentation as wire,
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
mod review;
pub use review::{Review, confirm, stage};
pub mod atlas;
pub mod view;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Presentation {
    pub selected: bool,
    pub open: bool,
    pub window_epoch: u64,
    pub last_revision: Option<u64>,
    pub nonce: u64,
    pub pending: Option<wire::Request>,
    pub reply: Option<wire::Reply>,
    pub review: Option<Review>,
    pub feedback: Option<String>,
}
impl Presentation {
    pub fn invalidate(&mut self) {
        self.window_epoch = self.window_epoch.saturating_add(1);
        self.nonce = self.nonce.saturating_add(1);
        self.pending = None;
        self.last_revision = None;
        self.reply = None;
        self.review = None;
        self.open = false;
    }
}
pub fn query(p: &mut WheelPanel) -> Option<FeatureCommand> {
    if !p.selected
        || !p.gems.selected
        || p.pending.is_some()
        || p.gems.pending.is_some()
        || p.diagram.pending.is_some()
        || p.retry.is_some()
    {
        return None;
    }
    let revision = p.confirmed_atelier.as_ref()?.revision;
    p.gems.nonce = p.gems.nonce.checked_add(1)?;
    let request = wire::Request {
        nonce: p.gems.nonce,
        revision,
    };
    p.gems.pending = Some(request);
    p.gems.last_revision = Some(revision);
    Some(FeatureCommand {
        capability: 148,
        kind: 6121,
        payload: request.encode()?,
    })
}
pub fn current(p: &WheelPanel) -> Option<&wire::Reply> {
    p.gems.reply.as_ref().filter(|r| {
        p.confirmed_atelier
            .as_ref()
            .is_some_and(|a| a.revision == r.request.revision)
    })
}
pub fn apply(p: &mut WheelPanel, c: &FeatureCommand, r: &GatedCommandResult) {
    if c.capability != 148 || c.kind != 6121 {
        return;
    }
    let Some(request) = wire::Request::decode(&c.payload) else {
        return;
    };
    if p.gems.pending != Some(request) {
        return;
    }
    p.gems.pending = None;
    let reply = wire::Reply::decode(&r.payload).filter(|reply| {
        reply.request == request
            && p.confirmed_atelier
                .as_ref()
                .is_some_and(|a| a.revision == request.revision)
    });
    if !matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) || reply.is_none()
    {
        p.gems.reply = None;
        p.gems.feedback =
            Some("Refresh current Gem details; the observation was refused or changed".into());
        return;
    }
    p.gems.reply = reply;
    p.gems.feedback = None;
}
#[cfg(test)]
mod tests;
