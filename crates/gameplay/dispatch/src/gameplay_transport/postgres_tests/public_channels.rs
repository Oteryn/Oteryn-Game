//! S04c: real CHAT command dispatch, Control roles, Help receipts and DB read-back.
use crate::bestiary_postgres_harness::{self as fixture, CHANNEL, Harness, TestResult, WORLD, debug, id};
use crate::foundation::{ChannelId, ChannelRuntimeV1, MessageType, WorldId, decode_wire_envelope};
use crate::gameplay_transport::{ComposedFreshAdmission, capabilities::SelectedCapabilities,
    connection::{AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority,
        IDLE_LIVENESS, SessionContinuity, serve_admitted}, tcp_tls::{read_frame, write_frame}};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_protocol_oteryn::{ClientCommandValue, CommandStatus, decode_command_result,
    encode_client_command, chat::{self, ChatDisposition, ChatIntent, ChatRoom, ChatRoomSet}};
use sqlx::Row;
use std::{sync::Arc, time::Duration};
use tokio::{io::AsyncWriteExt, sync::Mutex};

#[test]
fn public_channels_postgres_control_roles_help_mutation_replay_and_revocation() -> TestResult {
    let Some(admin) = fixture::configured_admin() else { return Ok(()); };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        fixture::runtime().map_err(debug)?.block_on(async move {
            let h = Harness::create(admin, "public_channels_wire", false).await.map_err(debug)?;
            let result = scenario(&h).await.map_err(|error| error.to_string());
            h.cleanup().await.map_err(debug)?;
            result
        })
    })?.join().map_err(|_| "public channels thread panicked")??;
    eprintln!("PUBLIC_CHANNELS POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn issue_role(h: &Harness, character: [u8;16], expected: i64, group: i16,
    account: i16, access: bool, receipt: u8) -> TestResult<i64>
{
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_control").execute(&mut *tx).await?;
    let revision: i64 = sqlx::query_scalar("SELECT game_chat_role_issue(encode($1,'hex')::uuid,$2,$3,$4,$5,encode($6,'hex')::uuid,$7)")
        .bind(character.as_slice()).bind(expected).bind(group).bind(account).bind(access)
        .bind(id(receipt).as_slice()).bind("public-channel-fixture")
        .fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(revision)
}

async fn runtime_cannot_issue_roles(h: &Harness, character: [u8;16]) -> TestResult {
    let mut read = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *read).await?;
    sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended((encode($1,'hex')::uuid)::text,717))")
        .bind(character.as_slice()).execute(&mut *read).await?;
    let group: i16 = sqlx::query_scalar("SELECT group_id FROM game_chat_roles WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_slice()).fetch_one(&mut *read).await?;
    assert_eq!(group,2);
    read.commit().await?;
    for statement in [
        "INSERT INTO game_chat_roles SELECT * FROM game_chat_roles WHERE character_id=encode($1,'hex')::uuid",
        "UPDATE game_chat_roles SET group_id=6 WHERE character_id=encode($1,'hex')::uuid",
        "DELETE FROM game_chat_roles WHERE character_id=encode($1,'hex')::uuid",
        "SELECT game_chat_role_issue(encode($1,'hex')::uuid,1::bigint,6::smallint,6::smallint,true,encode($1,'hex')::uuid,'forged')",
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
        match sqlx::query(statement).bind(character.as_slice()).execute(&mut *tx).await {
            Err(sqlx::Error::Database(error)) => assert_eq!(error.code().as_deref(),Some("42501")),
            other => return Err(format!("runtime role write must be denied: {other:?}").into()),
        }
        tx.rollback().await?;
    }
    let schema: String = sqlx::query_scalar("SELECT current_schema()::text").fetch_one(&h.pool).await?;
    let expected_path = format!("search_path=pg_catalog, {schema}, pg_temp");
    for (signature,definer) in [
        ("game_chat_role_issue(uuid,bigint,smallint,smallint,boolean,uuid,text)",true),
        ("game_chat_role_receipt_immutable()",false),
        ("game_chat_role_projection_consistent()",false),
    ] {
        let function = sqlx::query("SELECT p.prosecdef,pg_get_userbyid(p.proowner)::text AS owner,p.proconfig, \
            has_function_privilege('oteryn_game_runtime',p.oid,'EXECUTE') AS runtime_execute, \
            EXISTS(SELECT 1 FROM aclexplode(COALESCE(p.proacl,acldefault('f',p.proowner))) a \
                WHERE a.grantee=0 AND a.privilege_type='EXECUTE') AS public_execute \
            FROM pg_proc p WHERE p.oid=$1::text::regprocedure")
            .bind(signature).fetch_one(&h.pool).await?;
        assert_eq!(function.try_get::<bool,_>("prosecdef")?,definer,"{signature}");
        if definer { assert_eq!(function.try_get::<String,_>("owner")?,"oteryn_game_control"); }
        assert!(!function.try_get::<bool,_>("runtime_execute")?,"{signature}");
        assert!(!function.try_get::<bool,_>("public_execute")?,"{signature}");
        assert!(function.try_get::<Vec<String>,_>("proconfig")?.contains(&expected_path),"{signature}");
    }
    Ok(())
}

