//! Fenced Game gold custody and durable Platform coin intents. No Game coin balance exists.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_session_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::foundation::WorldId;
use oteryn_gameplay_native_admission_source::coins::{ReceiptProof, Request as PlatformRequest};
use oteryn_protocol_oteryn::coin_market::Request;
use serde_json::{Value, json};
use sqlx::{PgConnection, Row};
use std::{future::Future, pin::Pin};
#[path = "coin_market/finish.rs"]
mod finish;
#[path = "coin_market/gift.rs"]
mod gift;
#[path = "coin_market/jobs.rs"]
mod jobs;
#[path = "coin_market/prepare.rs"]
mod prepare;
type Result<T> = std::result::Result<T, DurabilityError>;
type Work<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
fn invalid() -> DurabilityError {
    DurabilityError::InvalidStoredState
}
#[derive(Clone)]
pub struct CoinOwner {
    root: DurabilityRoot,
    recovery: CharacterRecoveryFenceV1,
    node: NodeIncarnationProof,
    world: WorldId,
    fence: Option<CurrentCharacterItemFence>,
}
struct Identity {
    account: Option<String>,
    actor: Option<String>,
    world: String,
}
impl Identity {
    fn account(&self) -> Result<&str> {
        self.account.as_deref().ok_or_else(invalid)
    }
    fn actor(&self) -> Result<&str> {
        self.actor.as_deref().ok_or_else(invalid)
    }
}
#[derive(Debug, Clone)]
pub struct Pending {
    pub operation: String,
    pub request: PlatformRequest,
}
impl DurabilityRoot {
    pub fn coin_owner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: WorldId,
        fence: Option<CurrentCharacterItemFence>,
    ) -> Result<CoinOwner> {
        if fence.is_some_and(|f| f.runtime_scope.world_id() != world) {
            return Err(invalid());
        }
        Ok(CoinOwner {
            root: self.clone(),
            recovery: authority
                .record_for(self)
                .map_err(|_| DurabilityError::Unavailable)?,
            node: node.clone(),
            world,
            fence,
        })
    }
}
impl CoinOwner {
    async fn run<T: Send + 'static>(
        &self,
        work: impl for<'a> FnOnce(&'a mut PgConnection, Identity) -> Work<'a, T> + Send + 'static,
    ) -> Result<T> {
        let recovery = self.recovery.clone();
        let node = self.node.clone();
        let fence = self.fence;
        let world = self.world;
        self.root.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
   let identity=if let Some(fence)=fence {
    let account=assert_session_fence(&mut tx,&fence,&node).await?.map_err(|_|invalid())?;
    Identity{account:Some(account),actor:Some(uuid(fence.character_id.as_bytes())?),world:uuid(world.as_bytes())?}
   }else{
    if !prove_current_incarnation(&mut tx,&node).await?{return Err(invalid());}
    let fact=node.fact();
    let owned:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND state=1 AND holder_node_id=encode($2,'hex')::uuid AND holder_registration_revision=$3::text::numeric)").bind(world.as_bytes().as_slice()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).fetch_one(&mut *tx).await?;
    if !owned{return Err(invalid());}
    Identity{account:None,actor:None,world:uuid(world.as_bytes())?}
   };
   sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:coin-book:'||$1,0))").bind(&identity.world).execute(&mut *tx).await?;
   let value=work(&mut tx,identity).await?;commit_semantic_transaction(tx,deadline).await?;Ok(value)
  })).await
    }
    pub async fn account(&self) -> Result<String> {
        self.run(|_, id| Box::pin(async move { Ok(id.account()?.to_owned()) }))
            .await
    }
    pub async fn prepare(&self, request: Request) -> Result<Value> {
        self.run(move |tx, id| Box::pin(async move { prepare::execute(tx, id, request).await }))
            .await
    }
    pub async fn pending(&self) -> Result<Vec<Pending>> {
        self.run(|tx,id|Box::pin(async move{
  let rows=sqlx::query("SELECT operation_id::text,kind,account_id::text,seller::text,buyer::text,reservation_id::text,amount,price FROM game_coin_operations WHERE world_id=$1::uuid AND phase=0 AND next_attempt<=clock_timestamp() ORDER BY created_at,operation_id LIMIT 8").bind(&id.world).fetch_all(tx).await?;
  rows.into_iter().map(|r|pending_of(&r)).collect()
 })).await
    }
    pub async fn finish(&self, operation: String, proof: ReceiptProof) -> Result<()> {
        self.run(move |tx, id| {
            Box::pin(async move { finish::execute(tx, &id.world, &operation, &proof).await })
        })
        .await
    }
    pub async fn failure(&self, operation: String, code: String) -> Result<()> {
        self.run(move |tx,id|Box::pin(async move{
  if code.len()>64{return Err(invalid());}
  sqlx::query("UPDATE game_coin_operations SET last_error=$3,next_attempt=clock_timestamp()+interval '5 seconds' WHERE operation_id=$1::uuid AND world_id=$2::uuid AND phase=0").bind(operation).bind(id.world).bind(code).execute(tx).await?;Ok(())
 })).await
    }
    pub async fn tick(&self) -> Result<()> {
        self.run(|tx, id| Box::pin(async move { jobs::tick(tx, &id.world).await }))
            .await
    }
    pub async fn view(&self, own: bool, cursor: String) -> Result<Value> {
        let page = if cursor.is_empty() {
            0
        } else {
            cursor.parse::<i64>().map_err(|_| invalid())?
        };
        if !(0..=if own { 6 } else { 13 }).contains(&page) {
            return Err(invalid());
        }
        self.run(move |tx,id|Box::pin(async move{
  let rows=sqlx::query("WITH visible AS(SELECT o.*,row_number() OVER(PARTITION BY side ORDER BY CASE WHEN side=2 THEN price END,CASE WHEN side=1 THEN -price END,created_at,offer_id) AS side_row FROM game_coin_market_offers o WHERE world_id=$1::uuid AND (($2 AND account_id=$3::uuid) OR (NOT $2 AND state=1))) SELECT o.offer_id::text,c.name,o.side,o.amount,o.remaining,o.price,o.state,extract(epoch FROM o.expires_at)::bigint AS expires FROM visible o JOIN game_character_roots c ON c.character_id=o.character_id WHERE $2 OR o.side_row<=700 ORDER BY o.side,CASE WHEN $2 THEN -extract(epoch FROM o.created_at) END,CASE WHEN o.side=2 THEN o.price END,CASE WHEN o.side=1 THEN -o.price END,o.created_at,o.offer_id LIMIT 101 OFFSET $4").bind(&id.world).bind(own).bind(id.account()?).bind(page*100).fetch_all(&mut *tx).await?;
  let next=if rows.len()>100 && page<if own{6}else{13}{Some((page+1).to_string())}else{None};
  let rows=rows.iter().take(100).map(|r|Ok(json!({"offer":r.try_get::<String,_>("offer_id")?,"name":r.try_get::<String,_>("name")?,"side":if r.try_get::<i16,_>("side")?==1{"buy"}else{"sell"},"amount":r.try_get::<i64,_>("amount")?,"remaining":r.try_get::<i64,_>("remaining")?,"price":r.try_get::<i64,_>("price")?,"state":r.try_get::<i16,_>("state")?,"expires":r.try_get::<i64,_>("expires")?}))).collect::<Result<Vec<_>>>()?;
  let mut recent=Vec::new();
  if own {
   for r in sqlx::query("SELECT operation_id::text,kind,amount,phase,last_error FROM game_coin_operations WHERE account_id=$1::uuid AND world_id=$2::uuid ORDER BY created_at DESC,operation_id DESC LIMIT 100").bind(id.account()?).bind(&id.world).fetch_all(&mut *tx).await?{recent.push(json!({"operation":r.try_get::<String,_>("operation_id")?,"kind":r.try_get::<String,_>("kind")?,"amount":r.try_get::<i64,_>("amount")?,"pending":r.try_get::<i16,_>("phase")?!=1,"error":r.try_get::<String,_>("last_error")?}));}
  }
  Ok(json!({"result":"ok","offers":rows,"next_offer_cursor":next,"recent_operations":recent}))
 })).await
    }
    pub async fn detail(&self, operation: String) -> Result<Value> {
        self.run(move |tx,id|Box::pin(async move{
  let r=sqlx::query("SELECT o.operation_id::text,o.kind,o.amount,o.price,o.phase,o.last_error,r.canonical_receipt FROM game_coin_operations o LEFT JOIN game_coin_receipts r USING(operation_id) WHERE o.operation_id=$1::uuid AND $2::uuid IN(o.account_id,o.seller,o.buyer) LIMIT 1").bind(operation).bind(id.account()?).fetch_optional(tx).await?;
  let Some(r)=r else{return Ok(json!({"result":"not_found"}));};
  let receipt=r.try_get::<Option<String>,_>("canonical_receipt")?.map(|s|serde_json::from_str::<Value>(&s).map_err(|_|invalid())).transpose()?;
  let viewer=id.account()?.to_owned();
  let receipt=receipt.map(|value|public_receipt(value,&viewer));
  Ok(json!({"result":"ok","operation":r.try_get::<String,_>("operation_id")?,"kind":r.try_get::<String,_>("kind")?,"amount":r.try_get::<i64,_>("amount")?,"price":r.try_get::<i64,_>("price")?,"pending":r.try_get::<i16,_>("phase")?!=1,"error":r.try_get::<String,_>("last_error")?,"receipt":receipt}))
 })).await
    }
}
fn pending_of(r: &sqlx::postgres::PgRow) -> Result<Pending> {
    let operation: String = r.try_get("operation_id")?;
    let kind: String = r.try_get("kind")?;
    let amount = u64::try_from(r.try_get::<i64, _>("amount")?).map_err(|_| invalid())?;
    let request = match kind.as_str() {
        "reserve" | "reserve_take" => PlatformRequest::Reserve {
            operation: operation.clone(),
            account: r.try_get("seller")?,
            amount,
            price: u64::try_from(r.try_get::<i64, _>("price")?).map_err(|_| invalid())?,
        },
        "settle" => PlatformRequest::Settle {
            operation: operation.clone(),
            reservation: r.try_get("reservation_id")?,
            buyer: r.try_get("buyer")?,
            amount,
        },
        "transfer" => PlatformRequest::Transfer {
            operation: operation.clone(),
            account: r.try_get("seller")?,
            recipient: r.try_get("buyer")?,
            amount,
        },
        "release" => PlatformRequest::Release {
            operation: operation.clone(),
            account: r.try_get("seller")?,
            reservation: r.try_get("reservation_id")?,
        },
        _ => return Err(invalid()),
    };
    Ok(Pending { operation, request })
}
fn uuid(bytes: &[u8]) -> Result<String> {
    if bytes.len() != 16 {
        return Err(invalid());
    }
    let h = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    ))
}
#[allow(
    clippy::too_many_arguments,
    reason = "One gold movement explicitly binds its operation, account, actor, World and conservation deltas"
)]
async fn gold(
    tx: &mut PgConnection,
    op: &str,
    account: &str,
    actor: &str,
    world: &str,
    label: &str,
    delta: i64,
    escrow: i64,
    burn: i64,
) -> Result<()> {
    sqlx::query("SELECT game_coin_gold_move(game_market_identity($1::uuid,$6),$1::uuid,$2::uuid,$3::uuid,$4::uuid,$5,$7,$8)").bind(op).bind(account).bind(actor).bind(world).bind(delta).bind(label).bind(escrow).bind(burn).execute(tx).await?;
    Ok(())
}
async fn lock_accounts(tx: &mut PgConnection, world: &str, accounts: &[String]) -> Result<()> {
    sqlx::query("SELECT account_id FROM game_character_account_guards WHERE account_id::text=ANY($1) ORDER BY account_id FOR UPDATE").bind(accounts).fetch_all(&mut *tx).await?;
    let mut accounts = accounts.to_vec();
    accounts.sort();
    accounts.dedup();
    for a in &accounts {
        sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES($1::uuid,$2::uuid,0) ON CONFLICT DO NOTHING").bind(a).bind(world).execute(&mut *tx).await?;
    }
    sqlx::query("SELECT account_id FROM game_account_bank_balances WHERE world_id=$1::uuid AND account_id::text=ANY($2) ORDER BY account_id FOR UPDATE").bind(world).bind(&accounts).fetch_all(tx).await?;
    Ok(())
}

/// Party-bound projection: a counterparty may see a receipt but never another wallet snapshot.
pub fn public_receipt(mut value: Value, viewer: &str) -> Value {
    if let Some(object) = value.get_mut("receipt").and_then(Value::as_object_mut) {
        if object.get("account_id").and_then(Value::as_str) != Some(viewer) {
            for field in [
                "transferable_after",
                "non_transferable_after",
                "reserved_after",
            ] {
                object.remove(field);
            }
        }
        object.remove("account_id");
        object.remove("counterparty_account_id");
    }
    value
}
