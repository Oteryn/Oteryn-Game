use super::super::{AdmittedSession,FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView,CommandStatus,hireling::management::Request};

pub(in super::super) async fn route<A:FreshAdmissionAuthority>(authority:&A,admitted:&AdmittedSession,command:ClientCommandView<'_>)->Option<Dispatch>{
    let request=Request::decode(command.payload)?;
    let payload=authority.hireling_management(*admitted,command.command_id,request).await?;
    Some(Dispatch::Reply(CommandStatus::Accepted,payload))
}
