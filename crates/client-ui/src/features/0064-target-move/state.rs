use super::{ActorPosition, WalkResponse};

/// Re-requests of one click or minimap walk after a blocked step or a server cancel.
pub const MAX_WALK_RETRIES: u8 = 3;
/// An accepted walk with no own step for this long was dropped by the server.
pub const WALK_STALL: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WalkState {
    pub selected: bool,
    pub pending: bool,
    pub response: Option<WalkResponse>,
    pub feedback: Option<&'static str>,
    /// The destination of the current walk, kept until arrival, user input or a final refusal.
    pub goal: Option<ActorPosition>,
    pub retries: u8,
    /// The server reported "There is no way." while the walk was under way.
    pub blocked: bool,
    /// The last own position and when the walk last reached a new one.
    pub last_position: Option<ActorPosition>,
    pub last_progress: Option<std::time::Instant>,
}
