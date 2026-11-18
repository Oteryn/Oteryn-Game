use super::fixture::{Harness, debug, id};
use super::{TestResult, item_look};
use crate::content::item_ref::ItemDefinitionIndex;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::COMMAND_TYPE_ITEM_LOOK_INTENT;
use oteryn_protocol_oteryn::item_look::{
    ItemLookIntent, ItemLookResult, ItemLookSelection, ItemLookTarget, decode_item_look_result_for,
    encode_item_look_intent_for,
};
use oteryn_protocol_oteryn::item_view::{
    STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, decode_snapshot_chunk,
    encode_client_command,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

const GOBLETS: [(&str, i64); 4] = [
    ("oteryn:item.tibia.i5807", 1),
    ("oteryn:item.tibia.i5806", 1),
    ("oteryn:item.tibia.i5805", 1),
    ("oteryn:item.tibia.i5805", 1),
];

#[test]
fn fenced_look_dispatch_shows_crystal_arena_goblet_inscriptions() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty(), "PostgreSQL route must be nonempty when configured");
    item_look::large_stack(item_look::scenario_with_goblets)
}

pub(super) async fn seed_goblets(harness: &Harness) -> TestResult {
    item_look::seed_main_backpack(harness, &GOBLETS).await?;
    // Match the text rows written by the qpkgw5 arena reward claim.
    for seed in 71..=73 {
        sqlx::query("INSERT INTO game_item_text_revisions VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(seed).as_slice())
            .execute(&harness.pool)
            .await?;
        sqlx::query("INSERT INTO game_item_texts(item_instance_id,text,writer_character_id,written_at,text_revision) VALUES(encode($1,'hex')::uuid,'Awarded to Arena Victor.',NULL,statement_timestamp(),1)")
            .bind(id(seed).as_slice())
            .execute(&harness.pool)
            .await?;
    }
    Ok(())
}

pub(super) async fn client_looks(
    client: &mut DuplexStream,
    index: &ItemDefinitionIndex,
) -> TestResult {
    let mut entries = Vec::new();
    loop {
        let frame = item_look::read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            for domain in domains {
                if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                    entries = decode_character_inventory(domain.payload).map_err(debug)?.entries;
                }
            }
        }
        if envelope.message_type() == MessageType::SnapshotCommit {
            break;
        }
    }
    // This inventory capability sends definition refs without appearance IDs.
    assert_eq!(
        entries.iter().map(|entry| Some(entry.item_definition_ref)).collect::<Vec<_>>(),
        GOBLETS.map(|(key, _)| index.definition_ref("Item", key, "definition-r1"))
    );
    let selection = ItemLookSelection { extended: true };
    for (offset, (entry, expected)) in entries.iter().zip([
        Some("It is given to the courageous victor of the barbarian arena in greenhorn difficulty. Awarded to Arena Victor"),
        Some("It is given to the courageous victor of the barbarian arena in scrapper difficulty. Awarded to Arena Victor."),
        Some("It is given to the courageous victor of the barbarian arena in warlord difficulty. Awarded to Arena Victor."),
        None,
    ]).enumerate() {
        let command_id = u64::try_from(offset)? + 1;
        let payload = encode_item_look_intent_for(
            ItemLookIntent { target: ItemLookTarget::Item(entry.handle) },
            selection,
        ).map_err(debug)?;
        let frame = encode_client_command(1, &ClientCommandValue {
            command_id,
            command_type: COMMAND_TYPE_ITEM_LOOK_INTENT,
            payload: &payload,
        }).map_err(debug)?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        loop {
            let frame = item_look::read_frame(client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() != MessageType::CommandResult {
                continue;
            }
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command_id);
            assert_eq!(result.status, CommandStatus::Accepted);
            let ItemLookResult::Text(text) = decode_item_look_result_for(result.payload, selection)
                .map_err(debug)? else {
                return Err(format!("LOOK refused goblet {}", entry.item_definition_ref).into());
            };
            assert!(text.starts_with("You see "), "{text}");
            assert_eq!(text.lines().find(|line| line.contains("Awarded to")), expected, "{text}");
            break;
        }
    }
    client.shutdown().await?;
    let mut remaining = Vec::new();
    client.read_to_end(&mut remaining).await?;
    Ok(())
}
