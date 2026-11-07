//! Closed shared source-portal admission. WorldParty does not grant entry.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourcePortalGroup {
    kind: i16,
    identity: [u8; 16],
    membership: [u8; 32],
}
impl SourcePortalGroup {
    fn for_channel(world: [u8; 16], channel: [u8; 16]) -> Self {
        let mut digest = Sha256::new();
        digest.update(b"oteryn:summer-source-channel-room:v1");
        digest.update(world);
        digest.update(channel);
        Self {
            kind: 3,
            identity: channel,
            membership: digest.finalize().into(),
        }
    }
}
/// Actual current session/Character/Node/scope is authority. The source portal
/// has no party gate; the same World+Channel shares one active physical room.
/// Existing party ownership remains independent for contribution/XP consumers.
pub(super) async fn current_group(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    actor: CurrentCharacterItemFence,
) -> Result<Option<SourcePortalGroup>> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = actor.runtime_scope
    else {
        return Ok(None);
    };
    if assert_item_only_fence(tx, &actor, node).await?.is_err() {
        return Ok(None);
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:summer-source-room:rakesh:'||encode($1,'hex')||':'||encode($2,'hex'),0))")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .execute(&mut **tx).await?;
    Ok(Some(SourcePortalGroup::for_channel(
        *world_id.as_bytes(),
        *channel_id.as_bytes(),
    )))
}
pub(super) const RAKESH: &str = "oteryn:boss.rakesh_moonfang";

pub(super) fn plan_source_step(
    policy: &oteryn_gameplay_encounter_rules::boss_room::BossRoomPolicy,
    facts: &[oteryn_gameplay_encounter_rules::boss_room::ParticipantFact],
    now: u64,
) -> std::result::Result<
    oteryn_gameplay_encounter_rules::boss_room::FreshRoomPlan,
    oteryn_gameplay_encounter_rules::boss_room::AdmissionRefusal,
> {
    use oteryn_gameplay_encounter_rules::boss_room::{AdmissionRefusal as Refusal, FreshRoomPlan};
    if policy.minimum_players != 1
        || policy.maximum_players != ACTIVITY_PARTICIPANTS_MAX
        || policy.minimum_level != 1
        || policy.cooldown_ms != 7_200_000
        || policy.time_limit_ms != 600_000
        || policy.lever_positions.len() != 1
        || !policy.lever_positions.contains(&(33039, 32964, 8))
        || now > i64::MAX as u64
    {
        return Err(Refusal::InvalidPolicy);
    }
    let [fact] = facts else {
        return Err(Refusal::Count);
    };
    if fact.position != (33039, 32964, 8) {
        return Err(Refusal::Position);
    }
    if fact.level == 0 {
        return Err(Refusal::Level);
    }
    if now < fact.cooldown_until_ms {
        return Err(Refusal::Cooldown);
    }
    let until = |duration| {
        now.checked_add(duration)
            .filter(|value| *value <= i64::MAX as u64)
            .ok_or(Refusal::InvalidPolicy)
    };
    Ok(FreshRoomPlan {
        participants: vec![fact.character],
        remove_existing_creatures: true,
        spawn_boss: true,
        cooldown_until_ms: until(policy.cooldown_ms)?,
        deadline_ms: until(policy.time_limit_ms)?,
    })
}

pub(super) async fn replay_join(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    request: &ActivityEnterRequest,
    binding: [u8; 32],
) -> Result<Option<ActivityEnterOutcome>> {
    let [participant] = request.participants.as_slice() else {
        return Ok(None);
    };
    let row=sqlx::query("SELECT uuid_send(admission) AS admission,request_sha256 FROM game_summer_portal_joins \
        WHERE character_id=encode($1,'hex')::uuid AND source_game_session_id=encode($2,'hex')::uuid \
          AND source_connection_generation=$3::text::numeric AND step_occurrence=$4::text::numeric")
        .bind(participant.source.character_id.as_bytes().as_slice()).bind(participant.source.game_session_id.as_bytes().as_slice())
        .bind(participant.source.connection_generation.get().to_string()).bind(request.command_id.to_string())
        .fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.try_get::<Vec<u8>, _>("request_sha256")? != binding
        || bytes16(row.try_get("admission")?)? != request.handoff_id
    {
        return Ok(Some(ActivityEnterOutcome::Refused(
            ActivityRefusal::ReplayConflict,
        )));
    }
    if !prove_current_incarnation(tx, node).await?
        || !FreshAdmissionStore::from_root(root.clone())
            .committed_session_use(
                tx,
                request.initiating_character_id,
                participant.source.game_session_id,
            )
            .await?
    {
        return Ok(Some(ActivityEnterOutcome::Refused(
            ActivityRefusal::AuthorityRejected,
        )));
    }
    Ok(Some(ActivityEnterOutcome::Replayed(
        one_receipt(tx, request).await?,
    )))
}

