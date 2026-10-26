//! QUICKLOOT-1 production command port. The composed owner overrides this port to keep item
//! custody, capacity and session fencing in its existing transaction owner.
use oteryn_protocol_oteryn::quick_loot::{QuickLootRequest, QuickLootResult};
use super::connection::{AdmittedSession, UseCommand};

macro_rules! authority_methods { () => {
    /// Executes one decoded command 114 under the admitted session generation.
    fn quick_loot(&self, _admitted: AdmittedSession, _command: UseCommand, _request: QuickLootRequest)
        -> impl std::future::Future<Output = Option<QuickLootResult>> { async { None } }
}; }
pub(super) use authority_methods;

macro_rules! composed_methods { () => {
    async fn quick_loot(&self, _admitted: connection::AdmittedSession, _command: connection::UseCommand,
        _request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest)
        -> Option<oteryn_protocol_oteryn::quick_loot::QuickLootResult> { None }
}; }
pub(super) use composed_methods;
