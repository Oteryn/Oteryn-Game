use super::super::character_last_position::{
    LastPositionKey, LastPositionWrite, LastVitals, upsert_last_position,
};
use super::*;

/// Immutable return facts used to reconcile retries and retained death obligations.
struct PreviousReturn {
    request_binding: Vec<u8>,
    command_id: Option<String>,
    reason: i16,
    death_occurrence: Option<Vec<u8>>,
}

impl PreviousReturn {
    fn decode(row: &sqlx::postgres::PgRow) -> Result<Self> {
        Ok(Self {
            request_binding: row.try_get(0)?,
            command_id: row.try_get(1)?,
            reason: row.try_get(2)?,
            death_occurrence: row.try_get(3)?,
        })
    }
}

fn binding(r: &ActivityReturnRequest) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash_field(&mut hash, b"native-activity-return-v1");
    hash_field(&mut hash, &r.handoff_id);
    hash_field(&mut hash, r.source.character_id.as_bytes());
    hash_field(&mut hash, r.source.game_session_id.as_bytes());
    hash_field(
        &mut hash,
        &r.source.connection_generation.get().to_be_bytes(),
    );
    hash_field(
        &mut hash,
        &r.source.character_lease_generation.to_be_bytes(),
    );
    hash_field(&mut hash, &assignment_scope_key(r.source.runtime_scope));
    hash_field(
        &mut hash,
        &r.source.scope_ownership_generation.get().to_be_bytes(),
    );
    hash_field(&mut hash, &r.reason.stored().to_be_bytes());
    if let ActivityReturnReason::Death { occurrence } = r.reason {
        hash_field(&mut hash, &occurrence);
    }
    match r.reason {
        ActivityReturnReason::Abandoned { transport } => {
            hash_field(&mut hash, &transport.to_bytes())
        }
        ActivityReturnReason::CapabilityMismatch { epoch } => {
            hash_field(&mut hash, &epoch.get().to_be_bytes())
        }
        _ => {}
    }
    hash_field(&mut hash, &[u8::from(r.command_id.is_some())]);
    if r.command_id.is_none() {
        hash_field(&mut hash, &r.return_id);
    }
    hash.finalize().into()
}

