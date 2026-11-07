//! Same actual Royal-minted UUID through native Inbox retrieval and House installation.
use super::*;
use crate::durability::house_scope_handoff::{
    HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentRequest,
};
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::MovementLocalPosition;
use oteryn_protocol_oteryn::{CommandStatus, depot_search, house_wrap};

pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    kit: [u8; 16],
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("Royal source actor")?;
    let original_mint:sqlx::types::Uuid=sqlx::query_scalar("SELECT minted_transaction_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(kit.as_slice()).fetch_one(&h.pool).await?;
    let original_origin:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(o) FROM game_house_furniture_kit_origins o WHERE item_instance_id=encode($1,'hex')::uuid").bind(kit.as_slice()).fetch_one(&h.pool).await?;
    // Explicit fixture staging between authentic source regions, no travel claim.
    owner
        .runtime
        .lock()
        .await
        .stage_pinned_player_test_position(
            actor,
            admitted.game_session_id,
            MovementLocalPosition {
                x: 32352,
                y: 32226,
                floor: 7,
            },
        )
        .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (status, reply, _) = super::royal_peer::command(
        owner,
        admitted,
        command,
        depot_search::COMMAND_TYPE_DEPOT_SEARCH,
        depot_search::encode_request(depot_search::Request::Retrieve {
            client_id: 23398,
            all: false,
        })
        .map_err(debug)?,
        None,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let reply = depot_search::decode_reply(&reply).map_err(debug)?;
    assert_eq!(
        (reply.outcome, reply.retrieved),
        (depot_search::Outcome::Retrieved, 1)
    );
    let carried:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid)").bind(kit.as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(carried);
    let inbox:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id=encode($1,'hex')::uuid").bind(kit.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(inbox, 0);
    if super::royal::check_description() {
        let native = owner
            .active_generation
            .ok_or("Royal Look native generation")?
            .native_gameplay()
            .ok_or("Royal Look native data")?;
        let reference = native
            .item_index()
            .and_then(|index| {
                index.definition_ref("Item", "oteryn:item.tibia.i23398", "definition-r1")
            })
            .ok_or("Royal kit source reference")?;
        let command = admitted.continuity.next_command_id;
        let (status, payload, frames) = super::royal_peer::command(
            owner,
            admitted,
            command,
            oteryn_protocol_oteryn::item_look::ITEM_LOOK_COMMAND_ID.ok_or("native Look command")?,
            vec![],
            Some(reference),
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        let result = oteryn_protocol_oteryn::item_look::decode_item_look_result_for(
            &payload,
            oteryn_protocol_oteryn::item_look::ItemLookSelection { extended: true },
        )
        .map_err(debug)?;
        let oteryn_protocol_oteryn::item_look::ItemLookResult::Text(text) = result else {
            return Err("actual Royal kit Look must return existing text field".into());
        };
        assert!(
            text.contains("Unwrap it in your own house to create a golden outfit display."),
            "sourceKing137–139 customkitdescription must reach nativeLook30: {text}"
        );
        println!(
            "SRV7_ROYAL_KIT_LOOK_CAPTURE {}",
            serde_json::json!({"producer_test":"srv7_npc_king_actual_royal_kit_look_retains_source_description","selected_capabilities":admitted.continuity.selected_capabilities.as_slice(),"frames":frames.iter().map(|raw|raw.iter().map(|byte|format!("{byte:02x}")).collect::<String>()).collect::<Vec<_>>() })
        );
        return Ok(());
    }
    const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
    sqlx::query(include_str!(
        "../../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants SELECT session_user,encode($1,'hex')::uuid,$2,1").bind(id(42).as_slice()).bind(HOUSE).execute(&h.pool).await?;
    h.root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([82; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house: HouseId::new(admitted.world_id, HOUSE).ok_or("Royal House")?,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    owner
        .runtime
        .lock()
        .await
        .stage_pinned_player_test_position(
            actor,
            admitted.game_session_id,
            MovementLocalPosition {
                x: 32309,
                y: 32805,
                floor: 7,
            },
        )
        .map_err(debug)?;
    owner.admit_progression_session(admitted).await;
    let mut payload = vec![8, 1, 18, u8::try_from(HOUSE.len())?];
    payload.extend(HOUSE.as_bytes());
    let command = admitted.continuity.next_command_id;
    let (status, _, _) = super::royal_peer::command(
        owner,
        admitted,
        command,
        oteryn_protocol_oteryn::house_transfer::COMMAND_TYPE_HOUSE_TRANSFER,
        payload,
        None,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    // Historical metadata for the original compiler-proven House cells, as in
    // the existing real-source Artist fixture. No bundle geometry is changed.
    let actor_position = {
        let runtime = owner.runtime.lock().await;
        let state = runtime
            .house_actor_state(
                admitted.runtime_actor.ok_or("Royal admitted House actor")?,
                admitted.game_session_id,
            )
            .map_err(debug)?;
        [
            state.position.x,
            state.position.y,
            i32::from(state.position.floor),
        ]
    };
    let mut metadata = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *metadata)
        .await?;
    for at in [[32310, 32803, 7], actor_position] {
        let position: Vec<u8> = at
            .iter()
            .flat_map(|coordinate| coordinate.to_be_bytes())
            .collect();
        sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING").bind(id(42).as_slice()).bind(HOUSE).bind(position).execute(&mut *metadata).await?;
    }
    metadata.commit().await?;
    let(revision,acl):(i64,i64)=sqlx::query_as("SELECT i.state_revision::bigint,a.acl_revision::bigint FROM game_item_instances i CROSS JOIN game_house_access(encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid) a WHERE i.item_instance_id=encode($1,'hex')::uuid").bind(kit.as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let request = house_wrap::Request {
        action: house_wrap::Action::Unwrap,
        item: kit,
        expected_revision: u64::try_from(revision)?,
        acl_revision: u64::try_from(acl)?,
        position: [32310, 32803, 7],
    };
    let command = admitted.continuity.next_command_id;
    let (status, reply, _) = super::royal_peer::command(
        owner,
        admitted,
        command,
        house_wrap::COMMAND_WRAP,
        request.encode(),
        None,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert!(matches!(
        house_wrap::Reply::decode(&reply),
        Some(house_wrap::Reply::Committed(_))
    ));
    let current:String=sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(kit.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(current, "oteryn:item.tibia.i31510");
    let native = owner
        .active_generation
        .ok_or("Royal native generation")?
        .native_gameplay()
        .ok_or("Royal native data")?;
    let index = native.item_index().ok_or("Royal native Item index")?;
    let revision = index
        .revision(&current)
        .ok_or("Royal native display revision")?;
    let reference = index
        .definition_ref("Item", &current, revision)
        .ok_or("Royal native display reference")?;
    let viewer = {
        let runtime = owner.runtime.lock().await;
        let state = runtime
            .house_actor_state(
                admitted.runtime_actor.ok_or("Royal House viewer")?,
                admitted.game_session_id,
            )
            .map_err(debug)?;
        serde_json::json!({"house":state.house,"ownership_generation":state.generation.get(),"position":[state.position.x,state.position.y,state.position.floor]})
    };
    let command = admitted.continuity.next_command_id;
    let (status, reply, frames) = super::royal_peer::command(
        owner,
        admitted,
        command,
        oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,
        vec![],
        Some(reference),
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    );
    let mut cues = 0;
    for raw in &frames {
        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(raw).map_err(debug)?;
        if envelope.message_type() == oteryn_protocol_oteryn::MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS {
                let effects =
                    oteryn_protocol_oteryn::presentation::decode_item_effect_batch(delta.payload)
                        .map_err(debug)?;
                cues+=effects.iter().filter(|event|matches!(event.kind,oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect{effect_id:171,..})).count();
            }
        }
    }
    assert_eq!(
        cues, 1,
        "actual paid display USE publishes source171 exactly once"
    );
    let actual_capture = serde_json::json!({
      "producer_test":"srv7_npc_king_one_billion_actual_inbox_house_golden_display_pipeline",
      "source":{"crystal_sha":"201189f871d23597b82caf88fd611454a2ddefb8","kit_uuid":kit,"mint_transaction":original_mint.to_string()},
      "viewer":viewer,"selected_capabilities":admitted.continuity.selected_capabilities.as_slice(),
      "frames":frames.iter().map(|raw|raw.iter().map(|byte|format!("{byte:02x}")).collect::<String>()).collect::<Vec<_>>()
    });
    println!("SRV7_ROYAL_GOLDEN_NATIVE_CAPTURE {actual_capture}");
    let current:String=sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(kit.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(current, "oteryn:item.tibia.i31502");
    let final_mint:sqlx::types::Uuid=sqlx::query_scalar("SELECT minted_transaction_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(kit.as_slice()).fetch_one(&h.pool).await?;
    let final_origin:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(o) FROM game_house_furniture_kit_origins o WHERE item_instance_id=encode($1,'hex')::uuid").bind(kit.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(final_mint, original_mint);
    assert_eq!(final_origin, original_origin);
    let placed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid AND house_key=$2)").bind(kit.as_slice()).bind(HOUSE).fetch_one(&h.pool).await?;
    assert!(placed);
    Ok(())
}