pub(super) async fn matches_join(
    tx: &mut Transaction<'_, Postgres>,
    request: &ActivityEnterRequest,
    group: &SourcePortalGroup,
) -> Result<bool> {
    let [participant] = request.participants.as_slice() else {
        return Ok(false);
    };
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = participant.source.runtime_scope
    else {
        return Ok(false);
    };
    let RuntimeScopeRefV1::Instance { instance_id, .. } = request.destination_scope else {
        return Ok(false);
    };
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_summer_portal_rooms room JOIN game_activity_handoff_groups g ON g.handoff_id=room.admission \
        JOIN game_boss_room_activities a ON a.admission=g.handoff_id WHERE room.admission=encode($1,'hex')::uuid \
        AND room.world_id=encode($2,'hex')::uuid AND room.instance_id=encode($3,'hex')::uuid AND room.boss_key=$4 \
        AND room.source_channel_id=encode($5,'hex')::uuid AND room.group_kind=$6 AND room.group_id=encode($7,'hex')::uuid \
        AND room.membership_sha256=$8 AND g.state=2 AND g.ownership_generation=$9::text::numeric \
        AND a.state=1 AND a.deadline>floor(extract(epoch FROM clock_timestamp())*1000)::bigint)")
        .bind(request.handoff_id.as_slice()).bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice())
        .bind(RAKESH).bind(channel_id.as_bytes().as_slice()).bind(group.kind).bind(group.identity.as_slice())
        .bind(group.membership.as_slice()).bind(request.destination_generation.get().to_string())
        .fetch_one(&mut **tx).await.map_err(Into::into)
}