pub(super) async fn return_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    r: &ActivityReturnRequest,
    recovering: bool,
) -> Result<ActivityReturnOutcome> {
    let character = foundation_character(r.source)?;
    let row = sqlx::query("SELECT uuid_send(p.origin_channel_id) AS origin, p.account_id::text, g.content_revision,g.ownership_generation::text AS generation, \
        e.deadline,e.holder_node_id::text AS original_node,e.holder_registration_revision::text AS original_registration, \
        q.exit_x,q.exit_y,q.exit_floor,q.exit_coordinate_frame_digest,q.exit_map_revision_digest,q.exit_server_artifact_digest,q.saved_health,q.saved_mana,q.saved_rest \
        FROM game_activity_handoff_groups g JOIN game_activity_handoff_participants p USING(handoff_id) \
        JOIN game_activity_runtime_entries e USING(handoff_id) JOIN game_activity_runtime_placements q USING(handoff_id,character_id) \
        WHERE g.handoff_id=encode($1,'hex')::uuid AND p.character_id=encode($2,'hex')::uuid AND p.destination_game_session_id=encode($3,'hex')::uuid \
          AND g.state=2 AND g.world_id=encode($4,'hex')::uuid AND g.instance_id=encode($5,'hex')::uuid \
          AND g.ownership_generation=$6::text::numeric FOR SHARE OF g,p,e,q")
        .bind(r.handoff_id.as_slice()).bind(r.source.character_id.as_bytes().as_slice()).bind(r.source.game_session_id.as_bytes().as_slice())
        .bind(r.source.runtime_scope.world_id().as_bytes().as_slice())
        .bind(match r.source.runtime_scope { RuntimeScopeRefV1::Instance { instance_id, .. } => instance_id, _ => return Err(DurabilityError::InvalidStoredState) }.as_slice())
        .bind(r.source.scope_ownership_generation.get().to_string()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(ActivityReturnOutcome::Refused(
            ActivityRefusal::AuthorityRejected,
        ));
    };
    sqlx::query("SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(r.source.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    sqlx::query(
        "SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(r.source.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let previous = sqlx::query("SELECT request_binding,command_id::text,reason,uuid_send(death_occurrence_id) FROM game_activity_runtime_returns WHERE handoff_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
        .bind(r.handoff_id.as_slice()).bind(r.source.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let previous = previous.as_ref().map(PreviousReturn::decode).transpose()?;
    // Recovery must retain a death obligation. Replays use the immutable
    // return reason even after the destination consumed its pending respawn.
    let mut effective = r.clone();
    if recovering {
        let death = match previous.as_ref() {
            Some(PreviousReturn {
                reason: 5,
                death_occurrence: Some(occurrence),
                ..
            }) => Some(bytes16(occurrence.clone())?),
            Some(_) => None,
            None => super::departed_death::pending_death_occurrence(tx, r.source).await?,
        };
        effective.reason = death.map_or(ActivityReturnReason::OwnerLost, |occurrence| {
            ActivityReturnReason::Death { occurrence }
        });
        effective.command_id = None;
    }
    let r = &effective;
    let request_binding = binding(r);
    if let Some(PreviousReturn {
        request_binding: old_binding,
        command_id: command,
        ..
    }) = previous
    {
        if old_binding != request_binding || command.map(parse_u64).transpose()? != r.command_id {
            return Ok(ActivityReturnOutcome::Refused(
                ActivityRefusal::ReplayConflict,
            ));
        }
        if !prove_current_incarnation(tx, node).await?
            || !FreshAdmissionStore::from_root(root.clone())
                .committed_session_use(tx, character, r.source.game_session_id)
                .await?
        {
            return Ok(ActivityReturnOutcome::Refused(
                ActivityRefusal::AuthorityRejected,
            ));
        }
        return Ok(ActivityReturnOutcome::Replayed(
            load_receipt(tx, r.handoff_id, character).await?,
        ));
    }
    let origin = crate::foundation::ChannelId::decode(&bytes16(row.try_get("origin")?)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let destination_scope = RuntimeScopeRefV1::channel(r.source.runtime_scope.world_id(), origin);
    let guards = AdmissionGuardStore::from_root(root.clone());
    if guards
        .load_locked(
            tx,
            &AdmissionAuthorityGuardKeyV1::Runtime(destination_scope),
        )
        .await?
        .is_none()
        || !assert_runtime_scope_fence(tx, destination_scope, r.destination_generation, node)
            .await?
    {
        return Ok(ActivityReturnOutcome::Refused(ActivityRefusal::NoCapacity));
    }
    if recovering {
        if !recovery_source(
            tx,
            root,
            node,
            r.source,
            &row.try_get::<String, _>("account_id")?,
            &row.try_get::<String, _>("content_revision")?,
        )
        .await?
        {
            return Ok(ActivityReturnOutcome::Refused(
                ActivityRefusal::OwnerStillCurrent,
            ));
        }
    } else {
        if retained_participant(tx, root, r.source).await?.is_none()
            || assert_item_only_fence(tx, &r.source, node).await?.is_err()
        {
            return Ok(ActivityReturnOutcome::Refused(
                ActivityRefusal::AuthorityRejected,
            ));
        }
        let active: bool = sqlx::query_scalar("SELECT current_transport_ref IS NOT NULL FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(r.source.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if active != r.destination_transport.is_some() {
            return Ok(ActivityReturnOutcome::Refused(
                ActivityRefusal::InvalidInput,
            ));
        }
    }
    let now = now_ms(tx).await?;
    let due = match r.reason {
        ActivityReturnReason::Exit | ActivityReturnReason::OwnerLost | ActivityReturnReason::Death { .. } => true,
        ActivityReturnReason::Abandoned { transport } => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions \
            WHERE game_session_id=encode($1,'hex')::uuid AND session_state=2 AND current_transport_ref=$2)")
            .bind(r.source.game_session_id.as_bytes().as_slice()).bind(transport.to_bytes().as_slice()).fetch_one(&mut **tx).await?,
        ActivityReturnReason::CapabilityMismatch { epoch } => {
            let held: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions \
                WHERE game_session_id=encode($1,'hex')::uuid AND session_state=1 AND current_transport_ref IS NULL \
                  AND control_loss_epoch=$2::text::numeric AND original_grace_deadline>$3)")
                .bind(r.source.game_session_id.as_bytes().as_slice()).bind(epoch.get().to_string())
                .bind(now.div_euclid(1000)).fetch_one(&mut **tx).await?;
            held && super::super::fresh_admission::has_owning_loss_receipt(tx,
                r.source.game_session_id.as_bytes().as_slice(), epoch.get()).await?
        },
        // Idle timing belongs to the existing native input owner, just as for
        // TerminalRelease::Logout. This separate cause still independently
        // proves current active control; it never widens recorded-loss logout.
        ActivityReturnReason::IdleLogout => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions \
            WHERE game_session_id=encode($1,'hex')::uuid AND session_state=2 AND current_transport_ref IS NOT NULL)")
            .bind(r.source.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?,
        ActivityReturnReason::Logout => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s \
            WHERE s.game_session_id=encode($1,'hex')::uuid AND (($2 AND s.session_state=2 AND s.current_transport_ref IS NOT NULL) \
             OR (NOT $2 AND s.session_state=1 AND s.current_transport_ref IS NULL AND s.control_loss_epoch IS NOT NULL \
              AND s.original_grace_deadline IS NOT NULL AND s.original_grace_deadline <= $3)))")
            .bind(r.source.game_session_id.as_bytes().as_slice()).bind(r.command_id.is_some()).bind(now.div_euclid(1000)).fetch_one(&mut **tx).await?,
        ActivityReturnReason::Timeout => {
            let boss_state: Option<i16> = sqlx::query_scalar("SELECT state FROM game_boss_room_activities WHERE admission=encode($1,'hex')::uuid FOR UPDATE")
                .bind(r.handoff_id.as_slice()).fetch_optional(&mut **tx).await?;
            now >= row.try_get::<i64,_>("deadline")? && !matches!(boss_state, Some(2 | 4))
        },
        ActivityReturnReason::Defeated => sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_boss_room_activities WHERE admission=encode($1,'hex')::uuid AND state=2 AND exit_deadline <= $2)")
            .bind(r.handoff_id.as_slice()).bind(now).fetch_one(&mut **tx).await?,
    };
    if !due {
        return Ok(ActivityReturnOutcome::Refused(ActivityRefusal::NotDue));
    }
    let mut exit = LastPositionRecord {
        world_id: destination_scope.world_id(),
        cell: LastPositionCell {
            x: row.try_get("exit_x")?,
            y: row.try_get("exit_y")?,
            floor: row.try_get("exit_floor")?,
        },
        coordinate_frame_digest: bytes32(row.try_get("exit_coordinate_frame_digest")?)?,
        map_revision_digest: bytes32(row.try_get("exit_map_revision_digest")?)?,
        server_artifact_digest: bytes32(row.try_get("exit_server_artifact_digest")?)?,
        vitals: r.current_vitals,
        rest: r.current_rest,
    };
    if recovering {
        exit.vitals = match (
            row.try_get::<Option<i32>, _>("saved_health")?,
            row.try_get::<Option<i32>, _>("saved_mana")?,
        ) {
            (Some(health), Some(mana)) => Some(
                LastVitals::new(
                    u32::try_from(health).map_err(|_| DurabilityError::InvalidStoredState)?,
                    u32::try_from(mana).map_err(|_| DurabilityError::InvalidStoredState)?,
                )
                .ok_or(DurabilityError::InvalidStoredState)?,
            ),
            (None, None) => None,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        exit.rest = row
            .try_get::<Option<Vec<i32>>, _>("saved_rest")?
            .map(|values| {
                let converted = values
                    .into_iter()
                    .map(u32::try_from)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                converted
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)
            })
            .transpose()?;
    }
    if let ActivityReturnReason::Death { occurrence } = r.reason {
        let Some(cell) = death_return_cell(tx, r.source, occurrence).await? else {
            return Ok(ActivityReturnOutcome::Refused(
                ActivityRefusal::AuthorityRejected,
            ));
        };
        exit.cell = cell;
        // Death owns post-loss vitals and the pending obligation. A handoff may
        // neither heal the actor nor replace the obligation with a live save.
        exit.vitals = None;
        exit.rest = None;
    }
    let digest: Option<Vec<u8>> = sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(destination_scope.world_id().as_bytes().as_slice()).bind(origin.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    if digest.as_deref() != Some(exit.server_artifact_digest.as_slice()) {
        return Ok(ActivityReturnOutcome::Refused(
            ActivityRefusal::ContentChanged,
        ));
    }
    let next_lease = r
        .source
        .character_lease_generation
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if !matches!(r.reason, ActivityReturnReason::Death { .. })
        && upsert_last_position(
            tx,
            r.source.character_id.as_bytes(),
            LastPositionKey {
                character_lease_generation: next_lease,
                write_sequence: 1,
            },
            &exit,
        )
        .await?
            != LastPositionWrite::Written
    {
        return Ok(ActivityReturnOutcome::Refused(
            ActivityRefusal::AuthorityRejected,
        ));
    }
    // This is the same issuance/transport/publisher operation as enter, in reverse.
    super::entry::issue_successor(
        tx,
        root,
        super::entry::SuccessorSession {
            source: r.source,
            destination: r.destination_game_session_id,
            decision: r.return_id,
            scope: destination_scope,
            scope_generation: r.destination_generation,
            origin: None,
            transport: r.destination_transport,
        },
        now,
    )
    .await?;
    sqlx::query("INSERT INTO game_activity_runtime_returns(return_id,handoff_id,character_id,source_game_session_id,source_connection_generation,source_lease_generation,command_id,request_binding,destination_game_session_id,destination_ownership_generation,returned_at,reason,destination_transport_ref,death_occurrence_id,return_x,return_y,return_floor,return_coordinate_frame_digest,return_map_revision_digest,return_server_artifact_digest,release_transport_ref,release_loss_epoch) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::text::numeric,$7::text::numeric,$8,encode($9,'hex')::uuid,$10::text::numeric,$11,$12,$13,encode($14,'hex')::uuid,$15,$16,$17,$18,$19,$20,$21,$22::text::numeric)")
        .bind(r.return_id.as_slice()).bind(r.handoff_id.as_slice()).bind(r.source.character_id.as_bytes().as_slice()).bind(r.source.game_session_id.as_bytes().as_slice())
        .bind(r.source.connection_generation.get().to_string()).bind(r.source.character_lease_generation.to_string()).bind(r.command_id.map(|id|id.to_string())).bind(request_binding.as_slice())
        .bind(r.destination_game_session_id.as_bytes().as_slice()).bind(r.destination_generation.get().to_string()).bind(now).bind(r.reason.stored())
        .bind(r.destination_transport.map(|t|t.to_bytes().to_vec()))
        .bind(match r.reason { ActivityReturnReason::Death { occurrence } => Some(occurrence.to_vec()), _ => None })
        .bind(exit.cell.x).bind(exit.cell.y).bind(exit.cell.floor).bind(exit.coordinate_frame_digest.as_slice())
        .bind(exit.map_revision_digest.as_slice()).bind(exit.server_artifact_digest.as_slice())
        .bind(match r.reason { ActivityReturnReason::Abandoned { transport } => Some(transport.to_bytes().to_vec()), _ => None })
        .bind(match r.reason { ActivityReturnReason::CapabilityMismatch { epoch } => Some(epoch.get().to_string()), _ => None })
        .execute(&mut **tx).await?;
    sqlx::query("UPDATE game_boss_room_participants SET returned_at=$3 WHERE admission=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND returned_at IS NULL")
        .bind(r.handoff_id.as_slice()).bind(r.source.character_id.as_bytes().as_slice()).bind(now).execute(&mut **tx).await?;
    if r.reason == ActivityReturnReason::Timeout {
        sqlx::query("UPDATE game_boss_room_activities SET state=3,terminal_at=$2,exit_deadline=$2 WHERE admission=encode($1,'hex')::uuid AND state=1 AND deadline <= $2")
            .bind(r.handoff_id.as_slice()).bind(now).execute(&mut **tx).await?;
    }
    // Retirement is monotone and happens only after every participant's retained
    // return exists. Other members cannot be forgotten by the first exit.
    sqlx::query("UPDATE game_boss_room_activities a SET state=4,terminal_at=COALESCE(terminal_at,$2),exit_deadline=COALESCE(exit_deadline,$2) \
        WHERE admission=encode($1,'hex')::uuid AND NOT EXISTS (SELECT 1 FROM game_activity_handoff_participants p \
            WHERE p.handoff_id=a.admission AND NOT EXISTS (SELECT 1 FROM game_activity_runtime_returns r WHERE r.handoff_id=p.handoff_id AND r.character_id=p.character_id))")
        .bind(r.handoff_id.as_slice()).bind(now).execute(&mut **tx).await?;
    Ok(ActivityReturnOutcome::Returned(
        load_receipt(tx, r.handoff_id, character).await?,
    ))
}

