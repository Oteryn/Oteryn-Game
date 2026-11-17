use super::*;

#[test]
fn tileless_carried_context_actions_reach_the_session_dispatch() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(exchange_carried())
        })?
        .join()
        .map_err(|_| "carried context test panicked")?
}

async fn exchange_carried() -> Result<(), BoxError> {
    use oteryn_session::container_tree::{ContainerView, ContainerViews, encode_container_views};
    use oteryn_session::item_view::{
        CharacterInventory, EquipmentSlot, EquippedItem, ItemEntry,
        encode_character_inventory_with_equip_drop,
    };
    let item = |handle| -> Result<ItemEntry, BoxError> {
        Ok(ItemEntry {
            appearance_id: 0,
            handle: std::num::NonZeroU64::new(handle).ok_or("handle")?,
            item_definition_ref: std::num::NonZeroU32::MIN,
            count: std::num::NonZeroU32::MIN,
            sub_type: 0,
        })
    };
    let inventory = CharacterInventory {
        main_backpack: Some(item(1)?),
        entries: vec![item(2)?],
        equipment: vec![EquippedItem {
            slot: EquipmentSlot::Head,
            item: item(3)?,
        }],
    };
    let containers = ContainerViews {
        views: vec![ContainerView {
            view_id: 0,
            container_handle: item(1)?.handle,
            parent_handle: None,
            capacity: 20,
            entries: vec![item(4)?],
            browse: None,
        }],
    };
    let carried = (
        encode_character_inventory_with_equip_drop(&inventory, true)
            .map_err(|e| format!("{e:?}"))?,
        encode_container_views(&containers).map_err(|e| format!("{e:?}"))?,
    );
    let look = replies()?.item;
    let (client, mut server) = tokio::io::duplex(16 * 1024);
    let peer = tokio::spawn(async move {
        admit_peer_with_carried(&mut server, Some(carried)).await?;
        for index in 0..13 {
            let frame = read_frame(&mut server).await?;
            if index == 12 {
                let mut command = vec![8, 19];
                scalar(&mut command, 2, 30);
                // Field 1 of the LOOK payload is the server-issued item handle.
                bytes(&mut command, 4, &[8, 5]);
                assert_eq!(frame, envelope(7, 1, 0, &command));
            } else {
                assert!(!frame.is_empty());
            }
            let payload = if index % 3 == 0 {
                look.as_slice()
            } else {
                &[8, 1]
            };
            write_frame(
                &mut server,
                &answer(41 + index, 7 + index, CommandStatus::Accepted, payload),
            )
            .await?;
        }
        let mut extra = [0];
        assert_eq!(
            server.read(&mut extra).await?,
            0,
            "stale targets must not reach the wire"
        );
        Ok::<_, BoxError>(())
    });
    let result = async {
        let mut session = Session::admit(
            client,
            Admission {
                appearance_items: None,
                schema_revision: 1,
                character_id: CharacterId::decode(&identity(4))?,
                admission_material: b"grant",
                client_build_id: "i03-look-session-test",
                supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
                deadline: Duration::from_secs(5),
            },
        )
        .await?;
        for handle in 1..=4 {
            for action in [Action::Look, Action::Use, Action::Open] {
                let command = Command {
                    target: Target::Item(item(handle)?.handle),
                    action,
                    selection: None,
                };
                let result = send(&mut session, &command).await?;
                assert_ne!(result, "Target changed", "{handle} {action:?}");
                assert!(
                    if action == Action::Look {
                        result.contains("backpack")
                    } else {
                        result.ends_with("completed")
                    },
                    "{result}"
                );
            }
        }
        for action in [Action::Look, Action::Use, Action::Open] {
            assert_eq!(
                send(
                    &mut session,
                    &Command {
                        target: Target::Item(item(99)?.handle),
                        action,
                        selection: None
                    }
                )
                .await?,
                "Target changed"
            );
        }
        let ground = Command {
            target: Target::Item(item(5)?.handle),
            action: Action::Look,
            selection: Some(Selection {
                position: ActorPosition { floor: -7, ..SPOT },
                binding: None,
            }),
        };
        assert_eq!(
            send_framed(&mut session, &ground, false).await?,
            "Target changed"
        );
        assert!(
            send_framed(&mut session, &ground, true)
                .await?
                .contains("backpack")
        );
        Ok::<_, BoxError>(())
    }
    .await;
    if result.is_err() {
        peer.abort();
    }
    result?;
    peer.await??;
    Ok(())
}
