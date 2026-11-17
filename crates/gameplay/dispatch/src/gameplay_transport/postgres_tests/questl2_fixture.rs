use super::{TestResult,AdmittedSession,ComposedFreshAdmission};
use super::fixture::debug;
use crate::content::{NpcDataCatalogue,npc_catalogue_preproduction_limits};
use crate::foundation::{MovementFacing,NpcPlacement,NpcPlacementTable,NpcPlacements,MessageType,decode_wire_envelope};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
pub(super) async fn install_at<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    keys: [&str; N],
    positions: [Position; N],
) -> TestResult<[EntityRef; N]> {
    install_offsets(owner, admitted, keys, |_| (0, 0), Some(positions)).await
}

async fn install_offsets<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    keys: [&str; N],
    offset: impl Fn(i32) -> (i32, i32),
    positions: Option<[Position; N]>,
) -> TestResult<[EntityRef; N]> {
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    let source =
        oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog::runtime().map_err(debug)?;
    let native_npcs = source
        .programs()
        .iter()
        .flat_map(|program| program.document.rules.iter().map(|rule| rule.npc.as_str()))
        .chain(keys.iter().copied())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &native_npcs),
    ));
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Quest witness", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    runtime
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let at = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let mut placements = keys
        .into_iter()
        .enumerate()
        .map(|(i, key)| {
            let (dx, dy) = offset(i32::try_from(i)?);
            let cell = if let Some(positions) = &positions {
                positions[i]
            } else {
                Position {
                    x: u16::try_from(at.x + dx)?,
                    y: u16::try_from(at.y + dy)?,
                    floor: -i8::try_from(at.floor)?,
                }
            };
            Ok(NpcPlacements {
                key: key.into(),
                placements: vec![NpcPlacement {
                    x: cell.x,
                    y: cell.y,
                    floor: cell.floor,
                    direction: MovementFacing::South,
                }],
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    placements.sort_by(|left, right| left.key.cmp(&right.key));
    runtime
        .place_npcs(&NpcPlacementTable { npcs: placements }, |_| true, true)
        .map_err(debug)?;
    let mut found = Vec::new();
    for key in keys {
        found.push(
            runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find_map(|npc| {
                    (runtime.npc_key(npc.actor) == Some(key)).then_some(EntityRef {
                        identity: npc.actor.placement_identity(),
                        generation: npc.generation,
                    })
                })
                .ok_or("placed source NPC")?,
        );
    }
    found.try_into().map_err(|_| "exact NPC refs".into())
}

pub(super) async fn wire_sequence(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: Vec<(u64,u32,Vec<u8>)>,
) -> TestResult<Vec<Vec<u8>>> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::{AsyncReadExt,AsyncWriteExt};
    assert!(!commands.is_empty() && commands.len()<=16);
    let generation=admitted.continuity.connection_generation;
    let mut frames=Vec::new();
    for (id,kind,payload) in commands {
        frames.push((id,oteryn_protocol_oteryn::encode_client_command(generation,
            &oteryn_protocol_oteryn::ClientCommandValue {command_id:id,command_type:kind,payload:&payload})
            .map_err(debug)?));
    }
    let (mut server,mut client)=tokio::io::duplex(1<<20);
    let reader=tokio::spawn(async move {
        let receiving=async {
            let mut results=Vec::new();
            for (id,frame) in frames {
                client.write_all(&u32::try_from(frame.len())?.to_be_bytes()).await?;
                client.write_all(&frame).await?;
                loop {
                    let mut prefix=[0;4];client.read_exact(&mut prefix).await?;
                    let mut bytes=vec![0;u32::from_be_bytes(prefix) as usize];
                    client.read_exact(&mut bytes).await?;
                    let envelope=decode_wire_envelope(&bytes).map_err(debug)?;
                    if envelope.message_type()==MessageType::CommandResult {
                        let command=oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
                        assert_eq!(command.command_id,id);
                        results.push(command.payload.to_vec());break;
                    }
                }
            }
            client.shutdown().await?;
            loop {
                let mut prefix=[0;4];
                match client.read_exact(&mut prefix).await {
                    Ok(_)=>{},
                    Err(error) if error.kind()==std::io::ErrorKind::UnexpectedEof=>break,
                    Err(error)=>return Err(error.into()),
                }
                let mut bytes=vec![0;u32::from_be_bytes(prefix) as usize];
                client.read_exact(&mut bytes).await?;
                let envelope=decode_wire_envelope(&bytes).map_err(debug)?;
                assert_ne!(envelope.message_type(),MessageType::CommandResult,"unexpected extra result");
            }
            Ok(results)
        };
        let result:TestResult<Vec<Vec<u8>>>=receiving.await;
        result.map_err(|error|error.to_string())
    });
    let end=serve_admitted(&mut server,*admitted,owner,IDLE_LIVENESS).await;
    if let ConnectionEnd::AdmittedThenClosed(_,error)=&end {
        eprintln!("quest wire sequence closed={error:?}");
    }
    drop(server);
    let result=reader.await?.map_err(Into::into);
    if let ConnectionEnd::AdmittedThenDisconnected(a)|ConnectionEnd::AdmittedThenClosed(a,_)=end {*admitted=a;}
    result
}
