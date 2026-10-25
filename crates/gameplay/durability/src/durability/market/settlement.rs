//! Native Market transaction adapter. All SQL mutations go through 0191's definer.
use super::*;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::character_progression::assert_market_item_fence;
use crate::durability::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use oteryn_protocol_oteryn::market::{self as wire, MarketReply, MarketRequest};
use prost::Message;
use sha2::{Digest, Sha256};

impl DurabilityRoot {
    /// Recovery, full current session/lease/node fence, then the sole SQL owner.
    /// A lost commit acknowledgement is retried using the same CommandRef and binding.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_market(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        request: MarketRequest,
        definition: Option<super::super::item_mint::TypedDefinitionRef>,
        town: u16,
    ) -> Result<MarketReply> {
        let encoded = wire::encode_request(request).map_err(|_| invalid())?;
        if command.game_session_id() != fence.game_session_id || !matches!(request.action, 5..=8) {
            return Err(invalid());
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut hash = Sha256::new();
        hash.update(b"oteryn:market:v1");
        hash.update(fence.character_id.as_bytes());
        hash.update(fence.runtime_scope.world_id().as_bytes());
        hash.update(&encoded);
        if let Some(definition) = &definition {
            hash.update(definition.production_key.as_bytes());
            hash.update([0]);
            hash.update(definition.revision_ref.as_bytes());
        }
        let binding: [u8; 32] = hash.finalize().into();
        let mut id = Sha256::new();
        id.update(b"oteryn:market-transaction:v1");
        id.update(command.game_session_id().as_bytes());
        id.update(command.command_id().get().to_be_bytes());
        let digest = id.finalize();
        let mut transaction = [0; 16];
        transaction.copy_from_slice(&digest[..16]);
        transaction[6] = (transaction[6] & 15) | 0x70;
        transaction[8] = (transaction[8] & 63) | 0x80;
        let offer = if matches!(request.action, 5 | 6) {
            transaction
        } else {
            request.offer
        };
        let kind: i16 = match request.action {
            5 | 6 => 1,
            7 => 3,
            8 => 2,
            _ => return Err(invalid()),
        };
        let placement = if matches!(request.action, 5 | 6) {
            let side = if request.action == 5 {
                Side::Buy
            } else {
                Side::Sell
            };
            Some(
                rules::plan_placement(side, request.amount, request.price)
                    .map_err(|_| invalid())?,
            )
        } else {
            None
        };
        let fee_cause = if kind == 1 {
            Some(
                super::super::item_fee_burn_audit::OneItemFeeBurnCauseV1 {
                    cause: Some(
                        super::super::item_fee_burn::FeeBurnCause::MarketFee {
                            offer,
                            occurrence: transaction,
                        }
                        .audit(),
                    ),
                }
                .encode_to_vec(),
            )
        } else {
            None
        };
        let (key, revision) = definition
            .map(|d| (d.production_key, d.revision_ref))
            .unwrap_or_default();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            assert_market_item_fence(&mut tx,&fence,&node).await?.map_err(|_|DurabilityError::InvalidStoredState)?;
            let outcome:Vec<u8>=sqlx::query_scalar("SELECT game_market_write(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5::text::numeric,$6::text::numeric,encode($7,'hex')::uuid,$8,encode($9,'hex')::uuid,$10,$11,$12,$13,$14,$15,$16,$17,$18)")
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string()).bind(command.command_id().get().to_string())
                .bind(transaction.as_slice()).bind(kind).bind(offer.as_slice()).bind(key).bind(revision).bind(if request.action==5 {1_i16}else{2_i16})
                .bind(i32::try_from(request.amount).map_err(|_|invalid())?).bind(i64::try_from(request.price).map_err(|_|invalid())?).bind(request.anonymous).bind(i32::from(town)).bind(binding.as_slice()).bind(fee_cause)
                .fetch_one(&mut *tx).await?;
            if outcome.len()!=17 || !matches!(outcome[0],1..=10 | 15) || outcome[1..]!=offer {return Err(invalid());}
            if let Some(plan)=placement.filter(|_|outcome[0]==1) {
                let values=sqlx::query("SELECT fee,escrow_delta FROM game_market_settlements WHERE transaction_id=encode($1,'hex')::uuid AND kind=1 LIMIT 1").bind(transaction.as_slice()).fetch_one(&mut *tx).await?;
                if values.try_get::<i64,_>("fee")?!=i64::try_from(plan.fee).map_err(|_|invalid())?
                    || values.try_get::<i64,_>("escrow_delta")?!=i64::try_from(plan.escrow).map_err(|_|invalid())?
                    || plan.total!=u64::from(request.amount)*request.price {return Err(invalid());}
            }
            let reply=MarketReply{action:request.action,code:outcome[0],offer,rows:Vec::new()};
            commit_semantic_transaction(tx,deadline).await?;
            Ok(reply)
        })).await
    }
}