pub(super) async fn arm(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    request: &ActivityEnterRequest,
    group: &SourcePortalGroup,
    binding: [u8; 32],
) -> Result<()> {
    let [participant] = request.participants.as_slice() else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = participant.source.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let RuntimeScopeRefV1::Instance { instance_id, .. } = request.destination_scope else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let join = super::super::starter_grant::new_uuid(tx).await?;
    let fact = node.fact();
    sqlx::query("SELECT game_summer_portal_arm(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric, \
        encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9::text::numeric,$10,encode($11,'hex')::uuid,encode($12,'hex')::uuid, \
        encode($13,'hex')::uuid,$14::text::numeric,$15::text::numeric,$16::text::numeric,$17)")
        .bind(request.handoff_id.as_slice()).bind(join.as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(participant.source.scope_ownership_generation.get().to_string()).bind(instance_id.as_slice())
        .bind(request.destination_generation.get().to_string()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
        .bind(node.sql_possession_secret()).bind(participant.source.character_id.as_bytes().as_slice())
        .bind(participant.source.game_session_id.as_bytes().as_slice()).bind(participant.destination_game_session_id.as_bytes().as_slice())
        .bind(participant.source.connection_generation.get().to_string()).bind(participant.source.character_lease_generation.to_string())
        .bind(request.command_id.to_string()).bind(binding.as_slice()).execute(&mut **tx).await?;
    let same: bool = sqlx::query_scalar(
        "SELECT game_summer_portal_context_matches($1,encode($2,'hex')::uuid,$3)",
    )
    .bind(group.kind)
    .bind(group.identity.as_slice())
    .bind(group.membership.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if !same {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

pub(super) async fn join(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    request: &ActivityEnterRequest,
    now: i64,
) -> Result<ActivityEnterOutcome> {
    let plan = match boss_room::prepare_in_transaction(tx, request, now).await? {
        Ok(plan) => plan,
        Err(reason) => return Ok(ActivityEnterOutcome::Refused(reason)),
    };
    let [participant] = request.participants.as_slice() else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let RuntimeScopeRefV1::Channel { channel_id, .. } = participant.source.runtime_scope else {
        return Err(DurabilityError::InvalidStoredState);
    };
    entry::insert_participant(tx, request, participant).await?;
    entry::issue_successor(
        tx,
        root,
        entry::SuccessorSession {
            source: participant.source,
            destination: participant.destination_game_session_id,
            decision: request.handoff_id,
            scope: request.destination_scope,
            scope_generation: request.destination_generation,
            origin: Some(*channel_id.as_bytes()),
            transport: participant.destination_transport,
        },
        now,
    )
    .await?;
    boss_room::commit_source_join(tx, request, plan).await?;
    sqlx::query("SELECT game_summer_portal_commit()")
        .execute(&mut **tx)
        .await?;
    Ok(ActivityEnterOutcome::Entered(
        one_receipt(tx, request).await?,
    ))
}

pub(super) async fn one_receipt(
    tx: &mut Transaction<'_, Postgres>,
    request: &ActivityEnterRequest,
) -> Result<ActivityEnterReceipt> {
    let mut receipt = entry::load_receipt(tx, request.handoff_id).await?;
    receipt
        .participants
        .retain(|row| row.character_id == request.initiating_character_id);
    if receipt.participants.len() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(receipt)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingSourcePortalRoom {
    pub handoff_id: [u8; 16],
    pub allocation: boss_room::ActivityReadyAllocation,
}
impl ActivityHandoffOwner {
    /// Discover only the actual shared World+Channel source room. This
    /// read cannot allocate a scope, admit a member or prolong an encounter.
    pub async fn existing_summer_portal_room(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        actor: CurrentCharacterItemFence,
    ) -> Result<Option<ExistingSourcePortalRoom>> {
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let Some(group)=current_group(&mut tx,&node,actor).await? else{return Ok(None);};
            let RuntimeScopeRefV1::Channel{world_id,channel_id}=actor.runtime_scope else{return Ok(None);};
            let fact=node.fact();
            let row=sqlx::query("SELECT uuid_send(room.admission) AS admission,uuid_send(room.instance_id) AS instance, \
                ownership.ownership_generation::text AS generation,ownership.source_revision::text AS revision,ownership.decision_identity \
                FROM game_summer_portal_rooms room JOIN game_activity_handoff_groups g ON g.handoff_id=room.admission \
                JOIN game_boss_room_activities activity ON activity.admission=g.handoff_id \
                JOIN game_runtime_scope_assignments ownership ON ownership.scope_key='\\x03'::bytea||uuid_send(room.world_id)||uuid_send(room.instance_id) \
                JOIN game_durability_admission_runtime_guards guard ON guard.scope_key=ownership.scope_key \
                WHERE room.world_id=encode($1,'hex')::uuid AND room.source_channel_id=encode($2,'hex')::uuid AND room.boss_key=$3 \
                AND room.group_kind=$4 AND room.group_id=encode($5,'hex')::uuid AND room.membership_sha256=$6 \
                AND g.state=2 AND activity.state=1 AND activity.deadline>floor(extract(epoch FROM clock_timestamp())*1000)::bigint \
                AND ownership.state=1 AND ownership.ownership_generation=g.ownership_generation AND guard.ready \
                AND guard.ownership_generation=g.ownership_generation AND ownership.holder_node_id=encode($7,'hex')::uuid \
                AND ownership.holder_registration_revision=$8::text::numeric \
                ORDER BY room.admission LIMIT 2 FOR SHARE OF room,g,activity,ownership,guard")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(RAKESH).bind(group.kind)
                .bind(group.identity.as_slice()).bind(group.membership.as_slice()).bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string()).fetch_all(&mut *tx).await?;
            let result=match row.as_slice(){
                []=>None,
                [row]=>Some(ExistingSourcePortalRoom{
                    handoff_id:bytes16(row.try_get("admission")?)?,
                    allocation:boss_room::ActivityReadyAllocation{
                        scope:RuntimeScopeRefV1::instance(world_id,bytes16(row.try_get("instance")?)?).map_err(|_|DurabilityError::InvalidStoredState)?,
                        generation:ScopeOwnershipGeneration::new(parse_u64(row.try_get("generation")?)?).map_err(|_|DurabilityError::InvalidStoredState)?,
                        source_revision:parse_u64(row.try_get("revision")?)?,
                        decision_identity:row.try_get("decision_identity")?,
                    },
                }),
                _=>return Err(DurabilityError::InvalidStoredState),
            };
            commit_semantic_transaction(tx,deadline).await?;
            Ok(result)
        })).await
    }
}

impl ActivityHandoffOwner {
    /// A real Source STEP publishes only the exact sealed current template's
    /// static catalogue row before asking the ordinary boss admission owner.
    /// This is not membership, allocation or reward authority.
    pub async fn publish_qualified_summer_portal(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        actor: CurrentCharacterItemFence,
        template:&oteryn_gameplay_content::content::bosses::activity_template::QualifiedActivityRoomTemplate,
    ) -> Result<()> {
        if !template.source_portal()
            || template.boss_key() != RAKESH
            || template.template_revision()
                != "source-portal-b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105"
            || template.maximum_players() != ACTIVITY_PARTICIPANTS_MAX
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = actor.runtime_scope
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let content = template.content_revision().to_owned();
        let server = template.room().compiled().server_digest();
        let client = template.room().compiled().client_digest();
        let frame = template.origin_frame_digest();
        self.root.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            let current=assert_item_only_fence(&mut tx,&actor,&node).await?.map_err(|_|DurabilityError::Unavailable)?;
            if current.content_revision!=content{return Err(DurabilityError::InvalidStoredState);}
            let fact=node.fact();
            sqlx::query("SELECT game_summer_portal_publish(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric, \
                encode($4,'hex')::uuid,$5::text::numeric,$6,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9::text::numeric,$10::text::numeric,$11,$12,$13,$14)")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(actor.scope_ownership_generation.get().to_string())
                .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret())
                .bind(actor.character_id.as_bytes().as_slice()).bind(actor.game_session_id.as_bytes().as_slice()).bind(actor.connection_generation.get().to_string())
                .bind(actor.character_lease_generation.to_string()).bind(content).bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice())
                .execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(())
        })).await
    }
}

