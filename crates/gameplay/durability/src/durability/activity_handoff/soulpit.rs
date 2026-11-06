//! Soulpit entry consequence in the shared Instance handoff transaction.
use super::*;
use crate::durability::soulpit::{SoulpitEntryRequest, SoulpitWriteError};
use crate::durability::spell_owner_commit::SpellLanePermit;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;

pub(super) struct Entry<'a, 'root, 'recovery> {
    pub authority: &'a ReconciledCharacterAuthority<'root, 'recovery>,
    pub permit: &'a SpellLanePermit,
    pub request: &'a SoulpitEntryRequest,
    pub native: &'a NativeGameplayState,
}

pub(super) async fn write(
    root: &DurabilityRoot,
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    request: &ActivityEnterRequest,
    consequence: &Entry<'_, '_, '_>,
) -> Result<std::result::Result<(), ActivityRefusal>> {
    let Some(initiator) = request
        .participants
        .iter()
        .find(|p| p.source.character_id.as_bytes() == request.initiating_character_id.as_bytes())
    else {
        return Ok(Err(ActivityRefusal::InvalidInput));
    };
    match root
        .write_soulpit_entry_in_transaction(
            tx,
            consequence.permit,
            consequence.authority,
            node,
            initiator.source,
            consequence.request,
            consequence.native,
        )
        .await
    {
        Ok(_) => Ok(Ok(())),
        Err(SoulpitWriteError::Unavailable(error)) => Err(error),
        Err(SoulpitWriteError::ConflictingOccurrence) => Ok(Err(ActivityRefusal::ReplayConflict)),
        Err(SoulpitWriteError::AuthorityRejected) => Ok(Err(ActivityRefusal::AuthorityRejected)),
        Err(SoulpitWriteError::StaleState) => Ok(Err(ActivityRefusal::Position)),
        Err(SoulpitWriteError::Rule(crate::domain::soulpit::SoulpitError::LevelTooLow)) => {
            Ok(Err(ActivityRefusal::Level))
        }
        Err(SoulpitWriteError::Rule(crate::domain::soulpit::SoulpitError::NotOnEntryTile)) => {
            Ok(Err(ActivityRefusal::Position))
        }
        Err(SoulpitWriteError::Rule(crate::domain::soulpit::SoulpitError::InvalidParticipants)) => {
            Ok(Err(ActivityRefusal::PartyCount))
        }
        Err(SoulpitWriteError::Rule(crate::domain::soulpit::SoulpitError::ArenaOccupied)) => {
            Ok(Err(ActivityRefusal::NoCapacity))
        }
        Err(SoulpitWriteError::Rule(_)) => Ok(Err(ActivityRefusal::InvalidInput)),
    }
}

impl ActivityHandoffOwner {
    /// The same source sessions, physical reservations and current control allocation
    /// as every shared handoff. The core, encounter and successors commit together.
    pub async fn enter_soulpit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        mut request: ActivityEnterRequest,
        core: &SoulpitEntryRequest,
        native: &NativeGameplayState,
    ) -> Result<ActivityEnterOutcome> {
        request
            .participants
            .sort_by_key(|p| *p.source.character_id.as_bytes());
        if !entry::valid_request(&request, false)
            || request.activity_key != crate::durability::soulpit::SOULPIT_ACTIVITY_KEY
            || request.handoff_id != core.handoff_id
            || request.command_id != core.command.command_id().get()
            || request.participants.len() != core.participants.len()
            || request.time_limit_ms != crate::domain::soulpit::TIME_LIMIT_MICROS / 1000
            || request
                .participants
                .iter()
                .find(|p| {
                    p.source.character_id.as_bytes() == request.initiating_character_id.as_bytes()
                })
                .is_none_or(|p| p.source.game_session_id != core.command.game_session_id())
        {
            return Ok(ActivityEnterOutcome::Refused(ActivityRefusal::InvalidInput));
        }
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let root = self.root.clone();
        let node = node.clone();
        let mut context = Entry {
            authority,
            permit,
            request: core,
            native,
        };
        self.root.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, consequence| Box::pin(async move {
            // The Type2 activation fence is acquired before any admission statement.
            let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let Some(origin) = request.participants.first() else {
                return Ok(ActivityEnterOutcome::Refused(ActivityRefusal::InvalidInput));
            };
            let RuntimeScopeRefV1::Channel { world_id, channel_id } = origin.source.runtime_scope else {
                return Ok(ActivityEnterOutcome::Refused(ActivityRefusal::InvalidInput));
            };
            // Crystal fight.lua:27-31 has one arena per source Channel. Fresh
            // Instances isolate actors; they must not bypass that source occupancy.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:soulpit-room:'||encode($1,'hex')||':'||encode($2,'hex'),0))")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
                .execute(&mut **tx).await?;
            let occupied: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_activity_handoff_groups g JOIN game_activity_handoff_participants p USING(handoff_id) WHERE g.world_id=encode($1,'hex')::uuid AND p.origin_channel_id=encode($2,'hex')::uuid AND g.activity_key=$3 AND g.state IN(1,2) AND (g.state=1 OR NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE r.handoff_id=p.handoff_id AND r.character_id=p.character_id)) AND g.handoff_id<>encode($4,'hex')::uuid)")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
                .bind(crate::durability::soulpit::SOULPIT_ACTIVITY_KEY).bind(request.handoff_id.as_slice())
                .fetch_one(&mut **tx).await?;
            if occupied { return Ok(ActivityEnterOutcome::Refused(ActivityRefusal::NoCapacity)); }
            let result = entry::enter_in_transaction(&mut tx, &root, &node, &request, false, Some(consequence)).await?;
            // Generic historical handoff replay returns before its preparation phase.
            // Still bind the historical core and original USE bytes, before granting success.
            if matches!(result, ActivityEnterOutcome::Replayed(_))
                && let Err(refusal) = write(&root, &mut tx, &node, &request, consequence).await?
            {
                return Ok(ActivityEnterOutcome::Refused(refusal));
            }
            if !matches!(result, ActivityEnterOutcome::Refused(_)) { tx.commit(deadline).await?; }
            Ok(result)
        })).await
    }
}