impl DurabilityRoot {
    /// Private bounded projection, after the same recovery and native session fences.
    pub async fn query_market(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: MarketRequest,
        definition_key: String,
    ) -> Result<Vec<(Offer, Option<u32>)>> {
        wire::encode_request(request).map_err(|_| invalid())?;
        if !matches!(request.action, 1..=4 | 8) {
            return Err(invalid());
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            assert_market_item_fence(&mut tx,&fence,&node).await?.map_err(|_|invalid())?;
            advance_jobs(&mut tx, fence.runtime_scope.world_id()).await?;
            let rows = if request.action == 8 {
                sqlx::query("SELECT offer_id::text,world_id::text,definition_key,definition_revision,side,account_id::text,character_id::text,piece_price::text,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold::text,held_credit_gold::text,matching_pending,NULL::integer AS history_amount FROM game_market_offers WHERE offer_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid LIMIT 1")
                    .bind(request.offer.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_all(&mut *tx).await?
            } else if request.action == 4 {
                sqlx::query("SELECT o.offer_id::text,o.world_id::text,o.definition_key,o.definition_revision,o.side,o.account_id::text,o.character_id::text,o.piece_price::text,o.amount,o.remaining,o.anonymous,o.created_at_us,o.expires_at_us,o.state,o.ended_as,o.escrow_gold::text,o.held_credit_gold::text,o.matching_pending,h.amount AS history_amount FROM game_market_history h JOIN game_market_offers o USING(offer_id) WHERE h.character_id=encode($1,'hex')::uuid AND o.world_id=encode($2,'hex')::uuid ORDER BY h.occurred_at_us DESC,h.transaction_id DESC LIMIT least(32,600-$3) OFFSET $3")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(i64::from(request.page)*32).fetch_all(&mut *tx).await?
            } else {
                sqlx::query("SELECT o.offer_id::text,o.world_id::text,o.definition_key,o.definition_revision,o.side,o.account_id::text,o.character_id::text,o.piece_price::text,o.amount,o.remaining,o.anonymous,o.created_at_us,o.expires_at_us,o.state,o.ended_as,o.escrow_gold::text,o.held_credit_gold::text,o.matching_pending,NULL::integer AS history_amount FROM game_market_offers o WHERE world_id=encode($1,'hex')::uuid AND CASE $2 WHEN 1 THEN false WHEN 2 THEN definition_key=$3 AND state=1 AND expires_at_us>floor(extract(epoch FROM clock_timestamp())*1000000)::bigint WHEN 3 THEN character_id=encode($4,'hex')::uuid AND state=1 ELSE false END ORDER BY CASE WHEN $2=2 THEN side END,CASE WHEN $2=2 AND side=2 THEN piece_price END ASC,CASE WHEN $2=2 AND side=1 THEN piece_price END DESC,created_at_us,offer_id LIMIT 32 OFFSET $5")
                    .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(i32::from(request.action)).bind(definition_key).bind(fence.character_id.as_bytes().as_slice()).bind(i64::from(request.page)*32).fetch_all(&mut *tx).await?
            };
            let offers=rows.iter().map(|row| Ok((decode(row)?,row.try_get::<Option<i32>,_>("history_amount")?.map(|n|u32::try_from(n).map_err(|_|invalid())).transpose()?))).collect::<Result<Vec<_>>>()?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(offers)
        })).await
    }
}

