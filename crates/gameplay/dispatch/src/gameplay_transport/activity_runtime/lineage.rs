//! Exact local socket aliases; durable handoff history remains in its owner.
use super::{ActivityConnectionSwitch, AdmittedSession, GameSessionId};
use std::collections::HashMap;

pub(super) struct ActivityConnectionLineage {
    pub(super) source: AdmittedSession,
    pub(super) destination: AdmittedSession,
    /// The listener may retain the initial socket/loss snapshot through many
    /// delivered hops. Intermediate aliases need only pending-barrier lifetime.
    pub(super) root: bool,
}

pub(super) fn same_connection(left: AdmittedSession, right: AdmittedSession) -> bool {
    left.game_session_id == right.game_session_id
        && left.controller == right.controller
        && left.item_fence == right.item_fence
        && left.runtime_actor == right.runtime_actor
        && left.continuity.connection_generation == right.continuity.connection_generation
}

pub(super) fn current_connection(
    successors: &HashMap<GameSessionId, ActivityConnectionLineage>,
    mut source: AdmittedSession,
) -> AdmittedSession {
    for _ in 0..successors.len() {
        let Some(lineage) = successors.get(&source.game_session_id) else {
            break;
        };
        if !same_connection(lineage.source, source) {
            break;
        }
        source = lineage.destination;
    }
    source
}

/// Compression only follows the same captured controller, actor and complete
/// item fence. An older socket can never follow a later reconnect generation.
pub(super) fn advance_lineage(
    successors: &mut HashMap<GameSessionId, ActivityConnectionLineage>,
    source: AdmittedSession,
    destination: AdmittedSession,
) -> bool {
    let mut continued = false;
    for lineage in successors.values_mut() {
        if same_connection(lineage.destination, source) {
            lineage.destination = destination;
            continued = true;
        }
    }
    continued
}

pub(super) fn prune_delivered_lineage(
    successors: &mut HashMap<GameSessionId, ActivityConnectionLineage>,
    pending: &HashMap<GameSessionId, ActivityConnectionSwitch>,
) {
    successors.retain(|_, lineage| {
        lineage.root
            || pending.values().any(|switch| {
                same_connection(switch.source, lineage.source)
                    || same_connection(switch.destination, lineage.source)
            })
    });
}
