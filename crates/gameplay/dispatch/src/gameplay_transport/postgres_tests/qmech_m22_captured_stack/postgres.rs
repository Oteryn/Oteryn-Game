//! M22 quest wiring: the qm22 resolver uses the quantity locked in real inventory.
use super::super as fixtures;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use fixtures::fixture::{Harness, debug, id, run};
use fixtures::{Dependencies, TestResult, prepare_current_session};

#[test]
fn qmech_m22_postgres_recipe_uses_locked_target_quantity() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_STARTED M22 source recipe quantity API");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M22 quantity-API thread panicked".to_owned())
        .flatten();
    eprintln!(
        "POSTGRES_CASE_COMPLETED M22 source recipe quantity API duration={:.6}s success={}",
        started.elapsed().as_secs_f64(),
        result.is_ok()
    );
    result?;
    Ok(())
}

fn scenario() -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "qmech_m22_stack", true, false).await?;
        let result: TestResult = async {
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let mut deps = Dependencies::new()?;
            let (controller, pin) = fixtures::death_ml_loss_composed::production_generation_with_pin(deps.admitted.world_id, "content-1")?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native")?;
            let content_pin = ChannelContentPin::from_activation(
                deps.admitted.world_id, 1,
                generation.identity().server_artifact_digest(), generation.identity().client_artifact_digest(),
                pin.frame_binding().digest(), pin.qualified_room().map_revision_digest(), (3, 3, 7));
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(deps.admitted.world_id, deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?, 1, 1, 1,
                "runtime-scope-assignment:1", 2, content_pin).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime.initialize_pinned_test_position(deps.actor, MovementLocalPosition { x: 3, y: 3, floor: 7 }).map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            deps.initialize(&owner).await;
            let session = deps.admitted.game_session_id;
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).current_session_at(session).await?.0;
            let _fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
                .bind(id(41).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                .bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(generation.identity().server_artifact_digest().as_slice())
                .bind(generation.identity().client_artifact_digest().as_slice())
                .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            let entries = [("oteryn:item.tibia.i3736", 5), ("oteryn:item.tibia.i10297", 37)];
            fixtures::item_look::seed_main_backpack(&h, &entries).await?;
            let definition = |source| crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(), production_key: format!("oteryn:item.tibia.i{source}"), revision_ref: "definition-r1".into(),
            };
            let mut locked = h.pool.begin().await?;
            let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(id(72).as_slice()).fetch_one(&mut *locked).await?;
            let resolved = oteryn_gameplay_items::item_craft::carried_cooking_conversion_with_quantity(
                native, &definition(3736), &definition(10297), None, u32::try_from(quantity)?)
                .ok_or("source-qualified carried recipe")?;
            assert_eq!(resolved.target_debit, 37);
            assert_eq!(resolved.conversion.recipe, "oteryn:item-conversion.tainted-souls-antler-talisman");
            let [(output, count)] = resolved.conversion.outputs.as_slice() else { return Err("one transformed object".into()) };
            assert_eq!(output.production_key, "oteryn:item.tibia.i22008");
            assert_eq!(*count, 1, "non-stackable transformed talisman");
            locked.rollback().await?;
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