async fn command(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    steps: &[(ChatIntent, ChatDisposition)], replay_last: bool) -> TestResult<Vec<u64>>
{
    let session = *admitted;
    let (mut server, mut client) = tokio::io::duplex(16_384);
    let serving = async {
        let end = serve_admitted(&mut server, session, owner, IDLE_LIVENESS).await;
        let _ = server.shutdown().await;
        end
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type() == MessageType::SnapshotCommit { break; }
        }
        let mut ids = Vec::new();
        for (offset,(intent,expected)) in steps.iter().enumerate() {
            let command_id = session.continuity.next_command_id+u64::try_from(offset)?;
            ids.push(command_id);
            let payload = chat::encode_chat_intent(intent).map_err(debug)?;
            let frame = encode_client_command(session.continuity.connection_generation,
                &ClientCommandValue { command_id, command_type: chat::COMMAND_TYPE_CHAT_INTENT, payload: &payload }).map_err(debug)?;
            let count = if replay_last && offset+1 == steps.len() { 2 } else { 1 };
            for repeat in 0..count {
                write_frame(&mut client,&frame).await?;
                loop {
                    let frame = read_frame(&mut client).await?;
                    let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                    if envelope.message_type() != MessageType::CommandResult { continue; }
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id,command_id,
                        "CHAT_INTENT command {command_id}, expected disposition {expected:?}");
                    let expected_status = if repeat != 0 {
                        CommandStatus::DuplicateReplay
                    } else if *expected == ChatDisposition::Rejected {
                        CommandStatus::Rejected
                    } else {
                        CommandStatus::Accepted
                    };
                    assert_eq!(result.status,expected_status,
                        "CHAT_INTENT command {command_id}, expected disposition {expected:?}, replay {repeat}");
                    let result = chat::decode_chat_intent_result(result.payload).map_err(debug)?;
                    assert_eq!(result.disposition,*expected,"CHAT_INTENT command {command_id}, replay {repeat}");
                    if result.disposition == ChatDisposition::Muted { assert!((1..=180).contains(&result.wait_seconds)); }
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok::<_,Box<dyn std::error::Error>>(ids)
    };
    let (end,result) = tokio::time::timeout(Duration::from_secs(30),super::fixture::join_two(serving,driving)).await?;
    match end { ConnectionEnd::AdmittedThenDisconnected(current) => *admitted=current,
        other => return Err(format!("public channel end: {other:?}").into()) }
    result
}

fn help(text: impl Into<String>) -> ChatIntent { ChatIntent::Room { room: ChatRoom::Help, text:text.into() } }

async fn scenario(h: &Harness) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut currents = Vec::new();
    for (character, account, session) in [(60, 63, 70), (61, 64, 71), (62, 65, 72)] {
        currents.push(
            crate::world_party_runtime_postgres_cases::committed_fresh_player(
                h, character, account, session, now,
            )
            .await?,
        );
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        5,
        ChannelContentPin::test(world),
    )
    .map_err(debug)?;
    let mut admitted = Vec::new();
    for current in &currents {
        let session = current.current_game_session_id();
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        admitted.push(AdmittedSession {
            game_session_id: session,
            world_id: world,
            channel_id: channel,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: crate::gameplay_transport::item_fence_of(*current),
        });
    }
    let content = crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
        .map_err(debug)?;
    let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
        currents[0].current_runtime_scope(),
        currents[0].current_scope_generation(),
        content,
    );
    let door = Mutex::new(
        crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            currents[0].current_runtime_scope(),
            currents[0].current_scope_generation(),
        )
        .map_err(debug)?,
    );
    let descriptor=crate::native_admission_source::descriptor::ProducerDescriptor::new("platform-test".into(),("127.0.0.1".into(),1),"source.test".into(),"source.test".into(),
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec())],
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec())],
        rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der").to_vec())?).map_err(debug)?;
    let evidence = crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
    let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
    let achievements =
        crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
    let charms =
        crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
    let runtime = Mutex::new(runtime);
    let owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
        root: &h.root, character: &recovery, holder: &h.node, evidence: &evidence, world_id: world, channel_id: channel,
        required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(&[],currents[0].current_runtime_scope(),1,"policy-1",crate::gameplay_transport::world_required_capabilities::offered_for_scope(false,None)).map_err(debug)?,
        runtime: &runtime, door: &door, spells: &spells, achievements: &achievements, imported_charms: &charms, room: &room,
    }.build();
    let moderator = *admitted[0].item_fence.ok_or("moderator fence")?.character_id.as_bytes();
    let target = *admitted[1].item_fence.ok_or("target fence")?.character_id.as_bytes();
    let target_name: String = sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(target.as_slice()).fetch_one(&h.pool).await?;
    let ordinary = *admitted[2].item_fence.ok_or("ordinary fence")?.character_id.as_bytes();
    let ordinary_name: String = sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(ordinary.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(issue_role(h,moderator,0,2,2,false,90).await?,1);
    // Same Game group, ordinary account type: Help deliberately compares group to account.
    assert_eq!(issue_role(h,target,0,2,1,false,91).await?,1);
    runtime_cannot_issue_roles(h,moderator).await?;
    assert_eq!(issue_role(h,moderator,0,2,2,false,90).await?,1);
    assert!(issue_role(h,moderator,0,4,4,true,95).await.is_err());
    let ring = crate::chat::relay::RelayKeyRing::new(vec![crate::chat::relay::RelayKey::new(1,0,[1;32])]).map_err(debug)?;
    let relay = crate::gameplay_transport::chat_relay::ChatRelay::new(ring,id(WORLD),
        h.root.chat_relay_connect_options().ok_or("relay endpoint")?).map_err(debug)?;
    let micros: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
        .fetch_one(&h.pool).await?;
    let relay = Arc::new(relay);
    owner.states.chat.install_relay(relay.clone());
    for (index,a) in admitted.iter_mut().enumerate() {
        let supported: &[u32] = if index==2 { &[6,7,80] } else { &[6,7,80,259] };
        a.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            supported)
            .ok_or("public channel selection")?;
        assert_eq!(a.continuity.selected_capabilities.as_slice(),supported,
            "public channel fixture must select the complete bundle capability closure");
        owner.chat_admit(a.runtime_actor.ok_or("actor")?,a.game_session_id).await;
        owner.public_chat_admit(*a,ChatRoomSet::default()).await;
    }
    assert!(owner.states.chat.public_room_list(admitted[0].game_session_id).contains(ChatRoom::Tutor));
    assert!(!owner.states.chat.public_room_list(admitted[0].game_session_id).contains(ChatRoom::Gamemaster));
    assert!(owner.states.chat.public_room_list(admitted[0].game_session_id).contains(ChatRoom::AdvertisingRookgaard));
    assert!(!owner.states.chat.public_room_list(admitted[0].game_session_id).contains(ChatRoom::Advertising));
    let unavailable = command(&owner,&mut admitted[0],&[
        (help(format!("!mute {target_name}")),ChatDisposition::ChatUnavailable),
    ],true).await?[0];
    relay.force_ready(u64::try_from(micros)?);
    assert_eq!(owner.public_chat_preflight(admitted[0],unavailable,&help(format!("!mute {target_name}"))).await,
        Some(Some(chat::ChatIntentResult {disposition:ChatDisposition::ChatUnavailable,wait_seconds:0})));
    assert_eq!(owner.public_chat_preflight(admitted[0],unavailable,&help(format!("!unmute {target_name}"))).await,
        Some(Some(chat::ChatIntentResult {disposition:ChatDisposition::Rejected,wait_seconds:0})));
    let closed = command(&owner,&mut admitted[0],&[
        (help(format!("!mute {target_name}")),ChatDisposition::RoomNotOpen),
    ],false).await?[0];
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_chat_help_mutes")
        .fetch_one(&h.pool).await?,0);
    let ids = command(&owner,&mut admitted[0],&[
        (ChatIntent::OpenRoom(ChatRoom::Help),ChatDisposition::Ok),
        (ChatIntent::OpenRoom(ChatRoom::Tutor),ChatDisposition::Ok),
        (ChatIntent::OpenRoom(ChatRoom::Gamemaster),ChatDisposition::NotAllowed),
        (help(format!("!mute {target_name}")),ChatDisposition::Ok),
    ],true).await?;
    let first_mute = *ids.last().ok_or("mute command")?;
    let deadlines: (i64,i64) = sqlx::query_as("SELECT condition_until_ms,speech_until_ms FROM game_chat_help_mutes WHERE character_id=encode($1,'hex')::uuid")
        .bind(target.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(deadlines.0-deadlines.1,3_420_000);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_chat_help_receipts WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(admitted[0].game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?,3);
    assert_eq!(owner.public_chat_preflight(admitted[0],closed,&help(format!("!mute {target_name}"))).await,
        Some(Some(chat::ChatIntentResult {disposition:ChatDisposition::RoomNotOpen,wait_seconds:0})));
    let silenced = command(&owner,&mut admitted[1],&[
        (ChatIntent::OpenRoom(ChatRoom::Help),ChatDisposition::Ok),
        (help("silenced"),ChatDisposition::Muted),
        (help(format!("!mute {ordinary_name}")),ChatDisposition::Muted),
    ],false).await?;
    let muted_command = *silenced.last().ok_or("muted moderator command")?;
    command(&owner,&mut admitted[0],&[(help(format!("!unmute {target_name}")),ChatDisposition::Ok)],false).await?;
    let refused = owner.public_chat_preflight(admitted[1],muted_command,
        &help(format!("!mute {ordinary_name}"))).await;
    assert!(matches!(refused,Some(Some(chat::ChatIntentResult {disposition:ChatDisposition::Muted,wait_seconds:1..=180}))));
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_chat_help_mutes WHERE character_id=encode($1,'hex')::uuid")
        .bind(ordinary.as_slice()).fetch_one(&h.pool).await?,0);
    command(&owner,&mut admitted[1],&[(help("speaking again"),ChatDisposition::Ok)],false).await?;
    let fence = admitted[0].item_fence.ok_or("moderator fence")?;
    let replay = h.root.mutate_help_channel(&recovery,&h.node,fence,first_mute,
        crate::durability::chat_public::HelpAction::Mute,
        crate::domain::character_name::CharacterName::parse(&target_name).map_err(debug)?)
        .await.map_err(debug)?.ok_or("replay")?;
    assert_eq!(replay.result.disposition,ChatDisposition::Ok);
    assert!(!replay.applied);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT speech_until_ms FROM game_chat_help_mutes WHERE character_id=encode($1,'hex')::uuid")
        .bind(target.as_slice()).fetch_one(&h.pool).await?,0);
    let changed = h.root.mutate_help_channel(&recovery,&h.node,fence,first_mute,
        crate::durability::chat_public::HelpAction::Unmute,
        crate::domain::character_name::CharacterName::parse(&target_name).map_err(debug)?)
        .await.map_err(debug)?.ok_or("changed replay")?;
    assert_eq!(changed.result.disposition,ChatDisposition::Rejected);
    assert!(!changed.applied);
    command(&owner,&mut admitted[0],&[(help(format!("!mute {target_name}")),ChatDisposition::Ok)],false).await?;
    // Advance the durable deadline under the fixture owner instead of sleeping for three minutes.
    sqlx::query("UPDATE game_chat_help_mutes SET speech_until_ms=0 WHERE character_id=encode($1,'hex')::uuid")
        .bind(target.as_slice()).execute(&h.pool).await?;
    command(&owner,&mut admitted[1],&[(help("three-minute exhaustion ended"),ChatDisposition::Ok)],false).await?;
    command(&owner,&mut admitted[0],&[(help(format!("!unmute {target_name}")),ChatDisposition::Rejected)],false).await?;
    let refused = h.root.mutate_help_channel(&recovery,&h.node,fence,900,
        crate::durability::chat_public::HelpAction::Mute,
        crate::domain::character_name::CharacterName::parse(&target_name).map_err(debug)?)
        .await.map_err(debug)?.ok_or("still has one-hour condition")?;
    assert_eq!(refused.result.disposition,ChatDisposition::Rejected);
    assert!(!refused.applied);
    // One changed invariant: obsolete connection generation cannot write or record a result.
    let mut stale = fence;
    stale.connection_generation = crate::foundation::ConnectionGeneration::new(fence.connection_generation.get()+1).map_err(debug)?;
    assert!(h.root.mutate_help_channel(&recovery,&h.node,stale,901,
        crate::durability::chat_public::HelpAction::Unmute,
        crate::domain::character_name::CharacterName::parse(&target_name).map_err(debug)?)
        .await.map_err(debug)?.is_none());
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_chat_help_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=901")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?,0);
    // Account rank independently prevents a Tutor from muting an equal account tier.
    assert_eq!(issue_role(h,target,1,2,2,false,92).await?,2);
    let refused = h.root.mutate_help_channel(&recovery,&h.node,fence,902,
        crate::durability::chat_public::HelpAction::Mute,
        crate::domain::character_name::CharacterName::parse(&target_name).map_err(debug)?)
        .await.map_err(debug)?.ok_or("equal target account")?;
    assert_eq!(refused.result.disposition,ChatDisposition::NotAllowed);
    let offline = h.root.mutate_help_channel(&recovery,&h.node,fence,903,
        crate::durability::chat_public::HelpAction::Mute,
        crate::domain::character_name::CharacterName::parse("Absent Player").map_err(debug)?)
        .await.map_err(debug)?.ok_or("offline target")?;
    assert_eq!(offline.result.disposition,ChatDisposition::NotOnline);
    // Simulate COMMIT succeeded but ingress completion was lost: the next wire command
    // has a durable result already, then Control revokes the moderator's Game group.
    assert_eq!(issue_role(h,target,2,2,1,false,96).await?,3);
    sqlx::query("UPDATE game_chat_help_mutes SET condition_until_ms=0,speech_until_ms=0 WHERE character_id=encode($1,'hex')::uuid")
        .bind(target.as_slice()).execute(&h.pool).await?;
    assert_eq!(owner.public_chat_preflight(admitted[0],902,&help(format!("!mute {target_name}"))).await,
        Some(Some(chat::ChatIntentResult {disposition:ChatDisposition::NotAllowed,wait_seconds:0})));
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT speech_until_ms FROM game_chat_help_mutes WHERE character_id=encode($1,'hex')::uuid")
        .bind(target.as_slice()).fetch_one(&h.pool).await?,0);
    let pending_id = admitted[0].continuity.next_command_id;
    let pending = h.root.mutate_help_channel(&recovery,&h.node,fence,pending_id,
        crate::durability::chat_public::HelpAction::Mute,
        crate::domain::character_name::CharacterName::parse(&target_name).map_err(debug)?)
        .await.map_err(debug)?.ok_or("uncertain commit fixture")?;
    assert_eq!(pending.result.disposition,ChatDisposition::Ok);
    assert!(pending.applied);
    let pending_deadlines: (i64,i64) = sqlx::query_as("SELECT condition_until_ms,speech_until_ms FROM game_chat_help_mutes WHERE character_id=encode($1,'hex')::uuid")
        .bind(target.as_slice()).fetch_one(&h.pool).await?;
    // Group and account are not equated: account GM opens Gamemaster but ordinary group cannot Tutor.
    assert_eq!(issue_role(h,moderator,1,1,4,false,93).await?,2);
    let settled = command(&owner,&mut admitted[0],&[(help(format!("!mute {target_name}")),ChatDisposition::Ok)],true).await?;
    assert_eq!(settled,[pending_id]);
    let after_replay: (i64,i64) = sqlx::query_as("SELECT condition_until_ms,speech_until_ms FROM game_chat_help_mutes WHERE character_id=encode($1,'hex')::uuid")
        .bind(target.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after_replay,pending_deadlines);
    assert_eq!(owner.public_chat_preflight(admitted[0],pending_id,
        &help(format!("!unmute {target_name}"))).await,
        Some(Some(chat::ChatIntentResult {disposition:ChatDisposition::Rejected,wait_seconds:0})));
    command(&owner,&mut admitted[0],&[
        (ChatIntent::OpenRoom(ChatRoom::Gamemaster),ChatDisposition::Ok),
        (ChatIntent::OpenRoom(ChatRoom::Tutor),ChatDisposition::NotAllowed),
    ],false).await?;
    assert!(!owner.states.chat.show_rooms(admitted[0].game_session_id).contains(ChatRoom::Tutor));
    assert_eq!(issue_role(h,moderator,2,1,1,false,94).await?,3);
    command(&owner,&mut admitted[0],&[(ChatIntent::OpenRoom(ChatRoom::Gamemaster),ChatDisposition::NotAllowed)],false).await?;
    assert!(!owner.states.chat.show_rooms(admitted[0].game_session_id).contains(ChatRoom::Gamemaster));
    let legacy = admitted[2];
    owner.public_chat_admit(legacy,ChatRoomSet::default()).await;
    let listed = owner.states.chat.public_room_list(legacy.game_session_id);
    assert!(listed.contains(ChatRoom::Help));
    for room in [ChatRoom::Tutor,ChatRoom::Gamemaster,ChatRoom::AdvertisingRookgaard,ChatRoom::GuildLeaders] {
        assert!(!listed.contains(room));
        let payload = chat::encode_chat_intent(&ChatIntent::OpenRoom(room)).map_err(debug)?;
        assert!(crate::gameplay_transport::guild_chat::wire::dispatch(&owner,legacy,
            legacy.runtime_actor.ok_or("legacy actor")?,1,&payload).await.is_err());
    }
    let projection = sqlx::query("SELECT revision,group_id,account_type,audit_receipt IS NOT NULL AS audited FROM game_chat_roles WHERE character_id=encode($1,'hex')::uuid")
        .bind(moderator.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(projection.try_get::<i64,_>("revision")?,3);
    assert_eq!(projection.try_get::<i16,_>("group_id")?,1);
    assert_eq!(projection.try_get::<i16,_>("account_type")?,1);
    assert!(projection.try_get::<bool,_>("audited")?);
    Ok(())
}