/// The source native dispatcher has already frozen the actual actor at the
/// qualified return TP. SQL independently seals its current durable identity.
pub(super) async fn arm_exit(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    r: &ActivityReturnRequest,
) -> Result<bool> {
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = r.source.runtime_scope
    else {
        return Ok(false);
    };
    let qualified:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_summer_portal_rooms room JOIN game_activity_handoff_groups g ON g.handoff_id=room.admission WHERE room.admission=encode($1,'hex')::uuid AND room.world_id=encode($2,'hex')::uuid AND room.instance_id=encode($3,'hex')::uuid AND room.boss_key=$4 AND g.state=2 AND g.ownership_generation=$5::text::numeric AND room.source_portal_key='oteryn:source-portal.rakesh_moonfang')")
  .bind(r.handoff_id.as_slice()).bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).bind(RAKESH).bind(r.source.scope_ownership_generation.get().to_string()).fetch_one(&mut **tx).await?;
    if !qualified {
        return Ok(false);
    }
    let fact = node.fact();
    sqlx::query("SELECT game_summer_portal_exit_arm(encode($1,'hex')::uuid,$2::text::numeric,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric,$8::text::numeric,$9::text::numeric,encode($10,'hex')::uuid)")
  .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret()).bind(r.handoff_id.as_slice())
  .bind(r.source.character_id.as_bytes().as_slice()).bind(r.source.game_session_id.as_bytes().as_slice()).bind(r.source.connection_generation.get().to_string())
  .bind(r.source.character_lease_generation.to_string()).bind(r.source.scope_ownership_generation.get().to_string()).bind(r.return_id.as_slice()).execute(&mut **tx).await?;
    Ok(true)
}
