//! Adapter boundary to the shared boss activity owner, never an alternative scope owner.
//! PR #2768 answers 6542715454 and 6542791914 assign enter/return/recovery to #2769.

use super::super::connection::UseCommand;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::ExactActorRef;
use std::sync::{Arc, OnceLock};

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

/// The one concrete shared activity owner, backed by the existing native handoff.
pub use super::handoff_owner::NativeSoulpitHandoff as BossActivityHandoff;

/// The native shared owner is installed lazily for every composed runtime. Its
/// qualified template and independently ready allocation remain the capacity gate.
#[derive(Default)]
pub struct BossActivityHandoffSlot(OnceLock<Arc<BossActivityHandoff>>);

impl Clone for BossActivityHandoffSlot {
    fn clone(&self) -> Self {
        // Activity views retain the exact installed canonical owner Arc.
        Self(OnceLock::from(
            self.0
                .get_or_init(|| Arc::new(super::handoff_owner::NativeSoulpitHandoff))
                .clone(),
        ))
    }
}

impl BossActivityHandoffSlot {
    pub fn install(&self, owner: Arc<BossActivityHandoff>) -> Result<(), Arc<BossActivityHandoff>> {
        self.0.set(owner)
    }

    pub(super) fn get(&self) -> Option<Arc<BossActivityHandoff>> {
        Some(
            self.0
                .get_or_init(|| Arc::new(super::handoff_owner::NativeSoulpitHandoff))
                .clone(),
        )
    }
}
