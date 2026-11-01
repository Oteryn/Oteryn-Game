use super::super::character_last_position::{
    LastPositionKey, LastPositionWrite, upsert_last_position,
};
use super::*;

pub(super) fn valid_selection(selection: &[u32]) -> bool {
    selection.len() <= oteryn_protocol_oteryn::MAX_CAPABILITY_COUNT
        && selection.first().is_none_or(|id| *id != 0)
        && selection.windows(2).all(|pair| pair[0] < pair[1])
}

pub(super) fn valid_request(r: &ActivityEnterRequest, boss_room: bool) -> bool {
    let Some(first) = r.participants.first() else {
        return false;
    };
    let RuntimeScopeRefV1::Channel { world_id, .. } = first.source.runtime_scope else {
        return false;
    };
    let RuntimeScopeRefV1::Instance {
        world_id: target_world,
        instance_id,
    } = r.destination_scope
    else {
        return false;
    };
    if !uuid_v7(&r.handoff_id)
        || !uuid_v7(&instance_id)
        || r.command_id == 0
        || world_id != target_world
        || r.destination_assignment_source_revision == 0
        || r.activity_key.is_empty()
        || r.activity_key.len() > 512
        || !revision_valid(&r.template_revision)
        || !revision_valid(&r.content_revision)
        || r.time_limit_ms == 0
        || r.time_limit_ms > 86_400_000
        || r.participants.len() > ACTIVITY_PARTICIPANTS_MAX
    {
        return false;
    }
    let mut characters = std::collections::BTreeSet::new();
    let mut sessions = std::collections::BTreeSet::new();
    let mut transports = std::collections::BTreeSet::new();
    let mut cells = std::collections::BTreeSet::new();
    let mut has_initiator = false;
    for p in &r.participants {
        if p.source.runtime_scope != first.source.runtime_scope
            || p.source.character_lease_generation == 0
            || p.source.character_lease_generation == u64::MAX
            || p.destination_transport.is_none()
            || !valid_selection(&p.selected_capabilities)
            || p.destination_game_session_id == p.source.game_session_id
            || p.source_position_write_sequence == 0
            || p.source_position.world_id != world_id
            || p.exit_position.world_id != world_id
            || p.source_position.vitals.is_none()
            || !characters.insert(*p.source.character_id.as_bytes())
            || !sessions.insert(*p.destination_game_session_id.as_bytes())
            || !transports.insert(p.destination_transport.map(|t| t.to_bytes()))
            || (!boss_room
                && !cells.insert((
                    p.destination_cell.x,
                    p.destination_cell.y,
                    p.destination_cell.floor,
                )))
        {
            return false;
        }
        has_initiator |= p.source.character_id.as_bytes() == r.initiating_character_id.as_bytes();
    }
    has_initiator
}

fn binding(r: &ActivityEnterRequest) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash_field(&mut hash, b"native-activity-enter-v1");
    for text in [&r.activity_key, &r.template_revision, &r.content_revision] {
        hash_field(&mut hash, text.as_bytes());
    }
    hash_field(&mut hash, &r.time_limit_ms.to_be_bytes());
    for digest in [
        r.destination_content.coordinate_frame_digest,
        r.destination_content.map_revision_digest,
        r.destination_content.server_artifact_digest,
    ] {
        hash_field(&mut hash, &digest);
    }
    for p in &r.participants {
        hash_field(&mut hash, p.source.character_id.as_bytes());
        hash_field(&mut hash, p.source.game_session_id.as_bytes());
        hash_field(
            &mut hash,
            &p.source.connection_generation.get().to_be_bytes(),
        );
        hash_field(
            &mut hash,
            &p.source.character_lease_generation.to_be_bytes(),
        );
        hash_field(
            &mut hash,
            &p.source.scope_ownership_generation.get().to_be_bytes(),
        );
        hash_field(&mut hash, &assignment_scope_key(p.source.runtime_scope));
        hash_field(
            &mut hash,
            &(p.selected_capabilities.len() as u64).to_be_bytes(),
        );
        for capability in &p.selected_capabilities {
            hash_field(&mut hash, &capability.to_be_bytes());
        }
        for cell in [
            p.source_position.cell,
            p.destination_cell,
            p.exit_position.cell,
        ] {
            hash_field(&mut hash, &cell.x.to_be_bytes());
            hash_field(&mut hash, &cell.y.to_be_bytes());
            hash_field(&mut hash, &cell.floor.to_be_bytes());
        }
        for record in [p.source_position, p.exit_position] {
            for digest in [
                record.coordinate_frame_digest,
                record.map_revision_digest,
                record.server_artifact_digest,
            ] {
                hash_field(&mut hash, &digest);
            }
        }
    }
    hash.finalize().into()
}

