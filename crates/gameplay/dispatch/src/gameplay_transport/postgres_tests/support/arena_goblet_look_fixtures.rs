use super::fixture::{Harness, debug, id};
use super::{TestResult, item_look_fixtures as item_look};
use crate::content::item_ref::ItemDefinitionIndex;
use crate::foundation::{GameSessionId, MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::COMMAND_TYPE_ITEM_LOOK_INTENT;
use oteryn_protocol_oteryn::item_look::{
    ItemLookIntent, ItemLookRefusal, ItemLookResult, ItemLookSelection, ItemLookTarget,
    decode_item_look_result_for, encode_item_look_intent_for,
};
use oteryn_protocol_oteryn::item_view::{
    ItemHandle, STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, decode_snapshot_chunk,
    encode_client_command,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

const GOBLETS: [(&str, i64); 5] = [
    ("oteryn:item.tibia.i5807", 1),
    ("oteryn:item.tibia.i5806", 1),
    ("oteryn:item.tibia.i5805", 1),
    ("oteryn:item.tibia.i5805", 1),
    ("oteryn:item.tibia.i5805", 1),
];

pub(super) async fn seed_goblets(harness: &Harness) -> TestResult {
    item_look::seed_main_backpack(harness, &GOBLETS).await?;
    // Match the text rows written by the qpkgw5 arena reward claim.
    for seed in 71..=74 {
        // A player's text with the same prefix must never become an arena award.
        let writer = (seed == 74).then(|| id(41).to_vec());
        let inscription = if writer.is_some() {
            "Awarded to Impostor."
        } else {
            "Awarded to Arena Victor."
        };
        sqlx::query("INSERT INTO game_item_text_revisions VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(seed).as_slice())
            .execute(&harness.pool)
            .await?;
        sqlx::query("INSERT INTO game_item_texts(item_instance_id,text,writer_character_id,written_at,text_revision) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,statement_timestamp(),1)")
            .bind(id(seed).as_slice())
            .bind(inscription)
            .bind(writer)
            .execute(&harness.pool)
            .await?;
    }
    Ok(())
}

pub(super) async fn client_looks(
    client: &mut DuplexStream,
    index: &ItemDefinitionIndex,
    harness: &Harness,
    session: GameSessionId,
) -> TestResult {
    let mut entries = Vec::new();
    loop {
        let frame = item_look::read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            for domain in domains {
                if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                    entries = decode_character_inventory(domain.payload)
                        .map_err(debug)?
                        .entries;
                }
            }
        }
        if envelope.message_type() == MessageType::SnapshotCommit {
            break;
        }
    }
    // The golden goblets share a definition. Match the replies independent of display order.
    let mut expected = Vec::new();
    let inscriptions = [
        Some(concat!(
            "It is given to the courageous victor of the barbarian arena in ",
            "greenhorn difficulty. Awarded to Arena Victor"
        )),
        Some(concat!(
            "It is given to the courageous victor of the barbarian arena in ",
            "scrapper difficulty. Awarded to Arena Victor."
        )),
        Some(concat!(
            "It is given to the courageous victor of the barbarian arena in ",
            "warlord difficulty. Awarded to Arena Victor."
        )),
        None,
        None,
    ];
    for ((key, _), inscription) in GOBLETS.into_iter().zip(inscriptions) {
        let reference = index
            .definition_ref("Item", key, "definition-r1")
            .ok_or("goblet definition reference")?;
        expected.push((reference, inscription.map(str::to_owned)));
    }
    expected.sort_unstable();
    let mut observed_refs: Vec<_> = entries
        .iter()
        .map(|entry| entry.item_definition_ref)
        .collect();
    observed_refs.sort_unstable();
    assert_eq!(
        observed_refs,
        expected
            .iter()
            .map(|(reference, _)| *reference)
            .collect::<Vec<_>>()
    );

    let mut descriptions = Vec::new();
    let mut inscribed_handle = None;
    for (offset, entry) in entries.iter().enumerate() {
        let command_id = u64::try_from(offset)? + 1;
        let ItemLookResult::Text(text) = look(client, command_id, entry.handle).await? else {
            return Err(format!("LOOK refused goblet {}", entry.item_definition_ref).into());
        };
        assert!(text.starts_with("You see "), "{text}");
        let description = text
            .lines()
            .last()
            .filter(|line| line.contains("Awarded to"));
        assert_eq!(
            text.matches("Awarded to").count(),
            usize::from(description.is_some()),
            "{text}"
        );
        if description.is_some() {
            inscribed_handle = Some(entry.handle);
        }
        descriptions.push((entry.item_definition_ref, description.map(str::to_owned)));
    }
    descriptions.sort_unstable();
    assert_eq!(descriptions, expected, "Crystal arena goblet descriptions");

    // Supersede only the durable generation while this transport still holds its old fence.
    let advance_generation = "UPDATE game_durability_reconnect_sessions
        SET current_generation=current_generation+$2
        WHERE game_session_id=encode($1,'hex')::uuid";
    let handle = inscribed_handle.ok_or("inscribed goblet handle")?;
    assert_eq!(
        sqlx::query(advance_generation)
            .bind(session.as_bytes().as_slice())
            .bind(1_i64)
            .execute(&harness.pool)
            .await?
            .rows_affected(),
        1
    );
    let stale = look(client, u64::try_from(entries.len())? + 1, handle).await;
    assert_eq!(
        sqlx::query(advance_generation)
            .bind(session.as_bytes().as_slice())
            .bind(-1_i64)
            .execute(&harness.pool)
            .await?
            .rows_affected(),
        1
    );
    assert_eq!(
        stale?,
        ItemLookResult::Refused(ItemLookRefusal::NotAvailable)
    );
    client.shutdown().await?;
    let mut remaining = Vec::new();
    client.read_to_end(&mut remaining).await?;
    Ok(())
}

async fn look(
    client: &mut DuplexStream,
    command_id: u64,
    handle: ItemHandle,
) -> TestResult<ItemLookResult> {
    let selection = ItemLookSelection { extended: true };
    let payload = encode_item_look_intent_for(
        ItemLookIntent {
            target: ItemLookTarget::Item(handle),
        },
        selection,
    )
    .map_err(debug)?;
    let frame = encode_client_command(
        1,
        &ClientCommandValue {
            command_id,
            command_type: COMMAND_TYPE_ITEM_LOOK_INTENT,
            payload: &payload,
        },
    )
    .map_err(debug)?;
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
        return decode_item_look_result_for(result.payload, selection).map_err(debug);
    }
}
