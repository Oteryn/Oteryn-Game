// Shared map key materialization through the fenced production writer.
use super::*;
use crate::durability::map_item_mint::{MapItemMintError, MapItemMintOutcome, MapItemMintRequest};

#[test]
fn map_key_binding_silver_replays_without_duplicate_binding() -> TestResult {
    scenario(2969, 3007)
}
#[test]
fn map_key_binding_copper_replays_without_duplicate_binding() -> TestResult {
    scenario(2970, 3008)
}
#[test]
fn map_key_binding_golden_replays_without_duplicate_binding() -> TestResult {
    scenario(2972, 3002)
}

fn scenario(appearance: u16, number: u16) -> TestResult {
    run("map_key_binding", async move |h| {
        let started = std::time::Instant::now();
        eprintln!("QUESTPC-MAP-KEY-PG appearance={appearance} key={number}");
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let fence = harness::item_fence()?;
        let request = MapItemMintRequest {
            command: command(700)?,
            base_bundle_digest: [0xe2; 32],
            placement_key: oteryn_world_bundle::bundle::placement_key(-7, 120, 5, 1)
                .ok_or("placement")?,
            reset_epoch: 0,
            item: ItemDefinitionFacts {
                weight: None,
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: format!("oteryn:item.tibia.i{appearance}"),
                    revision_ref: "definition-r1".into(),
                },
                stack: ItemStackClass::NonStackable,
                container_capacity: None,
                container_slot_equip_pattern: false,
            },
            key_number: Some(number),
            quantity: 1,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        };
        let mut frozen = h
            .root
            .freeze_map_item_mint(&recovery, &h.node, fence, request.clone())
            .await
            .map_err(debug)?;
        let permit = SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
        let outcome = h
            .root
            .commit_map_item_mint(&permit, &recovery, &h.node, fence, &mut frozen)
            .await
            .map_err(debug)?;
        let MapItemMintOutcome::Committed(first) = outcome else {
            return Err("fresh key not committed".into());
        };
        let binding: Option<i32> = sqlx::query_scalar(
            "SELECT key_number FROM game_item_keys WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(first.item_instance_id.as_slice())
        .fetch_optional(&h.pool)
        .await?;
        assert_eq!(
            binding,
            Some(i32::from(number)),
            "trusted source binding must survive shared map MINT"
        );
        let envelope: Vec<u8> = sqlx::query_scalar(
            "SELECT envelope FROM game_item_audit_outbox WHERE event_id=encode($1,'hex')::uuid",
        )
        .bind(first.event_id.as_slice())
        .fetch_one(&h.pool)
        .await?;
        let audited =
            crate::durability::map_item_mint_audit::decode_map_item_mint_envelope(&envelope);
        let (_, audited) = audited.map_err(debug)?;
        assert_eq!(
            audited.after.ok_or("audit state")?.key_number,
            u32::from(number)
        );
        assert!(
            matches!(h.root.commit_map_item_mint(&permit, &recovery, &h.node, fence, &mut frozen).await.map_err(debug)?, MapItemMintOutcome::AlreadyCommitted(value) if value == first)
        );
        assert_eq!(h.count("game_item_keys").await?, 1);
        assert_eq!(h.count("game_map_item_mint_receipts").await?, 1);
        assert_eq!(h.count("game_item_audit_outbox").await?, 1);
        let mut changed = request.clone();
        changed.key_number = Some(number + 1);
        assert!(
            matches!(
                h.root
                    .freeze_map_item_mint(&recovery, &h.node, fence, changed)
                    .await,
                Err(MapItemMintError::ConflictingCause)
            ),
            "same command with changed binding cannot replay"
        );
        let mut competitor = request.clone();
        competitor.command = command(701)?;
        assert!(matches!(
            h.root
                .freeze_map_item_mint(&recovery, &h.node, fence, competitor)
                .await,
            Err(MapItemMintError::Refused(_))
        ));
        // A caller cannot select a fresh stock epoch before Control activates it.
        let mut reset = request.clone();
        reset.command = command(702)?;
        reset.reset_epoch = 1;
        assert!(matches!(
            h.root
                .freeze_map_item_mint(&recovery, &h.node, fence, reset.clone())
                .await,
            Err(MapItemMintError::AuthorityRejected)
        ));
        // Model an already activated pointer, as the existing map pickup consumer fixture.
        // This proves stock consumption against that pointer, not the reset activation writer.
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-key-fixture',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-11')")
            .bind(id(WORLD).as_slice()).bind(request.base_bundle_digest.as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(WORLD).as_slice())
            .execute(&h.pool)
            .await?;
        let mut reset = h
            .root
            .freeze_map_item_mint(&recovery, &h.node, fence, reset)
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .commit_map_item_mint(&permit, &recovery, &h.node, fence, &mut reset)
                .await
                .map_err(debug)?,
            MapItemMintOutcome::Committed(_)
        ));
        assert_eq!(h.count("game_item_keys").await?, 2);
        eprintln!(
            "QUESTPC-MAP-KEY-PG GREEN key={number} duration={:?}",
            started.elapsed()
        );
        Ok(())
    })
}
