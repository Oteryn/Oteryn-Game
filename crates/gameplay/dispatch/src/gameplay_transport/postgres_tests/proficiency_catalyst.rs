//! P08 Proficiency Catalysts through the production use-with owner: one unit burns with one
//! `catalyst` receipt, a replay burns nothing, and a non-weapon target spends nothing.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_proficiency::ProficiencyDefinitions;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::sync::Arc;

const SWORD: &str = "oteryn:item.tibia.i3264";
const CATALYST: &str = "oteryn:item.tibia.i51588";
const GOLD: &str = "oteryn:item.tibia.i3031";
const BAG: u8 = 160;
const UNITS: u8 = 162;
const WEAPON: u8 = 163;
const COIN: u8 = 164;

#[test]
fn proficiency_catalyst_burns_one_unit_with_one_receipt() -> TestResult {
    std::thread::Builder::new()
        .name("proficiency_catalyst".into())
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "proficiency catalyst scenario panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "proficiency_catalyst", false, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let catalogue = ProductionProficiencyCatalogue::embedded().ok_or("catalogue")?;
        let cap = u64::from(
            catalogue
                .resolve_weapon(SWORD)
                .ok_or("sword definition")?
                .shape
                .track_shape()
                .final_progress(),
        );
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        owner.proficiency_catalogue = Some(Arc::clone(&catalogue));
        deps.initialize(&owner).await;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        let binding = owner.progression_binding(session).ok_or("progression")?;
        let definitions =
            catalogue.for_content(&binding.context.content) as Arc<dyn ProficiencyDefinitions>;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&harness.pool).await?;
        seed_bag(&harness).await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?
            .item_fence();
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        let pool = &harness.pool;
        let quantity = |seed: u8| async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(seed).as_slice())
            .fetch_one(pool)
            .await
        };
        let counts = || async {
            sqlx::query_as::<_, (i64, i64)>("SELECT (SELECT count(*) FROM game_character_proficiency_receipts WHERE cause='catalyst'),(SELECT count(*) FROM game_item_use_records WHERE cause_kind=494)")
                .fetch_one(pool)
                .await
        };
        let use_with = async |command_id, target| {
            FreshAdmissionAuthority::use_proficiency_catalyst(
                &owner,
                actor,
                command(command_id),
                ItemKey::Instance(id(UNITS)),
                ItemKey::Instance(id(target)),
            )
            .await
            .map(|outcome| outcome.disposition)
        };

        // A coin is not a weapon with a proficiency: nothing is spent or written.
        assert_eq!(use_with(1, COIN).await, Some(UseDisposition::NotPossible));
        assert_eq!(quantity(UNITS).await?, 2);
        assert_eq!(counts().await?, (0, 0));

        // One unit burns and the sword's track gains 25,000 (TibiaWiki; Crystal
        // proficiency_catalysts.lua:3) in the same transaction as the burn.
        assert_eq!(use_with(2, WEAPON).await, Some(UseDisposition::Committed));
        assert_eq!(quantity(UNITS).await?, 1);
        assert_eq!(counts().await?, (1, 1));
        let paired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_use_records u JOIN game_character_proficiency_receipts r ON r.proficiency_occurrence_id=u.transaction_id JOIN game_character_proficiency_receipt_lines l USING(proficiency_occurrence_id) WHERE u.cause_kind=494 AND u.shape=1 AND r.cause='catalyst' AND l.progress_after-l.progress_before=$1 AND u.source_definition_production_key=$2")
            .bind(i64::try_from(cap.min(25_000))?)
            .bind(CATALYST)
            .fetch_one(pool)
            .await?;
        assert_eq!(paired, 1);

        // The same command again answers from its receipt without a second burn.
        assert_eq!(use_with(2, WEAPON).await, Some(UseDisposition::Committed));
        assert_eq!(quantity(UNITS).await?, 1);
        assert_eq!(counts().await?, (1, 1));

        let character = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?
            .character_id;
        let (tracks, _) = harness
            .root
            .read_character_proficiency_with_definitions(&authority, character, None, definitions)
            .await
            .map_err(debug)?;
        let stored = tracks
            .iter()
            .find(|track| track.state.item_key() == SWORD)
            .ok_or("durable track")?;
        assert_eq!(stored.state.progress(), cap.min(25_000));
        let projected = owner
            .states
            .proficiencies
            .lock()
            .map_err(|_| "states")?
            .get(&session)
            .and_then(|state| state.track(SWORD).map(|track| u64::from(track.progress)));
        assert_eq!(projected, Some(cap.min(25_000)));
        eprintln!("proficiency_catalyst: cases executed");
        Ok(())
    })
}

/// One main backpack holding two catalysts, the sword and a coin.
async fn seed_bag(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for table in ["game_item_container_entries", "game_item_container_slots"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE character_id=encode($1,'hex')::uuid"
        )))
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    }
    let item = |seed, key: &str, count| {
        crate::durability::item_fee_burn::test_support::item(seed, key, "definition-r1", count)
    };
    sqlx::raw_sql(sqlx::AssertSqlSafe(item(BAG, "oteryn:item.tibia.i2854", 1)))
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(42).as_slice()).bind(id(245).as_slice()).execute(&mut *tx).await?;
    for (ordinal, (seed, key, count)) in [(UNITS, CATALYST, 2), (WEAPON, SWORD, 1), (COIN, GOLD, 1)]
        .into_iter()
        .enumerate()
    {
        sqlx::raw_sql(sqlx::AssertSqlSafe(item(seed, key, count)))
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::bigint,encode($6,'hex')::uuid)").bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(i64::try_from(ordinal + 1)?).bind(id(246).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
