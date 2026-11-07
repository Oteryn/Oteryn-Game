//! Crystal 201189f8 Cake base/extraction movements and Brutus' two once-per-run rewards.
use super::{CakeWorldSnapshot, lock_world};
use crate::durability::account_achievement::add_progress;
use crate::durability::boss_encounter::BossClockError;
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::character_progression::assert_item_only_fence;
use crate::durability::db::{begin_type2_transaction, lock_admission_relations};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::item_transfer::{CurrentCharacterItemFence, ItemWeightLookup};
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintOutcome, RewardClaimMintRequest, seasonal,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, BossClockError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CakeCity {
    Carlin,
    Thais,
    Edron,
    Darashia,
    LibertyBay,
}
impl CakeCity {
    pub fn from_action_id(action: u16) -> Option<Self> {
        Some(match action {
            43910 => Self::Carlin,
            43911 => Self::Thais,
            43912 => Self::Edron,
            43913 => Self::Darashia,
            43914 => Self::LibertyBay,
            _ => return None,
        })
    }
    const fn bit(self) -> u8 {
        1 << match self {
            Self::Carlin => 0,
            Self::Thais => 1,
            Self::Edron => 2,
            Self::Darashia => 3,
            Self::LibertyBay => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CakeCharacterEvent {
    Help,
    Isle,
    Extract(CakeCity),
    Inspect,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CakeCharacterEffect {
    Status,
    Started,
    Armed,
    Extracted(CakeCity),
    NoMark,
    AlreadyVisited,
    Complete,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CakeCharacterProgress {
    pub event_start: i64,
    pub started: bool,
    pub isle_armed: bool,
    pub visited_cities: u8,
    pub item_reward_given: bool,
    pub cupcake_reward_given: bool,
    pub effect: CakeCharacterEffect,
}
impl CakeCharacterProgress {
    pub fn visited_count(&self) -> u32 {
        self.visited_cities.count_ones()
    }
    pub const fn completed(&self) -> bool {
        self.visited_cities == 31
    }
    fn initial(event_start: i64) -> Self {
        Self {
            event_start,
            started: false,
            isle_armed: false,
            visited_cities: 0,
            item_reward_given: false,
            cupcake_reward_given: false,
            effect: CakeCharacterEffect::Status,
        }
    }
}

fn active(world: &CakeWorldSnapshot, now: i64) -> bool {
    world.event_start <= now && now < world.event_end && world.state.stage > 0
}

pub fn reward_claim_key(event_start: i64, cupcake: bool) -> String {
    format!(
        "oteryn:seasonal.cake.run{event_start}.{}",
        if cupcake { "cupcake" } else { "item" }
    )
}
pub fn reward_source(event_start: i64) -> String {
    format!("seasonal:brutus:{event_start}")
}

impl DurabilityRoot {
    /// Stable movement occurrence includes actor identity, movement revision and epoch.
    /// A historical arm replay after an extraction returns its receipt, never re-arms.
    #[allow(
        clippy::too_many_arguments,
        reason = "complete current Character fence and source event"
    )]
    pub async fn commit_cake_character_event(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        occurrence: [u8; 32],
        event: CakeCharacterEvent,
        now_seconds: i64,
    ) -> Result<Option<CakeCharacterProgress>> {
        if now_seconds < 0 {
            return Err(BossClockError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| BossClockError::AuthorityRejected)?;
        let node = node.clone();
        let binding: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:cake-character:v1")
            .chain_update(fence.character_id.as_bytes())
            .chain_update(fence.runtime_scope.world_id().as_bytes())
            .chain_update(serde_json::to_vec(&event).map_err(|_| BossClockError::InvalidInput)?)
            .finalize()
            .into();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:cake-character:'||encode($1,'hex')||encode($2,'hex'),0))")
                .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_slice())
                .execute(&mut **tx).await?;
            if let Some(row) = sqlx::query("SELECT binding,result FROM game_seasonal_cake_character_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence=$2")
                .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_slice()).fetch_optional(&mut **tx).await? {
                if row.try_get::<Vec<u8>,_>("binding")? != binding {
                    return Ok(Err(BossClockError::InvalidInput));
                }
                let progress = serde_json::from_value(row.try_get("result")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                tx.commit(deadline).await?;
                return Ok(Ok(Some(progress)));
            }
            if assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(BossClockError::AuthorityRejected));
            }
            let Some(world) = lock_world(&mut tx, fence.runtime_scope.world_id(), now_seconds).await
                .map_err(world_error)? else { return Ok(Ok(None)); };
            if !active(&world, now_seconds) { return Ok(Ok(None)); }
            if event != CakeCharacterEvent::Isle && world.state.stage != 3 {
                return Ok(Ok(None));
            }
            let (kind, city) = match event {
                CakeCharacterEvent::Help => (0, None),
                CakeCharacterEvent::Isle => (1, None),
                CakeCharacterEvent::Extract(city) => (2, Some(match city { CakeCity::Carlin => 0, CakeCity::Thais => 1, CakeCity::Edron => 2, CakeCity::Darashia => 3, CakeCity::LibertyBay => 4 })),
                CakeCharacterEvent::Inspect => (3, None),
            };
            arm_character(&mut tx, &node, &fence, world.event_start, now_seconds,
                kind, city, Some(&occurrence), None).await?;
            let mut progress = load(&mut tx, &fence, world.event_start).await?;
            match event {
                CakeCharacterEvent::Inspect => {},
                CakeCharacterEvent::Help => { if progress.completed() { progress.effect = CakeCharacterEffect::Complete; } else { progress.started = true; progress.effect = CakeCharacterEffect::Started; } },
                CakeCharacterEvent::Isle => {
                    add_progress(&mut tx, fence.character_id, "oteryn:achievement/cake_conqueror", 1,
                        "oteryn:cake-isle", &occurrence).await?;
                    if world.state.stage == 3 {
                        add_progress(&mut tx, fence.character_id, "oteryn:achievement/i_did_my_part", 1,
                            "oteryn:cake-isle", &occurrence).await?;
                        if !progress.completed() && !progress.isle_armed {
                            progress.isle_armed = true; progress.effect = CakeCharacterEffect::Armed;
                        }
                    }
                },
                CakeCharacterEvent::Extract(city) => {
                    progress.effect = if progress.completed() { CakeCharacterEffect::Complete }
                    else if !progress.isle_armed { CakeCharacterEffect::NoMark }
                    else if progress.visited_cities & city.bit() != 0 { CakeCharacterEffect::AlreadyVisited }
                    else { progress.visited_cities |= city.bit(); progress.isle_armed = false;
                        CakeCharacterEffect::Extracted(city) };
                },
            }
            save(&mut tx, &fence, &progress).await?;
            sqlx::query("INSERT INTO game_seasonal_cake_character_receipts(character_id,occurrence,binding,result) VALUES(encode($1,'hex')::uuid,$2,$3,$4)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_slice())
                .bind(binding.as_slice()).bind(serde_json::to_value(&progress).map_err(|_| DurabilityError::InvalidStoredState)?)
                .execute(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(Ok(Some(progress)))
        })).await?
    }

