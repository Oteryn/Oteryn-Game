//! Coin market composition: Game gold custody, purpose-separated Platform coin authority.
use super::{ComposedFreshAdmission,connection::AdmittedSession};
use crate::durability::coin_market::CoinOwner;
use crate::native_admission_source::coins::{CoinClient,Reply,Request as PlatformRequest};
use oteryn_protocol_oteryn::coin_market as wire;
use serde_json::{Value,json};
macro_rules! authority_methods {()=>{
 fn coin_market_command(&self,_admitted:AdmittedSession,_id:u64,_payload:&[u8])->impl std::future::Future<Output=Option<Vec<u8>>>{async{oteryn_protocol_oteryn::coin_market::encode_reply(r#"{"result":"unavailable"}"#).ok()}}
};}
pub(super) use authority_methods;
macro_rules! composed_methods {()=>{
 async fn coin_market_command(&self,admitted:AdmittedSession,id:u64,payload:&[u8])->Option<Vec<u8>>{crate::gameplay_transport::coin_market::execute(self,admitted,id,payload).await}
};}
pub(super) use composed_methods;
pub async fn reconcile_coin_market(owner:&CoinOwner,coins:&mut CoinClient)->bool {
 if coins.market_enabled() && owner.tick().await.is_err(){return false;}
 let Ok(pending)=owner.pending().await else{return false;};
 for p in pending.into_iter().take(1) {
  match coins.receipt(&p.request).await {
   Ok(Ok(proof))=>{if owner.finish(p.operation,proof).await.is_err(){return false;}},
   Ok(Err(code))=>{if owner.failure(p.operation,code).await.is_err(){return false;}},
   Err(_)=>{if owner.failure(p.operation,"unavailable".into()).await.is_err(){return false;}},
  }
 }
 true
}
fn reply(v:Value)->Option<Vec<u8>>{wire::encode_reply(&v.to_string()).ok()}
async fn admitted_at_depot(owner:&ComposedFreshAdmission<'_, '_, '_>,admitted:AdmittedSession)->bool {
 let Some(actor)=admitted.runtime_actor else{return false;};
 let Some(_permit)=owner.spell_lane_permit().await else{return false;};
 let mut runtime=owner.runtime.lock().await;let states=owner.states.spell_states.lock().await;
 if super::equipment_move::native_for_move(&runtime,owner.active_generation,states.is_dead(actor),states.has_pending_spell_commit(actor,admitted.game_session_id)).is_none(){return false;}
 super::bundle_map_position(&mut runtime,owner.movement_cells(),actor).as_ref().and_then(super::depot::locker).is_some()
}
pub(super) async fn execute(owner:&ComposedFreshAdmission<'_, '_, '_>,admitted:AdmittedSession,id:u64,payload:&[u8])->Option<Vec<u8>> {
 let Ok(r)=wire::decode(payload) else{return reply(json!({"result":"invalid"}));};
 let Some(fence)=admitted.item_fence else{return reply(json!({"result":"rejected"}));};
 if id==0 || !admitted.continuity.selected_capabilities.contains(wire::CAPABILITY) || !admitted.continuity.selected_capabilities.contains(oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1) || admitted.game_session_id!=fence.game_session_id || admitted.continuity.connection_generation!=fence.connection_generation.get() || fence.runtime_scope!=crate::foundation::RuntimeScopeRefV1::channel(admitted.world_id,admitted.channel_id){return reply(json!({"result":"rejected"}));}
 if !admitted_at_depot(owner,admitted).await{return reply(json!({"result":"depot_required"}));}
 if matches!(r.action,3..=6) && crate::durability::bank::fee_payer_is_junior(fence.character_id){return reply(json!({"result":"junior"}));}
 let coin=owner.root.coin_owner(owner.character,owner.holder,admitted.world_id,Some(fence)).ok()?;
 let account=match coin.account().await {Ok(a)=>a,Err(_)=>return reply(json!({"result":"rejected"}))};
 let Some(platform)=&owner.store else{return reply(json!({"result":"unavailable"}));};
 let mut platform=platform.lock().await;let Some(coins)=platform.coins() else{return reply(json!({"result":"unavailable"}));};
 if coins.market_enabled() && coin.tick().await.is_err(){return reply(json!({"result":"unavailable"}));}
 if matches!(r.action,3..=6) && !coins.market_enabled(){return reply(json!({"result":"unavailable"}));}
 let value=match r.action {
  1|2=>coin.view(r.action==2,r.cursor.clone()).await.ok()?,
  3..=6=>{
   let result=coin.prepare(r.clone()).await.ok()?;
   if result["result"]=="ok" {
    reconcile_coin_market(&coin,coins).await;
    let op=uuid(&r.operation);
    coin.detail(op).await.ok()?
   }else{result}
  }
  7=>match coins.execute(&PlatformRequest::History{account,cursor:r.cursor}).await {
   Ok(Reply::History(h))=>json!({"result":"ok","history":h.entries.into_iter().map(|e|json!({"entry":e.entry_id,"operation":e.operation_id,"kind":e.kind,"coin_class":e.coin_class,"available_delta":e.available_delta,"reserved_delta":e.reserved_delta,"occurred_at":e.occurred_at})).collect::<Vec<_>>(),"next_cursor":h.next_cursor}),
   Ok(Reply::Refused(code))=>json!({"result":code}),_=>json!({"result":"unavailable"}),
  },
  8=>{
   let op=uuid(&r.offer);let local=coin.detail(op.clone()).await.ok()?;
   if local["result"]=="not_found" {
    match coins.execute(&PlatformRequest::Operation(op)).await {
     Ok(Reply::Mutation(m)) if m.receipt.account_id==account || m.receipt.counterparty_account_id.as_ref()==Some(&account)=>{
      let receipt=crate::durability::coin_market::public_receipt(serde_json::to_value(&m).ok()?,&account);
      json!({"result":"ok","pending":false,"receipt":receipt})
     },
     _=>json!({"result":"not_found"}),
    }
   }else{local}
  },
  9=>match coins.execute(&PlatformRequest::Balance(account)).await {
   Ok(Reply::Balance(b))=>json!({"result":"ok","transferable":b.transferable,"non_transferable":b.non_transferable,"reserved":b.reserved}),
   Ok(Reply::Refused(code))=>json!({"result":code}),_=>json!({"result":"unavailable"}),
  },
  _=>json!({"result":"invalid"}),
 };
 reply(value)
}
fn uuid(id:&[u8;16])->String{let h=id.iter().map(|b|format!("{b:02x}")).collect::<String>();format!("{}-{}-{}-{}-{}",&h[..8],&h[8..12],&h[12..16],&h[16..20],&h[20..])}
