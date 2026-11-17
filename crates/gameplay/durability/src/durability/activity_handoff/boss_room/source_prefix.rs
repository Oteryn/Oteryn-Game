//! Read-only source pad eligibility. Native effects still require the current
//! physical owner, exact source policy and original command for every pad.
use super::*;
use crate::foundation::CommandRef;
use oteryn_gameplay_encounter_rules::boss_room::source_prefix::plan_on_use_extra_prefix;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceBossLeverExtra {
    Hatred,
    Outburst,
}
fn source_kind(definition: &BossRoomDefinition) -> Option<SourceBossLeverExtra> {
    match (
        definition.boss_key.as_str(),
        definition.boss_creature_key.as_str(),
    ) {
        ("oteryn:boss.goshnar_s_hatred", "oteryn:creature.goshnar_s_hatred") => {
            Some(SourceBossLeverExtra::Hatred)
        }
        ("oteryn:boss.outburst", "oteryn:creature.outburst") => {
            Some(SourceBossLeverExtra::Outburst)
        }
        _ => None,
    }
}
#[derive(Debug, Clone)]
pub struct QualifiedBossPadExtra {
    kind: SourceBossLeverExtra,
    ordinal: u8,
    command: CommandRef,
    wire: [u8; 32],
    source: CurrentCharacterItemFence,
    position: LastPositionRecord,
    content_revision: String,
}
impl QualifiedBossPadExtra {
    pub const fn kind(&self) -> SourceBossLeverExtra {
        self.kind
    }
    pub const fn pad_ordinal(&self) -> u8 {
        self.ordinal
    }
    pub const fn command(&self) -> CommandRef {
        self.command
    }
    pub const fn wire_binding(&self) -> [u8; 32] {
        self.wire
    }
    pub const fn source(&self) -> CurrentCharacterItemFence {
        self.source
    }
    pub const fn position(&self) -> LastPositionRecord {
        self.position
    }
    pub fn content_revision(&self) -> &str {
        &self.content_revision
    }
}
#[derive(Debug, Clone)]
pub struct QualifiedBossPadPrefix {
    pads: Vec<QualifiedBossPadExtra>,
    refusal: Option<ActivityRefusal>,
}
impl QualifiedBossPadPrefix {
    pub fn pads(&self) -> &[QualifiedBossPadExtra] {
        &self.pads
    }
    pub const fn refusal(&self) -> Option<ActivityRefusal> {
        self.refusal
    }
}
impl ActivityHandoffOwner {
    /// Caller preserves the actual authored census order and holds the source
    /// physical lane across this read and effect publication. Participant and
    /// occupied-room source prerequisites precede this hook; the durable entry
    /// writer independently rechecks every fact before a successful handoff.
    #[allow(
        clippy::too_many_arguments,
        reason = "original command, current roster and source definition have independent owners"
    )]
    pub async fn read_source_pad_prefix(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        expected: &BossRoomDefinition,
        participants: &[ActivityParticipant],
        command: CommandRef,
        wire: [u8; 32],
    ) -> Result<Option<QualifiedBossPadPrefix>> {
        let Some(kind) = source_kind(expected) else {
            return Ok(None);
        };
        if participants.is_empty()
            || participants.len() > ACTIVITY_PARTICIPANTS_MAX
            || wire == [0; 32]
            || !participants
                .iter()
                .any(|p| p.source.game_session_id == command.game_session_id())
        {
            return Ok(None);
        }
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let expected = expected.clone();
        let participants = participants.to_vec();
        let node = node.clone();
        let root = self.root.clone();
        self.root.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let Some(initiator)=participants.iter().find(|p|p.source.game_session_id==command.game_session_id()) else {return Ok(None)};
            if retained_participant(&mut tx,&root,initiator.source).await?.is_none() {return Ok(None)}
            let Ok(current)=assert_item_only_fence(&mut tx,&initiator.source,&node).await? else {return Ok(None)};
            let source=initiator.source_position;
            let origin_pin=ActivityContentPin {
                coordinate_frame_digest:source.coordinate_frame_digest,
                map_revision_digest:source.map_revision_digest,
                server_artifact_digest:source.server_artifact_digest,
            };
            let Some(definition)=read_definition(&mut tx,&expected.boss_key,
                initiator.source.runtime_scope.world_id(),&current.content_revision,Some(origin_pin)).await? else {return Ok(None)};
            if definition!=expected || source_kind(&definition)!=Some(kind) {return Ok(None)}
            let scope=participants[0].source.runtime_scope;
            let generation=participants[0].source.scope_ownership_generation;
            let mut facts=Vec::with_capacity(participants.len());
            let mut ordinals=Vec::with_capacity(participants.len());
            let mut previous=None;
            for p in &participants {
                if p.source.runtime_scope!=scope || p.source.scope_ownership_generation!=generation
                    || p.source_position.world_id!=scope.world_id()
                    || retained_participant(&mut tx,&root,p.source).await?.is_none()
                    || assert_item_only_fence(&mut tx,&p.source,&node).await?.is_err() {return Ok(None)}
                let Some(ordinal)=definition.lever_positions.iter().position(|cell|*cell==p.source_position.cell)
                    else {return Ok(None)};
                if previous.is_some_and(|last|ordinal<=last) {return Ok(None)}
                previous=Some(ordinal);
                ordinals.push(u8::try_from(ordinal).map_err(|_|DurabilityError::InvalidStoredState)?);
                let level:Option<i64>=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid AND content_revision=$2 FOR SHARE")
                    .bind(p.source.character_id.as_bytes().as_slice()).bind(&definition.content_revision).fetch_optional(&mut *tx).await?;
                let Some(level)=level else {return Ok(None)};
                let cooldown:Option<i64>=sqlx::query_scalar("SELECT available_at FROM game_character_boss_cooldowns WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2 FOR SHARE")
                    .bind(p.source.character_id.as_bytes().as_slice()).bind(&definition.boss_key).fetch_optional(&mut *tx).await?;
                facts.push(ParticipantFact {character:*p.source.character_id.as_bytes(),
                    position:(p.source_position.cell.x,p.source_position.cell.y,p.source_position.cell.floor),
                    level:u32::try_from(level).map_err(|_|DurabilityError::InvalidStoredState)?,
                    cooldown_until_ms:u64::try_from(cooldown.unwrap_or(0)).map_err(|_|DurabilityError::InvalidStoredState)?});
            }
            let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint").fetch_one(&mut *tx).await?;
            let plan=plan_on_use_extra_prefix(&definition.policy,&facts,u64::try_from(now).map_err(|_|DurabilityError::InvalidStoredState)?);
            let pads=participants.iter().zip(ordinals).take(plan.eligible).map(|(p,ordinal)|QualifiedBossPadExtra {
                kind,ordinal,command,wire,source:p.source,position:p.source_position,content_revision:definition.content_revision.clone(),
            }).collect();
            let refusal=plan.refusal.map(|reason|match reason {
                AdmissionRefusal::Count=>ActivityRefusal::PartyCount,
                AdmissionRefusal::Level=>ActivityRefusal::Level,
                AdmissionRefusal::Cooldown=>ActivityRefusal::Cooldown,
                AdmissionRefusal::Position=>ActivityRefusal::Position,
                AdmissionRefusal::Duplicate|AdmissionRefusal::InvalidPolicy=>ActivityRefusal::AuthorityRejected,
            });
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Some(QualifiedBossPadPrefix {pads,refusal}))
        })).await
    }
}