async fn advance_jobs(
    tx: &mut Transaction<'_, Postgres>,
    world: crate::foundation::WorldId,
) -> Result<bool> {
    let rows=sqlx::query("SELECT o.offer_id::text,o.state,o.definition_key,o.matching_pending,o.expires_at_us<=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint AS expired FROM game_market_offers o LEFT JOIN game_account_bank_balances b ON b.account_id=o.account_id AND b.world_id=o.world_id WHERE o.world_id=encode($1,'hex')::uuid AND ((o.state=1 AND o.expires_at_us<=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint) OR (o.state=5 AND (o.held_credit_gold+coalesce(b.balance,0)<=9000000000000000 OR EXISTS(SELECT 1 FROM game_item_market_escrow_locations e WHERE e.offer_id=o.offer_id))) OR (o.state=1 AND o.matching_pending)) ORDER BY o.expires_at_us,o.offer_id LIMIT 100")
        .bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    for row in rows {
        let offer = uuid_text(row.try_get("offer_id")?)?;
        let book=sqlx::query("SELECT 1 FROM game_market_books WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 FOR UPDATE SKIP LOCKED")
            .bind(world.as_bytes().as_slice()).bind(row.try_get::<String,_>("definition_key")?).fetch_optional(&mut **tx).await?;
        if book.is_none() {
            continue;
        }

        if row.try_get::<bool, _>("matching_pending")? && !row.try_get::<bool, _>("expired")? {
            sqlx::query("SELECT game_market_match(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                .bind(world.as_bytes().as_slice())
                .bind(offer.as_slice())
                .execute(&mut **tx)
                .await?;
        } else if row.try_get::<i16, _>("state")? == 5 {
            sqlx::query(
                "SELECT game_market_release(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
            )
            .bind(world.as_bytes().as_slice())
            .bind(offer.as_slice())
            .execute(&mut **tx)
            .await?;
        } else {
            sqlx::query("SELECT game_market_write(encode($1,'hex')::uuid,NULL,NULL,NULL,NULL,NULL,game_market_identity(encode($2,'hex')::uuid,'expire'),4::smallint,encode($2,'hex')::uuid,NULL,NULL,NULL,NULL,NULL,NULL,NULL,sha256($2),NULL)")
                .bind(world.as_bytes().as_slice()).bind(offer.as_slice()).execute(&mut **tx).await?;
        }
        return Ok(true);
    }
    Ok(false)
}

impl DurabilityRoot {
    /// One restart-safe World step, authorized by current recovery and node ownership.
    pub async fn advance_market(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
    ) -> Result<bool> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !super::super::runtime_scope_assignment::prove_current_incarnation(&mut tx,&node).await? {return Err(DurabilityError::Unavailable);}
            let fact=node.fact();
            let current=sqlx::query("SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1 AND holder_node_id=encode($2,'hex')::uuid AND holder_registration_revision=$3::text::numeric AND state=1 ORDER BY scope_key LIMIT 1 FOR SHARE")
                .bind(world.as_bytes().as_slice()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).fetch_optional(&mut *tx).await?;
            if current.is_none() {return Err(DurabilityError::Unavailable);}
            let progress=advance_jobs(&mut tx,world).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(progress)
        })).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn market_fee_cause_binds_offer_and_occurrence() {
        let mut offer = [1; 16];
        offer[6] = 0x70;
        offer[8] = 0x80;
        let mut occurrence = offer;
        occurrence[15] = 2;
        let cause =
            super::super::super::item_fee_burn::FeeBurnCause::MarketFee { offer, occurrence };
        let wire = super::super::super::item_fee_burn_audit::OneItemFeeBurnCauseV1 {
            cause: Some(cause.audit()),
        }
        .encode_to_vec();
        let expected = [
            vec![0x32, 0x24, 0x0a, 0x10],
            offer.to_vec(),
            vec![0x12, 0x10],
            occurrence.to_vec(),
        ]
        .concat();
        assert_eq!(wire, expected);
    }
}
