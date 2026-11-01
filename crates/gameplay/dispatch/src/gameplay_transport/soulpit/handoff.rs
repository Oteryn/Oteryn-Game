//! Adapter boundary to the shared boss activity owner, never an alternative scope owner.
//! PR #2768 answers 6542715454 and 6542791914 assign enter/return/recovery to #2769.

use super::super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ExactActorRef, GameSessionId};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, OnceLock},
};

/// Source custody selected from the pinned inventory after a historical receipt probe.
/// The shared owner revalidates it in `write_soulpit_entry_in_transaction` before commit.
#[derive(Debug, Clone)]
pub struct SoulpitCoreSelection {
    pub item_instance_id: [u8; 16],
    pub definition: TypedDefinitionRef,
}

#[derive(Debug, Clone)]
pub struct SoulpitEntryIntent {
    pub actor: ExactActorRef,
    pub command: UseCommand,
    pub encoded_intent: Vec<u8>,
    /// `None` probes only: return the exact recorded result or `None`, without a write.
    pub selected: Option<SoulpitCoreSelection>,
}

/// The #2769 adapter owns allocation, all participant validation, session handoff and return.
/// It must call the Soulpit entry writer inside its existing Type2 transaction before source
/// sessions end. A missing ready scope refuses without consuming the selected core. It also
/// owns logout/death/recovery routing and calls the Soulpit lifecycle writer for those facts.
/// No method here grants a Channel authority to teleport into an Instance.
pub trait BossActivityHandoff: Send + Sync {
    /// World-owned cadence, including when the last participant disconnects. The shared
    /// owner enumerates only its live Instance activities, proves each current ownership
    /// generation and invokes `drive_soulpit_activity` on the global tick, or
    /// `drive_soulpit_deadlines` for precise source spawn/kick/ability deadlines. It returns
    /// the earliest next absolute owner-clock deadline, strictly after `now_micros`.
    /// A returned error means uncertain or lost authority; transient acquisition must be
    /// resolved inside the owner without installing a second physical wave.
    fn drive_soulpit<'a>(
        &'a self,
        admission: &'a ComposedFreshAdmission<'_, '_, '_>,
        now_micros: u64,
        global_tick: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Option<u64>, ()>> + Send + 'a>>;

    fn enter_soulpit<'a>(
        &'a self,
        admission: &'a ComposedFreshAdmission<'_, '_, '_>,
        request: SoulpitEntryIntent,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UseOutcome>, ()>> + Send + 'a>>;

    /// Called only after an actual STEP onto the source exit portal. The adapter proves
    /// membership in the current Soulpit handoff and uses its existing return/recovery owner.
    fn return_from_soulpit<'a>(
        &'a self,
        admission: &'a ComposedFreshAdmission<'_, '_, '_>,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>>;
}

/// Installed once by the shared activity composition root. Absence is no capacity, not a
/// request to create a Channel teleport or to burn a core ahead of an unavailable owner.
#[derive(Default)]
pub struct BossActivityHandoffSlot(OnceLock<Arc<dyn BossActivityHandoff>>);

impl BossActivityHandoffSlot {
    pub fn install(
        &self,
        owner: Arc<dyn BossActivityHandoff>,
    ) -> Result<(), Arc<dyn BossActivityHandoff>> {
        self.0.set(owner)
    }

    pub(super) fn get(&self) -> Option<Arc<dyn BossActivityHandoff>> {
        self.0.get().cloned()
    }
}
