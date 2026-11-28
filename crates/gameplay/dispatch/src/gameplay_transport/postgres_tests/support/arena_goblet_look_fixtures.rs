use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, arena_match_fixtures::fixture as arena, item_look_fixtures as item_look};
use crate::durability::arena_match::{MatchState, donor};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::arena::{self as wire, ArenaAction, ArenaDisposition};
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
    eprintln!("arenapkg-marker: fenced_look_dispatch_shows_crystal_arena_goblet_inscriptions db ready");
    item_look::seed_main_backpack(harness, &GOBLETS[3..]).await?;

    // Seed earned entitlement and leave the claims, items and inscriptions to
    // the production arena command and its existing reward MINT transaction.
    let mut deps = Dependencies::new()?;
    let session = deps.admitted.game_session_id;
    let state = MatchState {
        completed: 3, rank: 3, phase: 3, session: *session.as_bytes(),
        scope_generation: 1, run: id(231), ..MatchState::default()
    };
    let mut tx = harness.pool.begin().await?;
    sqlx::query(
        "INSERT INTO game_arena_match_receipts(character_id,game_session_id,command_id,payload,before_state,after_state,reply)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3,$4,$5,$6)"
    ).bind(id(41).as_slice()).bind(session.as_bytes().as_slice())
        .bind(wire::encode_match_request(ArenaAction::Advance).map_err(debug)?)
        .bind(serde_json::to_value(MatchState::default())?)
        .bind(serde_json::to_value(&state)?)
        .bind(wire::encode_match_result(&wire::ArenaResult {
            disposition: ArenaDisposition::Applied, state: state.wire(0, [0; 3]),
        }).map_err(debug)?)
        .execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_character_arena_matches(character_id,body,game_session_id,command_id)
         VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,1)"
    ).bind(id(41).as_slice()).bind(serde_json::to_value(&state)?).bind(session.as_bytes().as_slice())
        .execute(&mut *tx).await?;
    tx.commit().await?;

    let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        deps.admitted.world_id, "content-1",
    )?;
    let active = controller.active().ok_or("active arena generation")?;
    let native = active.native_gameplay().ok_or("native arena content")?;
    let (bytes, pins) = arena::bundle(native)?;
    let world = arena::boot_world(&deps, native, &bytes, &pins)?;
    let (x, y, floor) = donor::REWARD_ROOM;
    arena::runtime_at(&mut deps, active, &pin, &world, crate::foundation::MovementLocalPosition { x, y, floor })?;
    let cells = world.movement_cells(deps.room.movement_cells()).map_err(debug)?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(), &[wire::CAPABILITY_ARENA_V1],
    ).ok_or("arena reward capability")?;
    let (current, _) = crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(session).await?;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
        .bind(admitted.world_id.as_bytes().as_slice()).bind(admitted.channel_id.as_bytes().as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(admitted.world_id.as_bytes().as_slice()).bind(admitted.channel_id.as_bytes().as_slice())
        .bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice())
        .bind(world.frame_binding_digest(admitted.world_id).as_slice()).execute(&harness.pool).await?;
    use crate::gameplay_transport::spell_premium_coordinator::{SpellPremiumCoordinator, purchase_fixture};
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    harness.root.latch_premium_activation(&activation.activation_id, activation.switch_over_us,
        activation.switch_over_us + 60_000_000).await.map_err(debug)?;
    let premium = SpellPremiumCoordinator::new(consumer, Some(clock), Some(activation), Some(harness.root.clone()));
    owner.premium_coordinator = Some(&premium);
    owner.admit_premium(id(40), session);
    owner.quest_catalogue = Some(std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?.catalogue().clone()
    ));
    for rank in 1..=state.completed {
        let payload = wire::encode_match_request(ArenaAction::ClaimReward { rank, category: 4, choice: 1 }).map_err(debug)?;
        let reply = owner.arena_match_command(admitted, 1 + u64::from(rank), &payload).await.ok_or("live goblet reward reply")?;
        assert_eq!(wire::decode_match_result(&reply).map_err(debug)?.disposition, ArenaDisposition::Applied);
    }
    // Exercise the same LOOK owner with a real handle. The live connection's
    // periodic staff check can independently close a superseded transport.
    let inventory = owner.observe_character_inventory(deps.actor, session).await
        .ok_or("reward inventory")?;
    let reference = native.item_index().and_then(|index| {
        index.definition_ref("Item", GOBLETS[0].0, "definition-r1")
    }).ok_or("greenhorn definition")?;
    let key = inventory.entries.iter().find(|entry| entry.item_definition_ref == reference)
        .ok_or("greenhorn reward")?.key;
    use crate::gameplay_transport::item_view::{ItemViewContinuity, SessionItemView};
    let mut views = SessionItemView::resume(ItemViewContinuity::default());
    views.snapshot(inventory, None).map_err(debug)?;
    let handle = views.table().handle(&key).ok_or("greenhorn handle")?;
    let inspect = || owner.look_item(deps.actor, session, admitted.item_fence, &views, handle);
    assert!(matches!(inspect().await, ItemLookResult::Text(text) if text.contains("Awarded to")));
    let advance_generation = |step| sqlx::query(
        "UPDATE game_durability_reconnect_sessions
         SET current_generation=current_generation+$2
         WHERE game_session_id=encode($1,'hex')::uuid"
    ).bind(session.as_bytes().as_slice()).bind(step).execute(&harness.pool);
    assert_eq!(advance_generation(1_i64).await?.rows_affected(), 1);
    let stale = inspect().await;
    assert_eq!(advance_generation(-1_i64).await?.rows_affected(), 1);
    assert_eq!(stale, ItemLookResult::Refused(ItemLookRefusal::NotAvailable));
    Ok(())
}

