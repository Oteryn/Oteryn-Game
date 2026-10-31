//! Crystal boss-lever policy, current catalogue and durable room lifecycle.
//! Crystal@201189f `boss_lever.lua`:151-158,183-218,243,266-280;
//! `boss_lever_death.lua`:21-32. Values and placements come from authored content.
use super::super::boss_encounter::cooldown_storage::{
    BossCooldownAdmissionData, BossCooldownChange, BossRewardKey, set_boss_cooldowns_data,
};
use super::*;
use oteryn_gameplay_encounter_rules::boss_room::{
    AdmissionRefusal, BossRoomPolicy, ParticipantFact, plan_fresh_room,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossRoomDefinition {
    pub boss_key: String,
    pub boss_creature_key: String,
    pub content_revision: String,
    pub room_template_key: String,
    pub template_revision: String,
    pub policy: BossRoomPolicy,
    /// In authored order: each source cell maps to the corresponding Instance entry.
    pub lever_positions: Vec<LastPositionCell>,
    pub entry_positions: Vec<LastPositionCell>,
    pub exit_position: LastPositionCell,
    pub boss_position: LastPositionCell,
    pub after_kill_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityReadyAllocation {
    pub scope: RuntimeScopeRefV1,
    pub generation: ScopeOwnershipGeneration,
    pub source_revision: u64,
    pub decision_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityDueRoom {
    pub handoff_id: [u8; 16],
    pub scope: RuntimeScopeRefV1,
    pub generation: ScopeOwnershipGeneration,
    pub reason: ActivityReturnReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityDueRoomPage {
    pub rooms: Vec<ActivityDueRoom>,
    /// Last inspected immutable admission id, including any row whose final
    /// fence refused. An unavailable room cannot hide a later due room.
    pub next_after: Option<[u8; 16]>,
}

pub(super) struct BossAdmissionPlan {
    definition: BossRoomDefinition,
    now: i64,
    reset_epoch: u64,
}

impl ActivityHandoffOwner {
    pub async fn read_boss_room(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        boss_key: &str,
    ) -> Result<Option<BossRoomDefinition>> {
        if boss_key.is_empty() || boss_key.len() > 512 {
            return Ok(None);
        }
        let key = boss_key.to_owned();
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let root = self.root.clone();
        let node = node.clone();
        self.root
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if retained_participant(&mut tx, &root, fence).await?.is_none()
                        || assert_item_only_fence(&mut tx, &fence, &node)
                            .await?
                            .is_err()
                    {
                        return Ok(None);
                    }
                    let result = read_definition(&mut tx, &key).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// A bounded page of fresh, already assigned AND independently ready scopes.
    /// The caller still needs the matching loaded native template and fixed capacity.
    /// Pagination bounds inspection without capping the control plane's allocations.
    pub async fn ready_allocations(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
        after_instance: Option<[u8; 16]>,
    ) -> Result<Vec<ActivityReadyAllocation>> {
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let root = self.root.clone();
        self.root.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            let history:bool=sqlx::query_scalar("SELECT game_runtime_scope_assignment_history_valid()").fetch_one(&mut *tx).await?;
            if !history || !prove_current_incarnation(&mut tx,&node).await? { return Err(DurabilityError::Unavailable); }
            let fact=node.fact();
            let rows=sqlx::query("SELECT uuid_send(a.activity_instance_id) AS instance,a.ownership_generation::text AS generation,a.source_revision::text AS revision,a.decision_identity \
                FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g ON g.scope_key='\\x02'::bytea || uuid_send(a.world_id) || uuid_send(a.activity_instance_id) \
                WHERE a.world_id=encode($1,'hex')::uuid AND a.scope_kind=3 AND a.state=1 AND a.holder_node_id=encode($2,'hex')::uuid AND a.holder_registration_revision=$3::text::numeric \
                  AND g.ready AND g.ownership_generation=a.ownership_generation AND ($4::bytea IS NULL OR a.activity_instance_id>encode($4,'hex')::uuid) \
                  AND NOT EXISTS(SELECT 1 FROM game_activity_handoff_groups h WHERE h.world_id=a.world_id AND h.instance_id=a.activity_instance_id) \
                  AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.runtime_scope_world_id=a.world_id AND s.runtime_scope_instance_id=a.activity_instance_id) \
                ORDER BY a.activity_instance_id LIMIT 15 FOR SHARE OF a,g")
                .bind(world.as_bytes().as_slice()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
                .bind(after_instance.as_ref().map(|id|id.as_slice())).fetch_all(&mut *tx).await?;
            let guards=AdmissionGuardStore::from_root(root);
            let mut result=Vec::with_capacity(rows.len());
            for row in rows {
                let scope=RuntimeScopeRefV1::instance(world,bytes16(row.try_get("instance")?)?).map_err(|_|DurabilityError::InvalidStoredState)?;
                if guards.load_locked(&mut tx,&AdmissionAuthorityGuardKeyV1::Runtime(scope)).await?.is_none() { continue; }
                result.push(ActivityReadyAllocation {scope,generation:ScopeOwnershipGeneration::new(parse_u64(row.try_get("generation")?)?).map_err(|_|DurabilityError::InvalidStoredState)?,
                    source_revision:parse_u64(row.try_get("revision")?)?,decision_identity:row.try_get("decision_identity")?});
            }
            commit_semantic_transaction(tx,deadline).await?;Ok(result)
        })).await
    }

    /// The autonomous native owner drains captured lethal settlements before this
    /// due-page drives returns. A per-room deadline is rechecked by the return writer.
    pub async fn due_boss_rooms(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
    ) -> Result<Vec<ActivityDueRoom>> {
        Ok(self
            .due_boss_rooms_after(authority, node, world, None)
            .await?
            .rooms)
    }

    /// Bounded traversal for the autonomous owner. Ownership is filtered before
    /// LIMIT, and the immutable-id cursor advances even past a blocked return.
    pub async fn due_boss_rooms_after(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
        after: Option<[u8; 16]>,
    ) -> Result<ActivityDueRoomPage> {
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            let now=now_ms(&mut tx).await?;
            if !prove_current_incarnation(&mut tx,&node).await? {return Err(DurabilityError::Unavailable);}
            let fact=node.fact();
            let rows=sqlx::query("SELECT uuid_send(a.admission) AS id,uuid_send(a.instance_id) AS instance,g.ownership_generation::text AS generation,a.state \
                FROM game_boss_room_activities a JOIN game_activity_handoff_groups g ON g.handoff_id=a.admission \
                JOIN game_runtime_scope_assignments ownership ON ownership.scope_key='\\x03'::bytea || uuid_send(g.world_id) || uuid_send(g.instance_id) \
                JOIN game_durability_admission_runtime_guards readiness ON readiness.scope_key='\\x02'::bytea || uuid_send(g.world_id) || uuid_send(g.instance_id) \
                WHERE a.world_id=encode($1,'hex')::uuid AND ((a.state=1 AND a.deadline <= $2) OR (a.state=2 AND a.exit_deadline <= $2) OR a.state=3) \
                  AND ownership.state=1 AND ownership.ownership_generation=g.ownership_generation \
                  AND ownership.holder_node_id=encode($3,'hex')::uuid AND ownership.holder_registration_revision=$4::text::numeric \
                  AND readiness.ready AND readiness.ownership_generation=g.ownership_generation \
                  AND ($5::bytea IS NULL OR a.admission>encode($5,'hex')::uuid) \
                ORDER BY a.admission LIMIT 15")
                .bind(world.as_bytes().as_slice()).bind(now).bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string()).bind(after.as_ref().map(|id|id.as_slice()))
                .fetch_all(&mut *tx).await?;
            let next_after=if rows.len()==15 {rows.last().map(|row|bytes16(row.try_get("id")?)).transpose()?} else {None};
            let mut result=Vec::with_capacity(rows.len());
            for row in rows {
                let scope=RuntimeScopeRefV1::instance(world,bytes16(row.try_get("instance")?)?).map_err(|_|DurabilityError::InvalidStoredState)?;
                let generation=ScopeOwnershipGeneration::new(parse_u64(row.try_get("generation")?)?).map_err(|_|DurabilityError::InvalidStoredState)?;
                if assert_runtime_scope_fence(&mut tx,scope,generation,&node).await? {
                    result.push(ActivityDueRoom {handoff_id:bytes16(row.try_get("id")?)?,scope,generation,
                        reason:if row.try_get::<i16,_>("state")? == 2 {ActivityReturnReason::Defeated}else{ActivityReturnReason::Timeout}});
                }
            }
            commit_semantic_transaction(tx,deadline).await?;Ok(ActivityDueRoomPage {rooms:result,next_after})
        })).await
    }

    pub async fn record_boss_defeat(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        boss_key: &str,
        death: [u8; 16],
        minted: i64,
    ) -> Result<bool> {
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let boss_key = boss_key.to_owned();
        self.root
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                        return Ok(false);
                    }
                    let result = record_defeat_in_transaction(
                        &mut tx, scope, generation, &boss_key, death, minted,
                    )
                    .await?;
                    if result {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await
    }
}