pub(super) async fn recovery_source(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    source: CurrentCharacterItemFence,
    account: &str,
    content: &str,
) -> Result<bool> {
    let character = foundation_character(source)?;
    if !prove_current_incarnation(tx, node).await?
        || !FreshAdmissionStore::from_root(root.clone())
            .committed_session_use(tx, character, source.game_session_id)
            .await?
    {
        return Ok(false);
    }
    if !departed_scope(tx, source.runtime_scope, source.scope_ownership_generation).await? {
        return Ok(false);
    }
    let guards = AdmissionGuardStore::from_root(root.clone());
    for key in [
        AdmissionAuthorityGuardKeyV1::Character(character),
        AdmissionAuthorityGuardKeyV1::Account {
            account_id: account.to_owned(),
        },
    ] {
        if guards.load_locked(tx, &key).await?.is_none() {
            return Ok(false);
        }
    }
    let exact:bool=sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions s \
        JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id \
        JOIN game_durability_admission_account_guards a ON a.account_id=s.account_id \
        JOIN game_character_roots r ON r.character_id=s.character_id \
        WHERE s.game_session_id=encode($1,'hex')::uuid AND s.character_id=encode($2,'hex')::uuid AND s.account_id=$3::uuid \
          AND s.current_generation=$4::text::numeric AND s.character_lease_generation=$5::text::numeric AND s.session_state IN (1,2) \
          AND c.eligible AND c.holder_game_session_id=s.game_session_id AND c.lease_generation=s.character_lease_generation \
          AND a.presence_character_id=s.character_id AND a.holder_game_session_id=s.game_session_id \
          AND r.world_id=s.world_id AND r.account_id=s.account_id AND r.content_revision=$6 \
          AND s.runtime_scope_kind=2 AND s.runtime_scope_world_id=encode($7,'hex')::uuid \
          AND s.runtime_scope_instance_id=encode($8,'hex')::uuid AND s.scope_ownership_generation=$9::text::numeric)")
        .bind(source.game_session_id.as_bytes().as_slice()).bind(source.character_id.as_bytes().as_slice()).bind(account)
        .bind(source.connection_generation.get().to_string()).bind(source.character_lease_generation.to_string()).bind(content).bind(source.runtime_scope.world_id().as_bytes().as_slice())
        .bind(source.runtime_scope.instance_id().map(|id|id.to_vec())).bind(source.scope_ownership_generation.get().to_string()).fetch_one(&mut **tx).await?;
    Ok(exact)
}

