//! WB2/P02: current-session, root-serialized boss slots and atomic native gold fees.
use super::*;
use crate::domain::bosstiary::{loot_bonus_percent, remove_fee, slot_unlocked};
use crate::durability::charm_state::CharmRemoveFee;
use crate::durability::db::begin_type2_transaction;
use crate::durability::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnRequest, burn_fee_in_transaction,
};
use oteryn_protocol_oteryn::bosstiary_slots::{BossSlotReply, BossSlotRequest};
use serde_json::{Value, json};

impl DurabilityRoot {
    /// `catalogue` comes from the admitted canonical boss definitions, never request categories.
    /// Query and rejected operations have no durable writes. Every successful mutation retains
    /// its exact request/reply and atomically projects the slot state alongside any coin fee.
    #[allow(
        clippy::too_many_arguments,
        reason = "one fenced owner boundary and its admitted content"
    )]
    pub async fn bosstiary_slots(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: [u8; 16],
        request: BossSlotRequest,
        catalogue: Vec<BosstiaryBoss>,
        fee: Option<CharmRemoveFee>,
    ) -> Result<BossSlotReply, CharacterProgressionError> {
        let bytes = request
            .encode()
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let (request, detailed) = match request {
            BossSlotRequest::Detailed(inner) => (*inner, true),
            plain => (plain, false),
        };
        if catalogue.len() > 1024 {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(e)=assert_gameplay_fence(&mut tx,&fence,&node).await? {return Ok(Err(e));}
            let retained:Option<(Vec<u8>,Vec<u8>)>=sqlx::query_as("SELECT request,reply FROM game_character_boss_slot_operations WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_slice()).fetch_optional(&mut **tx).await?;
            if let Some((before,reply))=retained {
                if before!=bytes {return Ok(Err(CharacterProgressionError::InvalidInput));}
                let reply=BossSlotReply::decode(&reply).ok_or(DurabilityError::InvalidStoredState)?;
                tx.commit(deadline).await?;return Ok(Ok(reply));
            }
            let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut **tx).await?;
            let now=u64::try_from(now).map_err(|_|DurabilityError::InvalidStoredState)?;
            let daily:Option<(String,String)>=sqlx::query_as("SELECT a.reset_epoch::text,b.boss_key FROM game_world_reset_active a JOIN game_world_daily_boosts b USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid")
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            let (epoch,boosted)=daily.unwrap_or_else(||("0".into(),String::new()));
            let previous:Option<Value>=sqlx::query_scalar("SELECT state FROM game_character_boss_slots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            let mut bosses=[String::new(),String::new()];
            let mut removals=1_u8;let mut ready=0;
            if let Some(state)=&previous {
                for (i,boss) in bosses.iter_mut().enumerate() {*boss=state["bosses"][i].as_str().ok_or(DurabilityError::InvalidStoredState)?.into();}
                ready=state["ready"].as_u64().ok_or(DurabilityError::InvalidStoredState)?;
                if state["epoch"].as_str()==Some(epoch.as_str()) {removals=state["removals"].as_u64().and_then(|n|u8::try_from(n).ok()).ok_or(DurabilityError::InvalidStoredState)?;}
            }
            // Crystal io_bosstiary.cpp:119-127 clears each daily boss permanently, including
            // epochs skipped while this character is offline.
            let since=previous.as_ref().and_then(|s|s["epoch"].as_str()).unwrap_or("0");
            let cleared:Vec<String>=sqlx::query_scalar("SELECT DISTINCT boss_key FROM game_world_daily_boosts WHERE world_id=encode($1,'hex')::uuid AND reset_epoch>$2::text::numeric AND reset_epoch<=$3::text::numeric AND boss_key=ANY($4) LIMIT 2")
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(since).bind(&epoch).bind(&bosses[..]).fetch_all(&mut **tx).await?;
            for boss in &mut bosses {if *boss==boosted || cleared.contains(boss) {boss.clear();}}
            let keys:Vec<&str>=catalogue.iter().map(|b|b.key.as_str()).collect();
            let rows:Vec<(String,i64,i64)>=sqlx::query_as("SELECT boss_key,kill_count,boss_points FROM game_character_bosstiary_progress WHERE character_id=encode($1,'hex')::uuid AND boss_key=ANY($2) LIMIT 1024")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&keys).fetch_all(&mut **tx).await?;
            let points=rows.iter().try_fold(0_u32,|n,(_,_,p)|u32::try_from(*p).ok().and_then(|p|n.checked_add(p))).ok_or(DurabilityError::InvalidStoredState)?;
            let qualifies=|key:&str|catalogue.iter().find(|b|b.key==key).and_then(|b|rows.iter().find(|r|r.0==key).and_then(|r|u64::try_from(r.1).ok()).and_then(|n|progress(&b.category,n))).is_some_and(|p|p.stage>=1);
            let prowess=rows.iter().any(|r|r.0!=boosted && qualifies(&r.0));
            let display_points=if detailed {
                let n:i64=sqlx::query_scalar("SELECT COALESCE(sum(boss_points),0)::bigint FROM game_character_bosstiary_progress WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                u32::try_from(n).map_err(|_|DurabilityError::InvalidStoredState)?
            }else{points};
            let details=|assigned:&[String;2]| detailed.then(||slot_details(assigned,&catalogue,&rows,display_points,&boosted,now));
            let mut reply=BossSlotReply {details:details(&bosses),status:0,unlocked:u8::from(slot_unlocked(1,points,prowess)) | (u8::from(slot_unlocked(2,points,prowess))<<1),bonus:loot_bonus_percent(points,false),points,remove_fee:remove_fee(removals),ready_at_ms:ready,bosses:bosses.clone()};
            let unchanged=reply.clone();
            let mut paid=0;
            match &request {
                BossSlotRequest::Detailed(_)=>return Ok(Err(CharacterProgressionError::InvalidInput)),
                BossSlotRequest::Query=>{tx.commit(deadline).await?;return Ok(Ok(reply));}
                BossSlotRequest::Equip {slot,boss}=>{
                    let at=usize::from(*slot-1);
                    reply.status=if !slot_unlocked(*slot,points,prowess) || boss==&boosted || !qualifies(boss) {1}
                        else if !bosses[at].is_empty() || bosses.contains(boss) {2}
                        else if now<ready {3} else {0};
                    if reply.status==0 {bosses[at]=boss.clone();}
                }
                BossSlotRequest::Remove {slot}=>{
                    let at=usize::from(*slot-1);
                    reply.status=if !slot_unlocked(*slot,points,prowess) {1} else if bosses[at].is_empty() {2} else if now<ready {3} else {0};
                    if reply.status==0 {paid=remove_fee(removals);bosses[at].clear();removals=removals.saturating_add(1);}
                }
            }
            if reply.status!=0 {return Ok(Ok(reply));}
            // Crystal game.cpp:11068-11093, config.lua.dist:469: UI action cooldown 500-10ms.
            ready=now.checked_add(490).ok_or(DurabilityError::InvalidStoredState)?;
            reply.details=details(&bosses);reply.bosses=bosses.clone();reply.ready_at_ms=ready;reply.remove_fee=remove_fee(removals);
            let after=json!({"bosses":bosses,"removals":removals,"epoch":epoch,"ready":ready});
            let transaction=fee.as_ref().filter(|_|paid>0).map(|f|f.transaction_id);
            sqlx::query("INSERT INTO game_character_boss_slot_operations(character_id,occurrence_id,ordinal,request,reply,state_before,state_after,fee_gold,fee_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,COALESCE((SELECT max(ordinal)+1 FROM game_character_boss_slot_operations WHERE character_id=encode($1,'hex')::uuid),1),$3,$4,$5,$6,$7,CASE WHEN $8::bytea IS NULL THEN NULL ELSE encode($8,'hex')::uuid END)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_slice()).bind(&bytes).bind(reply.encode().ok_or(DurabilityError::InvalidStoredState)?).bind(&previous).bind(&after).bind(i64::try_from(paid).map_err(|_|DurabilityError::InvalidStoredState)?).bind(transaction.as_ref().map(|v|v.as_slice())).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_character_boss_slots(character_id,state) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(character_id) DO UPDATE SET state=EXCLUDED.state")
                .bind(fence.character_id.as_bytes().as_slice()).bind(after).execute(&mut **tx).await?;
            if paid>0 {
                let Some(fee)=fee else {return Ok(Err(CharacterProgressionError::InvalidInput));};
                let burn=FeeBurnRequest {cause:FeeBurnCause::BosstiarySlot {occurrence},fee_gold_units:paid,transaction_id:fee.transaction_id,event_id:fee.event_id,occurred_at_unix_ms:fee.occurred_at_unix_ms,server_build_id:"oteryn-game-server".into(),change:fee.change};
                match burn_fee_in_transaction(&mut tx,&fence,&burn).await {
                    Ok(crate::durability::item_fee_burn::FeeBurnOutcome::Burned(_))=>(),
                    Ok(_)=>return Err(DurabilityError::InvalidStoredState),
                    Err(FeeBurnError::InsufficientFunds)=>{let mut rejected=unchanged;rejected.status=4;return Ok(Ok(rejected));}
                    Err(FeeBurnError::ChangeDoesNotFit|FeeBurnError::CapacityExceeded)=>{let mut rejected=unchanged;rejected.status=5;return Ok(Ok(rejected));}
                    Err(FeeBurnError::Unavailable(e))=>return Err(e),Err(_)=>return Ok(Err(CharacterProgressionError::InvalidInput)),
                }
            }
            tx.commit(deadline).await?;Ok(Ok(reply))
        })).await?
    }
}