async fn read_definition(
    tx: &mut Transaction<'_, Postgres>,
    key: &str,
) -> Result<Option<BossRoomDefinition>> {
    let row=sqlx::query("SELECT boss_key,boss_creature_key,content_revision,room_template_key,template_revision,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,after_kill_seconds,lever_positions,entry_positions,exit_position,boss_position FROM game_boss_room_catalogue WHERE boss_key=$1 AND enabled FOR SHARE")
        .bind(key).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let Some(template_revision) = row.try_get::<Option<String>, _>("template_revision")? else {
        return Ok(None);
    };
    let Some(entry) = row.try_get::<Option<serde_json::Value>, _>("entry_positions")? else {
        return Ok(None);
    };
    let Some(exit) = row.try_get::<Option<serde_json::Value>, _>("exit_position")? else {
        return Ok(None);
    };
    let Some(boss) = row.try_get::<Option<serde_json::Value>, _>("boss_position")? else {
        return Ok(None);
    };
    let lever_positions = parse_cells(&row.try_get::<serde_json::Value, _>("lever_positions")?)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let entry_positions = parse_entry_cells(&entry).ok_or(DurabilityError::InvalidStoredState)?;
    let exit_position = parse_cell(&exit).ok_or(DurabilityError::InvalidStoredState)?;
    let boss_position = parse_cell(&boss).ok_or(DurabilityError::InvalidStoredState)?;
    let minimum_players = usize::try_from(row.try_get::<i16, _>("minimum_players")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let maximum_players = usize::try_from(row.try_get::<i16, _>("maximum_players")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if lever_positions.len() != entry_positions.len()
        || entry_positions.len() < maximum_players
        || !revision_valid(&template_revision)
    {
        return Ok(None);
    }
    let policy = BossRoomPolicy {
        minimum_players,
        maximum_players,
        minimum_level: u32::try_from(row.try_get::<i64, _>("minimum_level")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        cooldown_ms: milliseconds(row.try_get("cooldown_seconds")?)?,
        time_limit_ms: milliseconds(row.try_get("time_limit_seconds")?)?,
        lever_positions: lever_positions
            .iter()
            .map(|cell| (cell.x, cell.y, cell.floor))
            .collect(),
    };
    Ok(Some(BossRoomDefinition {
        boss_key: row.try_get("boss_key")?,
        boss_creature_key: row.try_get("boss_creature_key")?,
        content_revision: row.try_get("content_revision")?,
        room_template_key: row.try_get("room_template_key")?,
        template_revision,
        policy,
        lever_positions,
        entry_positions,
        exit_position,
        boss_position,
        after_kill_ms: milliseconds(row.try_get("after_kill_seconds")?)?,
    }))
}
fn milliseconds(seconds: i64) -> Result<u64> {
    u64::try_from(seconds)
        .ok()
        .and_then(|v| v.checked_mul(1000))
        .ok_or(DurabilityError::InvalidStoredState)
}
fn parse_cell(value: &serde_json::Value) -> Option<LastPositionCell> {
    Some(LastPositionCell {
        x: i32::try_from(value.get("x")?.as_i64()?).ok()?,
        y: i32::try_from(value.get("y")?.as_i64()?).ok()?,
        floor: i16::try_from(value.get("floor")?.as_i64()?).ok()?,
    })
}
fn parse_cells(value: &serde_json::Value) -> Option<Vec<LastPositionCell>> {
    let cells = parse_entry_cells(value)?;
    let distinct: std::collections::BTreeSet<_> =
        cells.iter().map(|c| (c.x, c.y, c.floor)).collect();
    (distinct.len() == cells.len()).then_some(cells)
}

fn parse_entry_cells(value: &serde_json::Value) -> Option<Vec<LastPositionCell>> {
    let rows = value.as_array()?;
    if rows.is_empty() || rows.len() > ACTIVITY_PARTICIPANTS_MAX {
        return None;
    }
    rows.iter().map(parse_cell).collect()
}

pub(super) async fn prepare_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    r: &ActivityEnterRequest,
    now: i64,
) -> Result<std::result::Result<BossAdmissionPlan, ActivityRefusal>> {
    let Some(definition) = read_definition(tx, &r.activity_key).await? else {
        return Ok(Err(ActivityRefusal::ContentUnavailable));
    };
    if definition.content_revision != r.content_revision
        || definition.template_revision != r.template_revision
        || definition.policy.time_limit_ms != r.time_limit_ms
    {
        return Ok(Err(ActivityRefusal::ContentChanged));
    }
    let mut facts = Vec::with_capacity(r.participants.len());
    for p in &r.participants {
        let Some(ordinal) = definition
            .lever_positions
            .iter()
            .position(|cell| *cell == p.source_position.cell)
        else {
            return Ok(Err(ActivityRefusal::Position));
        };
        if p.destination_cell != definition.entry_positions[ordinal]
            || p.exit_position.cell != definition.exit_position
        {
            return Ok(Err(ActivityRefusal::Position));
        }
        let level:Option<i64>=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid AND content_revision=$2 FOR SHARE")
            .bind(p.source.character_id.as_bytes().as_slice()).bind(&r.content_revision).fetch_optional(&mut **tx).await?;
        let Some(level) = level else {
            return Ok(Err(ActivityRefusal::AuthorityRejected));
        };
        let cooldown:Option<i64>=sqlx::query_scalar("SELECT available_at FROM game_character_boss_cooldowns WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2 FOR UPDATE")
            .bind(p.source.character_id.as_bytes().as_slice()).bind(&r.activity_key).fetch_optional(&mut **tx).await?;
        facts.push(ParticipantFact {
            character: *p.source.character_id.as_bytes(),
            position: (
                p.source_position.cell.x,
                p.source_position.cell.y,
                p.source_position.cell.floor,
            ),
            level: u32::try_from(level).map_err(|_| DurabilityError::InvalidStoredState)?,
            cooldown_until_ms: u64::try_from(cooldown.unwrap_or(0))
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        });
    }
    if let Err(reason) = plan_fresh_room(
        &definition.policy,
        &facts,
        u64::try_from(now).map_err(|_| DurabilityError::InvalidStoredState)?,
    ) {
        return Ok(Err(match reason {
            AdmissionRefusal::Count => ActivityRefusal::PartyCount,
            AdmissionRefusal::Level => ActivityRefusal::Level,
            AdmissionRefusal::Cooldown => ActivityRefusal::Cooldown,
            AdmissionRefusal::Position => ActivityRefusal::Position,
            AdmissionRefusal::Duplicate | AdmissionRefusal::InvalidPolicy => {
                ActivityRefusal::InvalidInput
            }
        }));
    }
    let reset:Option<String>=sqlx::query_scalar("SELECT a.reset_epoch::text FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid FOR SHARE OF a,e")
        .bind(r.destination_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(reset) = reset else {
        return Ok(Err(ActivityRefusal::AuthorityRejected));
    };
    Ok(Ok(BossAdmissionPlan {
        definition,
        now,
        reset_epoch: parse_u64(reset)?,
    }))
}

pub(super) async fn commit_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    r: &ActivityEnterRequest,
    plan: BossAdmissionPlan,
) -> Result<()> {
    let keys = r
        .participants
        .iter()
        .map(|p| BossRewardKey::new(p.source.character_id, &r.activity_key))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let result = set_boss_cooldowns_data(
        tx,
        &keys,
        &BossCooldownAdmissionData {
            admission: r.handoff_id,
            now: plan.now,
            reset_epoch: plan.reset_epoch,
            cooldown_s: plan.definition.policy.cooldown_ms / 1000,
            boosted: false,
        },
    )
    .await
    .map_err(|_| DurabilityError::Unavailable)?;
    if result != BossCooldownChange::Applied {
        return Err(DurabilityError::InvalidStoredState);
    }
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = r.destination_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let RuntimeScopeRefV1::Channel { channel_id, .. } = r.participants[0].source.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let deadline = plan
        .now
        .checked_add(
            i64::try_from(r.time_limit_ms).map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_boss_room_activities(admission,world_id,instance_id,boss_key,content_revision,origin_channel_id,state,opened_at,deadline,boss_generation,after_kill_ms,boss_creature_key) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,encode($6,'hex')::uuid,1,$7,$8,1,$9,$10)")
        .bind(r.handoff_id.as_slice()).bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).bind(&r.activity_key).bind(&r.content_revision).bind(channel_id.as_bytes().as_slice())
        .bind(plan.now).bind(deadline).bind(i64::try_from(plan.definition.after_kill_ms).map_err(|_|DurabilityError::InvalidStoredState)?).bind(&plan.definition.boss_creature_key).execute(&mut **tx).await?;
    for p in &r.participants {
        sqlx::query("INSERT INTO game_boss_room_participants(admission,character_id,origin_game_session_id,origin_lease_generation) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric)")
            .bind(r.handoff_id.as_slice()).bind(p.source.character_id.as_bytes().as_slice()).bind(p.source.game_session_id.as_bytes().as_slice()).bind(p.source.character_lease_generation.to_string()).execute(&mut **tx).await?;
    }
    Ok(())
}

/// The room's terminal classification is distinct from a retryable settlement
/// conflict. A captured late lethal must not keep timeout's queue drain pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BossRoomDefeatDecision {
    Recorded,
    Unrelated,
    Late,
    Conflict,
}