    /// Source eligibility, the native complete item/container mint and annual flag commit once.
    #[allow(
        clippy::too_many_arguments,
        reason = "native frozen mint and source-owned eligibility"
    )]
    pub async fn claim_cake_character_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: RewardClaimMintRequest,
        now_seconds: i64,
        weights: &dyn ItemWeightLookup,
    ) -> std::result::Result<Option<RewardClaimMintOutcome>, RewardClaimMintError> {
        if now_seconds < 0 {
            return Err(RewardClaimMintError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let mut context = (node, weights);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let (node, weights) = context;
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match seasonal::replay(&mut tx, fence.character_id, &request).await {
                        Ok(Some(committed)) => {
                            tx.commit(deadline).await?;
                            return Ok(Ok(Some(RewardClaimMintOutcome::AlreadyCommitted(
                                committed,
                            ))));
                        }
                        Ok(None) => {}
                        Err(RewardClaimMintError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    if assert_item_only_fence(&mut tx, &fence, node)
                        .await?
                        .is_err()
                    {
                        return Ok(Err(RewardClaimMintError::AuthorityRejected));
                    }
                    let Some(world) =
                        lock_world(&mut tx, fence.runtime_scope.world_id(), now_seconds)
                            .await
                            .map_err(world_error)?
                    else {
                        return Ok(Ok(None));
                    };
                    if !active(&world, now_seconds)
                        || world.state.stage != 3
                        || !world
                            .state
                            .bonus_from
                            .is_some_and(|start| start <= now_seconds)
                    {
                        return Ok(Ok(None));
                    }
                    let mut progress = load(&mut tx, &fence, world.event_start).await?;
                    if !progress.completed() {
                        return Ok(Ok(None));
                    }
                    let cupcake = match request.item.definition.production_key.as_str() {
                        "oteryn:item.tibia.i14756"
                        | "oteryn:item.tibia.i20347"
                        | "oteryn:item.tibia.i20350" => false,
                        "oteryn:item.tibia.i28484"
                        | "oteryn:item.tibia.i28485"
                        | "oteryn:item.tibia.i28486" => true,
                        _ => return Ok(Err(RewardClaimMintError::InvalidInput)),
                    };
                    if request.claim.family != "RewardClaim"
                        || request.claim.production_key
                            != reward_claim_key(world.event_start, cupcake)
                        || request.source_placement != reward_source(world.event_start)
                        || request.quantity != 1
                        || !request.contents.is_empty()
                        || request.root_text.is_some()
                        || request.key_number.is_some()
                    {
                        return Ok(Err(RewardClaimMintError::InvalidInput));
                    }
                    if (cupcake && progress.cupcake_reward_given)
                        || (!cupcake && progress.item_reward_given)
                    {
                        return Ok(Ok(None));
                    }
                    arm_character(
                        &mut tx,
                        node,
                        &fence,
                        world.event_start,
                        now_seconds,
                        if cupcake { 5 } else { 4 },
                        None,
                        None,
                        Some(request.command.command_id().get()),
                    )
                    .await?;
                    let committed =
                        match seasonal::mint(&mut tx, node, &fence, request, *weights).await {
                            Ok(committed) => committed,
                            Err(RewardClaimMintError::Unavailable(error)) => return Err(error),
                            Err(error) => return Ok(Err(error)),
                        };
                    if cupcake {
                        progress.cupcake_reward_given = true;
                    } else {
                        progress.item_reward_given = true;
                    }
                    save(&mut tx, &fence, &progress).await?;
                    tx.commit(deadline).await?;
                    Ok(Ok(Some(RewardClaimMintOutcome::Committed(committed))))
                })
            })
            .await?
    }
}

