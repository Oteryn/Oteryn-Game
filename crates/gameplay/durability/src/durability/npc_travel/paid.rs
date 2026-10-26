//! Paid routes participate in the existing item fee, travel receipt and arrival transaction.
use super::*;
use crate::durability::character_progression::assert_gameplay_fence;
use crate::durability::db::begin_type2_transaction;
use crate::durability::item_fee_burn::{FeeBurnError, FeeBurnRequest, burn_fee_in_transaction};
use crate::durability::item_mint_audit::Type2Transaction;
use oteryn_gameplay_content::content::npc_catalogue::travel::TravelRoute;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TravelRefusal {
    UnavailableRoute,
    PremiumRequired,
    CombatBlocked,
    Exhausted,
    ArrivalPending,
    StaleQuote,
    InsufficientFunds,
}

pub trait NpcTravelGate: Sync {
    fn travel_permitted(
        &self,
        tx: &mut Type2Transaction<'_>,
        account: [u8; 16],
    ) -> impl Future<Output = Result<bool, DurabilityError>> + Send;
}

impl<G: NpcTravelGate> NpcTravelGate for Option<&G> {
    async fn travel_permitted(
        &self,
        tx: &mut Type2Transaction<'_>,
        account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        match self {
            Some(gate) => gate.travel_permitted(tx, account).await,
            None => Ok(false),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaidTravelRequest {
    pub command: crate::foundation::CommandRef,
    pub arrival: PendingArrival,
    pub route: TravelRoute,
    pub fee: FeeBurnRequest,
}

async fn quote(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    route: &TravelRoute,
) -> Result<Result<u64, TravelRefusal>, DurabilityError> {
    if route.held
        || route.route.price > 1_000_000
        || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
    {
        return Ok(Err(TravelRefusal::UnavailableRoute));
    }
    let level: Option<i64> = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    if level.is_none_or(|level| level < i64::from(route.route.min_level.unwrap_or(0))) {
        return Ok(Err(TravelRefusal::UnavailableRoute));
    }
    let blocked = crate::durability::pvp_state::load_pvp_state(tx, fence.character_id)
        .await?
        .is_some_and(|row| {
            row.pz_block_remaining_ms.is_some() || row.kill_block_remaining_ms.is_some()
        });
    if blocked {
        return Ok(Err(TravelRefusal::CombatBlocked));
    }
    let cooldown: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_newhaven_travel_receipts WHERE character_id=encode($1,'hex')::uuid AND arrived_at>clock_timestamp()-interval '3 seconds')")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    if cooldown {
        return Ok(Err(TravelRefusal::Exhausted));
    }
    let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    if pending {
        return Ok(Err(TravelRefusal::ArrivalPending));
    }
    // Carpets: completed mission 6. Ships: Grand Postman rank, never ExampleQuest.
    let discount: bool = if route.postman {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 AND value>=$3)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(if route.carpet {"oteryn:quest-progress/quest/u7_24/the_postman_missions/mission06"} else {"oteryn:quest-progress/quest/u7_24/the_postman_missions/rank"}).bind(if route.carpet {13i64} else {3i64}).fetch_one(&mut ***tx).await?
    } else {
        false
    };
    Ok(Ok(route.route.price.saturating_sub(if discount {
        10
    } else {
        0
    })))
}

