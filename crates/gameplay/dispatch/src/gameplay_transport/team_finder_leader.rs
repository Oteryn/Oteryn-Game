//! S14a: the leader creates, edits and removes their own Team Finder listing.
//!
//! Crystal 201189f8 keeps the listing in memory (src/game/game.cpp:654-682) and drops it on
//! logout (game.cpp:1422-1426); here it is a fenced character-owned row that survives relog.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::{DurabilityError, spell_item_transaction as item_tx};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::team_finder::{
    CAPABILITY_TEAM_FINDER_LEADER_V1, LeaderListing, TeamTarget,
};

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

/// `None` removes the listing; Crystal's reset (protocolgame.cpp:3072-3074) always succeeds.
pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    listing: Option<LeaderListing>,
) -> Option<bool> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_TEAM_FINDER_LEADER_V1)
    {
        return Some(false);
    }
    let fence = admitted.item_fence?;
    let active = owner.active_generation?;
    let content = active.native_gameplay()?;
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
        let character = authority.character_id_bytes();
        sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(character.as_slice()).fetch_one(&mut *tx).await?;
        let replay: Option<bool> = sqlx::query_scalar("SELECT accepted FROM game_team_finder_leader_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
            .bind(admitted.game_session_id.as_bytes().as_slice()).bind(command_id.to_string()).fetch_optional(&mut *tx).await?;
        if let Some(accepted) = replay {
            return Ok(accepted);
        }
        match listing {
            Some(listing) => {
                let (kind, target, area) = match listing.target {
                    TeamTarget::Boss(id) => (1_i16, id, None),
                    TeamTarget::Hunt { kind, area } => (2, kind, Some(i32::from(area))),
                    TeamTarget::Quest(id) => (3, id, None),
                };
                sqlx::query("INSERT INTO game_team_finder_leader_listings(leader_character_id,min_level,max_level,vocation_mask,team_slots,free_slots,party_members_join,client_timestamp,team_type,target_id,hunt_area) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT(leader_character_id) DO UPDATE SET min_level=EXCLUDED.min_level,max_level=EXCLUDED.max_level,vocation_mask=EXCLUDED.vocation_mask,team_slots=EXCLUDED.team_slots,free_slots=EXCLUDED.free_slots,party_members_join=EXCLUDED.party_members_join,client_timestamp=EXCLUDED.client_timestamp,team_type=EXCLUDED.team_type,target_id=EXCLUDED.target_id,hunt_area=EXCLUDED.hunt_area")
                    .bind(character.as_slice())
                    .bind(i32::from(listing.min_level))
                    .bind(i32::from(listing.max_level))
                    .bind(i16::from(listing.vocation_mask))
                    .bind(i32::from(listing.slots))
                    .bind(i32::from(listing.free_slots))
                    .bind(listing.party_members_join)
                    .bind(i64::from(listing.timestamp))
                    .bind(kind)
                    .bind(i32::from(target))
                    .bind(area)
                    .execute(&mut *tx).await?;
            }
            None => {
                sqlx::query("DELETE FROM game_team_finder_leader_listings WHERE leader_character_id=encode($1,'hex')::uuid")
                    .bind(character.as_slice()).execute(&mut *tx).await?;
            }
        }
        sqlx::query("INSERT INTO game_team_finder_leader_receipts(game_session_id,command_id,leader_character_id,accepted) VALUES(encode($1,'hex')::uuid,$2::text::numeric(20,0),encode($3,'hex')::uuid,true)")
            .bind(admitted.game_session_id.as_bytes().as_slice()).bind(command_id.to_string()).bind(character.as_slice())
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(true)
    })).await.map_or(Some(false), Some)
}
