//! One current Quest DATA snapshot in the existing actual player-command tx.
use super::*;
use crate::durability::quest_state::soul_war::CurrentSoulWarTaintFacts;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};

/// The scoped read view remains private and cannot escape this data operation.
/// This is the same current-transaction scope projection as locked_tile_rows;
/// it neither reissues a namespace grant nor obtains a second write authority.
pub async fn read_current_soul_war_caster_taints_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    caster: ExactActorRef,
) -> Result<CurrentSoulWarTaintFacts> {
    check_transaction(tx, authority).await?;
    if authority.runtime_scope != runtime.binding().runtime_scope()
        || authority.ownership_generation != runtime.binding().scope_generation().get()
        || authority.compatible_content_digest != runtime.content_pin().server_artifact_digest()
        || runtime.owner_fence().is_err()
    {
        return Err(DurabilityError::Unavailable.into());
    }
    let scope = SpellItemScopeAuthority {
        physical_transaction: authority.physical_transaction.clone(),
        runtime_scope: authority.runtime_scope,
        world: authority.world,
        channel: authority.channel,
        ownership_generation: authority.ownership_generation,
    };
    let facts =
        crate::durability::quest_state::soul_war::read_current_soul_war_taints_in_transaction(
            tx,
            &authority.root,
            &scope,
            runtime,
            &[(caster, authority.command.game_session_id())],
        )
        .await?;
    let [fact] = facts.as_slice() else {
        return Err(DurabilityError::InvalidStoredState.into());
    };
    if fact.character != authority.character
        || fact.actor != caster
        || fact.session != authority.command.game_session_id()
    {
        return Err(DurabilityError::Unavailable.into());
    }
    check_transaction(tx, authority).await?;
    Ok(*fact)
}
