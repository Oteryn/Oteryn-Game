//! The carried Botany Almanach opens a read-only modal through the USE command.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, item_look, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::ChannelContentPin;
use crate::gameplay_transport::{
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{FirstEntryOutcome, IDLE_LIVENESS, serve_admitted},
    item_fence_of, tcp_tls,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_snapshot_chunk,
    decode_wire_envelope, encode_client_command,
    item_view::{STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory},
    server_modal::{COMMAND_TYPE_SERVER_MODAL_QUERY, Reply},
    world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, decode_use_result,
        encode_item_use_intent,
    },
};

#[test]
fn botany_book_use_opens_carried_text_window_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(scenario(admin))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Botany Almanach PostgreSQL worker panicked")??;
    Ok(())
}

async fn scenario(admin: String) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "botany_book", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let (controller, pin) = activated_with_loot_tables(deps.admitted.world_id, None);
    let content_pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
        (6, 0, 7),
    );
    let deps = Dependencies::with_content(content_pin, deps.room, deps.spells)?;
    deps.runtime
        .lock()
        .await
        .initialize_pinned_test_position(
            deps.actor,
            crate::foundation::MovementLocalPosition {
                x: 6,
                y: 0,
                floor: 7,
            },
        )
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = controller.active();
    deps.initialize(&owner).await;

    let claim: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/interactions/reward_claims/reward-claim-variants-00100-00104.json"
    )))?;
    let placement = claim["records"]
        .as_array()
        .ok_or("claims")?
        .iter()
        .find(|entry| {
            entry["definition"]["identity"]["key"]
                == "oteryn:reward-claim.quest.u8_6.an_interest_in_botany.botany_almanach"
        })
        .ok_or("Botany Almanach claim")?["definition"]["native_contents"]["placements"]
        .get(0)
        .ok_or("Botany Almanach placement")?;
    let definition = placement["root"]["key"]
        .as_str()
        .ok_or("almanach definition")?;
    let text = placement["root_text"].as_str().ok_or("almanach text")?;
    item_look::seed_main_backpack(&h, &[(definition, 1)]).await?;
    sqlx::query("INSERT INTO game_item_text_revisions VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(71).as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_item_texts VALUES(encode($1,'hex')::uuid,$2,NULL,now(),1)")
        .bind(id(71).as_slice())
        .bind(text)
        .execute(&h.pool)
        .await?;

    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(admitted.game_session_id)
        .await?;
    deps.runtime
        .lock()
        .await
        .bind_attacker_lease(
            deps.actor,
            admitted.game_session_id,
            current.current_character_lease(),
        )
        .map_err(debug)?;
    admitted.item_fence = item_fence_of(current);
    let supported = [4, 6, 15, 92, 450];
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &supported)
            .ok_or("item use and server modal capabilities")?;
    assert_eq!(
        admitted.continuity.selected_capabilities.as_slice(),
        supported
    );
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let playing = async {
        let mut handle = None;
        loop {
            let frame = tcp_tls::read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        handle = decode_character_inventory(domain.payload)
                            .map_err(debug)?
                            .entries
                            .first()
                            .map(|entry| entry.handle);
                    }
                }
            } else if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let intent = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(handle.ok_or("carried book handle")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let first = admitted.continuity.next_command_id;
        for (offset, (command_type, payload)) in [
            (COMMAND_TYPE_SERVER_MODAL_QUERY, &[][..]),
            (COMMAND_TYPE_USE_INTENT, intent.as_slice()),
            (COMMAND_TYPE_SERVER_MODAL_QUERY, &[][..]),
        ]
        .into_iter()
        .enumerate()
        {
            let command_id = first + u64::try_from(offset)?;
            let frame = encode_client_command(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
                    command_type,
                    payload,
                },
            )
            .map_err(debug)?;
            tcp_tls::write_frame(&mut client, &frame).await?;
            loop {
                let frame = tcp_tls::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() != MessageType::CommandResult {
                    continue;
                }
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command_id);
                assert_eq!(result.status, CommandStatus::Accepted);
                if command_type == COMMAND_TYPE_USE_INTENT {
                    assert_eq!(
                        decode_use_result(result.payload).map_err(debug)?,
                        UseDisposition::Committed
                    );
                } else if offset == 0 {
                    assert_eq!(Reply::decode(result.payload)?, Reply::None);
                } else {
                    let Reply::Dialog(dialog) = Reply::decode(result.payload)? else {
                        return Err("expected a TextWindow after USE".into());
                    };
                    assert_eq!(dialog.title, "Text");
                    assert_eq!(dialog.message, text);
                    assert!(dialog.choices.is_empty());
                    assert_eq!(dialog.buttons, vec![(1, "Ok".into())]);
                }
                break;
            }
        }
        drop(client);
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (result, end) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(playing, serving)
    })
    .await?;
    result?;
    assert!(matches!(
        end,
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(_)
    ));
    let stored: (String, String) = sqlx::query_as(
        "SELECT text, text_revision::text FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid",
    ).bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(stored, (text.to_owned(), "1".into()));
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
