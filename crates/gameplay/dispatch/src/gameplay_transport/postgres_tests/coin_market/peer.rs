//! Contract-exact test producer. This is never mounted by a production node.
use crate::native_admission_source::{coins as c,store,descriptor::Operation};
use std::{collections::BTreeMap,future::Future,pin::Pin,sync::{Arc,Mutex}};
#[derive(Default)]pub(super) struct Wallet{pub transferable:i64,pub non_transferable:i64,pub reserved:i64}
#[derive(Default)]pub(super) struct State {
 pub wallets:BTreeMap<String,Wallet>,pub reservations:BTreeMap<String,(String,i64)>,pub receipts:BTreeMap<String,(String,c::Mutation)>,
 pub entries:BTreeMap<String,Vec<c::Entry>>,pub dropped_ack:bool,pub mutations:usize,sequence:u64,
}
pub(super) type Shared=Arc<Mutex<State>>;
pub(super) struct Peer(pub Shared);
fn refusal(code:&str,status:u16)->(u16,Vec<u8>){(status,format!("{{\"result\":\"{code}\"}}").into_bytes())}
impl c::Transport for Peer {
 fn exchange<'a>(&'a mut self,method:&'a str,path:&'a str,body:&'a str)->Pin<Box<dyn Future<Output=Result<(u16,Vec<u8>),c::Error>>+Send+'a>>{
  Box::pin(async move{
   let mut s=self.0.lock().map_err(|_|c::Error::Unavailable)?;
   if method=="GET" {
    assert!(body.is_empty());
    if let Some(id)=path.strip_prefix("/internal/v1/coins/operations/"){
     return Ok(if let Some((_,r))=s.receipts.get(id){(200,serde_json::to_vec(r)?)}else{refusal("unknown_account",422)});
    }
    let rest=path.strip_prefix("/internal/v1/coins/accounts/").ok_or(c::Error::Invalid)?;let (a,resource)=rest.split_once('/').ok_or(c::Error::Invalid)?;
    let Some(w)=s.wallets.get(a) else{return Ok(refusal("unknown_account",422));};
    if resource=="balance" {return Ok((200,serde_json::to_vec(&c::Balance{result:"ok".into(),account_id:a.into(),transferable:w.transferable.to_string(),non_transferable:w.non_transferable.to_string(),reserved:w.reserved.to_string()})?));}
    let query=resource.strip_prefix("history?cursor=").ok_or(c::Error::Invalid)?;let (cursor,_)=query.split_once("&limit=25").ok_or(c::Error::Invalid)?;
    let mut rows=s.entries.get(a).cloned().unwrap_or_default();rows.sort_by(|a,b|b.entry_id.cmp(&a.entry_id));rows.retain(|e|cursor.is_empty()||e.entry_id.as_str()<cursor);rows.truncate(25);
    let next=rows.last().map(|e|e.entry_id.clone());return Ok((200,serde_json::to_vec(&c::History{result:"ok".into(),entries:rows,next_cursor:next})?));
   }
   assert_eq!(method,"POST");let value:serde_json::Value=serde_json::from_str(body)?;
   let id=value["operation_id"].as_str().ok_or(c::Error::Invalid)?.to_owned();
   let binding=format!("{method} {path} {body}");
   if let Some((original,r))=s.receipts.get(&id){
    if original!=&binding{return Ok(refusal("idempotency_conflict",409));}
    let mut r=r.clone();r.result="replay".into();return Ok((200,serde_json::to_vec(&r)?));
   }
   let field=|k:&str|value[k].as_str().map(str::to_owned).ok_or(c::Error::Invalid);
   let amount=||field("amount")?.parse::<i64>().map_err(|_|c::Error::Invalid);
   let (account,other,n,kind)=if path.ends_with("market/offers"){
    let a=field("seller_account_id")?;let n=amount()?;let price=field("gold_price")?.parse::<u64>().map_err(|_|c::Error::Invalid)?;
    assert_eq!(c::Request::Reserve{operation:id.clone(),account:a.clone(),amount:n as u64,price}.wire()?.2,body);
    let Some(w)=s.wallets.get_mut(&a)else{return Ok(refusal("unknown_account",422));};
    if w.transferable<n{return Ok(refusal("insufficient_transferable",409));}
    w.transferable-=n;w.reserved+=n;s.reservations.insert(id.clone(),(a.clone(),n));(a,None,n,"reserve")
   }else if path.ends_with("market/settlements"){
    let reservation=field("reservation_id")?;let buyer=field("buyer_account_id")?;let n=amount()?;
    assert_eq!(c::Request::Settle{operation:id.clone(),reservation:reservation.clone(),buyer:buyer.clone(),amount:n as u64}.wire()?.2,body);
    let Some((a,left))=s.reservations.get(&reservation).cloned()else{return Ok(refusal("insufficient_transferable",409));};
    if left<n{return Ok(refusal("insufficient_transferable",409));}
    s.reservations.insert(reservation,(a.clone(),left-n));s.wallets.get_mut(&a).ok_or(c::Error::Invalid)?.reserved-=n;s.wallets.get_mut(&buyer).ok_or(c::Error::Invalid)?.transferable+=n;(a,Some(buyer),n,"settlement")
   }else if path.ends_with("market/cancellations"){
    let reservation=field("reservation_id")?;let a=field("seller_account_id")?;
    assert_eq!(c::Request::Release{operation:id.clone(),account:a.clone(),reservation:reservation.clone()}.wire()?.2,body);
    let Some((owner,left))=s.reservations.get(&reservation).cloned()else{return Ok(refusal("insufficient_transferable",409));};
    if a!=owner||left==0{return Ok(refusal("insufficient_transferable",409));}
    s.reservations.insert(reservation,(a.clone(),0));let w=s.wallets.get_mut(&a).ok_or(c::Error::Invalid)?;w.reserved-=left;w.transferable+=left;(a,None,left,"release")
   }else{return Err(c::Error::Invalid)};
   let w=s.wallets.get(&account).ok_or(c::Error::Invalid)?;
   let receipt=c::Receipt{operation_id:id.clone(),kind:kind.into(),account_id:account.clone(),counterparty_account_id:other.clone(),amount:n.to_string(),transferable_after:w.transferable.to_string(),non_transferable_after:w.non_transferable.to_string(),reserved_after:w.reserved.to_string(),occurred_at:"2026-11-09T00:00:00Z".into()};
   let r=c::Mutation{result:"ok".into(),receipt};s.mutations+=1;s.receipts.insert(id.clone(),(binding,r.clone()));s.sequence+=1;
   let entry=c::Entry{entry_id:format!("019a65f0-0000-7000-8000-{:012x}",s.sequence),operation_id:id,kind:kind.into(),coin_class:"transferable".into(),counterparty_account_id:other.clone(),available_delta:if kind=="reserve"{-n}else if kind=="release"{n}else{0}.to_string(),reserved_delta:if kind=="reserve"{n}else{-n}.to_string(),occurred_at:"2026-11-09T00:00:00Z".into()};
   s.entries.entry(account.clone()).or_default().push(entry.clone());if let Some(other)=other{let mut e=entry;e.available_delta=n.to_string();e.reserved_delta="0".into();e.counterparty_account_id=Some(account);s.entries.entry(other).or_default().push(e);}
   if s.dropped_ack{s.dropped_ack=false;return Err(c::Error::Unavailable);}
   Ok((200,serde_json::to_vec(&r)?))
  })
 }
}
pub(super) struct NoStore;
impl store::StoreTransport for NoStore {
 fn exchange<'a>(&'a mut self,_operation:Operation,_body:&'a str)->Pin<Box<dyn Future<Output=Result<Vec<u8>,store::TransportFailure>>+Send+'a>>{Box::pin(async{Err(store::TransportFailure::Unavailable)})}
}
pub(super) fn client(state:&Shared)->c::CoinClient{c::CoinClient::new(Box::new(Peer(Arc::clone(state)))).with_market_enabled(true)}
