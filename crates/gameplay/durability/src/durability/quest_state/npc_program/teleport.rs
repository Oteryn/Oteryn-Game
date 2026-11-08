//! Source NPC teleport composition over the existing pending-arrival owner.
use super::*;
use crate::durability::character_last_position::LastPositionCell;
use crate::durability::npc_travel::PendingArrival;
use oteryn_gameplay_quests::quest::npc_rules::NpcRulePosition;
use oteryn_gameplay_quests::quest::npc_rules::teleport::NpcTeleportPlan;

/// Map-owner admitted destination. Locked source evaluation must independently match it.
#[derive(Clone, Copy, Debug)]
pub struct NpcTeleportAdmission {
    pub arrival: PendingArrival,
}
impl NpcTeleportAdmission {
    /// Quest data stores native floor `-z`; movement and arrival records use legacy `z`.
    pub fn runtime_destination(position: NpcRulePosition) -> Option<LastPositionCell> {
        if !(0..=65535).contains(&position.x)
            || !(0..=65535).contains(&position.y)
            || !(-15..=0).contains(&position.floor)
        {
            return None;
        }
        Some(LastPositionCell {
            x: position.x,
            y: position.y,
            floor: -position.floor,
        })
    }

    pub(super) fn matches(
        self,
        fence: CurrentCharacterGameplayFence,
        plan: &NpcTeleportPlan,
    ) -> bool {
        self.arrival.world_id == fence.runtime_scope.world_id()
            && Self::runtime_destination(plan.destination) == Some(self.arrival.destination)
    }
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "separate current authority, physical lane and map admission proofs"
    )]
    pub async fn commit_character_npc_teleport<
        C: super::super::super::item_loop_move::ItemLoopContent,
    >(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &super::super::super::spell_owner_commit::SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
        admission: NpcTeleportAdmission,
    ) -> Result<NpcProgramCommit> {
        self.commit_npc_program_source(
            authority,
            node,
            permit,
            fence,
            request,
            catalogue,
            items,
            NpcRuleCatalog::runtime().map_err(|_| CharacterProgressionError::InvalidInput)?,
            Some(admission),
        )
        .await
    }
}

pub fn preview_teleport(
    request: &NpcProgramRequest,
    tracks: &BTreeMap<String, i64>,
    catalogue: &QuestStateCatalogue,
    now: i64,
    eligibility: &NpcEligibility,
) -> std::result::Result<
    Option<NpcTeleportPlan>,
    oteryn_gameplay_quests::quest::npc_rules::NpcRuleError,
> {
    let Some(source) = NpcRuleCatalog::runtime()?.admitted_for_npc(&request.npc, catalogue)? else {
        return Ok(None);
    };
    evaluate(
        &source,
        request,
        tracks,
        now,
        NpcEvaluationFacts {
            eligibility,
            player_name: None,
        },
    )
}

pub(super) fn evaluate(
    source: &NpcRuleCatalog,
    request: &NpcProgramRequest,
    tracks: &BTreeMap<String, i64>,
    now: i64,
    facts: NpcEvaluationFacts<'_>,
) -> std::result::Result<
    Option<NpcTeleportPlan>,
    oteryn_gameplay_quests::quest::npc_rules::NpcRuleError,
> {
    source.plan_storage_teleport(&NpcRuleInput {
        npc: &request.npc,
        trigger: request.trigger,
        message: &request.message,
        player_name: facts.player_name,
        topic: request.topic,
        tracks,
        inventory: &NpcInventorySnapshot::default(),
        now_epoch_seconds: now,
        calendar_month: utc_calendar_month(now)?,
        level: facts.eligibility.level,
        premium_current: facts.eligibility.premium_current,
        vocation: &facts.eligibility.vocation,
        sex: 0,
        outfits: &BTreeMap::new(),
    })
}

pub(super) fn response(plan: &NpcRulePlan) -> NpcProgramCommit {
    NpcProgramCommit {
        transition: QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch),
        reply: plan.reply.clone(),
        next_topic: plan.next_topic,
        read_only: true,
        inventory_changed: false,
        end_conversation: items::ends(plan),
        end_delay_ms: items::end_delay(plan),
    }
}

