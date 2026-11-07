//! S14a template authoring and S14b/S14c active advertisement lifecycle.
//!
//! Crystal 201189f game.cpp:1422-1426 ends the advertisement on logout. The
//! retained template is durable; PR #2767 Q3B requires a new upsert after relog.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::{DurabilityError, spell_item_transaction as item_tx, team_finder};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::team_finder::{CAPABILITY_TEAM_FINDER_LEADER_V1, LeaderListing};

macro_rules! authority_methods {
    () => {
        fn team_finder_leader(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command: u64,
            _listing: Option<oteryn_protocol_oteryn::team_finder::LeaderListing>,
        ) -> impl std::future::Future<Output = Option<bool>> {
            async { Some(false) }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn team_finder_leader(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            command: u64,
            listing: Option<oteryn_protocol_oteryn::team_finder::LeaderListing>,
        ) -> Option<bool> {
            team_finder_leader::execute(self, admitted, command, listing).await
        }
    };
}
pub(super) use composed_methods;

/// `None` removes the template and active listing; a replay cannot republish it.
pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    listing: Option<LeaderListing>,
) -> Option<bool> {
    if !admitted.continuity.selected_capabilities.contains(CAPABILITY_TEAM_FINDER_LEADER_V1) {
        return Some(false);
    }
    let fence = admitted.item_fence?;
    let content = owner.active_generation?.native_gameplay()?;
    let permit = owner.spell_lane_permit().await?;
    let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
    let pass = owner.root.try_issue_semantic_pass().ok()?;
    let mut context = owner;
    pass.run_with_context(&mut context, move |holder, deadline, owner| Box::pin(async move {
        let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
        let authority = item_tx::assert_spell_item_authority_in_transaction(
            &mut tx, &permit, owner.root, owner.character, owner.holder, &fence,
            command, content.source_digest(),
        ).await.map_err(|_| DurabilityError::Unavailable)?;
        let accepted = team_finder::apply_leader_listing_in_transaction(&mut tx, &authority, listing)
            .await.map_err(|_| DurabilityError::Unavailable)?;
        crate::durability::party_kill_xp::commit_party_kill_transaction(tx, deadline).await?;
        Ok(accepted)
    })).await.ok()
}