/// Joins the existing native death settlement transaction. W03 persists Late as
/// its immutable no-chest settlement; a nonreward creature still has its normal
/// death owner. Neither case creates a victorious room or starts its kill timer.
pub(crate) async fn classify_and_record_defeat_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    boss_key: &str,
    death: [u8; 16],
    minted: i64,
) -> Result<BossRoomDefeatDecision> {
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = scope
    else {
        return Ok(BossRoomDefeatDecision::Unrelated);
    };
    if death == [0; 16] || minted < 0 {
        return Ok(BossRoomDefeatDecision::Conflict);
    }
    let row=sqlx::query("SELECT a.admission,a.state,a.opened_at,a.deadline,a.after_kill_ms,uuid_send(a.defeat_id) AS defeat_id,a.terminal_at FROM game_boss_room_activities a \
        JOIN game_activity_handoff_groups g ON g.handoff_id=a.admission WHERE a.world_id=encode($1,'hex')::uuid AND a.instance_id=encode($2,'hex')::uuid \
          AND g.ownership_generation=$3::text::numeric AND g.state=2 AND (a.boss_creature_key=$4 OR a.boss_key=$4) FOR UPDATE OF a")
        .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).bind(generation.get().to_string()).bind(boss_key).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(BossRoomDefeatDecision::Unrelated);
    };
    if minted < row.try_get::<i64, _>("opened_at")? || minted > now_ms(tx).await? {
        return Ok(BossRoomDefeatDecision::Conflict);
    }
    let state: i16 = row.try_get("state")?;
    let recorded: Option<Vec<u8>> = row.try_get("defeat_id")?;
    if matches!(state, 2 | 4)
        && recorded.as_deref() == Some(death.as_slice())
        && row.try_get::<Option<i64>, _>("terminal_at")? == Some(minted)
    {
        return Ok(BossRoomDefeatDecision::Recorded);
    }
    if !matches!(state, 1 | 3) || recorded.is_some() {
        return Ok(BossRoomDefeatDecision::Conflict);
    }
    if minted > row.try_get::<i64, _>("deadline")? {
        return Ok(BossRoomDefeatDecision::Late);
    }
    if state != 1 {
        return Ok(BossRoomDefeatDecision::Conflict);
    }
    // Crystal boss_lever_death.lua:26 cancels the fight timeout but creates
    // no after-kill timer when the authored value is zero.
    let after_kill_ms: i64 = row.try_get("after_kill_ms")?;
    let exit = if after_kill_ms == 0 {
        None
    } else {
        Some(
            minted
                .checked_add(after_kill_ms)
                .ok_or(DurabilityError::InvalidStoredState)?,
        )
    };
    sqlx::query("UPDATE game_boss_room_activities SET state=2,terminal_at=$3,exit_deadline=$4,defeat_id=encode($5,'hex')::uuid WHERE world_id=encode($1,'hex')::uuid AND instance_id=encode($2,'hex')::uuid AND state=1")
        .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).bind(minted).bind(exit).bind(death.as_slice()).execute(&mut **tx).await?;
    Ok(BossRoomDefeatDecision::Recorded)
}

