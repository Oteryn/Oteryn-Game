
    use crate::combat_input::PlayCommand;
    use crate::play::{mailbox::lock, play_channel};
    #[test]
    fn coin_market_real_caller_requires_review_and_retries_the_same_confirmation()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, mut commands, outbox) = play_channel();
        outbox.market_initialize(true);
        let fields = ["25".into(), "100".into(), String::new()];
        link.coin_market_control(2, fields.clone());
        assert!(commands.try_recv().is_err());
        link.coin_market_control(6, fields.clone());
        let PlayCommand::ServerMechanic(first) = commands.try_recv()? else {
            return Err("coin command".into());
        };
        let request =
            oteryn_protocol_oteryn::coin_market::decode(&first.payload).map_err(|_| "request")?;
        assert_eq!(
            (
                first.capability,
                first.command_type,
                request.action,
                request.amount,
                request.price
            ),
            (63, 6117, 3, 25, 100)
        );
        assert!(lock(&outbox.0).market.coins.view.is_none());
        outbox.coin_market_result(&oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: oteryn_protocol_oteryn::coin_market::encode_reply(
                r#"{"result":"unavailable"}"#,
            )
            .map_err(|_| "reply")?,
        });
        link.coin_market_control(7, fields);
        let PlayCommand::ServerMechanic(retry) = commands.try_recv()? else {
            return Err("coin retry".into());
        };
        assert_eq!(first, retry);
        Ok(())
    }
struct Peer;
impl crate::play::Stepper for Peer {
 fn market_selected(&self)->bool{true}
 async fn server_mechanic(&mut self,r:&oteryn_client_runtime::server_mechanic::ServerMechanicCommand)->Result<oteryn_session::GatedCommandResult,oteryn_session::SessionError>{
  use oteryn_session::{SessionError,CommandStatus};use oteryn_client_runtime::coin_market as wire;
  assert_eq!((r.capability,r.command_type),(63,wire::COMMAND));
  let r=wire::decode(&r.payload).map_err(|_|SessionError::SessionUnusable)?;
  let h=r.operation.iter().map(|b|format!("{b:02x}")).collect::<String>();let op=format!("{}-{}-{}-{}-{}",&h[..8],&h[8..12],&h[12..16],&h[16..20],&h[20..]);
  let view=serde_json::json!({"result":"ok","operation":op,"kind":"reserve","amount":r.amount,"price":r.price,"pending":false,"receipt":{"operation_id":op,"kind":"reserve","amount":r.amount.to_string(),"transferable_after":"70","non_transferable_after":"5","reserved_after":r.amount.to_string(),"occurred_at":"2026-11-09T00:00:00Z"}});
  Ok(oteryn_session::GatedCommandResult{command_id:1,server_sequence:1,status:CommandStatus::Accepted,payload:wire::encode_reply(&view.to_string()).map_err(|_|SessionError::SessionUnusable)?})
 }
 async fn serve(&mut self,duration:std::time::Duration)->Result<(),oteryn_session::SessionError>{tokio::time::sleep(duration).await;Ok(())}
 fn drain(&mut self)->Vec<oteryn_session::SessionEvent>{vec![]}
 async fn step(&mut self,_:oteryn_session::StepDirection)->Result<oteryn_session::StepOutcome,oteryn_session::SessionError>{Err(oteryn_session::SessionError::SessionUnusable)}
 async fn combat(&mut self,_:crate::combat_input::CombatCommand)->Result<crate::combat_input::CombatReport,oteryn_session::SessionError>{Err(oteryn_session::SessionError::SessionUnusable)}
 async fn logout(&mut self)->Result<Option<oteryn_session::LogoutResult>,oteryn_session::SessionError>{Ok(None)}
}
#[test]
fn coin_market_session_loop_applies_only_the_confirmed_server_receipt()->Result<(),Box<dyn std::error::Error+Send+Sync>>{
 std::thread::Builder::new().stack_size(128<<20).spawn(||{
  let runtime=oteryn_client_runtime::ClientRuntime::new()?;
  runtime.block_on(async{
   let (link,incoming,outbox)=play_channel();let task=tokio::spawn(crate::play::run_session(Peer,incoming,outbox));
   tokio::time::timeout(std::time::Duration::from_secs(3),async{while !link.market_panel().selected{tokio::task::yield_now().await;}}).await?;
   let fields=["30".into(),"100".into(),String::new()];link.coin_market_control(2,fields.clone());assert!(link.market_panel().coins.view.is_none());
   link.coin_market_control(6,fields);assert!(link.market_panel().coins.view.is_none());
   tokio::time::timeout(std::time::Duration::from_secs(3),async{while link.market_panel().coins.pending.is_some() || link.market_panel().coins.view.is_none(){tokio::task::yield_now().await;}}).await?;
   let panel=link.market_panel();assert!(panel.coins.confirmed.is_none());assert!(panel.coins.view.as_deref().is_some_and(|s|s.contains("Transferable after: 70")));assert!(panel.coins.view.as_deref().is_some_and(|s|s.contains("Receipt:")));
   drop(link);task.await?;Ok::<_,Box<dyn std::error::Error+Send+Sync>>(())
  })?
 })?.join().map_err(|_|"coin session test thread failed")?
}
