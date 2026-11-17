//! In-game transport for the existing character-info command, operation 5.
use super::*;
pub(crate) use oteryn_client_runtime::premium_overview::{Overview, State};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
#[derive(Debug, Default)]
pub(crate) struct Panel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub view: View,
    pub started: Option<Instant>,
}
impl Panel {
    pub fn request(&mut self) -> Option<FeatureCommand> {
        if !self.selected || self.pending {
            return None;
        }
        self.pending = true;
        self.started = Some(Instant::now());
        Some(FeatureCommand {
            capability: 97,
            kind: 113,
            payload: vec![5],
        })
    }
    pub fn apply(&mut self, command: &FeatureCommand, result: &GatedCommandResult) {
        if !self.pending
            || command.capability != 97
            || command.kind != 113
            || command.payload != [5]
        {
            return;
        }
        self.pending = false;
        if !matches!(
            result.status,
            oteryn_client_runtime::raw_feature::CommandStatus::Accepted
                | oteryn_client_runtime::raw_feature::CommandStatus::DuplicateReplay
        ) {
            self.fail("Premium overview is unavailable in this session.");
            return;
        }
        let Some(next) = Overview::decode(&result.payload) else {
            self.fail("Invalid Premium observation refused.");
            return;
        };
        if next.state == State::Unavailable {
            self.fail("Current Premium authority is unavailable. Last known data is unconfirmed.");
            if self.view.game_status.is_none() {
                self.view.game_status = Some(next);
            }
            return;
        }
        if self.view.game_status.as_ref().is_some_and(|old| {
            next.authority_revision < old.authority_revision
                || next.observed_at_us < old.observed_at_us
                || (next.authority_revision == old.authority_revision
                    && (next.expires_at_us != old.expires_at_us || next.state != old.state))
        }) {
            self.fail("Inconsistent Premium observation refused.");
            return;
        }
        let at = self.started.unwrap_or_else(Instant::now);
        self.view.requested_at = Some(at);
        self.view.fresh_until = Some(at + Duration::from_micros(next.valid_for_us));
        self.view.game_status = Some(next);
        self.view.feedback = "Premium observation received from your game session.".into();
    }
    pub fn fail(&mut self, message: &str) {
        self.pending = false;
        self.view.fresh_until = None;
        self.view.feedback = message.into();
    }
}
pub(super) fn text(view: &View, status: &Overview) -> String {
    let mut text = format!(
        "{}\nLast accepted Premium state: {:?}.\nAuthority revision: {}",
        view.feedback, status.state, status.authority_revision
    );
    let elapsed = view.requested_at.map_or(0, |at| {
        u64::try_from(at.elapsed().as_micros()).unwrap_or(u64::MAX)
    });
    let now = status
        .observed_at_us
        .saturating_add(i64::try_from(elapsed).unwrap_or(i64::MAX));
    let current = view.fresh()
        && status.state == State::Active
        && status.expires_at_us.is_some_and(|t| t > now);
    if let Some(until) = status.expires_at_us {
        let seconds = until.saturating_sub(now).max(0) / 1_000_000;
        text.push_str(&format!(
            "\nSubscription expiry: {} UTC Unix seconds. Remaining: {} days, {} hours, {} minutes.",
            until / 1_000_000,
            seconds / 86400,
            seconds / 3600 % 24,
            seconds / 60 % 60
        ));
    } else {
        text.push_str("\nNo subscription expiry is confirmed.");
    }
    if !current {
        text.push_str("\nCurrent access is not confirmed; refresh for a new observation.");
    }
    text.push_str("\n\nOteryn Premium features:");
    for f in &status.features {
        text.push_str(&format!(
            "\n{} [{}]\n{}",
            f.title,
            if current && f.enabled {
                "available"
            } else {
                "not confirmed available"
            },
            f.description
        ));
    }
    text.push_str(
        "\n\nGameplay still checks all current eligibility rules. This panel grants no access.",
    );
    text
}
