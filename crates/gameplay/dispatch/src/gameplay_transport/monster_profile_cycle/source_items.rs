//! Native monster field creation/removal through the existing durable Item owner.
use crate::content::{
    ProjectV2DefinitionRef, QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState,
};
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::durability::creature_source_items::{
    CreatureGroundReceipt, ScheduledCreatureSourceItems,
};
use crate::durability::spell_item_transaction::SpellItemError;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::ComposedFreshAdmission;

pub(super) type SourceItemResults = Vec<(
    ProjectV2DefinitionRef,
    Result<CreatureGroundReceipt, SpellItemError>,
)>;

/// Called after the read-only think pass ends, while the Channel runtime is still held.
/// Every cast opens the existing Item transaction with current recovery/assignment fencing;
/// source occurrence receipts make a repeated proposal reconcile without another mutation.
pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    permit: &SpellLanePermit,
    proposals: &[ProfileAbilityProposal],
) -> Result<SourceItemResults, SpellItemError> {
    let sources = proposals
        .iter()
        .filter_map(|proposal| {
            let identity = runtime
                .current_live_creature_identity(proposal.issuer)
                .ok()?;
            let record = native
                .creature_profiles()
                .records
                .iter()
                .find(|record| record.profile.target.key.as_bytes() == identity)?;
            native.source_item_ability(&record.profile.target, &proposal.ability)?;
            Some((&record.profile.target, proposal))
        })
        .collect::<Vec<_>>();
    if sources.is_empty() {
        return Ok(Vec::new());
    }
    let mut outcomes = Vec::new();
    outcomes
        .try_reserve_exact(sources.len())
        .map_err(|_| SpellItemError::Rejected("Creature source Item result capacity"))?;
    let stamp = runtime
        .issue_owner_work()
        .map_err(|_| SpellItemError::Rejected("Creature source Item current work"))?;
    let fence = runtime
        .owner_fence()
        .map_err(|_| SpellItemError::Rejected("Creature source Item current owner"))?;
    let objects = owner.door.lock().await;
    for (creature, proposal) in sources {
        let result = match ScheduledCreatureSourceItems::prepare(
            runtime, fence, stamp, native, creature, proposal,
        ) {
            Ok(scheduled) => {
                owner
                    .root
                    .commit_creature_source_items(
                        permit,
                        owner.character,
                        owner.holder,
                        runtime,
                        fence,
                        stamp,
                        room,
                        &*objects,
                        room.compiled(),
                        native,
                        &scheduled,
                    )
                    .await
            }
            Err(error) => Err(error),
        };
        outcomes.push((proposal.ability.clone(), result));
    }
    Ok(outcomes)
}
