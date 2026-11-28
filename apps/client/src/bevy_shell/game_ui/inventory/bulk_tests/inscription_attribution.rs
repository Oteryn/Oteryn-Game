use super::*;
use oteryn_session::item_inscription::COMMAND_TYPE_ITEM_INSCRIPTION_ATTRIBUTION;
use oteryn_session::test_support::wire::{
    self, DomainSnapshot, item_view, world_object, world_spatial,
    world_spatial_entities::{self as entities, EntityDetail, EntityKind, EntityRef},
};
use oteryn_session::{Admission, Session, read_frame, write_frame};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;
const DEADLINE: Duration = Duration::from_secs(5);

fn id(marker: u8) -> [u8; 16] {
    let mut bytes = [marker; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

async fn serve(
    mut stream: tokio::io::DuplexStream,
    selected: &[u32],
    read_kind: u32,
    reply: &[u8],
    item: ItemEntry,
) -> std::result::Result<(), Error> {
    read_frame(&mut stream).await?;
    let accepted = wire::encode_server_accepted(&wire::ServerAcceptedValue {
        game_session_id: wire::GameSessionId::decode(&id(1))?,
        world_id: wire::WorldId::decode(&id(2))?,
        channel_id: wire::ChannelId::decode(&id(3))?,
        connection_generation: 1,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: 1,
        selected_capabilities: selected,
    })?;
    write_frame(&mut stream, &accepted).await?;
    let position = world_spatial::ActorPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let spatial = entities::encode_world_spatial_entities_snapshot_with_item_handles(
        &entities::WorldSpatialEntitiesSnapshot {
            content_generation: [17; 32],
            actor_position: position,
            own_identity: id(4),
            entities: vec![entities::WorldSpatialEntity {
                motion: None,
                display_name: None,
                kind: EntityKind::Player,
                entity: EntityRef {
                    identity: id(4),
                    generation: 1,
                },
                position,
                detail: EntityDetail::Actor {
                    direction: world_spatial::StepDirection::South,
                    appearance_ref: 1,
                    health_percent: 100,
                },
            }],
        },
    )
    .map_err(|error| format!("{error:?}"))?;
    let overlay = world_object::encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("{error:?}"))?;
    let inventory = item_view::encode_character_inventory(&oteryn_session::CharacterInventory {
        main_backpack: Some(tests::entry(10, 1).map_err(|error| error.to_string())?),
        entries: vec![item],
        ..Default::default()
    })
    .map_err(|error| format!("{error:?}"))?;
    let container = item_view::encode_open_container(&Default::default())
        .map_err(|error| format!("{error:?}"))?;
    let domains = [
        (
            1,
            entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            spatial.as_slice(),
        ),
        (
            2,
            world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            overlay.as_slice(),
        ),
        (
            9,
            item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
            inventory.as_slice(),
        ),
        (
            11,
            item_view::SNAPSHOT_TYPE_OPEN_CONTAINER_V1,
            container.as_slice(),
        ),
    ]
    .map(|(domain_id, snapshot_type, payload)| DomainSnapshot {
        domain_id,
        revision: 1,
        snapshot_type,
        payload,
    });
    for frame in wire::encode_single_chunk_snapshot(1, 1, 0, &domains)? {
        write_frame(&mut stream, &frame).await?;
    }
    let frame = read_frame(&mut stream).await?;
    let command = wire::decode_wire_envelope(&frame)?.client_command(1)?;
    assert_eq!(command.command_type, read_kind);
    assert_eq!(command.payload, br#"{"item_handle":11}"#);
    let result = wire::encode_command_result(
        1,
        1,
        command.command_id,
        wire::CommandStatus::Accepted,
        reply,
    )?;
    write_frame(&mut stream, &result).await?;
    // Keep the connection live until the form has rendered.
    let mut byte = [0];
    assert_eq!(
        tokio::io::AsyncReadExt::read(&mut stream, &mut byte).await?,
        0
    );
    Ok(())
}

async fn wait_for(
    app: &mut App,
    ready: impl Fn(&PlayLink) -> bool,
) -> std::result::Result<(), Error> {
    tokio::time::timeout(DEADLINE, async {
        loop {
            app.update();
            let owner = app.world().non_send::<HostOwner>();
            let link = &owner.play.as_ref().ok_or("play")?.link;
            if ready(link) {
                return Ok::<(), Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await?
}

fn inscription_window(
    selected: &'static [u32],
    kind: u32,
    reply: &'static [u8],
) -> std::result::Result<String, Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let (client, server) = tokio::io::duplex(64 * 1024);
                    let item = tests::entry(11, 1).map_err(|error| error.to_string())?;
                    let mut peer = tokio::spawn(serve(server, selected, kind, reply, item));
                    let admission = Session::admit(
                        client,
                        Admission {
                            appearance_items: None,
                            schema_revision: 1,
                            character_id: wire::CharacterId::decode(&id(4))?,
                            admission_material: b"grant",
                            client_build_id: "inscription-read-test",
                            supported_capabilities: selected,
                            deadline: DEADLINE,
                        },
                    );
                    let session = tokio::select! {
                        admitted = admission => admitted?,
                        result = &mut peer => {
                            result??;
                            return Err("inscription peer closed during admission".into());
                        }
                    };
                    let (mut app, incoming, outbox) = app().map_err(|error| error.to_string())?;
                    super::super::super::feature_forms::install(&mut app);
                    let task = tokio::spawn(crate::play::run_session(session, incoming, outbox));
                    wait_for(&mut app, |link| link.mail_panel().selected).await?;
                    item_menu(&mut app, 11, true).map_err(|error| error.to_string())?;
                    let shown = wait_for(&mut app, |link| {
                        let panel = link.mail_panel();
                        panel.open && !panel.pending && panel.fields[2] == "Sealed text"
                    })
                    .await;
                    let text = {
                        let owner = app.world().non_send::<HostOwner>();
                        let link = &owner.play.as_ref().ok_or("play")?.link;
                        let form = property_form(link, "Writing and mail")
                            .map_err(|error| error.to_string())?;
                        link.feature_form_view(form).text
                    };
                    let rendered = has_text(&mut app, &text);
                    drop(app);
                    tokio::time::timeout(DEADLINE, task).await??;
                    tokio::time::timeout(DEADLINE, peer).await???;
                    shown?;
                    assert!(rendered, "{text}");
                    Ok(text)
                })
        })?
        .join()
        .map_err(|_| "inscription scenario thread panicked")?
}

#[test]
fn gap_0596_item_inscription_attribution() -> std::result::Result<(), Error> {
    let text = inscription_window(
        &[4, 6, 115, 511, 596],
        COMMAND_TYPE_ITEM_INSCRIPTION_ATTRIBUTION,
        br#"{"text":"Sealed text","max_length":1999,"writable":false,"writer_name":"Alice","written_at_unix_ms":1700000000000}"#,
    )?;
    assert!(text.contains("Alice · 2023-11-14 22:13:20 UTC"), "{text}");
    assert!(text.contains("cannot be written on"), "{text}");
    Ok(())
}

#[test]
fn gap_0596_item_inscription_legacy_bytes() -> std::result::Result<(), Error> {
    use oteryn_session::item_inscription::COMMAND_TYPE_ITEM_INSCRIPTION_READ;
    let text = inscription_window(
        &[4, 6, 115, 511],
        COMMAND_TYPE_ITEM_INSCRIPTION_READ,
        br#"{"text":"Sealed text","max_length":1999,"writable":false}"#,
    )?;
    assert!(text.contains("cannot be written on"), "{text}");
    assert!(!text.contains("Alice") && !text.contains("UTC"), "{text}");
    Ok(())
}