/// Compatibility for the existing nonreward death owner: a deterministic late
/// room kill is handled, while uncertain/conflicting authority remains refused.
pub(crate) async fn record_defeat_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    boss_key: &str,
    death: [u8; 16],
    minted: i64,
) -> Result<bool> {
    Ok(!matches!(
        classify_and_record_defeat_in_transaction(tx, scope, generation, boss_key, death, minted)
            .await?,
        BossRoomDefeatDecision::Conflict
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_cells_are_bounded_exact_and_never_defaulted() {
        assert!(parse_cells(&serde_json::json!([])).is_none());
        assert!(parse_cell(&serde_json::json!({"x":1,"y":2})).is_none());
        assert!(
            parse_cells(&serde_json::json!([{"x":1,"y":2,"floor":7},{"x":1,"y":2,"floor":7}]))
                .is_none()
        );
        assert_eq!(
            parse_entry_cells(
                &serde_json::json!([{"x":1,"y":2,"floor":7},{"x":1,"y":2,"floor":7}])
            )
            .map(|cells| cells.len()),
            Some(2)
        );
        assert_eq!(
            parse_cell(&serde_json::json!({"x":1,"y":2,"floor":7})),
            Some(LastPositionCell {
                x: 1,
                y: 2,
                floor: 7
            })
        );
        assert!(parse_cell(&serde_json::json!({"x":2147483648_i64,"y":2,"floor":7})).is_none());
    }
}
