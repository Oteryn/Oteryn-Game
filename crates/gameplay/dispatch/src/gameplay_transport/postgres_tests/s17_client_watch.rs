use oteryn_dev_client::{client_runtime as oteryn_client_runtime, client_ui as oteryn_client_ui};
// Canary livestream.cpp:486,541,724: real account observer route and live owner room.
use super::{
    Dependencies, TestResult,
    fixture::{Harness, debug, id, run},
    prepare_current_session,
};
use crate::gameplay_transport::{cast::CastNode, tcp_tls::read_frame};
use oteryn_client_runtime::CastViewerSession;
use oteryn_gameplay_social::cast::{
    Error, ObserverId,
    service::{CastService, Operation},
};
use oteryn_protocol_oteryn::cast::{Message, PublicMedia};
use std::{sync::Arc, time::Duration};
#[path = "s17_client/actor_peer.rs"]
mod actor_peer;
use actor_peer::Peer;
type Viewer = CastViewerSession<tokio::io::DuplexStream>;
type ObserverTask = tokio::task::JoinHandle<Result<(), Error>>;

async fn next_observation(
    viewer: &mut Viewer,
    observer: &mut ObserverTask,
    phase: &str,
) -> TestResult<Message> {
    match viewer.next().await {
        Ok(message) => Ok(message),
        Err(error) => {
            let server = tokio::time::timeout(Duration::from_secs(5), &mut *observer).await;
            if server.is_err() {
                observer.abort();
            }
            Err(format!("CLCAST {phase}: {error}; observer result: {server:?}").into())
        }
    }
}