impl DurabilityRoot {
    /// Scope-owned settlement reads bounded, admitted participants. A slot command after the
    /// death cannot retroactively earn bonus loot: select its retained operation at death time.
    #[allow(
        clippy::too_many_arguments,
        reason = "frozen scope death and bounded participant census"
    )]
    pub async fn boss_slot_loot_bonuses(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: crate::foundation::RuntimeScopeRefV1,
        generation: crate::foundation::ScopeOwnershipGeneration,
        boss: BosstiaryBoss,
        participants: Vec<[u8; 16]>,
        died_at_ms: u64,
        boosted: bool,
    ) -> Result<Vec<([u8; 16], u16)>, DurabilityError> {
        if participants.len() > 50 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !crate::durability::character_progression::assert_runtime_scope_fence(&mut tx,scope,generation,&node).await? {return Err(DurabilityError::InvalidStoredState);}
            let mut out=Vec::with_capacity(participants.len());
            for character in participants {
                let state:Option<Value>=sqlx::query_scalar("SELECT o.state_after FROM game_character_boss_slot_operations o JOIN game_character_roots r USING(character_id) WHERE o.character_id=encode($1,'hex')::uuid AND r.world_id=encode($2,'hex')::uuid AND o.occurred_at_ms<=$3::text::numeric AND NOT EXISTS(SELECT 1 FROM game_world_daily_boosts b JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE b.world_id=r.world_id AND b.boss_key=$4 AND b.reset_epoch>(o.state_after->>'epoch')::numeric AND e.activated_at<=to_timestamp($3::numeric/1000)) ORDER BY o.ordinal DESC LIMIT 1")
                    .bind(character.as_slice()).bind(scope.world_id().as_bytes().as_slice()).bind(died_at_ms.to_string()).bind(&boss.key).fetch_optional(&mut *tx).await?;
                let assigned=!boosted && state.as_ref().is_some_and(|s|s["bosses"].as_array().is_some_and(|a|a.iter().any(|v|v.as_str()==Some(boss.key.as_str()))));
                // Crystal config.lua.dist:243: boosted boss has +250% equipment loot.
                let bonus=if boosted {250} else if assigned {
                    let (points,kills):(i64,i64)=sqlx::query_as("SELECT COALESCE(sum(boss_points),0)::bigint,COALESCE(max(kill_count) FILTER(WHERE boss_key=$2),0)::bigint FROM game_character_bosstiary_progress WHERE character_id=encode($1,'hex')::uuid")
                        .bind(character.as_slice()).bind(&boss.key).fetch_one(&mut *tx).await?;
                    let mastered=progress(&boss.category,u64::try_from(kills).map_err(|_|DurabilityError::InvalidStoredState)?).is_some_and(|p|p.stage==3);
                    loot_bonus_percent(u32::try_from(points).map_err(|_|DurabilityError::InvalidStoredState)?,mastered)
                } else {0};
                out.push((character,bonus));
            }
            commit_semantic_transaction(tx,deadline).await?;Ok(out)
        })).await
    }
}

mod details;
use details::slot_details;
