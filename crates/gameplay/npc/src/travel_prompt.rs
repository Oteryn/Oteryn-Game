//! A selected travel route belongs to one conversation and one confirmation window.
#[derive(Debug, Clone)]
pub struct TravelPrompt<Route, Context> {
    route: Route,
    context: Context,
    expires_ms: u64,
    remaining: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TravelConfirmation<Route> {
    Refused,
    RepeatConfirmation,
    Ready(Route),
}

impl<Route: Copy, Context: Eq> TravelPrompt<Route, Context> {
    /// `now_ms` is the owner's monotonic time; a second confirmation does not renew it.
    pub fn new(route: Route, context: Context, now_ms: u64, repeat: bool) -> Self {
        Self {
            route,
            context,
            expires_ms: now_ms.saturating_add(60_000),
            remaining: if repeat { 2 } else { 1 },
        }
    }

    /// A stale binding or a consumed/expired prompt cannot authorize a departure.
    pub fn confirm(&mut self, context: &Context, now_ms: u64) -> TravelConfirmation<Route> {
        if !self.is_current(context, now_ms) {
            self.remaining = 0;
            return TravelConfirmation::Refused;
        }
        self.remaining -= 1;
        if self.remaining == 0 {
            TravelConfirmation::Ready(self.route)
        } else {
            TravelConfirmation::RepeatConfirmation
        }
    }

    /// Checking an ordinary talk message never consumes a confirmation or extends the window.
    pub fn is_current(&self, context: &Context, now_ms: u64) -> bool {
        self.remaining > 0 && &self.context == context && now_ms < self.expires_ms
    }
}

#[cfg(test)]
#[path = "travel_prompt_tests.rs"]
mod tests;
