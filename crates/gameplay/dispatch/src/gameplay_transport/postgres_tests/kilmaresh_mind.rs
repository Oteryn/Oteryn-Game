//! Crystal Mind potion through the ordinary carried USE owner and PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::TestResult;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::{
    SeamRooms,
    connection::{FreshAdmissionAuthority, UseCommand},
    item_view::ItemKey,
};
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn kilmaresh_mind_use_requires_equipped_ring_and_burns_one_potion_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| exercise().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Kilmaresh Mind case panicked")??;
    Ok(())
}

fn exercise() -> TestResult {
    run(async |admin| {
        let start = MovementLocalPosition { x: 33865, y: 31533, floor: 7 };
        let scene = super::srx3_soul_war_world_fixture::source_world(start)?;
        let deps = scene.dependencies()?;
        let h = Harness::create_with_admission_seed(admin, "kilmaresh_mind", false, false).await?;
        super::house_transfer::prepare_owned_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = scene.controller.active();
        owner.rooms = SeamRooms::new(None, Some(&scene.room));
        super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
        let mut admitted = deps.admitted;
        super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 250).await?;
        super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i31350", 1)]).await?;
        let actor = admitted.runtime_actor.ok_or("player")?;
        let use_command = |command_id| UseCommand {
            game_session_id: admitted.game_session_id,
            command_id,
            item_fence: admitted.item_fence,
        };
        let source = ItemKey::Instance(id(71));
        let refused = owner.use_item(actor, use_command(1000), source).await;
        assert_eq!(refused.disposition, UseDisposition::NotPossible);
        assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), start);
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i31306','definition-r1',1,1,encode($3,'hex')::uuid)")
            .bind(id(75).as_slice()).bind(id(42).as_slice()).bind(id(76).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,8,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(75).as_slice()).bind(id(42).as_slice()).bind(id(76).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let used = owner.use_item(actor, use_command(1001), source).await;
        assert_eq!(used.disposition, UseDisposition::Committed);
        let landed = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position();
        assert!(matches!((landed.x, landed.y, landed.floor), (33913, 31653, 9) | (33921, 31650, 9)));
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i31350' AND lifecycle=1").fetch_one(&h.pool).await?;
        assert_eq!(count, 0);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records WHERE recipe='oteryn:kilmaresh.mind-potion'").fetch_one(&h.pool).await?;
        assert_eq!(receipts, 1);
        Ok(())
    })
}