pub(super) async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &NpcProgramRequest,
    plan: &NpcRulePlan,
    admission: NpcTeleportAdmission,
) -> StoredResult<()> {
    let a = admission.arrival;
    sqlx::query("INSERT INTO game_quest_npc_teleport_receipts(character_id,game_session_id,command_id,transition_key,request_binding,program_digest,occurrence_id,world_id,tile_x,tile_y,tile_floor,reply,next_topic,end_conversation,end_delay_ms,quest_changed,runtime_coordinates) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,$10,$11,$12,$13,$14,$15,$16,true)")
        .bind(character.as_bytes().as_slice()).bind(request.command.game_session_id().as_bytes().as_slice())
        .bind(request.command.command_id().get().to_string()).bind(&request.transition_key).bind(request.binding(character).as_slice())
        .bind(plan.program_digest.as_slice()).bind(a.occurrence.as_bytes().as_slice()).bind(a.world_id.as_bytes().as_slice())
        .bind(a.destination.x).bind(a.destination.y).bind(a.destination.floor).bind(&plan.reply).bind(plan.next_topic)
        .bind(items::ends(plan)).bind(i32::try_from(items::end_delay(plan)).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(!plan.track_changes.is_empty() || plan.completes).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_character_pending_arrivals(character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor,source_npc_teleport) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6,true)")
        .bind(character.as_bytes().as_slice()).bind(a.occurrence.as_bytes().as_slice()).bind(a.world_id.as_bytes().as_slice())
        .bind(a.destination.x).bind(a.destination.y).bind(a.destination.floor).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn replay_matches(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &NpcProgramRequest,
    admission: NpcTeleportAdmission,
) -> StoredResult<bool> {
    let row = sqlx::query("SELECT occurrence_id::text,world_id::text,tile_x,tile_y,tile_floor,runtime_coordinates FROM game_quest_npc_teleport_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric")
        .bind(character.as_bytes().as_slice()).bind(request.command.game_session_id().as_bytes().as_slice())
        .bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(true);
    };
    let a = admission.arrival;
    let stored_floor: i16 = row.try_get("tile_floor")?;
    let stored_floor = if row.try_get::<bool, _>("runtime_coordinates")? {
        stored_floor
    } else {
        stored_floor
            .checked_neg()
            .ok_or(DurabilityError::InvalidStoredState)?
    };
    Ok(
        uuid_text(&row.try_get::<String, _>("occurrence_id")?)? == a.occurrence.as_bytes()
            && uuid_text(&row.try_get::<String, _>("world_id")?)? == *a.world_id.as_bytes()
            && row.try_get::<i32, _>("tile_x")? == a.destination.x
            && row.try_get::<i32, _>("tile_y")? == a.destination.y
            && stored_floor == a.destination.floor,
    )
}

pub(super) async fn load_read_only(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &NpcProgramRequest,
) -> StoredResult<Result<Option<NpcProgramCommit>>> {
    let row = sqlx::query("SELECT request_binding,reply,next_topic,end_conversation,end_delay_ms,quest_changed FROM game_quest_npc_teleport_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric")
        .bind(character.as_bytes().as_slice()).bind(request.command.game_session_id().as_bytes().as_slice())
        .bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(Ok(None));
    };
    if row.try_get::<Vec<u8>, _>("request_binding")? != request.binding(character) {
        return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
    }
    if row.try_get::<bool, _>("quest_changed")? {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Ok(Some(NpcProgramCommit {
        transition: QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch),
        reply: row.try_get("reply")?,
        next_topic: row.try_get("next_topic")?,
        read_only: true,
        inventory_changed: false,
        end_conversation: row.try_get("end_conversation")?,
        end_delay_ms: u64::try_from(row.try_get::<i32, _>("end_delay_ms")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    })))
}

#[cfg(test)]
mod tests;
