//! LOOP-MAP-1 fenced Character writer. The existing fee owner performs coins/bank/audit.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest, FeeChangeFacts,
    burn_fee_in_transaction,
};
use super::item_mint_audit::Type2Transaction;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::cyclopedia::{BasicArea, MapAction, MapProgress, MapRefusal, Point};
use crate::foundation::CommandRef;
use oteryn_protocol_oteryn::cyclopedia::{MapResult, encode_result};
use sha2::{Digest, Sha256};
use sqlx::Row;

mod storage;
mod world;
pub use world::CyclopediaWorldBinding;
#[derive(Debug, Clone)]
pub struct MapRequest {
    pub command: CommandRef,
    pub action: MapAction,
    pub binding: [u8; 32],
    pub occurrence: [u8; 16],
    pub bundle: [u8; 32],
    pub epoch: u64,
    pub pool: Vec<Point>,
    pub at: Point,
    pub fee: Option<FeeChangeFacts>,
    pub transaction: [u8; 16],
    pub event: [u8; 16],
    pub now: i64,
}
pub trait CyclopediaPremiumGate: Sync {
    fn premium(
        &self,
        tx: &mut Type2Transaction<'_>,
        account: [u8; 16],
    ) -> impl Future<Output = Result<bool, DurabilityError>> + Send;
}
impl DurabilityRoot {
    pub(super) async fn commit_cyclopedia<G: CyclopediaPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: MapRequest,
        gate: &G,
    ) -> Result<(u64, Vec<u8>), CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut gate = gate;
        self.try_issue_semantic_pass()?.run_with_context(&mut gate, move |holder, deadline, gate| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if request.command.game_session_id() != fence.game_session_id || request.pool.len() > 128 || request.pool.len() < 8 || request.now <= 0 {
                return Ok(Err(CharacterProgressionError::InvalidInput));
            }
            // Current authority is checked even on replay; a retained receipt grants no custody.
            let root = match assert_gameplay_fence(&mut tx,&fence,&node).await? { Ok(r)=>r, Err(e)=>return Ok(Err(e)) };
            let state = sqlx::query("SELECT character_revision::text,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if numeric_u64(&state,"character_revision")? != root.revision || !state_matches_root(&state,&root) {
                return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
            }
            let pending: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if pending { return Ok(Err(CharacterProgressionError::RespawnPending)); }
            if let Some(row)=sqlx::query("SELECT binding,result FROM game_character_cyclopedia_receipts WHERE character_id=encode($1,'hex')::uuid AND command_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await? {
                if row.try_get::<Vec<u8>,_>("binding")? != request.binding { return Ok(Err(CharacterProgressionError::ConflictingOccurrence)); }
                return Ok(Ok((root.revision,row.try_get("result")?)));
            }
            // A pending reset holds admission's relation fence; active epoch and bundle must
            // still match this boot. No client value chooses either.
            let epoch = sqlx::query("SELECT a.reset_epoch::text,e.target_digest FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid FOR SHARE OF a")
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            if !epoch.as_ref().is_some_and(|r| numeric_u64(r,"reset_epoch").ok()==Some(request.epoch) && r.try_get::<Vec<u8>,_>("target_digest").ok().as_deref()==Some(request.bundle.as_slice())) {
                return Ok(Err(CharacterProgressionError::AuthorityRejected));
            }
            let area=BasicArea::embedded().ok_or(DurabilityError::InvalidStoredState)?;
            let mut progress=storage::load(&mut tx,&fence,&request).await?;
            let account: String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            // Oteryn default, not Reference-qualified: donations require no Premium.
            let premium = if matches!(request.action,MapAction::Start|MapAction::Shuffle) {
                gate.premium(&mut tx,super::character_progression::uuid_text(&account)?).await?
            } else { false };
            let changed = match area.apply(&mut progress,request.action,&request.pool,request.at,premium,&request.occurrence) {
                Ok(changed)=>changed,
                Err(reason)=> {
                    let code=match reason { MapRefusal::Invalid=>1, MapRefusal::PremiumRequired=>2, MapRefusal::AlreadyDiscovered=>3, MapRefusal::NoExploration=>4, MapRefusal::InsufficientPool=>5 };
                    let (pool,improved)=storage::world(&mut tx,fence.runtime_scope.world_id(),request.bundle,request.epoch).await?;
                    return Ok(Ok((root.revision,projection(code,root.revision,&progress,pool,improved,&request)?)));
                }
            };
            // Gold owner locks root, eligible coins, bank in that order, before the pool.
            let donation=if let MapAction::Donate(amount)=request.action {
                if crate::domain::cyclopedia::DonationAmount::new(i64::from(amount)).is_err() { return Ok(Err(CharacterProgressionError::InvalidInput)); }
                let Some(change)=request.fee.clone() else { return Ok(Err(CharacterProgressionError::InvalidInput)); };
                let fee=FeeBurnRequest { cause:FeeBurnCause::AreaDonation {world:fence.runtime_scope.world_id(),area:area.area,occurrence:request.occurrence},fee_gold_units:u64::from(amount),transaction_id:request.transaction,event_id:request.event,occurred_at_unix_ms:request.now,server_build_id:"oteryn-game-server".into(),change };
                match burn_fee_in_transaction(&mut tx,&fence,&fee).await {
                    Ok(FeeBurnOutcome::Burned(_))=>(),
                    Ok(FeeBurnOutcome::AlreadyBurned(_))=>return Err(DurabilityError::InvalidStoredState),
                    Err(FeeBurnError::InsufficientFunds|FeeBurnError::ChangeDoesNotFit|FeeBurnError::CapacityExceeded)=> {
                        let (pool,improved)=storage::world(&mut tx,fence.runtime_scope.world_id(),request.bundle,request.epoch).await?;
                        return Ok(Ok((root.revision,projection(6,root.revision,&progress,pool,improved,&request)?)));
                    }
                    Err(FeeBurnError::Unavailable(e))=>return Err(e),
                    Err(_)=>return Ok(Err(CharacterProgressionError::InvalidInput)),
                }
                u64::from(amount)
            } else {0};
            let (pool,improved)=storage::world(&mut tx,fence.runtime_scope.world_id(),request.bundle,request.epoch).await?;
            if donation>0 && improved { return Ok(Ok((root.revision,projection(7,root.revision,&progress,pool,improved,&request)?))); }
            let pool_after=pool.checked_add(donation).filter(|v|*v<=i64::MAX as u64).ok_or(DurabilityError::InvalidStoredState)?;
            let advanced=changed || donation>0;
            let revision=root.revision.checked_add(u64::from(advanced)).ok_or(DurabilityError::InvalidStoredState)?;
            let bytes=projection(0,revision,&progress,pool_after,improved,&request)?;
            if request.action != MapAction::Query {
                storage::save(&mut tx,&fence,&request,&progress,advanced,donation,pool,pool_after,&bytes).await?;
            }
            tx.commit(deadline).await?;
            Ok(Ok((revision,bytes)))
        })).await?
    }
}
fn projection(
    code: u8,
    revision: u64,
    progress: &MapProgress,
    pool: u64,
    improved: bool,
    request: &MapRequest,
) -> Result<Vec<u8>, DurabilityError> {
    let found = if progress.discovered {
        7
    } else {
        progress
            .exploration
            .as_ref()
            .map_or(0, |e| e.found.count_ones() as u8)
    };
    let visible = progress
        .exploration
        .as_ref()
        .map(|e| {
            e.points
                .iter()
                .enumerate()
                .filter(|(i, p)| e.found & (1 << i) == 0 && p.in_window(request.at))
                .map(|(_, p)| {
                    Ok((
                        u16::try_from(p.x).map_err(|_| DurabilityError::InvalidStoredState)?,
                        u16::try_from(p.y).map_err(|_| DurabilityError::InvalidStoredState)?,
                        i8::try_from(p.floor).map_err(|_| DurabilityError::InvalidStoredState)?,
                    ))
                })
                .collect::<Result<Vec<_>, DurabilityError>>()
        })
        .transpose()?
        .unwrap_or_default();
    encode_result(&MapResult {
        code,
        revision,
        discovered: progress.discovered,
        found,
        pool,
        epoch: request.epoch,
        improved,
        visible,
    })
    .map_err(|_| DurabilityError::InvalidStoredState)
}
