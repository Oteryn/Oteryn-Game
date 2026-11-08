//! Soulpit composes its debit and encounter with the shared session handoff.
use super::*;
use crate::durability::soulpit::{SOULPIT_ACTIVITY_KEY, SoulpitEntryRequest};
use crate::durability::spell_owner_commit::SpellLanePermit;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;

pub(super) struct SoulpitEntryContext<'a> {
    pub recovery: &'a crate::character_recovery_fence::CharacterRecoveryFenceV1,
    pub request: &'a SoulpitEntryRequest,
    pub native: &'a NativeGameplayState,
}

#[derive(Debug, Clone, Copy)]
pub struct SoulpitBinding {
    pub handoff_id: [u8; 16],
    pub scope: RuntimeScopeRefV1,
    pub generation: ScopeOwnershipGeneration,
}

#[derive(Debug, Clone, Copy)]
pub struct SoulpitMembership {
    pub character: crate::domain::CharacterId,
    pub session: GameSessionId,
    pub slot: u8,
}

impl ActivityHandoffOwner {
    /// Crystal 201189f data-global/scripts/quests/soulpit/soulpit_fight.lua:37-106.
    /// A failed debit or any failed successor rolls back the entire group.
    #[allow(
        clippy::too_many_arguments,
        reason = "independent lane, authority and content proofs"
    )]
    pub async fn enter_soulpit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        mut request: ActivityEnterRequest,
        entry: &SoulpitEntryRequest,
        native: &NativeGameplayState,
    ) -> Result<ActivityEnterOutcome> {
        request
            .participants
            .sort_by_key(|p| *p.source.character_id.as_bytes());
        if request.activity_key != SOULPIT_ACTIVITY_KEY
            || request.handoff_id != entry.handoff_id
            || request.command_id != entry.command.command_id().get()
            || !request.participants.iter().any(|p| {
                p.source.character_id.as_bytes() == request.initiating_character_id.as_bytes()
                    && p.source.game_session_id == entry.command.game_session_id()
            })
            || !entry::valid_request(&request, false)
        {
            return Ok(ActivityEnterOutcome::Refused(ActivityRefusal::InvalidInput));
        }
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let initiator = request
            .participants
            .iter()
            .find(|p| {
                p.source.character_id.as_bytes() == request.initiating_character_id.as_bytes()
            })
            .ok_or(DurabilityError::InvalidStoredState)?;
        permit.check_scope(initiator.source.runtime_scope)?;
        let node = node.clone();
        let entry = entry.clone();
        let native = native.clone();
        let root = self.root.clone();
        self.root
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let context = SoulpitEntryContext {
                        recovery: &recovery,
                        request: &entry,
                        native: &native,
                    };
                    let result = entry::enter_in_transaction(
                        &mut tx,
                        &root,
                        &node,
                        &request,
                        false,
                        Some(context),
                    )
                    .await?;
                    if !matches!(result, ActivityEnterOutcome::Refused(_)) {
                        tx.commit(deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await
    }

    /// Bounded current-owner traversal, including terminal fights with returns still due.
    pub async fn active_soulpits(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
        after: Option<[u8; 16]>,
    ) -> Result<Vec<SoulpitBinding>> {
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx, &node).await? { return Err(DurabilityError::Unavailable); }
            let history: bool = sqlx::query_scalar("SELECT game_runtime_scope_assignment_history_valid()")
                .fetch_one(&mut *tx).await?;
            if !history { return Err(DurabilityError::Unavailable); }
            let fact = node.fact();
            // Readiness must filter before LIMIT: fifteen unready retained fights must
            // not hide later ready Instances by turning a full page into a short one.
            let rows = sqlx::query("SELECT uuid_send(g.handoff_id) AS handoff,uuid_send(g.instance_id) AS instance,g.ownership_generation::text AS generation \
                FROM game_activity_handoff_groups g JOIN game_soulpit_encounters f USING(handoff_id) \
                JOIN game_runtime_scope_assignments a ON a.scope_key='\\x03'::bytea || uuid_send(g.world_id) || uuid_send(g.instance_id) \
                JOIN game_durability_admission_runtime_guards rg ON rg.scope_key='\\x02'::bytea || uuid_send(g.world_id) || uuid_send(g.instance_id) \
                WHERE g.world_id=encode($1,'hex')::uuid AND g.activity_key=$2 AND g.state=2 \
                  AND a.scope_kind=3 AND a.state=1 AND a.world_id=g.world_id AND a.activity_instance_id=g.instance_id \
                  AND a.channel_id IS NULL AND a.house_key IS NULL AND a.ownership_generation=g.ownership_generation \
                  AND a.holder_node_id=encode($3,'hex')::uuid AND a.holder_registration_revision=$4::text::numeric \
                  AND rg.ready AND rg.ownership_generation=a.ownership_generation \
                  AND ($5::bytea IS NULL OR g.handoff_id>encode($5,'hex')::uuid) \
                  AND EXISTS(SELECT 1 FROM game_activity_handoff_participants p WHERE p.handoff_id=g.handoff_id AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE r.handoff_id=p.handoff_id AND r.character_id=p.character_id)) \
                ORDER BY g.handoff_id LIMIT 15 FOR SHARE OF a,rg")
                .bind(world.as_bytes().as_slice()).bind(SOULPIT_ACTIVITY_KEY)
                .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
                .bind(after.as_ref().map(|id| id.as_slice())).fetch_all(&mut *tx).await?;
            let mut result = Vec::with_capacity(rows.len());
            for row in rows {
                let scope = RuntimeScopeRefV1::instance(world, bytes16(row.try_get("instance")?)?).map_err(|_| DurabilityError::InvalidStoredState)?;
                let generation = ScopeOwnershipGeneration::new(parse_u64(row.try_get("generation")?)?).map_err(|_| DurabilityError::InvalidStoredState)?;
                if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                    return Err(DurabilityError::Unavailable);
                }
                result.push(SoulpitBinding { handoff_id: bytes16(row.try_get("handoff")?)?, scope, generation });
            }
            Ok(result)
        })).await
    }

    pub async fn soulpit_members(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        binding: SoulpitBinding,
    ) -> Result<Vec<SoulpitMembership>> {
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            if !assert_runtime_scope_fence(&mut tx, binding.scope, binding.generation, &node).await? {
                return Err(DurabilityError::Unavailable);
            }
            let RuntimeScopeRefV1::Instance {world_id,instance_id} = binding.scope else { return Err(DurabilityError::InvalidStoredState); };
            let rows = sqlx::query("SELECT uuid_send(p.character_id) AS character,uuid_send(p.destination_game_session_id) AS session,s.staging_slot \
                FROM game_soulpit_participants s JOIN game_activity_handoff_participants p USING(handoff_id,character_id) \
                JOIN game_activity_handoff_groups g USING(handoff_id) \
                WHERE g.handoff_id=encode($1,'hex')::uuid AND g.world_id=encode($2,'hex')::uuid AND g.instance_id=encode($3,'hex')::uuid \
                  AND g.state=2 AND g.ownership_generation=$4::text::numeric AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE r.handoff_id=p.handoff_id AND r.character_id=p.character_id) \
                ORDER BY p.character_id LIMIT 5")
                .bind(binding.handoff_id.as_slice()).bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice())
                .bind(binding.generation.get().to_string()).fetch_all(&mut *tx).await?;
            rows.into_iter().map(|row| Ok(SoulpitMembership {
                character: crate::domain::CharacterId::from_bytes(bytes16(row.try_get("character")?)?).map_err(|_| DurabilityError::InvalidStoredState)?,
                session: GameSessionId::decode(&bytes16(row.try_get("session")?)?).map_err(|_| DurabilityError::InvalidStoredState)?,
                slot: u8::try_from(row.try_get::<i16,_>("staging_slot")?).map_err(|_| DurabilityError::InvalidStoredState)?,
            })).collect()
        })).await
    }
}