async fn viewer(node: CastNode, marker: u8, nonce: u8) -> TestResult<(Viewer, ObserverTask)> {
    let (client, mut server) = tokio::io::duplex(65536);
    let ticket = format!("ocv1.{marker:02x}{}{nonce:02x}", "ab".repeat(30));
    let units = Arc::new(tokio::sync::Semaphore::new(1));
    let admission_units = Arc::clone(&units);
    let mut task = tokio::task::spawn_local(async move {
        let unit = admission_units
            .acquire()
            .await
            .map_err(|_| Error::Unavailable)?;
        let first = read_frame(&mut server)
            .await
            .map_err(|_| Error::Unavailable)?;
        let stop = crate::gameplay_transport::CancellationToken::new();
        crate::gameplay_transport::cast_observer::serve(server, &first, node, &stop, unit).await
    });
    let viewer = match Viewer::join(client, ticket.as_bytes(), Duration::from_secs(5)).await {
        Ok(viewer) => viewer,
        Err(error) => {
            let server = tokio::time::timeout(Duration::from_secs(5), &mut task).await;
            if server.is_err() {
                task.abort();
            }
            return Err(format!("CLCAST observer join: {error}; server result: {server:?}").into());
        }
    };
    assert!(!task.is_finished(), "the joined observer remains active");
    let _next_admission = units
        .try_acquire()
        .map_err(|_| "a joined observer must release its admission unit")?;
    Ok((viewer, task))
}
async fn moderate(peer: &mut Peer, subject: [u8; 16], action: u8) -> TestResult {
    let request = oteryn_client_ui::cast_chat::protocol::command(
        &peer.chat,
        &peer.control,
        "",
        &hex(subject),
        action,
        [action; 16],
    )?;
    let (status, bytes) = peer
        .exchange(request.command_type, &request.payload)
        .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    let reply = Message::decode(&bytes).map_err(debug)?;
    peer.control.apply(reply)?;
    Ok(())
}
fn hex(bytes: [u8; 16]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
fn scenario(case: u8) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(64<<20).spawn(move ||->Result<(),String>{
        run(async |admin| { tokio::task::LocalSet::new().run_until(async move {
            let started=std::time::Instant::now();
                use crate::map::boot::tests::{DOOR_AT,door_world};
                let world_id=crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
                let world=door_world()?;
                let (controller,activated)=super::death_ml_loss_composed::production_generation_with_pin(world_id,"content-1")?;
                let generation=controller.active().ok_or("active generation")?;
                let pin=crate::foundation::ChannelContentPin::from_activation(world_id,1,generation.identity().server_artifact_digest(),generation.identity().client_artifact_digest(),activated.frame_binding().digest(),world.map_revision_digest(),(i32::from(DOOR_AT.x)-1,i32::from(DOOR_AT.y),-i16::from(DOOR_AT.floor)));
                let deps=Dependencies::with_content(pin,crate::content::qualify_native_entry_room(world_id).map_err(debug)?,crate::spell::cast::v1_spell_book().map_err(debug)?)?;
                let cells=world.movement_cells(deps.room.movement_cells()).map_err(debug)?;
            let h=Harness::create_with_admission_seed(admin,&format!("clcast_view_{case}"),true,false).await?;
            prepare_current_session(&h).await?;
            let upkeep_root=h.root.clone();
            let upkeep=tokio::spawn(async move {let mut cadence=tokio::time::interval(Duration::from_millis(10));loop{cadence.tick().await;if upkeep_root.has_ready_demand()&&upkeep_root.maintain_ready_once().await.is_err(){upkeep_root.request_ready();}}});
            let result=async {
                let seal=h.recovery.seal_current().map_err(debug)?;
                let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut owner=deps.owner(&h,&authority);owner.movement_cells=crate::gameplay_transport::activity_runtime::SharedOwner::Borrowed(&cells);owner.active_generation=Some(generation);
                deps.initialize(&owner).await;
                let current=crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).current_session_at(deps.admitted.game_session_id).await?.0;
                let mut admitted=deps.admitted;
                admitted.item_fence=crate::gameplay_transport::item_fence_of(current);
                admitted.controller.as_mut().ok_or("controller")?.transport=current.current_transport().ok_or("transport")?;
                admitted.first_entry=crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
                let offers:Vec<_>=[6,7,290,291,292].into_iter().map(|id|crate::gameplay_transport::capabilities::OfferedCapability{id,requires:&[]}).collect();
                admitted.continuity.selected_capabilities=crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offers,&[6,7,290,291,292]).ok_or("capabilities")?;
                let fence=admitted.item_fence.ok_or("fence")?;
                {let mut runtime=deps.runtime.lock().await;runtime.bind_attacker_lease(deps.actor,admitted.game_session_id,crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,fence.character_lease_generation).map_err(debug)?).map_err(debug)?;runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;}
                let before=owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position();
                let delay=if case==3{300}else{0};
                let (access,host)=super::s17_client::settings_host::host_with_policy(*h.node.fact().node_id().as_bytes(),delay,case==3).await?;
                let service=CastService::spawn(h.root.clone(),h.node.clone(),50,Arc::clone(&access))?;
                let node=CastNode{service:service.clone(),access};
                owner.states.casts.set(node.clone()).map_err(|_|"cast set")?;
                let (mut server,client)=tokio::io::duplex(65536);
                let serving=crate::gameplay_transport::connection::serve_admitted(&mut server,admitted,&owner,crate::gameplay_transport::connection::IDLE_LIVENESS);
                let driving=async {
                    let mut peer=Peer::join(client,admitted).await?;
                    assert_eq!(peer.cast(503,Message::ConsentQuery).await?,Message::ConsentState{revision:0,allowed:false});
                    assert_eq!(peer.cast(503,Message::ConsentSet{revision:0,allowed:true}).await?,Message::ConsentState{revision:1,allowed:true});
                    let (status,_)=peer.exchange(503,&Message::ConsentSet{revision:0,allowed:false}.encode().map_err(debug)?).await?;
                    assert_eq!(status,oteryn_protocol_oteryn::CommandStatus::Rejected);
                    peer.cast(502,Message::Start{revision:0,access_revision:1,delay,cap:50,password:false}).await?;
                    let (mut watching,mut observer)=peer.observe(viewer(node.clone(),1,1)).await??;
                    assert_eq!(watching.chat_allowed(),case==3);assert_eq!(watching.delay(),delay);
                    if case==2 {
                        let mut picture=oteryn_client_runtime::cast_view::Picture::default();
                        picture.apply(Message::Ready{epoch:watching.epoch(),chat:watching.chat_allowed(),delay:watching.delay()},std::time::Instant::now())?;
                        peer.observe(tokio::time::timeout(Duration::from_secs(10),async {while picture.floor.is_none(){picture.apply(next_observation(&mut watching,&mut observer,"map").await?,std::time::Instant::now())?;}Ok::<(),Box<dyn std::error::Error>>(())})).await???;
                        assert!(picture.tiles.values().any(|items|items.contains(&1629)),"actual donor door appearance reaches native view");
                        assert!(!picture.actors.is_empty(),"caster creature picture is projected");
                        let text="Hello from the caster";
                        let speech=oteryn_protocol_oteryn::chat::encode_chat_intent(&oteryn_protocol_oteryn::chat::ChatIntent::Say{mode:oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,text:text.into()}).map_err(debug)?;
                        let (status,_)=peer.exchange(13,&speech).await?;assert_eq!(status,oteryn_protocol_oteryn::CommandStatus::Accepted);
                        peer.observe(tokio::time::timeout(Duration::from_secs(10),async {loop{let message=next_observation(&mut watching,&mut observer,"speech").await?;let found=matches!(&message,Message::Media{media:PublicMedia::Speech{text:heard,..},..} if heard==text);picture.apply(message,std::time::Instant::now())?;if found{break;}}Ok::<(),Box<dyn std::error::Error>>(())})).await???;
                        assert!(picture.speech.iter().any(|line|line==text));
                        let position=oteryn_protocol_oteryn::world_spatial::ActorPosition{x:before.x,y:before.y,floor:before.floor};
                        owner.cast_public_effects(admitted,&[oteryn_protocol_oteryn::presentation::PresentationEvent{decision_ordinal:1,emission_ordinal:1,kind:oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect{position,effect_id:1,source:oteryn_protocol_oteryn::presentation::PresentationSource::Global}}]).await;
                        peer.observe(tokio::time::timeout(Duration::from_secs(10),async {loop{let message=next_observation(&mut watching,&mut observer,"effects").await?;let found=matches!(&message,Message::Media{media:PublicMedia::Effects(_),..});picture.apply(message,std::time::Instant::now())?;if found{break;}}Ok::<(),Box<dyn std::error::Error>>(())})).await???;
                        assert!(!picture.effects.is_empty());
                        peer.observe(watching.leave()).await??;let _=observer.await?;
                    }else{
                        let handle=peer.roster(None).await?;
                        peer.observe(watching.chat("Live viewer text at D300")).await??;
                        peer.text("Live viewer text at D300").await?;
                        let output=Arc::new(std::sync::Mutex::new(Vec::new()));let captured=output.clone();
                        service.apply(Operation::Flush(ObserverId::from_handle(handle),Box::new(move |_,content|{if !matches!(content,oteryn_gameplay_social::cast::Content::Clear)&& let Ok(mut rows)=captured.lock(){rows.push(content.clone());}true}))).await?;
                        assert!(output.lock().map_err(|_|"capture lock")?.is_empty(),"viewer content remains delayed while caster text is live");
                        peer.cast(504,Message::Reply("Live caster reply".into())).await?;peer.text("Live caster reply").await?;
                        assert_eq!(peer.gameplay_lines,0,"cast chat never enters gameplay chat");
                        moderate(&mut peer,handle,1).await?;
                        assert!(matches!(service.apply(Operation::Chat(ObserverId::from_handle(handle),"muted text".into())).await,Err(Error::Muted)));
                        moderate(&mut peer,handle,2).await?;
                        peer.observe(watching.chat("After unmute")).await??;peer.text("After unmute").await?;
                        moderate(&mut peer,handle,3).await?;
                        assert!(peer.observe(tokio::time::timeout(Duration::from_secs(5),watching.next())).await??.is_err());let _=observer.await?;
                        let (mut second,second_task)=peer.observe(viewer(node.clone(),2,1)).await??;
                        let second_handle=peer.roster(Some(handle)).await?;
                        moderate(&mut peer,second_handle,4).await?;
                        assert!(peer.observe(tokio::time::timeout(Duration::from_secs(5),second.next())).await??.is_err());let _=second_task.await?;
                        let bans:i64=sqlx::query_scalar("SELECT count(*) FROM game_cast_account_bans").fetch_one(&h.pool).await?;assert_eq!(bans,1);
                        assert!(peer.observe(viewer(node.clone(),2,2)).await?.is_err(),"a fresh ticket cannot bypass the durable account ban");
                    }
                    let revision=peer.control.revision;
                    peer.cast(502,Message::Stop{revision}).await?;
                    assert_eq!(peer.cast(502,Message::ControlQuery).await?,Message::End);
                    peer.close().await?;Ok::<(),Box<dyn std::error::Error>>(())
                };
                let (_,result)=tokio::time::timeout(Duration::from_secs(40),super::fixture::join_two(serving,driving)).await?;result?;
                let roots:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_roots").fetch_one(&h.pool).await?;assert_eq!(roots,1);
                assert_eq!(owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position(),before);
                let _=service.apply(Operation::Shutdown).await;host.abort();
                eprintln!("CLCAST S17 case {case} real-dispatch PostgreSQL: {:.3}s",started.elapsed().as_secs_f64());
                Ok::<(),Box<dyn std::error::Error>>(())
            }.await;
            upkeep.abort();h.cleanup().await?;result
        }).await }).map_err(|e|e.to_string())
    })?.join().map_err(|_|"CLCAST PostgreSQL thread failed")?.map_err(Into::into)
}
#[test]
fn s17b_native_observer_map_creatures_effects_and_public_speech_dispatch() -> TestResult {
    scenario(2)
}
#[test]
fn s17c_native_live_chat_mute_kick_and_persistent_ban_dispatch() -> TestResult {
    scenario(3)
}
