//! Session outcomes.
use super::*;

/// One delta a command's outcome carried, already validated and applied to the session's state:
/// its own `server_sequence` and the domain revisions it moved between.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedDelta<T> {
    pub server_sequence: u64,
    pub base_revision: u64,
    pub new_revision: u64,
    pub value: T,
}

/// One command's decoded outcome: the `CommandResult` (its `command_id`, `status`, the typed
/// `disposition` and the `server_sequence` it arrived at) plus, for `Moved` (domain 1) and `Cast`
/// (domain 3), the first delta of that domain applied after the result, by revision alone: the
/// frames carry no command id, so whatever caused it, it is the one reported. A `Committed` use
/// names no domain and returns at its result, so `world_object_overlay_delta` is always `None`;
/// its effects arrive through [`Session::take_events`]. Every other disposition carries no delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome<D> {
    pub command_id: u64,
    pub status: CommandStatus,
    pub disposition: D,
    pub result_server_sequence: u64,
    pub world_spatial_delta: Option<AppliedDelta<WorldSpatialObservation>>,
    pub world_object_overlay_delta: Option<AppliedDelta<WorldObjectOverlayEntry>>,
    /// The gate message id (1 to 127) of a `SEALED` use result under capability 11, so a client
    /// can show the gate's own reason; `None` for every other disposition and command.
    pub gate_message_id: Option<u8>,
}

/// Outcome of `Session::step` (FIRST-CONTROL-WIRE-V1, command type 1).
pub type StepOutcome = CommandOutcome<StepDisposition>;
/// Outcome of `Session::use_object` (USE-WIRE-V1, command type 2).
pub type UseOutcome = CommandOutcome<UseDisposition>;

/// Outcome of `Session::cast_spell` (SPELL wire contract, command type 3). A `Cast` is followed by
/// exactly one `ACTOR_VITALS` delta (the cast pays its cost); every other disposition, including
/// `Rejected` while the server gate is closed, changes nothing and carries no delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastOutcome {
    pub command_id: u64,
    pub status: CommandStatus,
    pub disposition: SpellCastDisposition,
    pub result_server_sequence: u64,
    pub actor_vitals_delta: Option<AppliedDelta<ActorVitals>>,
}

/// The decoded `CommandResult` of a capability-21 `LOGOUT_INTENT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogoutOutcome {
    pub command_id: u64,
    pub status: CommandStatus,
    /// `None` is a `REJECTED` with an empty payload: the server refused the command before its
    /// logout authority (a capability or rate-limit refusal).
    pub result: Option<LogoutResult>,
    pub result_server_sequence: u64,
}

/// Outcome of `Session::attack_target` and `Session::set_fight_modes` (ATTACK-0, command types
/// 11 and 12). The resulting `ACTOR_COMBAT_STATE` delta, if any, goes through the event queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackOutcome {
    pub command_id: u64,
    pub status: CommandStatus,
    pub disposition: AttackIntentDisposition,
    pub result_server_sequence: u64,
}

/// Most applied-but-undrained pushed deltas [`Session::take_events`] holds. The next one fails the
/// session closed with [`SessionError::EventQueueOverflow`].
pub const MAX_QUEUED_EVENTS: usize = 256;

/// Most chat lines [`ChatLog`] keeps: the oldest is dropped past it (the `CHAT0-RL-11` egress
/// queue bound sets the size).
pub const MAX_CHAT_LOG_LINES: usize = 64;

/// The chat state of a session that selected capability 7: the open rooms and a ring of the last
/// [`MAX_CHAT_LOG_LINES`] lines, oldest first. `Dropped` markers are lines like any other.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatLog {
    pub(super) rooms: ChatRoomSet,
    pub(super) guild: GuildRoomState,
    pub(super) private_channels: Option<ChatChannelList>,
    pub(super) lines: VecDeque<ChatLine>,
}

impl ChatLog {
    #[must_use]
    pub const fn rooms(&self) -> ChatRoomSet {
        self.rooms
    }

    /// The kept lines, oldest first.
    pub fn lines(&self) -> impl ExactSizeIterator<Item = &ChatLine> {
        self.lines.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub(super) fn push(&mut self, line: ChatLine) {
        if self.lines.len() >= MAX_CHAT_LOG_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }
}

/// Outcome of `Session::chat` (CHAT-1b, command type 13). The command returns at its result: the
/// line it produced (the sender's own echo included) and a room change arrive as pushed deltas,
/// through [`Session::take_events`] and [`Session::chat_log`]. A `Muted` or `Exhausted` result
/// carries `wait_seconds` and changes no local state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatOutcome {
    pub command_id: u64,
    pub status: CommandStatus,
    pub disposition: ChatDisposition,
    pub wait_seconds: u32,
    pub result_server_sequence: u64,
}

/// Outcome of `Session::move_item` (ITEM-MOVE-WIRE-1, command type 9). The command returns at its
/// result: a `Moved` result's domain 9 and domain 11 deltas arrive as pushed deltas, through
/// [`Session::take_events`], [`Session::inventory`] and [`Session::open_container`]. Every other
/// outcome (`Stale` included) changes no local state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMoveOutcomeResult {
    pub command_id: u64,
    pub status: CommandStatus,
    pub outcome: ItemMoveOutcome,
    pub result_server_sequence: u64,
}
