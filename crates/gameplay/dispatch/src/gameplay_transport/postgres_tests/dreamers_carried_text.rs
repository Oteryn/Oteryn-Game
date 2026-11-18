//! Crystal 201189f data-global/startup/tables/chest.lua:2124-2129.
use super::TestResult;
use super::fixture::debug;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted},
    tcp_tls::{read_frame, write_frame},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_snapshot_chunk,
    decode_state_delta, decode_wire_envelope, encode_client_command,
    container_tree::{
        COMMAND_TYPE_CONTAINER_VIEW_INTENT, ContainerViewIntent, STATE_DOMAIN_CONTAINER_VIEWS,
        decode_container_views, encode_container_view_intent,
    },
    item_view::{STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory},
    server_modal::{
        Answer, COMMAND_TYPE_SERVER_MODAL_ANSWER, COMMAND_TYPE_SERVER_MODAL_QUERY, Reply,
    },
    world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, decode_use_result,
        encode_item_use_intent,
    },
};
use tokio::io::AsyncWriteExt;

#[test]
fn dreamers_chest_book_opens_its_authored_body_in_the_carried_read_window() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("dreamers-carried-text", || {
        super::quest_container_chests::read_dreamers_book()
    })
}

pub(super) async fn read(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    fence: CurrentCharacterItemFence,
) -> TestResult {
    use crate::foundation::{CharacterId, CharacterLease};

    let mut supported = admitted.continuity.selected_capabilities.as_slice().to_vec();
    supported.push(oteryn_protocol_oteryn::container_tree::CAPABILITY_CONTAINER_TREE_V1);
    supported.push(oteryn_protocol_oteryn::server_modal::CAPABILITY_SERVER_MODAL_V1);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &supported)
            .ok_or("book modal capabilities")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner.runtime.lock().await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            ).map_err(debug)?,
        ).map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let playing = async {
        let mut backpack = None;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload())?.1 {
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        backpack = decode_character_inventory(domain.payload).map_err(debug)?
                            .main_backpack.map(|item| item.handle);
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let backpack = backpack.ok_or("advertised backpack")?;
        let mut next = admitted.continuity.next_command_id;
        let open = encode_container_view_intent(&ContainerViewIntent::Open {
            handle: backpack,
            replace_view: None,
        }).map_err(debug)?;
        write_frame(&mut client, &encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue { command_id: next, command_type: COMMAND_TYPE_CONTAINER_VIEW_INTENT, payload: &open },
        )?).await?;
        let mut book = None;
        let mut opened = false;
        while !opened || book.is_none() {
            let bytes = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&bytes)?;
            match env.message_type() {
                MessageType::CommandResult => {
                    let result = decode_command_result(env.payload())?;
                    assert_eq!(result.command_id, next);
                    assert_eq!(result.status, CommandStatus::Accepted);
                    opened = true;
                }
                MessageType::StateDelta => {
                    let delta = decode_state_delta(env.payload())?;
                    if delta.domain_id == STATE_DOMAIN_CONTAINER_VIEWS {
                        let displayed = decode_container_views(delta.payload).map_err(debug)?;
                        book = displayed.views.iter()
                            .find(|view| view.container_handle == backpack)
                            .and_then(|view| view.entries.first()).map(|entry| entry.handle);
                    }
                }
                _ => {}
            }
        }
        next += 1;
        let use_intent = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(book.ok_or("advertised quest book")?),
            use_with: None,
            use_with_position: None,
            use_with_map_item: None,
        }).map_err(debug)?;
        let command = super::server_modal_bed::command;
        let (status, payload) = command(&mut client, admitted, &mut next, COMMAND_TYPE_USE_INTENT, &use_intent).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(decode_use_result(&payload).map_err(debug)?, UseDisposition::Committed);

        let (status, payload) = command(&mut client, admitted, &mut next, COMMAND_TYPE_SERVER_MODAL_QUERY, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let Reply::Dialog(dialog) = Reply::decode(&payload).map_err(debug)? else {
            return Err("book USE must provide a text window".into());
        };
        assert_eq!(dialog.title, "book");
        assert_eq!(dialog.message, "Askuro\nMantar\nIldoni\nMaxmof\nXemodo\nOxanti");
        assert!(dialog.choices.is_empty());
        assert_eq!(dialog.buttons, vec![(1, "Close".into())]);
        let close = Answer { dialog_id: dialog.id, button_id: 1, choice_id: None };
        let (status, payload) = command(&mut client, admitted, &mut next, COMMAND_TYPE_SERVER_MODAL_ANSWER, &close.encode().map_err(debug)?).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(Reply::decode(&payload).map_err(debug)?, Reply::Answered(close));
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let (end, read) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, playing),
    ).await?;
    read?;
    assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)), "{end:?}");
    Ok(())
}
