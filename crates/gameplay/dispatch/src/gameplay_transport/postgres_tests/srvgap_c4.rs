//! C4 reconciliation: all 42 source items already belong to the event/cosmetic owners.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_items::{item_cosmetics, novelty_items};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

#[test]
fn reaudit_usable_item_actions() -> TestResult {
    // Enumerate the retired C4 set independently of the main owners' selection code.
    // A new missing mapping must not silently revive a second receipt owner.
    let cosmetics = [
        6579, 12786, 12787, 16252, 16253, 16254, 16255, 16256, 16257, 23538, 23684, 23685, 27655,
        27656, 27657, 31737, 31738, 32629, 32630, 32631, 34075, 34076, 35595, 35695, 37604, 37605,
        37746, 39544, 39545, 43899, 43900, 50060, 50061, 50064, 50067, 51656, 51657,
    ];
    for item in cosmetics {
        assert!(
            item_cosmetics::rule(&format!("oteryn:item.tibia.i{item}")).is_some(),
            "main owns {item}"
        );
    }
    for item in [906, 3002, 6572, 6576, 6578] {
        assert!(
            novelty_items::rule_for_key(&format!("oteryn:item.tibia.i{item}")).is_some(),
            "main owns {item}"
        );
    }
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        run(async |admin| {
            let harness = Harness::create_with_admission_seed(admin, "srvgap_c4", true, false).await?;
            super::reset();
            prepare_current_session(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let (controller, pin) = crate::content::native_gameplay::test_support::cosmetics_fixture::activated_with_optional_cosmetics(
                deps.admitted.world_id, None,
                Some(br#"{"schema":"OTERYN_NATIVE_COSMETICS/v1","availability":"unavailable"}"#),
            );
            let active = controller.active().ok_or("active")?;
            assert!(active.native_gameplay().ok_or("native")?.cosmetics().is_none());
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(active);
            deps.initialize(&owner).await;
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone()).current_session_at(deps.admitted.game_session_id).await?.0;
            let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
            let keys = ["oteryn:item.tibia.i16252", "oteryn:item.tibia.i16253", "oteryn:item.tibia.i23538", "oteryn:item.tibia.i6579", "oteryn:item.tibia.i37746"];
            super::item_look::seed_main_backpack(&harness, &keys.map(|key| (key, 1))).await?;
            for (offset, key) in keys.iter().enumerate() {
                let source = id(71 + u8::try_from(offset)?);
                let intent = UseIntent {
                    target: UseTarget::Item(std::num::NonZeroU64::new(2 + u64::try_from(offset)?).ok_or("handle")?),
                    use_with: None, use_with_map_item: None, use_with_position: None,
                };
                let result = owner.use_event_item(deps.actor, UseCommand {
                    game_session_id: deps.admitted.game_session_id,
                    command_id: 1 + u64::try_from(offset)?, item_fence: Some(fence),
                }, intent, Some(ItemKey::Instance(source)), false).await.map_err(|_| "main cosmetic owner")?.ok_or("cosmetic result")?;
                assert_eq!(result.disposition, UseDisposition::NotPossible, "{key}: main requires admitted cosmetic mappings");
                let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(quantity, 1, "{key}: refusal preserves its source");
            }
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records").fetch_one(&harness.pool).await?;
            assert_eq!(receipts, 0, "unavailable mappings never produce a receipt");
            let retired: bool = sqlx::query_scalar("SELECT to_regclass('game_usable_action_receipts') IS NULL").fetch_one(&harness.pool).await?;
            assert!(retired, "unmerged C4 migrations must not install another receipt owner");
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            eprintln!("SRVGAP_C4_PG_COMPLETE: 42 mappings checked; five cosmetic sources refused without spending");
            Ok(())
        }).map_err(|e| e.to_string())
    })?.join().map_err(|_| "C4 reconciliation scenario panicked")??;
    Ok(())
}
