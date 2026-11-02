//! Identity-specific management complements the existing command220 without guessing a Hireling.
use super::{ComposedFreshAdmission,capabilities::OfferedCapability,connection::{AdmittedSession,UseCommand}};
use oteryn_protocol_oteryn::hireling::{CAPABILITY_HIRELING_V1,management as wire};

pub(super) fn offered(base:&[OfferedCapability],available:bool)->Vec<OfferedCapability>{
    let mut offers:Vec<_>=base.iter().copied().filter(|c|c.id!=wire::CAPABILITY_HIRELING_MANAGEMENT_V1).collect();
    if available&&offers.iter().any(|c|c.id==CAPABILITY_HIRELING_V1){
        let at=offers.partition_point(|c|c.id<wire::CAPABILITY_HIRELING_MANAGEMENT_V1);
        offers.insert(at,OfferedCapability{id:wire::CAPABILITY_HIRELING_MANAGEMENT_V1,requires:&[CAPABILITY_HIRELING_V1]});
    }offers
}
macro_rules! authority_methods {()=>{
    fn hireling_management_available(&self)->bool{false}
    fn hireling_management(&self,_:$crate::gameplay_transport::connection::AdmittedSession,_:u64,_:oteryn_protocol_oteryn::hireling::management::Request)->impl std::future::Future<Output=Option<Vec<u8>>>{async{None}}
};}
pub(super) use authority_methods;
macro_rules! composed_methods {()=>{
    fn hireling_management_available(&self)->bool{self.hireling_available()}
    async fn hireling_management(&self,admitted:connection::AdmittedSession,command:u64,request:oteryn_protocol_oteryn::hireling::management::Request)->Option<Vec<u8>>{hireling_management::execute(self,admitted,command,request).await}
};}
pub(super) use composed_methods;

pub(super) async fn execute(owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:AdmittedSession,command:u64,request:wire::Request)->Option<Vec<u8>>{
    if !admitted.continuity.selected_capabilities.contains(wire::CAPABILITY_HIRELING_MANAGEMENT_V1){return None}
    let fence=admitted.item_fence?;
    let reply=match request {
        wire::Request::Projection=>{
            let actor=admitted.runtime_actor?;
            super::hireling::refresh(owner,actor,UseCommand{game_session_id:admitted.game_session_id,command_id:command,item_fence:Some(fence)}).await?;
            let runtime=owner.runtime.lock().await;
            ComposedFreshAdmission::hireling_actor_bindings(&runtime,actor,admitted.game_session_id)?
        }
        wire::Request::List=>owner.root.query_hirelings(owner.character,owner.holder,fence,*admitted.channel_id.as_bytes(),command).await.ok()?,
        wire::Request::ReturnToLamp{hireling}=>super::hireling::return_to_lamp(owner,admitted.runtime_actor?,UseCommand{game_session_id:admitted.game_session_id,command_id:command,item_fence:Some(fence)},hireling).await?,
        wire::Request::Outfit{..}=>{
            let reply=owner.root.set_hireling_outfit(owner.character,owner.holder,fence,*admitted.channel_id.as_bytes(),command,request).await.ok()?;
            super::hireling::refresh_if_house(owner,admitted.runtime_actor?,UseCommand{game_session_id:admitted.game_session_id,command_id:command,item_fence:Some(fence)}).await?;reply
        }
    };
    reply.encode()
}
