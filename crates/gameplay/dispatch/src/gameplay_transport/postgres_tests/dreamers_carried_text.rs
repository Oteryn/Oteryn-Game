//! Crystal 201189f data-global/startup/tables/chest.lua:2124-2129.
use super::TestResult;
use super::fixture::debug;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted},
    tcp_tls::read_frame,
};
use oteryn_protocol_oteryn::{
    CommandStatus, MessageType, decode_snapshot_chunk, decode_wire_envelope,
    item_view::{
        CAPABILITY_ITEM_APPEARANCE_V1, CAPABILITY_ITEM_EQUIP_DROP_V1,
        STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory_with_appearances,
    },
    server_modal::{
        Answer, COMMAND_TYPE_SERVER_MODAL_ANSWER, COMMAND_TYPE_SERVER_MODAL_QUERY, Reply,
    },
    world_object::{
        CAPABILITY_ITEM_USE_V1, COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget,
        decode_use_result, encode_item_use_intent,
    },
};
use tokio::io::AsyncWriteExt;

#[test]
fn dreamers_chest_book_opens_its_authored_body_in_the_carried_read_window() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
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

    let mut supported = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    supported.push(CAPABILITY_ITEM_USE_V1);
    supported.push(oteryn_protocol_oteryn::server_modal::CAPABILITY_SERVER_MODAL_V1);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &supported)
            .ok_or("book modal capabilities")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let playing = async {
        let mut book = None;
        let selected = &admitted.continuity.selected_capabilities;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload())?.1 {
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory = decode_character_inventory_with_appearances(
                            domain.payload,
                            selected.contains(CAPABILITY_ITEM_EQUIP_DROP_V1),
                            selected.contains(CAPABILITY_ITEM_APPEARANCE_V1),
                        )
                        .map_err(debug)?;
                        assert_eq!(inventory.entries.len(), 1, "the chest's one carried reward");
                        book = inventory.entries.first().map(|item| item.handle);
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let mut next = admitted.continuity.next_command_id;
        let use_intent = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(book.ok_or("advertised quest book")?),
            use_with: None,
            use_with_position: None,
            use_with_map_item: None,
        })
        .map_err(debug)?;
        let command = super::server_modal_bed::command;
        let (status, payload) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_USE_INTENT,
            &use_intent,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            decode_use_result(&payload).map_err(debug)?,
            UseDisposition::Committed
        );

        let (status, payload) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_SERVER_MODAL_QUERY,
            &[],
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        let Reply::Dialog(dialog) = Reply::decode(&payload).map_err(debug)? else {
            return Err("book USE must provide a text window".into());
        };
        assert_eq!(dialog.title, "book");
        assert_eq!(
            dialog.message,
            "Askuro\nMantar\nIldoni\nMaxmof\nXemodo\nOxanti"
        );
        assert!(dialog.choices.is_empty());
        assert_eq!(dialog.buttons, vec![(1, "Close".into())]);
        let close = Answer {
            dialog_id: dialog.id,
            button_id: 1,
            choice_id: None,
        };
        let (status, payload) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_SERVER_MODAL_ANSWER,
            &close.encode().map_err(debug)?,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            Reply::decode(&payload).map_err(debug)?,
            Reply::Answered(close)
        );
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
    )
    .await?;
    read?;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
        "{end:?}"
    );
    Ok(())
}
