//! Carried USE exercises the same item burn and Character build transaction as a player.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::BuildOccurrence;
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_items::concoctions::Concoction;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const BAG: u8 = 170;
const FIRST: u8 = 171;
const SECOND: u8 = 172;
const WEALTH: u8 = 173;

#[test]
fn stamina_extension_use_burns_once_and_caps_live_and_durable_stamina() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_big_stack(admin, 2500, 2520, false)
}

#[test]
fn stamina_extension_replay_and_cooldown_leave_custody_unchanged() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_big_stack(admin, 2300, 2360, true)
}

fn on_big_stack(admin: String, before: u16, after: u16, replay: bool) -> TestResult {
    std::thread::Builder::new()
        .name("stamina_extension".into())
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(scenario(admin, before, after, replay))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "stamina extension scenario panicked")?
        .map_err(Into::into)
}

async fn scenario(admin: String, before: u16, after: u16, replay: bool) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "stamina_extension", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
    let generation = controller.active().ok_or("active")?;
    let room = pin.qualified_room().clone();
    let deps = Dependencies::with_content(
        pin.into_channel_pin(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    deps.initialize(&owner).await;
    let actor = deps.actor;
    let session = deps.admitted.game_session_id;
    let gameplay_fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("gameplay fence")?;
    let fence = gameplay_fence.item_fence();
    h.root
        .commit_hunting_stamina(
            &authority,
            owner.holder,
            gameplay_fence,
            BuildOccurrence::from_bytes(id(180)).map_err(debug)?,
            (2520, before),
        )
        .await
        .map_err(debug)?;
    {
        let runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .load_stamina(&runtime, actor, session, before);
    }
    seed_items(&h).await?;

    let use_item = async |command_id, seed| {
        let command = UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        owner
            .use_backpack_item(actor, command, ItemKey::Instance(id(seed)))
            .await
            .disposition
    };
    let assert_stamina = async |expected| -> TestResult {
        assert_eq!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .live_stamina(actor, session),
            Some(expected)
        );
        assert_eq!(
            h.root
                .read_hunting_stamina(&authority, fence.character_id)
                .await
                .map_err(debug)?,
            expected
        );
        Ok(())
    };
    assert_eq!(use_item(900, FIRST).await, UseDisposition::Committed);
    assert_eq!(custody(&h, FIRST).await?, (0, false));
    assert_stamina(after).await?;
    let receipt: (i32, i32, bool, bool) = sqlx::query_as(
        "SELECT e.stamina_before,e.stamina_after,e.created_xact_id=u.created_xact_id, \
                b.created_xact_id=u.created_xact_id \
           FROM game_character_concoction_effects e JOIN game_item_use_records u USING(transaction_id) \
           JOIN game_character_build_receipts b ON b.build_occurrence_id=u.transaction_id \
          WHERE e.concoction='stamina_extension' AND u.cause_kind=527 AND u.shape=1",
    ).fetch_one(&h.pool).await?;
    assert_eq!(receipt, (i32::from(before), i32::from(after), true, true));

    if replay {
        assert_eq!(use_item(901, WEALTH).await, UseDisposition::Committed);
        assert_eq!(use_item(902, SECOND).await, UseDisposition::Exhausted);
        assert_eq!(use_item(900, SECOND).await, UseDisposition::Rejected);
        assert_eq!(use_item(900, FIRST).await, UseDisposition::Committed);
        assert_eq!(custody(&h, FIRST).await?, (0, false));
        assert_eq!(custody(&h, SECOND).await?, (1, true));
        let counts: (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_concoction_effects), \
                    (SELECT count(*) FROM game_item_use_records)",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(counts, (2, 2));
        assert_stamina(after).await?;

        // Freeze only the effect clock. Admission, lease and ownership still use real time.
        set_effect_clock(&h, 86_399).await?;
        assert_eq!(use_item(903, SECOND).await, UseDisposition::Exhausted);
        assert_eq!(custody(&h, SECOND).await?, (1, true));
        set_effect_clock(&h, 86_400).await?;
        assert_eq!(use_item(904, SECOND).await, UseDisposition::Committed);
        assert_eq!(custody(&h, SECOND).await?, (0, false));
        assert_stamina(after + 60).await?;
    }
    Ok(())
}

async fn seed_items(h: &Harness) -> TestResult {
    use crate::durability::item_fee_burn::test_support::item;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key) in [
        (BAG, "oteryn:item.tibia.i2854"),
        (FIRST, Concoction::StaminaExtension.production_key()),
        (SECOND, Concoction::StaminaExtension.production_key()),
        (WEALTH, Concoction::WealthDuplex.production_key()),
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(item(seed, key, "definition-r1", 1)))
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query(
        "INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)",
    ).bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(42).as_slice()).bind(id(181).as_slice())
        .execute(&mut *tx).await?;
    for (ordinal, seed) in [FIRST, SECOND, WEALTH].into_iter().enumerate() {
        sqlx::query(
            "INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) \
             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)",
        ).bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice())
            .bind(i64::try_from(ordinal)? + 1).bind(id(seed + 20).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn custody(h: &Harness, seed: u8) -> TestResult<(i64, bool)> {
    Ok(sqlx::query_as(
        "SELECT quantity,EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) \
           FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid",
    ).bind(id(seed).as_slice()).fetch_one(&h.pool).await?)
}

async fn set_effect_clock(h: &Harness, seconds: i64) -> TestResult {
    let ddl: String = sqlx::query_scalar(
        "SELECT format('CREATE OR REPLACE FUNCTION public.game_daily_effect_now() \
         RETURNS timestamptz LANGUAGE sql VOLATILE AS %L', \
         format('SELECT %L::timestamptz', min(started_at)+make_interval(secs => $1::bigint::double precision))) \
         FROM game_character_concoction_effects WHERE concoction='stamina_extension'",
    ).bind(seconds).fetch_one(&h.pool).await?;
    // PostgreSQL's %L quotes the isolated fixture timestamp.
    sqlx::query(sqlx::AssertSqlSafe(ddl))
        .execute(&h.pool)
        .await?;
    Ok(())
}
