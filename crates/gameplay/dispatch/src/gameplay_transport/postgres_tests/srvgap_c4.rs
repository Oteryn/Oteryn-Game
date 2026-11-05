//! SRVGAP-C4: USE reaches the durable item owner, including replay and refusals.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn reaudit_usable_item_actions() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("SRVGAP_C4_PG_SKIPPED: PostgreSQL unavailable");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        run(async |admin| {
            let started = std::time::Instant::now();
            let harness = Harness::create_with_admission_seed(admin, "srvgap_c4", true, false).await?;
            super::reset();
            prepare_current_session(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(deps.admitted.world_id, None);
            let active = controller.active().ok_or("active")?;
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(active);
            deps.initialize(&owner).await;
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone()).current_session_at(deps.admitted.game_session_id).await?.0;
            let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
            super::item_look::seed_main_backpack(&harness, &[("oteryn:item.tibia.i906", 1), ("oteryn:item.tibia.i16253", 1), ("oteryn:item.tibia.i16252", 2), ("oteryn:item.tibia.i6572", 1), ("oteryn:item.tibia.i6576", 1), ("oteryn:item.tibia.i6579", 1), ("oteryn:item.tibia.i37746", 1), ("oteryn:item.tibia.i23538", 2), ("oteryn:item.tibia.i3002", 1), ("oteryn:item.tibia.i906", 2)]).await?;
            let command = |n| UseCommand { game_session_id: deps.admitted.game_session_id, command_id: n, item_fence: Some(fence) };
            let item = ItemKey::Instance(id(71));
            let first = owner.use_item_completion(deps.actor, command(1), item).await.ok_or("terminal result")?;
            assert_eq!(first.disposition, UseDisposition::Committed, "explosive present USE must consume and emit its sourced effect");
            let replay = owner.use_item_completion(deps.actor, command(1), item).await.ok_or("replay")?;
            assert_eq!(replay.disposition, UseDisposition::Committed);
            let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(quantity, 0);
            let conflict = owner.use_item_completion(deps.actor, command(1), ItemKey::Instance(id(72))).await.ok_or("conflict")?;
            assert_eq!(conflict.disposition, UseDisposition::Rejected);
            let refused = owner.use_item_completion(deps.actor, command(2), ItemKey::Instance(id(72))).await.ok_or("refusal")?;
            assert_ne!(refused.disposition, UseDisposition::Committed, "addon requires Premium and the base outfit");
            let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(quantity, 1);
            use crate::durability::item_use::novelty_appearance::{ActionPremium, ActionRequest};
            use crate::foundation::{CommandId, CommandRef};
            struct Premium;
            impl ActionPremium for Premium { fn current(&self, _account: [u8;16]) -> bool { true } }
            let permit = owner.spell_lane_permit().await.ok_or("permit")?;
            let native = active.native_gameplay().ok_or("native")?;
            let content = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native);
            for (number, seed, expected) in [(3,73,UseDisposition::Committed), (4,73,UseDisposition::NothingToUse), (5,72,UseDisposition::Committed), (6,74,UseDisposition::Committed), (7,76,UseDisposition::Committed), (8,77,UseDisposition::Committed), (9,78,UseDisposition::Committed), (10,78,UseDisposition::NothingToUse)] {
                let request = ActionRequest { command: CommandRef::new(command(1).game_session_id, CommandId::new(number).map_err(debug)?), source: id(seed), intent: [b"srvgap-c4-pg:".as_slice(), &number.to_be_bytes()].concat(), player_target: false };
                let result = harness.root.use_novelty_appearance(&permit, &authority, &harness.node, fence, request.clone(), native, &content, &Premium, false).await?.ok_or("action result")?;
                assert_eq!(result.disposition, expected, "command {number}");
                if expected == UseDisposition::Committed {
                    let replay = harness.root.use_novelty_appearance(&permit, &authority, &harness.node, fence, request, native, &content, &Premium, true).await?.ok_or("receipt")?;
                    assert!(replay.replay);
                    assert_eq!(replay.action, result.action);
                }
            }
            let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE source_kind='item_action'").fetch_one(&harness.pool).await?;
            assert_eq!(grants, 6, "base, addon, doll base+addon, random mount, direct mount");
            let timers: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_temporal_schedules WHERE definition_production_key='oteryn:item.tibia.i6573' AND target_production_key='oteryn:item.tibia.i6572' AND duration_millis=4000").fetch_one(&harness.pool).await?;
            assert_eq!(timers, 1);
            drop(permit);
            owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
            let before = { let runtime = owner.runtime.lock().await; owner.states.spell_states.lock().await.get(&runtime, deps.actor, command(1).game_session_id).ok_or("actor")?.health };
            let firework = owner.use_item_completion(deps.actor, command(11), ItemKey::Instance(id(75))).await.ok_or("firework")?;
            assert_eq!(firework.disposition, UseDisposition::Committed);
            let after = { let runtime = owner.runtime.lock().await; owner.states.spell_states.lock().await.get(&runtime, deps.actor, command(1).game_session_id).ok_or("actor")?.health };
            assert_eq!(after, before.saturating_sub(10));
            assert_eq!(owner.use_item_completion(deps.actor, command(11), ItemKey::Instance(id(75))).await.ok_or("firework replay")?.disposition, UseDisposition::Committed);
            let replay_health = { let runtime = owner.runtime.lock().await; owner.states.spell_states.lock().await.get(&runtime, deps.actor, command(1).game_session_id).ok_or("actor")?.health };
            assert_eq!(replay_health, after);
            let target = {
                let mut runtime = owner.runtime.lock().await;
                let from = runtime.read_actor_position(deps.actor).map_err(debug)?.position();
                let session = crate::foundation::GameSessionId::decode(&id(51)).map_err(debug)?;
                let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
                let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime.initialize_pinned_test_position(actor, crate::foundation::MovementLocalPosition { x: from.x + 4, y: from.y, floor: from.floor }).map_err(debug)?;
                actor
            };
            let intent = oteryn_protocol_oteryn::world_object::UseIntent {
                target: oteryn_protocol_oteryn::world_object::UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
                use_with: Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef { identity: target.placement_identity(), generation: target.actor_local_generation() }),
                use_with_map_item: None, use_with_position: None,
            };
            let far = owner.use_usable_action(deps.actor, command(12), &intent, Some(ItemKey::Instance(id(79))), true, false).await.map_err(|_| "voodoo owner")?.ok_or("voodoo result")?;
            assert_eq!(far.disposition, UseDisposition::TooFar, "a visible player outside Action's one-tile range must not activate the doll");
            let permit = owner.spell_lane_permit().await.ok_or("timer permit")?;
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(8);
            loop {
                harness.root.drain_spell_item_deadlines(&permit, &authority, &harness.node, fence.runtime_scope, fence.scope_ownership_generation.get()).await.map_err(debug)?;
                let returned: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p JOIN game_item_instances i USING(item_instance_id) WHERE p.definition_production_key='oteryn:item.tibia.i6573' AND i.definition_production_key='oteryn:item.tibia.i6572')").fetch_one(&harness.pool).await?;
                if returned { break; }
                if tokio::time::Instant::now() >= deadline { return Err("trumpet did not decay through the existing Item timer".into()); }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            // A receipt committed by main's event-item owner stays with that owner,
            // even when another source unit is still present after the acknowledgement is lost.
            let legacy_intent = oteryn_protocol_oteryn::world_object::UseIntent {
                target: oteryn_protocol_oteryn::world_object::UseTarget::Item(std::num::NonZeroU64::new(2).ok_or("legacy handle")?),
                use_with: None, use_with_map_item: None, use_with_position: None,
            };
            let legacy_request = crate::durability::consume_transform::ConsumeTransformRequest {
                command: CommandRef::new(command(1).game_session_id, CommandId::new(13).map_err(debug)?),
                source: id(80), target: None,
                intent: oteryn_protocol_oteryn::world_object::encode_item_use_intent(&legacy_intent).map_err(debug)?,
            };
            let legacy = harness.root.consume_transform_detailed(&permit, &authority, &harness.node, fence, legacy_request, native, &content, false).await?.ok_or("main event receipt")?;
            assert_eq!(legacy.disposition, UseDisposition::Committed);
            drop(permit);
            let deferred = owner.use_usable_action(deps.actor, command(13), &legacy_intent, Some(ItemKey::Instance(id(80))), false, false).await.map_err(|_| "legacy probe")?;
            assert!(deferred.is_none(), "C4 must defer replay to main's existing receipt owner");
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(80).as_slice()).fetch_one(&harness.pool).await?, 1);
            let replay = owner.use_event_item(deps.actor, command(13), legacy_intent, Some(ItemKey::Instance(id(80))), false).await.map_err(|_| "main replay")?.ok_or("main replay result")?;
            assert_eq!(replay.disposition, UseDisposition::Committed);
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            eprintln!("SRVGAP_C4_PG_COMPLETE duration={:?}", started.elapsed());
            Ok(())
        }).map_err(|e| e.to_string())
    })?.join().map_err(|_| "C4 scenario panicked")??;
    Ok(())
}
