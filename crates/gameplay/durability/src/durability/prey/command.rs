use super::super::{
    DurabilityError, DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::{
        CharacterProgressionError as Error, CurrentCharacterGameplayFence, assert_gameplay_fence,
        uuid_text,
    },
    character_wheel::WheelPremiumGate,
    db::{begin_type2_transaction, lock_admission_relations},
    item_fee_burn::{
        FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest, FeeChangeFacts,
        burn_fee_in_transaction,
    },
    runtime_scope_assignment::NodeIncarnationProof,
};
use super::{
    draw::{Draw, PreyCatalogue},
    storage,
};
use oteryn_gameplay_progression::prey::{BonusType, ListReroll, PreyBonus, RerollQuote};
use oteryn_protocol_oteryn::prey::{self as wire, Reply, Request, Slot};
use sha2::{Digest, Sha256};
use sqlx::Row;

#[derive(Debug, Clone)]
pub enum PreyMutation {
    Initialize,
    Command(Request),
    /// Trusted hunting seconds since the last acknowledged session checkpoint.
    Checkpoint(u32),
}
impl PreyMutation {
    fn binding(&self) -> Result<Vec<u8>, Error> {
        Ok(match self {
            Self::Initialize => vec![0],
            Self::Command(r) => [
                vec![1],
                wire::encode_request(r).map_err(|_| Error::InvalidInput)?,
            ]
            .concat(),
            Self::Checkpoint(s) => [vec![2], s.to_be_bytes().to_vec()].concat(),
        })
    }
}

