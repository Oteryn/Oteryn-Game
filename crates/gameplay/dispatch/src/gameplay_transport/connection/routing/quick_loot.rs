use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{ClientCommandView, quick_loot::{decode_quick_loot_request, encode_quick_loot_result}};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(authority:&A, admitted:&AdmittedSession, command:ClientCommandView<'_>)->Option<Dispatch>{
    let request=match decode_quick_loot_request(command.payload){Ok(value)=>value,Err(_)=>return Some(Dispatch::Reply(CommandStatus::Rejected,Vec::new()))};
    let result=authority.quick_loot(*admitted,transport::connection::UseCommand{game_session_id:admitted.game_session_id,command_id:command.command_id},request).await?;
    Some(Dispatch::Reply(CommandStatus::Accepted,encode_quick_loot_result(&result).ok()?))
}
