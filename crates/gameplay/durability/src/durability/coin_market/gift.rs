use super::*;
use oteryn_protocol_oteryn::coin_transfer::Request as Gift;
use sha2::{Digest, Sha256};
impl CoinOwner {
    pub async fn gift_review(&self, r: Gift) -> Result<Value> {
        oteryn_protocol_oteryn::coin_transfer::encode(&r).map_err(|_| invalid())?;
        let f = self.fence.ok_or_else(invalid)?;
        self.run(move|tx,id|Box::pin(async move{
   let rows=sqlx::query("SELECT character_id::text,account_id::text,name FROM game_character_roots WHERE lower(name)=lower($1) AND lifecycle=1 ORDER BY character_id LIMIT 2 FOR SHARE").bind(&r.recipient).fetch_all(&mut *tx).await?;
   if rows.len()!=1{return Ok(json!({"result":if rows.is_empty(){"recipient_not_found"}else{"ambiguous_recipient"}}));}
   let row=&rows[0];let recipient:String=row.try_get("character_id")?;let account:String=row.try_get("account_id")?;let name:String=row.try_get("name")?;
   if account==id.account()?{return Ok(json!({"result":"same_account"}));}
   let quote:String=sqlx::query_scalar("INSERT INTO game_coin_gift_quotes(account_id,actor,world_id,session_id,connection_generation,lease_generation,recipient,recipient_account,recipient_name,amount) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::text::numeric,$6::text::numeric,$7::uuid,$8::uuid,$9,$10) RETURNING quote_id::text").bind(id.account()?).bind(id.actor()?).bind(&id.world).bind(uuid(f.game_session_id.as_bytes())?).bind(f.connection_generation.get().to_string()).bind(f.character_lease_generation.to_string()).bind(recipient).bind(account).bind(&name).bind(r.amount as i64).fetch_one(tx).await?;
   Ok(json!({"result":"ok","quote":quote,"recipient":name,"amount":r.amount.to_string(),"expires_in_seconds":60}))
  })).await
    }
    pub async fn gift_confirm(&self, r: Gift) -> Result<Value> {
        let bytes = oteryn_protocol_oteryn::coin_transfer::encode(&r).map_err(|_| invalid())?;
        let f = self.fence.ok_or_else(invalid)?;
        self.run(move|tx,id|Box::pin(async move{
   let op=uuid(&r.operation)?;let quote=uuid(&r.quote)?;
   let mut hash=Sha256::new();hash.update(b"oteryn:coin-gift:v1");hash.update(bytes);hash.update(id.account()?.as_bytes());hash.update(id.world.as_bytes());let binding=hash.finalize().to_vec();
   if let Some(old)=sqlx::query("SELECT binding,phase FROM game_coin_operations WHERE operation_id=$1::uuid").bind(&op).fetch_optional(&mut *tx).await?{
    return Ok(if old.try_get::<Vec<u8>,_>("binding")?==binding{json!({"result":"ok","operation":op,"pending":old.try_get::<i16,_>("phase")?!=1})}else{json!({"result":"idempotency_conflict"})});
   }
   let row=sqlx::query("SELECT recipient::text,recipient_account::text,recipient_name,amount,consumed_by::text FROM game_coin_gift_quotes WHERE quote_id=$1::uuid AND account_id=$2::uuid AND actor=$3::uuid AND world_id=$4::uuid AND session_id=$5::uuid AND connection_generation=$6::text::numeric AND lease_generation=$7::text::numeric AND expires_at>clock_timestamp() FOR UPDATE").bind(quote).bind(id.account()?).bind(id.actor()?).bind(&id.world).bind(uuid(f.game_session_id.as_bytes())?).bind(f.connection_generation.get().to_string()).bind(f.character_lease_generation.to_string()).fetch_optional(&mut *tx).await?;
   let Some(row)=row else{return Ok(json!({"result":"quote_expired"}));};
   if row.try_get::<Option<String>,_>("consumed_by")?.is_some() || row.try_get::<String,_>("recipient_name")?!=r.recipient || row.try_get::<i64,_>("amount")?!=r.amount as i64{return Ok(json!({"result":"quote_changed_or_used"}));}
   let recipient:String=row.try_get("recipient")?;let account:String=row.try_get("recipient_account")?;
   // Root ownership is read and locked at the authority-consuming boundary.
   let locked=sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=$1::uuid AND account_id=$2::uuid AND name=$3 AND lifecycle=1 FOR SHARE").bind(&recipient).bind(&account).bind(&r.recipient).fetch_optional(&mut *tx).await?;
   if locked.is_none() || account==id.account()?{return Ok(json!({"result":"recipient_changed"}));}
   let p=prepare::Plan{operation:op.clone(),world:id.world.clone(),actor:id.actor()?.into(),account:id.account()?.into(),kind:"transfer".into(),seller:Some(id.account()?.into()),buyer:Some(account),seller_character:Some(id.actor()?.into()),buyer_character:Some(recipient),buy_offer:None,sell_offer:None,reservation:None,parent:None,amount:r.amount as i64,price:0,fee:0,held:0,binding,phase:0};
   prepare::insert(tx,&p).await?;
   sqlx::query("UPDATE game_coin_gift_quotes SET consumed_by=$2::uuid WHERE quote_id=$1::uuid").bind(uuid(&r.quote)?).bind(&op).execute(tx).await?;
   Ok(json!({"result":"ok","operation":op,"pending":true}))
  })).await
    }
}
