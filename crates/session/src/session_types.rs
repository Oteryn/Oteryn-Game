use super::*;

/// Everything [`Session::admit`] needs to admit with one grant and read its join snapshot.
#[derive(Debug, Clone, Copy)]
pub struct Admission<'a> {
    /// Verified client artifact Item references for appearance transforms, when loaded.
    pub appearance_items: Option<AppearanceItemDictionary<'a>>,
    /// FND-02 `ClientBootstrap.schema_revision`. Any nonzero value the server accepts.
    pub schema_revision: u32,
    pub character_id: CharacterId,
    /// The signed admission grant, as the wire's `admission_material` bytes.
    pub admission_material: &'a [u8],
    pub client_build_id: &'a str,
    /// FND-02 `supported_capability_id`s this client sends in `ClientBootstrap` (CAP-NEG-1). The
    /// server selects the ones it offers; the session keeps only that selection.
    /// [`CLIENT_SUPPORTED_CAPABILITIES`] is the client's default.
    pub supported_capabilities: &'a [u32],
    /// Bounds each individual frame read of the join sequence (`ServerAccepted`, `SnapshotBegin`,
    /// `SnapshotChunk`, `SnapshotCommit`) and every later per-frame read and write. A stalled
    /// server fails with `SessionError::Timeout` naming the stage, rather than hanging.
    pub deadline: Duration,
}

/// `oteryn-protocol-oteryn`'s own domain codecs (state domains 1 `WORLD_SPATIAL` and 2
/// `WORLD_OBJECT_OVERLAY`).
#[derive(Debug, Clone)]
pub struct JoinSnapshot {
    pub game_session_id: GameSessionId,
    pub world_spatial: WorldSpatialObservation,
    /// The `WORLD_SPATIAL_VISIBILITY` domain revision the snapshot carried: the base every later
    /// domain-1 `StateDelta` must name.
    pub world_spatial_revision: u64,
    pub world_object_overlay: Vec<WorldObjectOverlayEntry>,
    /// The `WORLD_OBJECT_OVERLAY` domain revision the snapshot carried: the base every later
    /// domain-2 `StateDelta` must name.
    pub world_object_overlay_revision: u64,
}

/// A duplicate-status `CommandResult` (FND-02 §13.2) for an earlier `CommandId` of this session:
/// the server did not execute that command again. `payload` is the replayed typed result for
/// `DuplicateReplay` (undecoded; it belongs to the original command's type) and empty for
/// `DuplicateOutcomeExpired`. It promises no delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateOutcome {
    pub command_id: u64,
    pub status: CommandStatus,
    pub server_sequence: u64,
    pub payload: Vec<u8>,
}

/// Most duplicate results retained between `take_duplicate_outcomes` calls; more fails closed.
pub(super) const MAX_RETAINED_DUPLICATES: usize = 64;

/// Client-side dead-link detection only, never server authority to replace a session.
/// FND04B-LIVENESS-IDLE-PROBE-MS (5000) * (IDLE-MISSED (3) + first probe).
pub(super) const SERVER_SILENCE_DEADLINE: Duration = Duration::from_secs(20);
