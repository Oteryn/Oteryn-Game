//! S04b: capability 80's existing wire dispatch on two assigned nodes of one World.
use super::private_channel_peers as peers;

use super::chat_world_fixture as world;
use super::fixture::{Harness,debug,id,join_two,run};
use super::{Dependencies,TestResult,prepare_current_session,reset};
use crate::chat::relay::{RelayDestination,RelayKey,RelayKeyRing,RelayLine,channel_name};
use crate::gameplay_transport::{ComposedFreshAdmission,capabilities::SelectedCapabilities,chat_relay::ChatRelay,connection::{AdmittedSession,FreshAdmissionAuthority},guild_chat::wire};
use crate::gameplay_transport::spell_premium_coordinator::{SpellPremiumCoordinator,purchase_fixture};
use oteryn_foundation::CancellationToken;
use oteryn_protocol_oteryn::chat::{self,ChatDisposition,ChatIntent,ChatLine};
use oteryn_protocol_oteryn::chat::private_channel::PrivateChannelIntent as Intent;
use std::{future::Future,sync::Arc,time::Duration};

#[test]
fn private_channel_postgres_world_lifecycle_fences_atomic_replay_and_epoch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    std::thread::Builder::new().name("chat-world-private".into()).stack_size(64*1024*1024).spawn(|| {
        run(async |admin| {
            reset();
            let h=Harness::create_with_admission_seed(admin,"chat_world_private",true,false).await?;
            prepare_current_session(&h).await?;
            let (second_node,mut second)=world::admit_second_node(&h).await?;
            {
                let seal=h.recovery.seal_current().map_err(debug)?;
                let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut first=Dependencies::new()?;
                let mut fence=super::fixture::fence(1)?.item_fence();
                fence.character_lease_generation=2;
                first.admitted.item_fence=Some(fence);
                {
                    let mut runtime=first.runtime.lock().await;
                    runtime.initialize_first_entry_position(first.actor).map_err(debug)?;
                    runtime.bind_attacker_lease(first.actor,first.admitted.game_session_id,
                        crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,2).map_err(debug)?).map_err(debug)?;
                }
                let (consumer,clock,_,_)=purchase_fixture::current(id(40));
                let coordinator=SpellPremiumCoordinator::new(consumer,Some(clock),None,Some(h.root.clone()));
                first.admitted.continuity.selected_capabilities=select()?;
                second.admitted.continuity.selected_capabilities=select()?;
                let mut owner=first.owner(&h,&authority);
                owner.premium_coordinator=Some(&coordinator);
                let mut target=second.owner(&h,&authority);
                target.holder=&second_node;
                assert_ne!(owner.channel_id,target.channel_id);
                assert_ne!(owner.holder.fact().node_id(),target.holder.fact().node_id());
                first.initialize(&owner).await;
                second.initialize(&target).await;
                owner.chat_admit(first.actor,first.admitted.game_session_id).await;
                target.chat_admit(second.actor,second.admitted.game_session_id).await;
                owner.public_chat_admit(first.admitted,chat::ChatRoomSet::default()).await;
                target.public_chat_admit(second.admitted,chat::ChatRoomSet::default()).await;
                owner.private_channel_admit(first.admitted,true).await;
                target.private_channel_admit(second.admitted,true).await;
                let mut legacy=first.admitted;
                legacy.continuity.selected_capabilities=SelectedCapabilities::select(crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,&[6,7,42]).ok_or("legacy selection")?;
                let payload=chat::encode_chat_intent(&ChatIntent::PrivateChannel(Intent::List)).map_err(debug)?;
                assert!(wire::dispatch(&owner,legacy,first.actor,98,&payload).await.is_err(),"capability 80 is required at the wire");
                assert!(wire::dispatch(&owner,first.admitted,first.actor,99,&[0xff]).await.is_err(),"malformed payload cannot reach the owner");
                let mut left=100_u64;
                let mut right=200_u64;
                assert_wire(&target,second.admitted,&mut right,Intent::Create,ChatDisposition::NotAllowed).await?;
                assert_wire(&owner,first.admitted,&mut left,Intent::Invite {name:world::RECIPIENT_NAME.into()},ChatDisposition::NotAllowed).await?;
                let create_id=left;
                assert_wire(&owner,first.admitted,&mut left,Intent::Create,ChatDisposition::Ok).await?;
                assert_eq!(dispatch(&owner,first.admitted,create_id,Intent::Create).await?,ChatDisposition::Ok);
                assert_eq!(dispatch(&owner,first.admitted,create_id,Intent::List).await?,ChatDisposition::Rejected,"same CommandRef cannot be rebound");
                assert_eq!(count(&h,"SELECT count(*) FROM game_private_chat_channels").await?,1);
                assert_wire(&owner,first.admitted,&mut left,Intent::Create,ChatDisposition::Rejected).await?;
                let list=owner.private_channel_list(first.admitted).await.ok_or("owned list")?;
                let channel=list.own_channel.ok_or("created channel")?;
                assert_eq!(channel,100,"Crystal private slot range begins at 100");
                let first_epoch=epoch(&h,channel).await?;
                let unrelated=line(&h,channel,first_epoch,"Owner",4).await?;
                for _ in 0..=crate::chat::egress::EGRESS_MAX_LINES {target.states.chat.deliver(&unrelated);}
                assert!(target.chat_next_bound(second.admitted,&mut 64).await.is_none(),"unrelated private-room traffic never enters another session's bounded queue");
                assert_wire(&target,second.admitted,&mut right,Intent::Join {channel_id:channel},ChatDisposition::NotInvited).await?;
                assert_wire(&owner,first.admitted,&mut left,Intent::Invite {name:"Nobody".into()},ChatDisposition::NotOnline).await?;
                let own_name:String=sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_wire(&owner,first.admitted,&mut left,Intent::Invite {name:own_name.clone()},ChatDisposition::Rejected).await?;
                let invite_id=left;
                assert_wire(&owner,first.admitted,&mut left,Intent::Invite {name:world::RECIPIENT_NAME.to_ascii_lowercase()},ChatDisposition::Ok).await?;
                assert_eq!(dispatch(&owner,first.admitted,invite_id,Intent::Invite {name:world::RECIPIENT_NAME.to_ascii_lowercase()}).await?,ChatDisposition::Ok);
                assert_eq!(count(&h,"SELECT count(*) FROM game_private_chat_members").await?,2);
                let invited=target.private_channel_list(second.admitted).await.ok_or("cross-node invitation list")?;
                assert_eq!(invited.invited.len(),1);
                assert_eq!(invited.invited[0].channel_id,channel);
                assert_wire(&target,second.admitted,&mut right,Intent::Say {channel_id:channel,text:"before joining".into()},ChatDisposition::RoomNotOpen).await?;
                assert_wire(&target,second.admitted,&mut right,Intent::Join {channel_id:channel},ChatDisposition::Ok).await?;
                assert_eq!(count(&h,"SELECT count(*) FROM game_private_chat_members WHERE joined").await?,2);
                let unavailable_id=left;
                assert_wire(&owner,first.admitted,&mut left,Intent::Say {channel_id:channel,text:"without listener".into()},ChatDisposition::ChatUnavailable).await?;
                let receipts=count(&h,"SELECT count(*) FROM game_private_chat_receipts").await?;
                let mut stale=first.admitted;
                stale.continuity.connection_generation+=1;
                assert_eq!(dispatch(&owner,stale,left,Intent::List).await?,ChatDisposition::Rejected);
                let mut foreign=first.admitted;
                foreign.world_id=crate::foundation::WorldId::decode(&id(99)).map_err(debug)?;
                assert_eq!(dispatch(&owner,foreign,left,Intent::List).await?,ChatDisposition::Rejected);
                assert_eq!(count(&h,"SELECT count(*) FROM game_private_chat_receipts").await?,receipts);
                owner.private_channel_admit(stale,false).await;
                owner.private_channel_admit(foreign,false).await;
                assert_wire(&owner,first.admitted,&mut left,Intent::List,ChatDisposition::Ok).await?;

                let ring=RelayKeyRing::new(vec![RelayKey::new(1,0,[21;32])]).map_err(debug)?;
                let options=h.root.chat_relay_connect_options().ok_or("relay endpoint")?;
                let first_relay=Arc::new(ChatRelay::new(ring.clone(),id(42),options.clone()).map_err(debug)?);
                let second_relay=Arc::new(ChatRelay::new(ring,id(42),options).map_err(debug)?);
                owner.states.chat.install_relay(first_relay.clone());
                target.states.chat.install_relay(second_relay.clone());
                let shutdown=CancellationToken::new();
                let listeners=join_two(first_relay.run(&owner.states.chat,&shutdown),second_relay.run(&target.states.chat,&shutdown));
                let scenario=async {
                    while !first_relay.is_ready() || !second_relay.is_ready() { tokio::time::sleep(Duration::from_millis(10)).await; }
                    let relay_channel=channel_name(&crate::durability::guild_chat_fixture::uuid(id(42))).map_err(debug)?;
                    let mut audit=sqlx::postgres::PgListener::connect_with(&h.pool).await?;
                    audit.listen(&relay_channel).await?;
                    assert_eq!(dispatch(&owner,first.admitted,unavailable_id,Intent::Say {channel_id:channel,text:"without listener".into()}).await?,ChatDisposition::ChatUnavailable,"a terminal readiness refusal cannot publish after recovery");
                    sqlx::query("SELECT pg_notify($1,'barrier')").bind(&relay_channel).execute(&h.pool).await?;
                    assert_eq!(audit.recv().await?.payload(),"barrier");
                    let say_id=left;
                    let message=Intent::Say {channel_id:channel,text:"private across nodes".into()};
                    assert_wire(&owner,first.admitted,&mut left,message.clone(),ChatDisposition::Ok).await?;
                    assert_eq!(dispatch(&owner,first.admitted,say_id,message).await?,ChatDisposition::Ok);
                    assert_ne!(audit.recv().await?.payload(),"barrier");
                    sqlx::query("SELECT pg_notify($1,'barrier')").bind(&relay_channel).execute(&h.pool).await?;
                    assert_eq!(audit.recv().await?.payload(),"barrier","receipt replay emits no second notification");
                    let delivered=receive(&target,second.admitted).await?;
                    assert!(matches!(delivered,ChatLine::PrivateChannel(line) if line.channel_id==channel && line.text=="private across nodes"));
                    target.chat_delivered(second.admitted.game_session_id).await;
                    assert!(matches!(receive(&owner,first.admitted).await?,ChatLine::PrivateChannel(_)));
                    owner.chat_delivered(first.admitted.game_session_id).await;
                    let queued=line(&h,channel,first_epoch,&own_name,5).await?;
                    target.states.chat.deliver(&queued);
                    let mut stale_reader=second.admitted;
                    stale_reader.continuity.connection_generation+=1;
                    assert!(target.chat_next_bound(stale_reader,&mut 64).await.is_none());
                    assert!(matches!(receive(&target,second.admitted).await?,ChatLine::PrivateChannel(_)),"a noncurrent connection cannot pop the current receiver's queue head");
                    target.chat_delivered(second.admitted.game_session_id).await;
                    let say_receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_private_chat_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                        .bind(first.admitted.game_session_id.as_bytes().as_slice()).bind(say_id.to_string()).fetch_one(&h.pool).await?;
                    assert_eq!(say_receipts,1,"one immutable receipt for original plus replay");
                    // An already authenticated, queued old line loses admission when the owner
                    // excludes a member. It must not escape through a delayed refresh.
                    let old=line(&h,channel,first_epoch,&own_name,1).await?;
                    target.states.chat.deliver(&old);
                    assert_wire(&owner,first.admitted,&mut left,Intent::Exclude {name:world::RECIPIENT_NAME.into()},ChatDisposition::Ok).await?;
                    assert!(target.chat_next_bound(second.admitted,&mut 64).await.is_none());
                    assert_wire(&owner,first.admitted,&mut left,Intent::Exclude {name:world::RECIPIENT_NAME.into()},ChatDisposition::NotInvited).await?;
                    assert_wire(&target,second.admitted,&mut right,Intent::Join {channel_id:channel},ChatDisposition::NotInvited).await?;
                    assert_wire(&owner,first.admitted,&mut left,Intent::Invite {name:world::RECIPIENT_NAME.into()},ChatDisposition::Ok).await?;
                    assert_wire(&target,second.admitted,&mut right,Intent::Join {channel_id:channel},ChatDisposition::Ok).await?;
                    assert_wire(&target,second.admitted,&mut right,Intent::Leave {channel_id:channel},ChatDisposition::Ok).await?;
                    assert_wire(&target,second.admitted,&mut right,Intent::Say {channel_id:channel,text:"after leaving".into()},ChatDisposition::RoomNotOpen).await?;
                    assert_wire(&target,second.admitted,&mut right,Intent::Join {channel_id:channel},ChatDisposition::Ok).await?;
                    // Departure and rejoin cannot expose earlier lines from the same channel.
                    target.states.chat.deliver(&old);
                    assert!(target.chat_next_bound(second.admitted,&mut 64).await.is_none());
                    assert_wire(&owner,first.admitted,&mut left,Intent::Leave {channel_id:channel},ChatDisposition::Ok).await?;
                    assert_eq!(count(&h,"SELECT count(*) FROM game_private_chat_channels").await?,0);
                    assert_eq!(count(&h,"SELECT count(*) FROM game_private_chat_members").await?,0);
                    assert_wire(&owner,first.admitted,&mut left,Intent::Create,ChatDisposition::Ok).await?;
                    assert_eq!(owner.private_channel_list(first.admitted).await.ok_or("replacement list")?.own_channel,Some(channel));
                    assert!(epoch(&h,channel).await?>first_epoch);
                    assert_wire(&owner,first.admitted,&mut left,Intent::Invite {name:world::RECIPIENT_NAME.into()},ChatDisposition::Ok).await?;
                    assert_wire(&target,second.admitted,&mut right,Intent::Join {channel_id:channel},ChatDisposition::Ok).await?;
                    target.states.chat.deliver(&old);
                    assert!(target.chat_next_bound(second.admitted,&mut 64).await.is_none(),"reused u16 ID grants no old epoch");
                    invitation_caps(&h,&owner,first.admitted,&mut left,channel,second.admitted).await?;
                    target.private_channel_admit(second.admitted,false).await;
                    assert_wire(&owner,first.admitted,&mut left,Intent::Exclude {name:world::RECIPIENT_NAME.into()},ChatDisposition::NotInvited).await?;
                    assert_wire(&owner,first.admitted,&mut left,Intent::Invite {name:world::RECIPIENT_NAME.into()},ChatDisposition::Rejected).await?;
                    assert_wire(&target,second.admitted,&mut right,Intent::Join {channel_id:channel},ChatDisposition::Rejected).await?;
                    target.private_channel_admit(second.admitted,true).await;
                    assert_wire(&owner,first.admitted,&mut left,Intent::Invite {name:world::RECIPIENT_NAME.into()},ChatDisposition::Ok).await?;
                    assert_wire(&target,second.admitted,&mut right,Intent::Join {channel_id:channel},ChatDisposition::Ok).await?;
                    let prepared=h.root.prepare_private_channel(&authority,owner.holder,fence,9_000,Intent::List).await.map_err(debug)?;
                    let crate::durability::private_channel::Prepared::Fresh(prepared)=prepared else { return Err("expected a fresh private write".into()); };
                    owner.private_channel_admit(first.admitted,false).await;
                    assert_eq!(prepared.commit(crate::domain::premium::PremiumStatus::NotCurrent,None,None).await.map_err(debug)?.disposition,ChatDisposition::Rejected,"commit rechecks negotiation after prepare");
                    assert_eq!(count(&h,"SELECT count(*) FROM game_private_chat_receipts WHERE command_id=9000").await?,0);
                    assert_eq!(count(&h,"SELECT count(*) FROM game_private_chat_channels").await?,0);
                    assert_eq!(count(&h,"SELECT count(*) FROM game_private_chat_members").await?,0);
                    assert!(target.private_channel_list(second.admitted).await.ok_or("closed list")?.invited.is_empty());
                    owner.private_channel_admit(first.admitted,true).await;
                    assert_wire(&owner,first.admitted,&mut left,Intent::Create,ChatDisposition::Ok).await?;
                    assert_wire(&owner,first.admitted,&mut left,Intent::Invite {name:world::RECIPIENT_NAME.into()},ChatDisposition::Ok).await?;
                    assert_wire(&target,second.admitted,&mut right,Intent::Join {channel_id:channel},ChatDisposition::Ok).await?;
                    let current_epoch=epoch(&h,channel).await?;
                    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1,control_loss_epoch=1,original_grace_deadline=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint+30000000,predecessor_generation=current_generation WHERE game_session_id=encode($1,'hex')::uuid")
                        .bind(first.admitted.game_session_id.as_bytes().as_slice()).execute(&h.pool).await?;
                    let grace_line=line(&h,channel,current_epoch,&own_name,2).await?;
                    target.states.chat.deliver(&grace_line);
                    assert!(matches!(receive(&target,second.admitted).await?,ChatLine::PrivateChannel(_)),"a reconnectable owner retains its room during grace");
                    target.chat_delivered(second.admitted.game_session_id).await;
                    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3,current_transport_ref=NULL WHERE game_session_id=encode($1,'hex')::uuid")
                        .bind(first.admitted.game_session_id.as_bytes().as_slice()).execute(&h.pool).await?;
                    target.states.chat.deliver(&line(&h,channel,current_epoch,&own_name,3).await?);
                    assert!(target.chat_next_bound(second.admitted,&mut 64).await.is_none(),"terminal owner closes queued delivery immediately");
                    assert_wire(&target,second.admitted,&mut right,Intent::List,ChatDisposition::Ok).await?;
                    assert!(target.private_channel_list(second.admitted).await.ok_or("terminal owner list")?.invited.is_empty());
                    assert_eq!(count(&h,"SELECT count(*) FROM game_private_chat_channels").await?,0);
                    assert_eq!(count(&h,"SELECT count(*) FROM game_private_chat_members").await?,0);
                    assert_eq!(count(&h,"SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name LIKE 'game_private_chat_%' AND column_name IN('text','payload','ciphertext','request')").await?,0,"no chat text retained");
                    Ok::<(),Box<dyn std::error::Error>>(())
                };
                let mut listeners=std::pin::pin!(listeners);
                let mut scenario=std::pin::pin!(scenario);
                tokio::time::timeout(Duration::from_micros(crate::chat::relay::FRESHNESS_MICROS+crate::chat::relay::RELAY_LEASE_MICROS)+Duration::from_secs(30),std::future::poll_fn(|cx| {
                    let _=listeners.as_mut().poll(cx);scenario.as_mut().poll(cx)
                })).await??;
            }
            h.cleanup().await?;
            eprintln!("PRIVATE_CHANNEL POSTGRES_CASE_COMPLETED: World lifecycle, current fences, atomic relay receipt, exclusion and epoch reuse");
            Ok(())
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"private channel scenario panicked")?.map_err(Into::into)
}

fn select()->TestResult<SelectedCapabilities> {
    SelectedCapabilities::select(crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,&[6,7,42,80]).ok_or_else(||"private capability selection".into())
}
async fn dispatch(owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:AdmittedSession,command:u64,intent:Intent)->TestResult<ChatDisposition> {
    let payload=chat::encode_chat_intent(&ChatIntent::PrivateChannel(intent)).map_err(debug)?;
    let (result,spell)=wire::dispatch(owner,admitted,admitted.runtime_actor.ok_or("actor")?,command,&payload).await.map_err(|_|"private wire rejected")?.ok_or("private unknown result")?;
    assert!(spell.is_none());
    Ok(result.disposition)
}
async fn assert_wire(owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:AdmittedSession,command:&mut u64,intent:Intent,expected:ChatDisposition)->TestResult {
    assert_eq!(dispatch(owner,admitted,*command,intent).await?,expected);
    *command+=1;
    Ok(())
}
async fn count(h:&Harness,query:&'static str)->TestResult<i64> { Ok(sqlx::query_scalar(query).fetch_one(&h.pool).await?) }
async fn epoch(h:&Harness,channel:u16)->TestResult<u64> {
    let epoch:i64=sqlx::query_scalar("SELECT epoch FROM game_private_chat_channels WHERE world_id=encode($1,'hex')::uuid AND channel_id=$2").bind(id(42).as_slice()).bind(i32::from(channel)).fetch_one(&h.pool).await?;
    Ok(u64::try_from(epoch)?)
}
async fn receive(owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:AdmittedSession)->TestResult<ChatLine> {
    Ok(tokio::time::timeout(Duration::from_secs(5),async {
        loop { if let Some(line)=owner.chat_next_bound(admitted,&mut 64).await { break line; } tokio::time::sleep(Duration::from_millis(10)).await; }
    }).await?)
}
async fn line(h:&Harness,channel_id:u16,epoch:u64,name:&str,marker:u8)->TestResult<RelayLine> {
    let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint").fetch_one(&h.pool).await?;
    Ok(RelayLine {message_id:[marker;16],sent_at_micros:u64::try_from(now)?,destination:RelayDestination::PrivateChannel {channel_id,epoch},sender_name:crate::domain::character_name::CharacterName::parse(name).map_err(debug)?,text:crate::chat::ChatText::parse("queued old channel line").map_err(debug)?})
}

async fn invitation_caps(h:&Harness,owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:AdmittedSession,command:&mut u64,channel:u16,target:AdmittedSession)->TestResult {
    let base=admitted.item_fence.ok_or("cap fixture fence")?;
    let mut fixtures=Vec::new();
    for index in 0_u8..16 {
        let n=600+u32::from(index);
        let name=format!("Invite {}",char::from(b'A'+index));
        let fence=peers::active_peer(h,owner,base,n,id(42),&name).await?;
        fixtures.push((fence,name));
    }
    let _foreign=peers::active_peer(h,owner,base,616,id(99),"Foreign Friend").await?;
    assert_wire(owner,admitted,command,Intent::Invite {name:"Foreign Friend".into()},ChatDisposition::NotOnline).await?;
    // One real invitee already occupies a slot. The 15th extra fills the 16-invitation cap.
    for (index,(_,name)) in fixtures.iter().enumerate() {
        assert_wire(owner,admitted,command,Intent::Invite {name:name.clone()},if index<15 {ChatDisposition::Ok}else{ChatDisposition::Rejected}).await?;
    }
    assert_wire(owner,admitted,command,Intent::Exclude {name:world::RECIPIENT_NAME.into()},ChatDisposition::Ok).await?;
    // The sender now has one free slot, while the recipient is independently invited to 16
    // fixture rooms. This isolates the invitee cap from the channel's own capacity check.
    for (index,(fence,_)) in fixtures.iter().enumerate() {
        let fixture_channel=300+i32::try_from(index)?;
        sqlx::query("INSERT INTO game_private_chat_channels(world_id,channel_id,owner_session_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)")
            .bind(id(42).as_slice()).bind(fixture_channel).bind(fence.game_session_id.as_bytes().as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_private_chat_members(world_id,channel_id,game_session_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)")
            .bind(id(42).as_slice()).bind(fixture_channel).bind(target.game_session_id.as_bytes().as_slice()).execute(&h.pool).await?;
    }
    assert_wire(owner,admitted,command,Intent::Invite {name:world::RECIPIENT_NAME.into()},ChatDisposition::Rejected).await?;
    sqlx::query("DELETE FROM game_private_chat_members WHERE world_id=encode($1,'hex')::uuid AND channel_id=300")
        .bind(id(42).as_slice()).execute(&h.pool).await?;
    assert_wire(owner,admitted,command,Intent::Invite {name:world::RECIPIENT_NAME.into()},ChatDisposition::Ok).await?;
    // Remove only this helper's synthetic peers after proving both native mutation refusals.
    sqlx::query("DELETE FROM game_private_chat_channels WHERE world_id=encode($1,'hex')::uuid AND channel_id BETWEEN 300 AND 315")
        .bind(id(42).as_slice()).execute(&h.pool).await?;
    for (fence,_) in fixtures {
        sqlx::query("DELETE FROM game_private_chat_members WHERE world_id=encode($1,'hex')::uuid AND channel_id=$2 AND game_session_id=encode($3,'hex')::uuid")
            .bind(id(42).as_slice()).bind(i32::from(channel)).bind(fence.game_session_id.as_bytes().as_slice()).execute(&h.pool).await?;
    }
    Ok(())
}
