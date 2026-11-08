//! SRVGAP1108: watch USE reads the production World clock.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

#[test]
fn srvgap_watch_use_reports_world_time() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        run(async |admin| {
            let harness = Harness::create_with_admission_seed(admin, "srvgap_watch", true, false).await?;
            super::reset();
            prepare_current_session(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(deps.admitted.world_id, None);
            let active = controller.active().ok_or("active")?;
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(active);
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone()).current_session_at(deps.admitted.game_session_id).await?.0;
            let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
            {
                let mut runtime = owner.runtime.lock().await;
                runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
                runtime.bind_attacker_lease(deps.actor, deps.admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                        fence.character_lease_generation,
                    ).map_err(debug)?).map_err(debug)?;
            }
            deps.initialize(&owner).await;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
            owner.states.chat.admit(deps.admitted.game_session_id, "Watch user", owner.owner_now());
            let keys = ["oteryn:item.tibia.i2906", "oteryn:item.tibia.i6091"];
            super::item_look::seed_main_backpack(&harness, &keys.map(|key| (key, 1))).await?;
            let initial_items: serde_json::Value = sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i").fetch_one(&harness.pool).await?;
            for (offset, key) in keys.iter().enumerate() {
                let source = id(71 + u8::try_from(offset)?);
                let intent = UseIntent { target: UseTarget::Item(std::num::NonZeroU64::new(2 + u64::try_from(offset)?).ok_or("handle")?), use_with: None, use_with_map_item: None, use_with_position: None };
                let before = crate::world_clock::WorldClockSource::new(crate::world_clock::DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS).tibian_minute(owner.world_clock_utc_ms().ok_or("clock")?).ok_or("minute")?;
                let result = owner.use_event_item(deps.actor, UseCommand { game_session_id: deps.admitted.game_session_id, command_id: 1 + u64::try_from(offset)?, item_fence: Some(fence) }, intent, Some(ItemKey::Instance(source)), false).await.map_err(|_| "watch owner")?.ok_or("watch handled")?;
                assert_eq!(result.disposition, UseDisposition::Committed, "{key}");
                let after = crate::world_clock::WorldClockSource::new(crate::world_clock::DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS).tibian_minute(owner.world_clock_utc_ms().ok_or("clock")?).ok_or("minute")?;
                let lines = owner.states.chat.drain(deps.admitted.game_session_id);
                assert!(lines.iter().any(|line| matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { speaker_name, text } if speaker_name == "Game" && [before, after].iter().any(|minute| text == &format!("The time is {}:{:02}.", minute / 60, minute % 60)))), "{lines:?}");
            }
            let mut intent = UseIntent { target: UseTarget::Item(std::num::NonZeroU64::new(2).ok_or("handle")?), use_with: None, use_with_map_item: None, use_with_position: None };
            let command = UseCommand { game_session_id: deps.admitted.game_session_id, command_id: 20, item_fence: Some(fence) };
            // An unrelated container and a retired source stay on their existing paths.
            for source in [id(70), id(99)] {
                assert!(owner.use_event_item(deps.actor, command, intent.clone(), Some(ItemKey::Instance(source)), false).await.map_err(|_| "delegation")?.is_none());
            }
            intent.use_with_map_item = Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(std::num::NonZeroU64::new(3).ok_or("target handle")?));
            assert!(owner.use_event_item(deps.actor, command, intent.clone(), Some(ItemKey::Instance(id(71))), false).await.map_err(|_| "use-with delegation")?.is_none());
            intent.use_with_map_item = None;
            assert!(owner.use_event_item(deps.actor, UseCommand { item_fence: None, ..command }, intent.clone(), Some(ItemKey::Instance(id(71))), false).await.map_err(|_| "missing fence")?.is_none());
            let stale_fence = crate::durability::item_transfer::CurrentCharacterItemFence {
                character_lease_generation: fence.character_lease_generation + 1,
                ..fence
            };
            let stale = owner.use_event_item(deps.actor,
                UseCommand { item_fence: Some(stale_fence), ..command }, intent,
                Some(ItemKey::Instance(id(71))), false).await;
            assert!(!matches!(stale, Ok(Some(result)) if result.disposition == UseDisposition::Committed),
                "a stale lease never publishes a time read");
            assert!(owner.states.chat.drain(deps.admitted.game_session_id).is_empty());
            let final_items: serde_json::Value = sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i").fetch_one(&harness.pool).await?;
            assert_eq!(initial_items, final_items, "watch USE preserves quantity, lifecycle and revision of every item");
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records").fetch_one(&harness.pool).await?;
            assert_eq!(receipts, 0, "a clock read creates no consume receipt");
            drop(owner); drop(authority); drop(seal); harness.cleanup().await?;
            eprintln!("SRVGAP_WATCH_PG_COMPLETE two watches read World clock without consumption");
            Ok(())
        }).map_err(|e| e.to_string())
    })?.join().map_err(|_| "watch scenario panicked")??;
    Ok(())
}