fn world_error(error: BossClockError) -> DurabilityError {
    match error {
        BossClockError::Unavailable(error) => error,
        _ => DurabilityError::InvalidStoredState,
    }
}
async fn load(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterItemFence,
    event_start: i64,
) -> std::result::Result<CakeCharacterProgress, DurabilityError> {
    let row = sqlx::query("SELECT started,isle_armed,visited_cities,item_reward_given,cupcake_reward_given FROM game_seasonal_cake_characters WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND event_start=$3 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(event_start).fetch_optional(&mut ***tx).await?;
    let Some(row) = row else {
        return Ok(CakeCharacterProgress::initial(event_start));
    };
    Ok(CakeCharacterProgress {
        event_start,
        started: row.try_get("started")?,
        isle_armed: row.try_get("isle_armed")?,
        visited_cities: u8::try_from(row.try_get::<i16, _>("visited_cities")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        item_reward_given: row.try_get("item_reward_given")?,
        cupcake_reward_given: row.try_get("cupcake_reward_given")?,
        effect: CakeCharacterEffect::Status,
    })
}
async fn save(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterItemFence,
    progress: &CakeCharacterProgress,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_seasonal_cake_characters(character_id,world_id,event_start,started,isle_armed,visited_cities,item_reward_given,cupcake_reward_given) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7,$8) ON CONFLICT(character_id,world_id,event_start) DO UPDATE SET started=EXCLUDED.started,isle_armed=EXCLUDED.isle_armed,visited_cities=EXCLUDED.visited_cities,item_reward_given=EXCLUDED.item_reward_given,cupcake_reward_given=EXCLUDED.cupcake_reward_given")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(progress.event_start).bind(progress.started).bind(progress.isle_armed).bind(i16::from(progress.visited_cities))
        .bind(progress.item_reward_given).bind(progress.cupcake_reward_given).execute(&mut ***tx).await?;
    Ok(())
}
#[allow(
    clippy::too_many_arguments,
    reason = "SQL source transition includes the independent full session fence"
)]
async fn arm_character(
    tx: &mut Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    event_start: i64,
    now: i64,
    kind: i16,
    city: Option<i16>,
    occurrence: Option<&[u8]>,
    command: Option<u64>,
) -> std::result::Result<(), DurabilityError> {
    let crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } = fence.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let fact = node.fact();
    sqlx::query("SELECT game_cake_arm_character(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,$12::text::numeric,$13::smallint,$14::smallint,$15,$16::text::numeric)")
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(channel_id.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
        .bind(node.sql_possession_secret()).bind(event_start).bind(now)
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .bind(kind).bind(city).bind(occurrence).bind(command.map(|value|value.to_string()))
        .execute(&mut ***tx).await?;
    Ok(())
}