pub(super) async fn enter_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    r: &ActivityEnterRequest,
    boss_room: bool,
) -> Result<ActivityEnterOutcome> {
    let initiator = r
        .participants
        .iter()
        .find(|p| p.source.character_id.as_bytes() == r.initiating_character_id.as_bytes())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let request_binding = binding(r);
    // Lock all source sessions before roots, both in Character order. This also
    // serializes command receipt absence without reversing a progression writer's
    // session-then-root order or locking the initiator ahead of other members.
    for p in &r.participants {
        sqlx::query("SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(p.source.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    }
    for p in &r.participants {
        sqlx::query("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(p.source.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    }
    let old: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as(
        "SELECT uuid_send(handoff_id), request_binding FROM game_activity_runtime_entries \
         WHERE initiating_character_id=encode($1,'hex')::uuid AND initiating_game_session_id=encode($2,'hex')::uuid \
           AND initiating_connection_generation=$3::text::numeric AND command_id=$4::text::numeric")
        .bind(r.initiating_character_id.as_bytes().as_slice()).bind(initiator.source.game_session_id.as_bytes().as_slice())
        .bind(initiator.source.connection_generation.get().to_string()).bind(r.command_id.to_string())
        .fetch_optional(&mut **tx).await?;
    if let Some((handoff, old_binding)) = old {
        if old_binding != request_binding {
            return Ok(ActivityEnterOutcome::Refused(
                ActivityRefusal::ReplayConflict,
            ));
        }
        if !prove_current_incarnation(tx, node).await?
            || !FreshAdmissionStore::from_root(root.clone())
                .committed_session_use(
                    tx,
                    r.initiating_character_id,
                    initiator.source.game_session_id,
                )
                .await?
        {
            return Ok(ActivityEnterOutcome::Refused(
                ActivityRefusal::AuthorityRejected,
            ));
        }
        return Ok(ActivityEnterOutcome::Replayed(
            load_receipt(tx, bytes16(handoff)?).await?,
        ));
    }
    let guards = AdmissionGuardStore::from_root(root.clone());
    if guards
        .load_locked(
            tx,
            &AdmissionAuthorityGuardKeyV1::Runtime(r.destination_scope),
        )
        .await?
        .is_none()
        || !assert_runtime_scope_fence(tx, r.destination_scope, r.destination_generation, node)
            .await?
    {
        return Ok(ActivityEnterOutcome::Refused(ActivityRefusal::NoCapacity));
    }
    let exact: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_runtime_scope_assignments a \
         WHERE a.scope_key=$1 AND a.source_revision=$2::text::numeric \
           AND NOT EXISTS (SELECT 1 FROM game_activity_handoff_groups g WHERE g.world_id=a.world_id AND g.instance_id=a.activity_instance_id) \
           AND NOT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.runtime_scope_world_id=a.world_id \
             AND s.runtime_scope_instance_id=a.activity_instance_id))")
        .bind(assignment_scope_key(r.destination_scope).as_slice()).bind(r.destination_assignment_source_revision.to_string())
        .fetch_one(&mut **tx).await?;
    if !exact {
        return Ok(ActivityEnterOutcome::Refused(ActivityRefusal::NoCapacity));
    }
    let mut accounts = std::collections::BTreeSet::new();
    for p in &r.participants {
        let Some(account) = retained_participant(tx, root, p.source).await? else {
            return Ok(ActivityEnterOutcome::Refused(
                ActivityRefusal::AuthorityRejected,
            ));
        };
        if let Some(transport) = p.destination_transport {
            let used: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_transport_ref_reservations WHERE transport_ref=$1)")
                .bind(transport.to_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if used {
                return Ok(ActivityEnterOutcome::Refused(ActivityRefusal::InvalidInput));
            }
        }
        if !accounts.insert(account) {
            return Ok(ActivityEnterOutcome::Refused(ActivityRefusal::InvalidInput));
        }
        let Ok(current) = assert_item_only_fence(tx, &p.source, node).await? else {
            return Ok(ActivityEnterOutcome::Refused(
                ActivityRefusal::AuthorityRejected,
            ));
        };
        if current.content_revision != r.content_revision {
            return Ok(ActivityEnterOutcome::Refused(
                ActivityRefusal::ContentChanged,
            ));
        }
        if upsert_last_position(
            tx,
            p.source.character_id.as_bytes(),
            LastPositionKey {
                character_lease_generation: p.source.character_lease_generation,
                write_sequence: p.source_position_write_sequence,
            },
            &p.source_position,
        )
        .await?
            == LastPositionWrite::PendingObligation
        {
            return Ok(ActivityEnterOutcome::Refused(
                ActivityRefusal::AuthorityRejected,
            ));
        }
    }
    let now = now_ms(tx).await?;
    let boss_plan = if boss_room {
        match super::boss_room::prepare_in_transaction(tx, r, now).await? {
            Ok(plan) => Some(plan),
            Err(reason) => return Ok(ActivityEnterOutcome::Refused(reason)),
        }
    } else {
        None
    };
    let deadline = i64::try_from(r.time_limit_ms)
        .ok()
        .and_then(|v| now.checked_add(v))
        .ok_or(DurabilityError::InvalidStoredState)?;
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = r.destination_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    sqlx::query("INSERT INTO game_activity_handoff_groups(handoff_id,world_id,instance_id,activity_key,template_revision,content_revision,ownership_generation,participant_count,state,prepared_at) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6,$7::text::numeric,$8,1,$9)")
        .bind(r.handoff_id.as_slice()).bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).bind(&r.activity_key)
        .bind(&r.template_revision).bind(&r.content_revision).bind(r.destination_generation.get().to_string())
        .bind(i16::try_from(r.participants.len()).map_err(|_| DurabilityError::InvalidStoredState)?).bind(now).execute(&mut **tx).await?;
    let fact = node.fact();
    sqlx::query("INSERT INTO game_activity_runtime_entries(handoff_id,initiating_character_id,initiating_game_session_id,initiating_connection_generation,command_id,request_binding,holder_node_id,holder_registration_revision,assignment_source_revision,destination_coordinate_frame_digest,destination_map_revision_digest,destination_server_artifact_digest,deadline) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5::text::numeric,$6,encode($7,'hex')::uuid,$8::text::numeric,$9::text::numeric,$10,$11,$12,$13)")
        .bind(r.handoff_id.as_slice()).bind(r.initiating_character_id.as_bytes().as_slice()).bind(initiator.source.game_session_id.as_bytes().as_slice())
        .bind(initiator.source.connection_generation.get().to_string()).bind(r.command_id.to_string()).bind(request_binding.as_slice())
        .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(r.destination_assignment_source_revision.to_string())
        .bind(r.destination_content.coordinate_frame_digest.as_slice()).bind(r.destination_content.map_revision_digest.as_slice())
        .bind(r.destination_content.server_artifact_digest.as_slice()).bind(deadline).execute(&mut **tx).await?;
    for p in &r.participants {
        insert_participant(tx, r, p).await?;
    }
    // 0265 source-ended trigger would abort PREPARED. Decide before terminalizing
    // any source; its deferred proof checks all fresh destinations at COMMIT.
    sqlx::query("UPDATE game_activity_handoff_groups SET state=2,decided_at=$2 WHERE handoff_id=encode($1,'hex')::uuid AND state=1")
        .bind(r.handoff_id.as_slice()).bind(now).execute(&mut **tx).await?;
    for p in &r.participants {
        let RuntimeScopeRefV1::Channel { channel_id, .. } = p.source.runtime_scope else {
            return Err(DurabilityError::InvalidStoredState);
        };
        issue_successor(
            tx,
            root,
            SuccessorSession {
                source: p.source,
                destination: p.destination_game_session_id,
                decision: r.handoff_id,
                scope: r.destination_scope,
                scope_generation: r.destination_generation,
                origin: Some(*channel_id.as_bytes()),
                transport: p.destination_transport,
            },
            now,
        )
        .await?;
    }
    if let Some(plan) = boss_plan {
        super::boss_room::commit_in_transaction(tx, r, plan).await?;
    }
    Ok(ActivityEnterOutcome::Entered(
        load_receipt(tx, r.handoff_id).await?,
    ))
}

async fn insert_participant(
    tx: &mut Transaction<'_, Postgres>,
    r: &ActivityEnterRequest,
    p: &ActivityParticipant,
) -> Result<()> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = p.source.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let s = p.source_position;
    sqlx::query("INSERT INTO game_activity_handoff_participants(handoff_id,world_id,character_id,account_id,origin_channel_id,source_game_session_id,destination_game_session_id,source_connection_generation,source_lease_generation,source_ownership_generation,profile_revision,ruleset_revision,content_revision,starter_template_revision,entry_x,entry_y,entry_floor,coordinate_frame_digest,map_revision_digest,server_artifact_digest) \
        SELECT encode($1,'hex')::uuid, r.world_id,r.character_id,r.account_id,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7::text::numeric,$8::text::numeric,r.profile_revision,r.ruleset_revision,r.content_revision,r.starter_template_revision,$9,$10,$11,$12,$13,$14 \
        FROM game_character_roots r WHERE r.character_id=encode($2,'hex')::uuid AND r.world_id=encode($15,'hex')::uuid")
        .bind(r.handoff_id.as_slice()).bind(p.source.character_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(p.source.game_session_id.as_bytes().as_slice()).bind(p.destination_game_session_id.as_bytes().as_slice())
        .bind(p.source.connection_generation.get().to_string()).bind(p.source.character_lease_generation.to_string()).bind(p.source.scope_ownership_generation.get().to_string())
        .bind(s.cell.x).bind(s.cell.y).bind(s.cell.floor).bind(s.coordinate_frame_digest.as_slice()).bind(s.map_revision_digest.as_slice()).bind(s.server_artifact_digest.as_slice())
        .bind(world_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    let exit = p.exit_position;
    let rest = s
        .rest
        .map(|values| {
            values
                .into_iter()
                .map(i32::try_from)
                .collect::<std::result::Result<Vec<_>, _>>()
        })
        .transpose()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_activity_runtime_placements(handoff_id,character_id,destination_x,destination_y,destination_floor,exit_x,exit_y,exit_floor,exit_coordinate_frame_digest,exit_map_revision_digest,exit_server_artifact_digest,saved_health,saved_mana,saved_rest,destination_transport_ref,selected_capabilities) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)")
        .bind(r.handoff_id.as_slice()).bind(p.source.character_id.as_bytes().as_slice()).bind(p.destination_cell.x).bind(p.destination_cell.y).bind(p.destination_cell.floor)
        .bind(exit.cell.x).bind(exit.cell.y).bind(exit.cell.floor).bind(exit.coordinate_frame_digest.as_slice()).bind(exit.map_revision_digest.as_slice()).bind(exit.server_artifact_digest.as_slice())
        .bind(s.vitals.map(|v| i32::try_from(v.health())).transpose().map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(s.vitals.map(|v| i32::try_from(v.mana())).transpose().map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(rest).bind(p.destination_transport.map(|t| t.to_bytes().to_vec()))
        .bind(p.selected_capabilities.iter().map(|id| i64::from(*id)).collect::<Vec<_>>())
        .execute(&mut **tx).await?;
    Ok(())
}

/// Inputs for one successor; issuance revalidates its source inside the transaction.
pub(super) struct SuccessorSession {
    pub(super) source: CurrentCharacterItemFence,
    pub(super) destination: GameSessionId,
    pub(super) decision: [u8; 16],
    pub(super) scope: RuntimeScopeRefV1,
    pub(super) scope_generation: ScopeOwnershipGeneration,
    pub(super) origin: Option<[u8; 16]>,
    pub(super) transport: Option<crate::foundation::AuthenticatedTransportRefV1>,
}

/// The existing session issuance and guard publishers remain the only writers.
/// The source is locked and terminal before its successor transport is installed.
pub(super) async fn issue_successor(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    successor: SuccessorSession,
    now: i64,
) -> Result<()> {
    let SuccessorSession {
        source,
        destination,
        decision,
        scope,
        scope_generation,
        origin,
        transport,
    } = successor;
    let character = foundation_character(source)?;
    let membership =
        super::super::fresh_admission::unused_session_revision(tx, character, destination).await?;
    // Resolve before terminalizing the source: a completed reconnect's budget
    // is still live here. Scope changes never issue another protection right.
    let protection = FreshAdmissionStore::from_root(root.clone())
        .activity_handoff_protection_in_transaction(tx, source, now.div_euclid(1000))
        .await?;
    let row = sqlx::query("SELECT account_id::text, current_transport_ref, original_grace_deadline,COALESCE(fresh_replay_key,initial_fresh_replay_key) AS initial_fresh FROM game_durability_reconnect_sessions \
        WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND current_generation=$3::text::numeric \
          AND character_lease_generation=$4::text::numeric AND session_state IN (1,2) FOR UPDATE")
        .bind(source.game_session_id.as_bytes().as_slice()).bind(source.character_id.as_bytes().as_slice())
        .bind(source.connection_generation.get().to_string()).bind(source.character_lease_generation.to_string()).fetch_optional(&mut **tx).await?
        .ok_or(DurabilityError::Unavailable)?;
    let account: String = row.try_get("account_id")?;
    let initial: Vec<u8> = row
        .try_get::<Option<Vec<u8>>, _>("initial_fresh")?
        .ok_or(DurabilityError::Unavailable)?;
    let old_transport: Option<Vec<u8>> = row.try_get("current_transport_ref")?;
    let transport = transport.map(|t| t.to_bytes().to_vec());
    if transport.is_some() && transport == old_transport {
        return Err(DurabilityError::InvalidStoredState);
    }
    let grace = row
        .try_get::<Option<i64>, _>("original_grace_deadline")?
        .unwrap_or(now.div_euclid(1000));
    sqlx::query("UPDATE game_durability_reconnect_attempts SET state=$2 WHERE game_session_id=encode($1,'hex')::uuid AND state IN ($3,$4)")
        .bind(source.game_session_id.as_bytes().as_slice()).bind(super::super::V2_STALE_TERMINAL)
        .bind(super::super::V2_PREPARED).bind(super::super::V2_COMMITTED).execute(&mut **tx).await?;
    let changed = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3,current_transport_ref=NULL,prepared_attempt_ref=NULL \
        WHERE game_session_id=encode($1,'hex')::uuid AND session_state IN (1,2)")
        .bind(source.game_session_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    if changed.rows_affected() != 1 {
        return Err(DurabilityError::Unavailable);
    }
    super::super::fresh_admission::commit_session_use(
        tx,
        character,
        destination,
        decision,
        membership,
    )
    .await?;
    let lease = source
        .character_lease_generation
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (kind, channel, instance) = match scope {
        RuntimeScopeRefV1::Channel { channel_id, .. } => {
            (1_i16, Some(*channel_id.as_bytes()), None)
        }
        RuntimeScopeRefV1::Instance { instance_id, .. } => (2_i16, None, Some(instance_id)),
    };
    let active = transport.is_some();
    sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,runtime_scope_instance_id,origin_channel_id,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state,control_loss_epoch,original_grace_deadline,predecessor_generation,initial_fresh_replay_key,activity_handoff_id,activity_return_id,activity_recovery_protection) \
        VALUES(encode($1,'hex')::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($4,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9::text::numeric,$10::text::numeric,1,$11,$12,$13::text::numeric,$14,$15::text::numeric,$16,encode($17,'hex')::uuid,encode($18,'hex')::uuid,$19)")
        .bind(destination.as_bytes().as_slice()).bind(&account).bind(source.character_id.as_bytes().as_slice()).bind(scope.world_id().as_bytes().as_slice())
        .bind(kind).bind(channel.as_ref().map(|v| v.as_slice())).bind(instance.as_ref().map(|v| v.as_slice())).bind(origin.as_ref().map(|v| v.as_slice()))
        .bind(lease.to_string()).bind(scope_generation.get().to_string()).bind(transport.as_deref()).bind(if active {2_i16} else {1_i16})
        .bind(if active {None} else {Some("1")}).bind(if active {None} else {Some(grace)}).bind(if active {None} else {Some("1")})
        .bind(initial.as_slice()).bind(if kind == 2 { Some(decision.as_slice()) } else { None })
        .bind(if kind == 1 { Some(decision.as_slice()) } else { None })
        .bind(protection)
        .execute(&mut **tx).await?;
    if let Some(transport) = transport {
        sqlx::query("INSERT INTO game_durability_transport_ref_reservations(transport_ref,game_session_id,reconnect_attempt_ref,reservation_owner,fresh_replay_key,activity_handoff_id,activity_return_id) VALUES($1,encode($2,'hex')::uuid,NULL,3,NULL,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(transport).bind(destination.as_bytes().as_slice())
            .bind(if kind == 2 { Some(decision.as_slice()) } else { None })
            .bind(if kind == 1 { Some(decision.as_slice()) } else { None }).execute(&mut **tx).await?;
    }
    super::super::fresh_admission_composition::transfer_activity_claims_in_transaction(
        tx,
        root,
        &account,
        character,
        (source.game_session_id, destination),
        lease,
        now.div_euclid(1000),
    )
    .await?;
    Ok(())
}

pub(super) async fn load_receipt(
    tx: &mut Transaction<'_, Postgres>,
    handoff: [u8; 16],
) -> Result<ActivityEnterReceipt> {
    let row = sqlx::query("SELECT uuid_send(g.world_id) AS world,uuid_send(g.instance_id) AS instance,g.ownership_generation::text AS generation,e.deadline \
        FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id) WHERE g.handoff_id=encode($1,'hex')::uuid AND g.state=2")
        .bind(handoff.as_slice()).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::InvalidStoredState)?;
    let world = crate::foundation::WorldId::decode(&bytes16(row.try_get("world")?)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let scope = RuntimeScopeRefV1::instance(world, bytes16(row.try_get("instance")?)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let generation = ScopeOwnershipGeneration::new(parse_u64(row.try_get("generation")?)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let rows = sqlx::query("SELECT uuid_send(p.character_id) AS character,uuid_send(p.source_game_session_id) AS source,uuid_send(p.destination_game_session_id) AS destination,p.source_lease_generation::text AS lease, \
        q.destination_x,q.destination_y,q.destination_floor,q.exit_x,q.exit_y,q.exit_floor,q.destination_transport_ref FROM game_activity_handoff_participants p \
        JOIN game_activity_runtime_placements q USING(handoff_id,character_id) WHERE p.handoff_id=encode($1,'hex')::uuid ORDER BY p.character_id LIMIT 16")
        .bind(handoff.as_slice()).fetch_all(&mut **tx).await?;
    if rows.is_empty() || rows.len() > ACTIVITY_PARTICIPANTS_MAX {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut participants = Vec::with_capacity(rows.len());
    for row in rows {
        participants.push(ActivityParticipantReceipt {
            character_id: CharacterId::decode(&bytes16(row.try_get("character")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            source_game_session_id: GameSessionId::decode(&bytes16(row.try_get("source")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            destination_game_session_id: GameSessionId::decode(&bytes16(
                row.try_get("destination")?,
            )?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
            destination_lease_generation: parse_u64(row.try_get("lease")?)?
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?,
            destination_transport: row
                .try_get::<Option<Vec<u8>>, _>("destination_transport_ref")?
                .map(|bytes| {
                    crate::foundation::AuthenticatedTransportRefV1::decode(&bytes)
                        .map_err(|_| DurabilityError::InvalidStoredState)
                })
                .transpose()?,
            destination_cell: LastPositionCell {
                x: row.try_get("destination_x")?,
                y: row.try_get("destination_y")?,
                floor: row.try_get("destination_floor")?,
            },
            exit_cell: LastPositionCell {
                x: row.try_get("exit_x")?,
                y: row.try_get("exit_y")?,
                floor: row.try_get("exit_floor")?,
            },
        });
    }
    Ok(ActivityEnterReceipt {
        handoff_id: handoff,
        destination_scope: scope,
        destination_generation: generation,
        deadline_unix_ms: row.try_get("deadline")?,
        participants,
    })
}
