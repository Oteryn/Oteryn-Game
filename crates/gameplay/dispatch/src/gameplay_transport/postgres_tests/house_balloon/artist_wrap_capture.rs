//! The original B3 easel pickup/House placement/Rotate path has already completed.
use super::*;
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    item: [u8; 16],
    reference: std::num::NonZeroU32,
) -> TestResult {
    assert!(admitted.continuity.selected_capabilities.contains(387));
    let root = super::super::raid_live::hare_production_activation::repository_root();
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
        &root.join("content/spells.manifest.json"),
    )
    .map_err(debug)?;
    assert!(input.source_world.is_some());
    let independent =
        crate::content::qualify_selected_native_gameplay_room(admitted.world_id, &input)
            .map_err(debug)?;
    let public = &independent.compiled().client_artifact;
    let metadata = super::wrap_capture_metadata::metadata(owner, admitted, public).await?;
    if let Some(path) = std::env::var_os("OTERYN_SRV7_ARTIST_WRAP_CLIENT_ARTIFACT") {
        std::fs::write(path, public)?;
    }
    let before:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
    let mut phases = Vec::new();
    for (kind, body, carried) in [
        (9, vec![0], false),
        (402, vec![5], true),
        (402, vec![], true),
    ] {
        let command = admitted.continuity.next_command_id;
        let (status, result, frames, client) = super::super::house_wrap::capture_peer(
            owner,
            admitted,
            command,
            kind,
            body,
            Some(reference),
            carried,
        )
        .await?;
        assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        if kind == 9 {
            assert_eq!(
                oteryn_protocol_oteryn::item_view::decode_item_move_result(&result)
                    .map_err(debug)?,
                oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
            );
        } else if result.len() == 1 {
            assert_eq!(
                oteryn_protocol_oteryn::house_wrap::decode_opaque_wrap_affordance(&result),
                Some(true)
            );
        } else {
            assert!(matches!(
                oteryn_protocol_oteryn::house_wrap::Reply::decode(&result),
                Some(oteryn_protocol_oteryn::house_wrap::Reply::Committed(_))
            ));
        }
        let durable: Option<serde_json::Value> = if kind == 402 {
            sqlx::query_scalar("SELECT to_jsonb(r) FROM game_house_furniture_wrap_receipts r JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id WHERE r.game_session_id=encode($1,'hex')::uuid AND r.command_id=$2::text::numeric AND r.item_instance_id=encode($3,'hex')::uuid AND r.action=1 AND r.after_key='oteryn:item.tibia.i23398' AND i.last_transaction_id=r.transaction_id AND i.state_revision=(r.before_item->>'state_revision')::numeric+1")
                .bind(admitted.game_session_id.as_bytes().as_slice()).bind(command.to_string())
                .bind(item.as_slice()).fetch_optional(&h.pool).await?
        } else {
            None
        };
        if kind == 402 {
            assert_eq!(
                durable.is_some(),
                result.len() != 1,
                "only actual carried mutation owns a wrap receipt"
            );
        }
        phases.push(serde_json::json!({"command_id":command,"command_type":kind,"client_command_envelope_hex":hex(&client),"server_frames_hex":frames.iter().map(|f|hex(f)).collect::<Vec<_>>(),"result_hex":hex(&result),"durable_receipt":durable}));
    }
    let after:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after["definition_production_key"],
        "oteryn:item.tibia.i23398"
    );
    assert_eq!(after["item_instance_id"], before["item_instance_id"]);
    assert_eq!(
        after["minted_transaction_id"],
        before["minted_transaction_id"]
    );
    let capture = serde_json::json!({"schema":"OTERYN_ARTIST_OPAQUE_WRAP_NATIVE_CAPTURE/v1","producer_test":"srv7_artist_easel_actual_map_pickup_then_house_rotate","qualification":"Original actual B3 pickup and House rotate remain asserted; new actual TAKE9 then advisory402/marker5 and carried402/marker4. serve_admitted epochs have no Accepted; no cold admission claim.","source":metadata,"item_uuid_hex":hex(&item),"before":before,"after":after,"phases":phases});
    std::fs::write(
        std::env::var_os("OTERYN_SRV7_ARTIST_WRAP_CAPTURE").ok_or("Artist capture output")?,
        serde_json::to_vec_pretty(&capture)?,
    )?;
    Ok(())
}
