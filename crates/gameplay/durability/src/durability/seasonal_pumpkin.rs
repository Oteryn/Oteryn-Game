//! Current Halloween rewardroom eligibility, immutable first visit and native source body.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::begin_type2_transaction;
use super::item_mint_audit::Type2Transaction;
use super::item_transfer::{CurrentCharacterItemFence, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{CharacterId, CommandRef, WorldId};
use sqlx::Row;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PumpkinRewardVisit {
    pub death: [u8; 16],
    pub first_entry_ms: i64,
    pub deadline_ms: i64,
    pub source_session: [u8; 16],
    pub source_occurrence: [u8; 32],
}
impl PumpkinRewardVisit {
    pub fn draw_nonce(&self) -> u64 {
        u64::from_be_bytes([
            self.source_occurrence[0],
            self.source_occurrence[1],
            self.source_occurrence[2],
            self.source_occurrence[3],
            self.source_occurrence[4],
            self.source_occurrence[5],
            self.source_occurrence[6],
            self.source_occurrence[7],
        ])
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PumpkinPortalWindow {
    pub death: [u8; 16],
    pub until_ms: i64,
}
#[derive(Clone, Debug)]
pub struct PumpkinSkinningSource {
    pub death: [u8; 16],
    pub knife: [u8; 16],
    pub body_placement: Vec<u8>,
    pub body_revision: u64,
    pub map_digest: [u8; 32],
    pub wire_binding: [u8; 32],
}
fn visit(row: sqlx::postgres::PgRow) -> Result<PumpkinRewardVisit, DurabilityError> {
    let death: Vec<u8> = row.try_get("death")?;
    let source_session: Vec<u8> = row.try_get("source_session")?;
    let source_occurrence: Vec<u8> = row.try_get("source_occurrence")?;
    Ok(PumpkinRewardVisit {
        death: death
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        first_entry_ms: row.try_get("first_entry_ms")?,
        deadline_ms: row.try_get("deadline_ms")?,
        source_session: source_session
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        source_occurrence: source_occurrence
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    })
}
pub(crate) async fn live_visit(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    world: WorldId,
    death: Option<[u8; 16]>,
) -> Result<Option<PumpkinRewardVisit>, DurabilityError> {
    sqlx::query("SELECT uuid_send(death_id) AS death,first_entry_ms,deadline_ms,uuid_send(source_game_session_id) AS source_session,source_occurrence FROM game_pumpkin_reward_visits WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND($3::bytea IS NULL OR death_id=encode($3,'hex')::uuid) AND deadline_ms>floor(extract(epoch FROM public.game_daily_effect_now())*1000)::bigint ORDER BY first_entry_ms DESC LIMIT 1 FOR SHARE")
 .bind(character.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(death.map(|d|d.to_vec())).fetch_optional(&mut ***tx).await?.map(visit).transpose()
}
#[allow(
    clippy::too_many_arguments,
    reason = "current Node/session fence and source-specific visit or body occurrence are independent proofs"
)]
pub(crate) async fn arm_source(
    tx: &mut Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    death: [u8; 16],
    occurrence: Option<[u8; 32]>,
    command: Option<CommandRef>,
    body: Option<&PumpkinSkinningSource>,
    replay_visit: bool,
) -> Result<(), DurabilityError> {
    let (world, channel) = scope_of(fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let fact = node.fact();
    sqlx::query("SELECT game_pumpkin_source_arm(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,$5::text::numeric,$6,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9::text::numeric,$10::text::numeric,$11,encode($12,'hex')::uuid,$13,$14::text::numeric,encode($15,'hex')::uuid,$16,$17::text::numeric,$18,$19)")
 .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
 .bind(if body.is_some(){"claim"}else if replay_visit{"replay_visit"}else{"visit"}).bind(death.as_slice()).bind(occurrence.map(|o|o.to_vec())).bind(command.map(|c|c.command_id().get().to_string())).bind(body.map(|b|b.knife.to_vec())).bind(body.map(|b|b.body_placement.as_slice())).bind(body.map(|b|b.body_revision.to_string())).bind(body.map(|b|b.map_digest.to_vec())).bind(body.map(|b|b.wire_binding.to_vec())).execute(&mut ***tx).await?;
    Ok(())
}
impl DurabilityRoot {
    pub async fn pumpkin_reward_portal_window(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world: WorldId,
        channel: crate::foundation::ChannelId,
    ) -> Result<Option<PumpkinPortalWindow>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
       let mut tx=begin_type2_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
       let row:Option<(Vec<u8>,i64)>=sqlx::query_as("SELECT uuid_send(death_id),minted_at+1800000 FROM game_boss_reward_deaths WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1 AND channel_id=encode($2,'hex')::uuid AND boss_key='oteryn:creature.the_mutated_pumpkin' AND settlement_outcome=1 AND minted_at<=floor(extract(epoch FROM public.game_daily_effect_now())*1000)::bigint AND minted_at+1800000>floor(extract(epoch FROM public.game_daily_effect_now())*1000)::bigint ORDER BY minted_at DESC,death_id DESC LIMIT 1")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
       let result=row.map(|(death,until_ms)|->Result<PumpkinPortalWindow,DurabilityError>{Ok(PumpkinPortalWindow{death:death.try_into().map_err(|_|DurabilityError::InvalidStoredState)?,until_ms})}).transpose()?;
       tx.commit(deadline).await?;Ok(result)
      })).await
    }
    /// Read only; a missing/expired visit is never authority to stay in the room.
    pub async fn current_pumpkin_reward_visit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        world: WorldId,
    ) -> Result<Option<PumpkinRewardVisit>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let result = live_visit(&mut tx, character, world, None).await?;
                    tx.commit(deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
    /// The caller has committed the real STEP onto the closed source portal.
    /// A replay/re-entry preserves the first five-minute deadline; it never refreshes it.
    pub async fn admit_pumpkin_reward_visit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        step_occurrence: [u8; 32],
    ) -> Result<Option<PumpkinRewardVisit>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
 let mut tx=begin_type2_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
 let(world,channel)=scope_of(&fence).map_err(|_|DurabilityError::InvalidStoredState)?;
 let replay=sqlx::query("SELECT uuid_send(death_id) AS death,first_entry_ms,deadline_ms,uuid_send(source_game_session_id) AS source_session,source_occurrence FROM game_pumpkin_reward_visits WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND source_occurrence=$3 AND deadline_ms>floor(extract(epoch FROM public.game_daily_effect_now())*1000)::bigint ORDER BY first_entry_ms DESC LIMIT 1 FOR SHARE")
 .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(step_occurrence.as_slice()).fetch_optional(&mut **tx).await?.map(visit).transpose()?;
 if let Some(replay)=replay {arm_source(&mut tx,&node,&fence,replay.death,Some(step_occurrence),None,None,true).await?;tx.commit(deadline).await?;return Ok(Some(replay));}
 let death:Option<Vec<u8>>=sqlx::query_scalar("SELECT uuid_send(d.death_id) FROM game_boss_reward_deaths d JOIN game_boss_reward_contributors c USING(world_id,death_id) WHERE d.world_id=encode($1,'hex')::uuid AND d.scope_kind=1 AND d.channel_id=encode($3,'hex')::uuid AND d.boss_key='oteryn:creature.the_mutated_pumpkin' AND d.settlement_outcome=1 AND c.character_id=encode($2,'hex')::uuid AND(c.damage_dealt>0 OR c.damage_received>0 OR c.healing_others>0) AND d.minted_at<=floor(extract(epoch FROM public.game_daily_effect_now())*1000)::bigint AND d.minted_at+1800000>floor(extract(epoch FROM public.game_daily_effect_now())*1000)::bigint ORDER BY d.minted_at DESC,d.death_id DESC LIMIT 1 FOR SHARE OF d,c")
 .bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
 let Some(death)=death else{tx.commit(deadline).await?;return Ok(None)};
 let death:[u8;16]=death.try_into().map_err(|_|DurabilityError::InvalidStoredState)?;
 arm_source(&mut tx,&node,&fence,death,Some(step_occurrence),None,None,false).await?;
 sqlx::query("INSERT INTO game_pumpkin_reward_visits(world_id,death_id,character_id,first_entry_ms,deadline_ms,source_game_session_id,source_occurrence,created_xact_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,300001,encode($4,'hex')::uuid,$5,pg_current_xact_id()) ON CONFLICT(character_id,world_id,death_id) DO NOTHING")
 .bind(world.as_bytes().as_slice()).bind(death.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(step_occurrence.as_slice()).execute(&mut **tx).await?;
 let character = CharacterId::decode(fence.character_id.as_bytes()).map_err(|_| DurabilityError::InvalidStoredState)?;
 let result=live_visit(&mut tx,character,world,Some(death)).await?;tx.commit(deadline).await?;Ok(result)
 })).await
    }
}
