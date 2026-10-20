//! PARTY_PVP0 §4.2 membership cache and committed source-read pass functions.
//! Runtime/admission scheduling and CHAT-2 relay composition remain CP slot work.
#![allow(
    dead_code,
    reason = "PARTY-1 source owner awaits the allocated runtime wiring slot"
)]

pub mod view;
#[cfg(test)]
mod view_tests;

/// Read the actual current Channel census in one transaction and one membership batch.
/// This is source DATA for the node's full refresh/revision check, never current authority.
/// A node combines ALL its scope reads before replacing its views. An unknown/failed scope
/// leaves the full-refresh ticket incomplete, and therefore party readiness disabled.
pub async fn read_scope(
    root: &crate::durability::DurabilityRoot,
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    recovery: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
    runtime: &crate::foundation::ChannelRuntimeV1,
) -> Result<
    Vec<crate::foundation::PartySnapshot>,
    crate::durability::spell_item_transaction::SpellItemError,
> {
    root.read_current_world_party_views(permit, recovery, node, runtime)
        .await
}

/// Admission/readmission/reconnect/channel-entry pass. The actual scope/actor/session is
/// independently checked, presence and membership are read in the native transaction, and
/// nothing is returned to the cache before COMMIT. Call begin_admission before awaiting it;
/// on failure the character stays unavailable and the owner retries the source read.
pub async fn read_admission(
    root: &crate::durability::DurabilityRoot,
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    recovery: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> Result<
    crate::foundation::PartySnapshot,
    crate::durability::spell_item_transaction::SpellItemError,
> {
    root.refresh_current_world_party_presence(permit, recovery, node, runtime, actor, session)
        .await
}