/// Independent control history, not absence from a local native directory.
/// Shared by the live-source recovery and its already committed offline return.
pub(super) async fn departed_scope(
    tx: &mut Transaction<'_, Postgres>,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
) -> Result<bool> {
    let history: bool = sqlx::query_scalar("SELECT game_runtime_scope_assignment_history_valid()")
        .fetch_one(&mut **tx)
        .await?;
    if !history {
        return Ok(false);
    }
    let lost: Option<bool> = sqlx::query_scalar("SELECT a.state<>1 OR a.ownership_generation<>$2::text::numeric OR NOT game_node_lock_current_registration(a.holder_node_id,a.holder_registration_revision) FROM game_runtime_scope_assignments a WHERE a.scope_key=$1 FOR SHARE")
        .bind(assignment_scope_key(scope).as_slice()).bind(generation.get().to_string())
        .fetch_optional(&mut **tx).await?;
    Ok(lost == Some(true))
}

pub(super) async fn load_receipt(
    tx: &mut Transaction<'_, Postgres>,
    handoff: [u8; 16],
    character: CharacterId,
) -> Result<ActivityReturnReceipt> {
    let row=sqlx::query("SELECT uuid_send(r.return_id) AS id,uuid_send(r.source_game_session_id) AS source,uuid_send(r.destination_game_session_id) AS destination, \
        uuid_send(p.world_id) AS world,uuid_send(p.origin_channel_id) AS channel,r.destination_ownership_generation::text AS generation,r.source_lease_generation::text AS lease,r.destination_transport_ref,r.return_x AS exit_x,r.return_y AS exit_y,r.return_floor AS exit_floor \
        FROM game_activity_runtime_returns r JOIN game_activity_handoff_participants p USING(handoff_id,character_id) \
        JOIN game_activity_runtime_placements q USING(handoff_id,character_id) WHERE r.handoff_id=encode($1,'hex')::uuid AND r.character_id=encode($2,'hex')::uuid")
        .bind(handoff.as_slice()).bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let world = crate::foundation::WorldId::decode(&bytes16(row.try_get("world")?)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let channel = crate::foundation::ChannelId::decode(&bytes16(row.try_get("channel")?)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(ActivityReturnReceipt {
        return_id: bytes16(row.try_get("id")?)?,
        handoff_id: handoff,
        character_id: character,
        source_game_session_id: GameSessionId::decode(&bytes16(row.try_get("source")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        destination_game_session_id: GameSessionId::decode(&bytes16(row.try_get("destination")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        destination_scope: RuntimeScopeRefV1::channel(world, channel),
        destination_generation: ScopeOwnershipGeneration::new(parse_u64(
            row.try_get("generation")?,
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
        exit_cell: LastPositionCell {
            x: row.try_get("exit_x")?,
            y: row.try_get("exit_y")?,
            floor: row.try_get("exit_floor")?,
        },
    })
}

/// The authored return placement needed to reserve a real Channel cell before SQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityReturnPlacement {
    pub handoff_id: [u8; 16],
    /// Retained participant account, independently checked under the current fence.
    pub account_id: [u8; 16],
    pub destination_scope: RuntimeScopeRefV1,
    pub destination_generation: ScopeOwnershipGeneration,
    pub exit_position: LastPositionRecord,
    /// Only a retained boss-room record authorizes its donor forced exit.
    pub boss_room: bool,
}

impl ActivityHandoffOwner {
    pub async fn read_return_placement(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        source: CurrentCharacterItemFence,
    ) -> Result<Option<ActivityReturnPlacement>> {
        self.read_return_placement_inner(authority, node, source, None)
            .await
    }

    pub async fn read_death_return_placement(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        source: CurrentCharacterItemFence,
        occurrence: [u8; 16],
    ) -> Result<Option<ActivityReturnPlacement>> {
        if !uuid_v7(&occurrence) {
            return Ok(None);
        }
        self.read_return_placement_inner(authority, node, source, Some(occurrence))
            .await
    }

    async fn read_return_placement_inner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        source: CurrentCharacterItemFence,
        death: Option<[u8; 16]>,
    ) -> Result<Option<ActivityReturnPlacement>> {
        let RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } = source.runtime_scope
        else {
            return Ok(None);
        };
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let root = self.root.clone();
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            if retained_participant(&mut tx,&root,source).await?.is_none() || assert_item_only_fence(&mut tx,&source,&node).await?.is_err(){return Ok(None);}
            let row=sqlx::query("SELECT uuid_send(g.handoff_id) AS handoff,uuid_send(p.account_id) AS account,uuid_send(p.origin_channel_id) AS origin,a.ownership_generation::text AS generation, \
                q.exit_x,q.exit_y,q.exit_floor,q.exit_coordinate_frame_digest,q.exit_map_revision_digest,q.exit_server_artifact_digest, \
                EXISTS(SELECT 1 FROM game_boss_room_activities b WHERE b.admission=g.handoff_id) AS boss_room \
                FROM game_activity_handoff_groups g JOIN game_activity_handoff_participants p USING(handoff_id) JOIN game_activity_runtime_placements q USING(handoff_id,character_id) \
                JOIN game_runtime_scope_assignments a ON a.scope_key='\\x01'::bytea || uuid_send(p.world_id) || uuid_send(p.origin_channel_id) \
                WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid AND g.ownership_generation=$3::text::numeric AND g.state=2 \
                  AND p.character_id=encode($4,'hex')::uuid AND p.destination_game_session_id=encode($5,'hex')::uuid AND a.state=1 \
                  AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE (r.handoff_id,r.character_id)=(p.handoff_id,p.character_id)) \
                FOR SHARE OF g,p,q,a")
                .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).bind(source.scope_ownership_generation.get().to_string())
                .bind(source.character_id.as_bytes().as_slice()).bind(source.game_session_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(row)=row else{return Ok(None);};
            let channel=crate::foundation::ChannelId::decode(&bytes16(row.try_get("origin")?)?).map_err(|_|DurabilityError::InvalidStoredState)?;
            let destination_scope=RuntimeScopeRefV1::channel(world_id,channel);
            let destination_generation=ScopeOwnershipGeneration::new(parse_u64(row.try_get("generation")?)?).map_err(|_|DurabilityError::InvalidStoredState)?;
            if !assert_runtime_scope_fence(&mut tx,destination_scope,destination_generation,&node).await? {return Ok(None);}
            let mut result=ActivityReturnPlacement{handoff_id:bytes16(row.try_get("handoff")?)?,account_id:bytes16(row.try_get("account")?)?,destination_scope,destination_generation,boss_room:row.try_get("boss_room")?,
                exit_position:LastPositionRecord{world_id,cell:LastPositionCell{x:row.try_get("exit_x")?,y:row.try_get("exit_y")?,floor:row.try_get("exit_floor")?},
                    coordinate_frame_digest:bytes32(row.try_get("exit_coordinate_frame_digest")?)?,map_revision_digest:bytes32(row.try_get("exit_map_revision_digest")?)?,
                    server_artifact_digest:bytes32(row.try_get("exit_server_artifact_digest")?)?,vitals:None,rest:None}};
            if let Some(occurrence)=death {
                let Some(cell)=death_return_cell(&mut tx,source,occurrence).await? else{return Ok(None);};
                result.exit_position.cell=cell;
                result.boss_room=false;
            }
            commit_semantic_transaction(tx,deadline).await?;Ok(Some(result))
        })).await
    }
}

/// The existing death receipt and pending row own this placement; the return
/// caller supplies neither a temple nor loss/item-completion authority.
async fn death_return_cell(
    tx: &mut Transaction<'_, Postgres>,
    source: CurrentCharacterItemFence,
    occurrence: [u8; 16],
) -> Result<Option<LastPositionCell>> {
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = source.runtime_scope
    else {
        return Ok(None);
    };
    let bytes:Option<Vec<u8>>=sqlx::query_scalar("SELECT p.respawn_position FROM game_character_pending_respawns p \
        JOIN game_character_death_receipts d ON (d.death_occurrence_id,d.character_id,d.respawn_position)=(p.death_occurrence_id,p.character_id,p.respawn_position) \
        WHERE p.character_id=encode($1,'hex')::uuid AND p.death_occurrence_id=encode($2,'hex')::uuid \
          AND d.death_scope_kind=2 AND d.death_world_id=encode($3,'hex')::uuid AND d.death_instance_id=encode($4,'hex')::uuid \
          AND d.death_scope_generation=$5::text::numeric AND game_player_death_items_complete(p.death_occurrence_id) FOR SHARE OF p,d")
        .bind(source.character_id.as_bytes().as_slice()).bind(occurrence.as_slice()).bind(world_id.as_bytes().as_slice())
        .bind(instance_id.as_slice()).bind(source.scope_ownership_generation.get().to_string()).fetch_optional(&mut **tx).await?;
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let Ok(bytes) = <[u8; 10]>::try_from(bytes) else {
        return Ok(None);
    };
    Ok(Some(LastPositionCell {
        x: i32::from_be_bytes(
            bytes[..4]
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        ),
        y: i32::from_be_bytes(
            bytes[4..8]
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        ),
        floor: i16::from_be_bytes(
            bytes[8..]
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        ),
    }))
}
