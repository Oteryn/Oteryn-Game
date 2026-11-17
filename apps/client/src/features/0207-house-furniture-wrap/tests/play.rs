use crate::combat_input::{CombatCommand,CombatReport};
use crate::play::{PlayLink,Stepper,play_channel,run_session};
use oteryn_client_runtime::raw_feature::{FeatureCommand,house_wrap as wire};
use oteryn_client_ui::house_furniture_wrap::Intent;
use oteryn_session::{CommandStatus,GatedCommandResult,LogoutResult,SessionError,SessionEvent,StepDirection,StepOutcome};
use std::{collections::VecDeque,sync::{Arc,Mutex,PoisonError},time::Duration};
type Error=Box<dyn std::error::Error+Send+Sync>;
type Exchange=(Vec<u8>,CommandStatus,Vec<u8>);
struct Script {exchanges:Mutex<VecDeque<Exchange>>,release:tokio::sync::Semaphore}
struct Peer(Arc<Script>);
impl Stepper for Peer {
    fn feature_selected(&self,capability:u32)->bool {capability==207}
    async fn feature(&mut self,command:&FeatureCommand)->Result<GatedCommandResult,SessionError> {
        assert_eq!((command.capability,command.kind),(207,402));
        let (payload,status,reply)=self.0.exchanges.lock().unwrap_or_else(PoisonError::into_inner).pop_front().ok_or(SessionError::SessionUnusable)?;
        assert_eq!(command.payload,payload);
        self.0.release.acquire().await.map_err(|_|SessionError::SessionUnusable)?.forget();
        Ok(GatedCommandResult {command_id:1,server_sequence:1,status,payload:reply})
    }
    async fn serve(&mut self,duration:Duration)->Result<(),SessionError>{tokio::time::sleep(duration).await;Ok(())}
    fn drain(&mut self)->Vec<SessionEvent>{vec![]}
    async fn step(&mut self,_:StepDirection)->Result<StepOutcome,SessionError>{Err(SessionError::SessionUnusable)}
    async fn combat(&mut self,_:CombatCommand)->Result<CombatReport,SessionError>{Err(SessionError::SessionUnusable)}
    async fn logout(&mut self)->Result<Option<LogoutResult>,SessionError>{Ok(None)}
}
async fn ready(link:&PlayLink)->Result<(),Error> {
    tokio::time::timeout(Duration::from_secs(3),async {
        while !link.house_furniture_panel().selected || link.house_furniture_panel().pending {tokio::task::yield_now().await;}
    }).await?;Ok(())
}
fn queue(script:&Script,payload:Vec<u8>,status:CommandStatus,reply:Vec<u8>) {
    script.exchanges.lock().unwrap_or_else(PoisonError::into_inner).push_back((payload,status,reply));
}
async fn complete(link:&PlayLink,script:&Script)->Result<(),Error> {
    assert!(link.house_furniture_panel().pending);
    link.house_furniture_request(Intent::Apply); // Pending commands cannot be duplicated.
    script.release.add_permits(1);ready(link).await
}
#[test]
fn srx1_native_furniture_catalogue_wrap_unwrap_and_stale_refusal()->Result<(),Error> {
    std::thread::Builder::new().stack_size(64<<20).spawn(||->Result<(),String>{
        let runtime=oteryn_client_runtime::ClientRuntime::new().map_err(|error|format!("{error:?}"))?;
        runtime.block_on(async {
            let script=Arc::new(Script {exchanges:Mutex::new(VecDeque::new()),release:tokio::sync::Semaphore::new(0)});
            let (link,mut incoming,outbox)=play_channel();
            link.house_furniture_request(Intent::Refresh);
            assert!(incoming.try_recv().is_err());
            let task=tokio::spawn(run_session(Peer(script.clone()),incoming,outbox));ready(&link).await?;
            let mut catalogue=wire::Catalogue {page:0,more:false,acl_revision:4,position:[10,20,7],entries:vec![wire::CatalogueEntry {item:[7;16],revision:1,action:wire::Action::Wrap,name:"oven".into()}]};
            queue(&script,vec![3,0,0],CommandStatus::Accepted,catalogue.encode().ok_or("catalogue")?);
            let event=oteryn_input_actions::NormalizedInputEvent::Key {
                code:oteryn_input_actions::KeyCode::new(9).map_err(|error|format!("{error:?}"))?,state:oteryn_input_actions::ButtonState::Pressed,
                modifiers:oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control).with(oteryn_input_actions::Modifier::Alt),repeat:false};
            assert!(link.house_furniture_input(&[event]));complete(&link,&script).await?;
            let text=oteryn_client_ui::house_furniture_wrap::panel_text(&link.house_furniture_panel());
            assert!(text.contains("1. oven — Wrap"));assert!(text.contains("floor 7"));
            assert!(!text.contains("07070707"));
            link.house_furniture_edit("1".into());
            queue(&script,catalogue.request(0).ok_or("request")?.encode(),CommandStatus::Accepted,wire::Reply::Committed(2).encode());
            link.house_furniture_request(Intent::Apply);
            assert!(!link.house_furniture_panel().pending);
            assert!(link.house_furniture_panel().feedback.contains("again to confirm"));
            link.house_furniture_request(Intent::Apply);complete(&link,&script).await?;
            assert!(link.house_furniture_panel().feedback.contains("revision 2"));
            assert!(link.house_furniture_panel().catalogue.is_none());
            catalogue.entries[0].revision=2;catalogue.entries[0].action=wire::Action::Unwrap;catalogue.entries[0].name="oven kit".into();
            queue(&script,vec![3,0,0],CommandStatus::Accepted,catalogue.encode().ok_or("catalogue")?);
            link.house_furniture_request(Intent::Refresh);complete(&link,&script).await?;
            assert!(oteryn_client_ui::house_furniture_wrap::panel_text(&link.house_furniture_panel()).contains("oven kit — Place"));
            link.house_furniture_edit("1".into());
            queue(&script,catalogue.request(0).ok_or("request")?.encode(),CommandStatus::Accepted,wire::Reply::Committed(3).encode());
            link.house_furniture_request(Intent::Apply);
            assert!(!link.house_furniture_panel().pending);
            assert!(link.house_furniture_panel().feedback.contains("again to confirm"));
            link.house_furniture_request(Intent::Apply);complete(&link,&script).await?;
            assert!(link.house_furniture_panel().feedback.contains("revision 3"));
            assert!(link.house_furniture_panel().catalogue.is_none());
            catalogue.entries[0].revision=3;catalogue.entries[0].action=wire::Action::Wrap;catalogue.entries[0].name="oven".into();
            queue(&script,vec![3,0,0],CommandStatus::Accepted,catalogue.encode().ok_or("catalogue")?);
            link.house_furniture_request(Intent::Refresh);complete(&link,&script).await?;
            link.house_furniture_edit("1".into());
            queue(&script,catalogue.request(0).ok_or("request")?.encode(),CommandStatus::Rejected,wire::Reply::Refused(wire::Refusal::Stale).encode());
            link.house_furniture_request(Intent::Apply);
            assert!(!link.house_furniture_panel().pending);
            assert!(link.house_furniture_panel().feedback.contains("again to confirm"));
            link.house_furniture_request(Intent::Apply);complete(&link,&script).await?;
            assert!(link.house_furniture_panel().feedback.contains("access changed"));
            assert!(link.house_furniture_panel().catalogue.is_none());
            queue(&script,vec![3,0,0],CommandStatus::Accepted,vec![7,0]);
            link.house_furniture_request(Intent::Refresh);complete(&link,&script).await?;
            assert_eq!(link.house_furniture_panel().feedback,"Invalid furniture catalogue");
            assert!(script.exchanges.lock().unwrap_or_else(PoisonError::into_inner).is_empty());
            drop(link);task.await?;Ok::<(),Error>(())
        }).map_err(|error|format!("{error:?}"))?.map_err(|error|error.to_string())
    })?.join().map_err(|_|"furniture play worker panicked")?.map_err(Into::into)
}
