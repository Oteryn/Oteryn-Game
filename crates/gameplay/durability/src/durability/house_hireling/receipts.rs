use super::super::{DurabilityError,item_transfer::CurrentCharacterItemFence};
use oteryn_protocol_oteryn::hireling::management::{Entry,Reply};
use sha2::{Digest,Sha256};
use sqlx::{Postgres,Row,Transaction};

/// Distinguish an immutable ingress conflict from an unavailable receipt read.
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum HirelingReceiptRead { Missing, Found(Reply), Conflict }

pub(super) fn binding(payload:&[u8])->[u8;32]{
    let mut digest=Sha256::new();digest.update(b"oteryn:hireling:command:v1\0");digest.update(payload);digest.finalize().into()
}

/// Stable fresh value identities for a native mint reservation. The UUIDv7 timestamp prefix
/// comes from its original command session; the remaining bits are domain-separated hashes.
pub fn command_identity(command:crate::foundation::CommandRef, purpose:&[u8])->[u8;16] {
    let mut digest=Sha256::new();digest.update(b"oteryn:hireling:value-id:v1\0");
    digest.update(command.game_session_id().as_bytes());digest.update(command.command_id().get().to_be_bytes());digest.update(purpose);
    let bytes=digest.finalize();let mut id=[0;16];id.copy_from_slice(&bytes[..16]);
    id[..6].copy_from_slice(&command.game_session_id().as_bytes()[..6]);
    id[6]=(id[6]&0x0f)|0x70;id[8]=(id[8]&0x3f)|0x80;id
}

pub fn return_binding(hireling:[u8;16])->[u8;32] {
    binding(&oteryn_protocol_oteryn::hireling::management::Request::ReturnToLamp{hireling}.encode())
}
pub(super) async fn replay(
    tx:&mut Transaction<'_,Postgres>,fence:&CurrentCharacterItemFence,command:u64,digest:&[u8;32],
)->Result<Option<Reply>,DurabilityError>{
    match read_state(tx,fence,command,digest).await? {
        HirelingReceiptRead::Missing=>Ok(None),
        HirelingReceiptRead::Found(reply)=>Ok(Some(reply)),
        HirelingReceiptRead::Conflict=>Err(DurabilityError::InvalidStoredState),
    }
}

async fn read_state(
    tx:&mut Transaction<'_,Postgres>,fence:&CurrentCharacterItemFence,command:u64,digest:&[u8;32],
)->Result<HirelingReceiptRead,DurabilityError>{
    if command==0{return Err(DurabilityError::InvalidStoredState)}
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:hireling:'||encode($1,'hex')||':'||$2,0))")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string()).execute(&mut **tx).await?;
    let row=sqlx::query("SELECT uuid_send(character_id) AS character,request_binding,response FROM game_house_hireling_receipts \
        WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string()).fetch_optional(&mut **tx).await?;
    let Some(row)=row else{return Ok(HirelingReceiptRead::Missing)};
    if row.try_get::<Option<Vec<u8>>,_>("character")?.as_deref()!=Some(fence.character_id.as_bytes().as_slice())
        || row.try_get::<Option<Vec<u8>>,_>("request_binding")?.as_deref()!=Some(digest.as_slice()) {
        return Ok(HirelingReceiptRead::Conflict)
    }
    let payload=row.try_get::<Option<Vec<u8>>,_>("response")?.ok_or(DurabilityError::InvalidStoredState)?;
    Reply::decode(&payload).map(HirelingReceiptRead::Found).ok_or(DurabilityError::InvalidStoredState)
}

impl super::super::DurabilityRoot {
    /// Read under the same recovery proof and CommandRef lock as every lamp writer.
    /// A transient failure is never represented as a terminal ingress conflict.
    pub async fn read_hireling_receipt_state(
        &self,authority:&super::super::character_authority::ReconciledCharacterAuthority<'_,'_>,
        current:CurrentCharacterItemFence,command:u64,digest:[u8;32],
    )->Result<HirelingReceiptRead,DurabilityError>{
        use super::super::character_authority::assert_recovery_fence;
        use super::super::db::{begin_semantic_transaction,commit_semantic_transaction};
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let result=read_state(&mut tx,&current,command,&digest).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(result)
        })).await
    }
}
pub(super) async fn write(
    tx:&mut Transaction<'_,Postgres>,fence:&CurrentCharacterItemFence,command:u64,
    id:Option<[u8;16]>,digest:[u8;32],reply:&Reply,position:Option<(u16,u16,i8)>,
)->Result<(),DurabilityError>{
    let payload=reply.encode().ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_house_hireling_receipts \
        (game_session_id,command_id,character_id,hireling_id,request_binding,response,active,x,y,floor) \
        VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8,$9,$10)")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(id.as_ref().map(|v|v.as_slice()))
        .bind(digest.as_slice()).bind(payload).bind(position.is_some())
        .bind(i32::from(position.map_or(0,|p|p.0))).bind(i32::from(position.map_or(0,|p|p.1)))
        .bind(i16::from(position.map_or(0,|p|p.2))).execute(&mut **tx).await?;
    Ok(())
}

pub(super) fn entry(snapshot:super::HirelingSnapshot)->Entry {
    let outfits=snapshot.outfits.iter().fold(0,|bits,id|bits | 1 << (id-2001));
    Entry{hireling:snapshot.id,name:snapshot.name,sex:snapshot.sex,look_type:snapshot.look_type,
        colors:snapshot.colors,skills:snapshot.skills,outfits,position:snapshot.position}
}