impl DurabilityRoot {
    /// Called with the Character revision slot held, with current admission and pinned content.
    /// Returns the bounded wire state and current permanent slot facts for live Premium gating.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_prey_command<G: WheelPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: [u8; 16],
        mutation: PreyMutation,
        catalogue: &PreyCatalogue,
        coins: Option<FeeChangeFacts>,
        gate: &G,
    ) -> Result<(Reply, [bool; 3]), Error> {
        let binding = Sha256::digest(mutation.binding()?).to_vec();
        let recovery = authority
            .record_for(self)
            .map_err(|_| Error::AuthorityRejected)?;
        let node = node.clone();
        let mut gate = gate;
        self.try_issue_semantic_pass()?.run_with_context(&mut gate,move |holder,deadline,gate| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            // Account serialization matches DAILYFX; it also serializes absent balance rows.
            let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            let premium = gate.allocation_permitted(&mut tx,uuid_text(&account)?).await?;
            // Always validate current authority, including on receipt replay.
            let root = match assert_gameplay_fence(&mut tx,&fence,&node).await? { Ok(r)=>r, Err(e)=>return Ok(Err(e)) };
            sqlx::query("SELECT account_id FROM game_character_account_guards WHERE account_id=$1::text::uuid FOR UPDATE").bind(&account).fetch_one(&mut **tx).await?;
            let world = fence.runtime_scope.world_id();
            let mut permanent=[false;3];
            for target in sqlx::query_scalar::<_,i16>("SELECT u.target FROM game_account_prey_unlocks u JOIN game_character_roots r ON r.character_id=u.claiming_character_id WHERE u.account_id=$1::text::uuid AND u.world_id=encode($2,'hex')::uuid AND u.account_id=r.account_id AND u.world_id=r.world_id AND u.unlock_kind='PREY_SLOT_PERMANENT' ORDER BY u.target LIMIT 2")
                .bind(&account).bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await? { permanent[target as usize-1]=true; }
            if !matches!(mutation,PreyMutation::Command(Request::Query)) && let Some(row) = sqlx::query("SELECT request_sha256,result FROM game_character_prey_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence=encode($2,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_slice()).fetch_optional(&mut **tx).await? {
                if row.try_get::<Vec<u8>,_>("request_sha256")? != binding { return Ok(Err(Error::ConflictingOccurrence)); }
                let reply=wire::decode_reply(&row.try_get::<Vec<u8>,_>("result")?).map_err(|_|DurabilityError::InvalidStoredState)?;
                tx.commit(deadline).await?;
                return Ok(Ok((reply,permanent)));
            }
            let character = fence.character_id.as_bytes();
            let vocation: Option<String> = sqlx::query_scalar("SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid").bind(character.as_slice()).fetch_optional(&mut **tx).await?;
            let eligible = vocation.as_deref().is_some_and(|v| !v.is_empty() && v!="none");
            let mut access=[eligible,eligible && premium,false];
            for (i,flag) in permanent.iter().enumerate() {access[i]|=eligible && *flag;}
            let level: i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(character.as_slice()).fetch_one(&mut **tx).await?;
            let level=u32::try_from(level).map_err(|_|DurabilityError::InvalidStoredState)?;
            let balance:i16=sqlx::query_scalar("SELECT balance FROM game_account_prey_wildcards WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE").bind(&account).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.unwrap_or(0);
            let mut reply=storage::load(&mut tx,character,&access,balance as u8).await?;
            if matches!(mutation,PreyMutation::Command(Request::Query)) {
                tx.commit(deadline).await?; return Ok(Ok((reply,permanent)));
            }
            let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&mut **tx).await?;
            let secret:Vec<u8>=sqlx::query_scalar("SELECT secret FROM game_prey_decision_root WHERE singleton").fetch_one(&mut **tx).await?;
            let mut draw=Draw::new(&secret,&occurrence,b"oteryn.prey.list.v1");
            let mut bonus_draw=Draw::new(&secret,&occurrence,b"oteryn.prey.bonus.v1");
            let mut spend=0_i16;
            let mut fee=0;
            let mut dirty=Vec::new();
            match &mutation {
                PreyMutation::Initialize=>{
                    for (i, unlocked) in access.iter().enumerate() {
                        if *unlocked && reply.slots[i].state==0 {
                            // A lapsed slot has a retained row and must not be reinitialized.
                            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_prey_slots WHERE character_id=encode($1,'hex')::uuid AND slot=$2)").bind(character.as_slice()).bind((i+1) as i16).fetch_one(&mut **tx).await?;
                            if exists { continue; }
                            let black=blacklist(&reply,i);
                            let hash=Sha256::digest([b"oteryn.prey.init.v1".as_slice(),character.as_slice(),&[(i+1) as u8]].concat());
                            let mut initial=[0;16]; initial.copy_from_slice(&hash[..16]);
                            reply.slots[i].creatures=Draw::new(&secret,&initial,b"oteryn.prey.list.v1").list(catalogue,level,&black)?;
                            reply.slots[i].state=1;
                            reply.slots[i].free_reroll_at=now as u64;
                            dirty.push(i);
                        }
                    }
                }
                PreyMutation::Checkpoint(seconds)=>{
                    if *seconds==0 || *seconds>7200 { return Ok(Err(Error::InvalidInput)); }
                    for (i, unlocked) in access.iter().enumerate() {
                        if !*unlocked || reply.slots[i].state!=2 { continue; }
                        let slot=&mut reply.slots[i];
                        if slot.hunting_seconds>*seconds { slot.hunting_seconds-=seconds; }
                        else {
                            let cost=match slot.option { 1=>1,2=>5,_=>0 };
                            if cost>0 && balance-spend>=cost {
                                spend+=cost;
                                if slot.option==1 { set_bonus(slot,bonus_draw.bonus(bonus(slot))); }
                                slot.hunting_seconds=7200;
                            } else {
                                slot.state=1; slot.selected.clear();slot.bonus=0;slot.grade=0;slot.percent=0;slot.option=0;slot.hunting_seconds=0;
                                let black=blacklist(&reply,i);
                                reply.slots[i].creatures=draw.list(catalogue,level,&black)?;
                            }
                        }
                        dirty.push(i);
                    }
                }
                PreyMutation::Command(request)=>{
                    let slot_id=match request { Request::Query=>unreachable!(),Request::ListReroll{slot}|Request::Select{slot,..}|Request::SelectAny{slot,..}|Request::BonusReroll{slot}|Request::SetOption{slot,..}=>*slot };
                    if !(1..=3).contains(&slot_id) {return Ok(Err(Error::InvalidInput));}
                    let i=usize::from(slot_id-1);
                    if !access[i] || reply.slots[i].state==0 {return Ok(Err(Error::AuthorityRejected));}
                    let black=blacklist(&reply,i);
                    let slot=&mut reply.slots[i];
                    match request {
                        Request::ListReroll{..}=>{
                            let Ok(quote)=RerollQuote::try_from(ListReroll {now:now as u64,free_reroll_at:slot.free_reroll_at,level}) else {return Ok(Err(Error::InvalidInput));};
                            fee=quote.gold; slot.free_reroll_at=quote.free_reroll_at;
                            slot.state=1;slot.selected.clear();slot.hunting_seconds=0;
                            slot.creatures=draw.list(catalogue,level,&black)?;
                        }
                        Request::Select{index,..}=>{
                            if slot.state!=1 {return Ok(Err(Error::InvalidInput));}
                            let Some(key)=slot.creatures.get(usize::from(*index).wrapping_sub(1)).cloned() else {return Ok(Err(Error::InvalidInput));};
                            if black.contains(&key) {return Ok(Err(Error::InvalidInput));}
                            if catalogue.unsupported_loot.contains(&key) {reply.status=wire::STATUS_CONTENT_UNAVAILABLE;tx.commit(deadline).await?;return Ok(Ok((reply,permanent)));}
                            select(slot,key,&mut bonus_draw);
                        }
                        Request::SelectAny{creature,..}=>{
                            if slot.state!=1 || !catalogue.creatures.iter().any(|c| &c.0==creature) || black.contains(creature) {return Ok(Err(Error::InvalidInput));}
                            if catalogue.unsupported_loot.contains(creature) {reply.status=wire::STATUS_CONTENT_UNAVAILABLE;tx.commit(deadline).await?;return Ok(Ok((reply,permanent)));}
                            spend=5; select(slot,creature.clone(),&mut bonus_draw);
                        }
                        Request::BonusReroll{..}=>{
                            if slot.state!=2 {return Ok(Err(Error::InvalidInput));}
                            spend=1; set_bonus(slot,bonus_draw.bonus(bonus(slot)));slot.hunting_seconds=7200;
                        }
                        Request::SetOption{option,..}=>{
                            let cost=match option {0=>0,1=>1,2=>5,_=>return Ok(Err(Error::InvalidInput))};
                            if balance<cost {return Ok(Err(Error::InvalidInput));} slot.option = *option;
                        }
                        Request::Query=>unreachable!(),
                    }
                    dirty.push(i);
                }
            }
            if spend>balance {return Ok(Err(Error::InvalidInput));}
            if dirty.is_empty() {tx.commit(deadline).await?;return Ok(Ok((reply,permanent)));}
            if fee>0 {
                let slot=reply.slots[dirty[0]].slot;
                let Some(coins)=coins else{return Ok(Err(Error::InvalidInput));};
                let request=FeeBurnRequest { cause:FeeBurnCause::PreyListReroll{slot,occurrence},fee_gold_units:fee,transaction_id:occurrence,event_id:fee_id(occurrence),occurred_at_unix_ms:now*1000,server_build_id:super::super::character_authority::SERVER_BUILD_ID.into(),change:coins };
                match burn_fee_in_transaction(&mut tx,&fence,&request).await { Ok(FeeBurnOutcome::Burned(_))=>(),Err(FeeBurnError::Unavailable(e))=>return Err(e),_=>return Ok(Err(Error::InvalidInput)) }
            }
            reply.wildcard_balance=(balance-spend) as u8;
            for i in dirty {storage::save(&mut tx,character,&reply.slots[i]).await?;}
            let result=wire::encode_reply(&reply).map_err(|_|DurabilityError::InvalidStoredState)?;
            let next=root.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_character_prey_receipts(character_id,occurrence,request_sha256,result,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,wildcards_spent,fee_gold_units) SELECT character_id,encode($2,'hex')::uuid,$3,$4,$5::text::numeric,$6::text::numeric,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,$7,$8 FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_slice()).bind(occurrence.as_slice()).bind(binding).bind(result).bind(root.revision.to_string()).bind(next.to_string()).bind(spend).bind(fee as i64).execute(&mut **tx).await?;
            if spend>0 {
                sqlx::query("INSERT INTO game_account_prey_wildcard_entries(claim_id,account_id,world_id,previous_claim_id,kind,requested,credited,balance_before,balance_after) SELECT encode($3,'hex')::uuid,account_id,world_id,last_claim_id,'SPEND',$4,0,balance,balance-$4 FROM game_account_prey_wildcards WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid")
                    .bind(&account).bind(world.as_bytes().as_slice()).bind(occurrence.as_slice()).bind(spend).execute(&mut **tx).await?;
                sqlx::query("UPDATE game_account_prey_wildcards SET balance=balance-$4,last_claim_id=encode($3,'hex')::uuid WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid").bind(&account).bind(world.as_bytes().as_slice()).bind(occurrence.as_slice()).bind(spend).execute(&mut **tx).await?;
            }
            for query in ["UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric","UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric"] {
                if sqlx::query(query).bind(character.as_slice()).bind(next.to_string()).bind(root.revision.to_string()).execute(&mut **tx).await?.rows_affected()!=1 { return Err(DurabilityError::InvalidStoredState);}
            }
            tx.commit(deadline).await?;Ok(Ok((reply,permanent)))
        })).await?
    }
}
fn blacklist(reply: &Reply, i: usize) -> Vec<String> {
    reply
        .slots
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != i)
        .flat_map(|(_, s)| {
            s.creatures
                .iter()
                .cloned()
                .chain((!s.selected.is_empty()).then_some(s.selected.clone()))
        })
        .collect()
}
fn bonus(slot: &Slot) -> Option<PreyBonus> {
    let kind = match slot.bonus {
        1 => BonusType::Damage,
        2 => BonusType::Reduction,
        3 => BonusType::Experience,
        4 => BonusType::Loot,
        _ => return None,
    };
    Some(PreyBonus {
        kind,
        grade: slot.grade,
        percent: slot.percent,
    })
}
fn set_bonus(slot: &mut Slot, b: PreyBonus) {
    slot.bonus = match b.kind {
        BonusType::Damage => 1,
        BonusType::Reduction => 2,
        BonusType::Experience => 3,
        BonusType::Loot => 4,
    };
    slot.grade = b.grade;
    slot.percent = b.percent;
}
fn select(slot: &mut Slot, key: String, draw: &mut Draw) {
    if slot.bonus == 0 {
        set_bonus(slot, draw.bonus(None));
    }
    slot.creatures.retain(|k| k != &key);
    slot.selected = key;
    slot.state = 2;
    slot.hunting_seconds = 7200;
}
fn fee_id(mut id: [u8; 16]) -> [u8; 16] {
    id[15] ^= 1;
    id
}