pub(super) async fn client_looks(
    client: &mut DuplexStream,
    index: &ItemDefinitionIndex,
    harness: &Harness,
    _session: GameSessionId,
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
    let name: String = sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
    let award = |difficulty, period| {
        Some(format!("It is given to the courageous victor of the barbarian arena in {difficulty} difficulty. Awarded to {name}{period}"))
    };
    let inscriptions = [award("greenhorn", ""), award("scrapper", "."), award("warlord", "."), None, None];
    for ((key, _), inscription) in GOBLETS.into_iter().zip(inscriptions) {
        let reference = index
            .definition_ref("Item", key, "definition-r1")
            .ok_or("goblet definition reference")?;
        expected.push((reference, inscription));
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
        descriptions.push((entry.item_definition_ref, description.map(str::to_owned)));
    }
    descriptions.sort_unstable();
    assert_eq!(descriptions, expected, "Crystal arena goblet descriptions");

    // A player-written inscription with the same text is still not an award.
    let change_writer = |writer: Option<Vec<u8>>| async move {
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        let changed = sqlx::query(
            "UPDATE game_item_texts t SET writer_character_id=encode($1,'hex')::uuid
             FROM game_reward_claim_mint_receipts r
             WHERE t.item_instance_id=r.item_instance_id AND r.claim_production_key=$2"
        ).bind(writer).bind(donor::claim_key(1, 4)).execute(&mut *tx).await?;
        assert_eq!(changed.rows_affected(), 1);
        tx.commit().await?;
        Ok::<_, sqlx::Error>(())
    };
    let greenhorn = entries.iter().find(|entry| {
        index.definition_ref("Item", GOBLETS[0].0, "definition-r1") == Some(entry.item_definition_ref)
    }).ok_or("greenhorn goblet")?;
    let next = u64::try_from(entries.len())? + 1;
    change_writer(Some(id(41).to_vec())).await?;
    let forged = look(client, next, greenhorn.handle).await;
    change_writer(None).await?;
    let ItemLookResult::Text(forged) = forged? else {
        return Err("player-written goblet LOOK refused".into());
    };
    assert!(!forged.contains("Awarded to"), "{forged}");

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