impl DurabilityRoot {
    pub async fn quote_npc_travel<G: NpcTravelGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        route: TravelRoute,
        gate: &G,
    ) -> Result<Result<u64, TravelRefusal>, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut gate = gate;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut gate, move |holder, deadline, gate| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let permitted = premium(&mut tx, fence, &route, *gate).await?;
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    if !permitted {
                        return Ok(Ok(Err(TravelRefusal::PremiumRequired)));
                    }
                    let result = quote(&mut tx, fence, &route).await?;
                    tx.commit(deadline).await?;
                    Ok(Ok(result))
                })
            })
            .await?
    }

    pub async fn commit_npc_travel<G: NpcTravelGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: PaidTravelRequest,
        gate: &G,
    ) -> Result<Result<Option<PendingArrival>, TravelRefusal>, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut gate = gate;
        self.try_issue_semantic_pass()?.run_with_context(&mut gate, move |holder, deadline, gate| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let permitted = premium(&mut tx, fence, &request.route, *gate).await?;
            if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {return Ok(Err(error));}
            let arrival = request.arrival;
            let route = &request.route;
            let key = format!("{}/{}", route.service, route.route.key);
            let destination = &route.route.destination;
            if request.command.game_session_id()!=fence.game_session_id
                || arrival.world_id!=fence.runtime_scope.world_id()
                || (arrival.destination.x,arrival.destination.y,arrival.destination.floor)!=(destination.x,destination.y,destination.floor)
                || request.fee.cause != (crate::durability::item_fee_burn::FeeBurnCause::NpcTravel { service: route.service.clone(), occurrence: arrival.occurrence.as_bytes() })
            {return Ok(Err(CharacterProgressionError::InvalidInput));}
            let mut hash = Sha256::new();
            hash.update(b"oteryn:paid-travel-binding:v1");
            for text in [&route.service, &route.route.key, &route.keyword, &route.route.destination.coordinate_frame] {
                hash.update((text.len() as u64).to_be_bytes());
                hash.update(text.as_bytes());
            }
            hash.update(route.route.price.to_be_bytes());
            hash.update(route.route.min_level.unwrap_or(0).to_be_bytes());
            hash.update([u8::from(route.route.premium), u8::from(route.postman), u8::from(route.carpet), u8::from(route.held)]);
            hash.update(arrival.destination.x.to_be_bytes());
            hash.update(arrival.destination.y.to_be_bytes());
            hash.update(arrival.destination.floor.to_be_bytes());
            hash.update(request.fee.fee_gold_units.to_be_bytes());
            hash.update(arrival.occurrence.as_bytes());
            hash.update(arrival.world_id.as_bytes());
            let binding = hash.finalize().to_vec();
            let previous = sqlx::query("SELECT request_binding FROM game_newhaven_travel_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric(20,0)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            if let Some(previous)=previous {
                if previous.try_get::<Option<Vec<u8>>,_>("request_binding")? != Some(binding) {return Ok(Err(CharacterProgressionError::ConflictingOccurrence));}
                let pending = read_fenced(&mut tx,&recovery,&node,fence).await.map_err(|_| DurabilityError::InvalidStoredState)?;
                tx.commit(deadline).await?;
                return Ok(Ok(Ok(pending.filter(|row| row.occurrence==arrival.occurrence))));
            }
            if !permitted {return Ok(Ok(Err(TravelRefusal::PremiumRequired)));}
            let fare = match quote(&mut tx, fence, route).await? {Ok(fare)=>fare,Err(refusal)=>return Ok(Ok(Err(refusal)))};
            if fare!=request.fee.fee_gold_units {return Ok(Ok(Err(TravelRefusal::StaleQuote)));}
            if fare>0 {
                match burn_fee_in_transaction(&mut tx,&fence,&request.fee).await {
                    Ok(_) => (),
                    Err(FeeBurnError::InsufficientFunds | FeeBurnError::ChangeDoesNotFit | FeeBurnError::CapacityExceeded) => return Ok(Ok(Err(TravelRefusal::InsufficientFunds))),
                    Err(FeeBurnError::Unavailable(error)) => return Err(error),
                    Err(_) => return Ok(Err(CharacterProgressionError::InvalidInput)),
                }
            }
            sqlx::query("INSERT INTO game_newhaven_travel_receipts(character_id,game_session_id,command_id,occurrence_id,route,request_binding,fee_transaction_id,fare_gold_units,base_fare_gold_units,postman_discount_applied) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),encode($4,'hex')::uuid,$5,$6,CASE WHEN $7 THEN encode($8,'hex')::uuid END,$9,$10,$11)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(arrival.occurrence.as_bytes().as_slice()).bind(key).bind(binding).bind(fare>0).bind(request.fee.transaction_id.as_slice()).bind(i64::try_from(fare).map_err(|_| DurabilityError::InvalidStoredState)?).bind(i64::try_from(route.route.price).map_err(|_| DurabilityError::InvalidStoredState)?).bind(fare<route.route.price).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_character_pending_arrivals(character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(arrival.occurrence.as_bytes().as_slice()).bind(arrival.world_id.as_bytes().as_slice()).bind(arrival.destination.x).bind(arrival.destination.y).bind(arrival.destination.floor).execute(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(Ok(Ok(Some(arrival))))
        })).await?
    }
}

async fn premium<G: NpcTravelGate>(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    route: &TravelRoute,
    gate: &G,
) -> Result<bool, DurabilityError> {
    if !route.route.premium {
        return Ok(true);
    }
    let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    gate.travel_permitted(tx, uuid_text(&account)?).await
}
