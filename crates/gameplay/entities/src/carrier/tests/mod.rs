use super::spawn_realization::{SpawnCellState, spawn_points_within_ceilings};
use super::*;

// Keep one module so test paths and shared fixture visibility stay unchanged.
include!("admission.rs");
include!("continuity_visibility.rs");
include!("corpse_projection.rs");
include!("respawn.rs");
include!("runtime_assignment.rs");
include!("spawn_activation.rs");
include!("spawn_backlog.rs");
include!("spawn_limits.rs");
include!("spawn_mixed_sources.rs");
include!("spawn_owner_windows.rs");
include!("support.rs");
