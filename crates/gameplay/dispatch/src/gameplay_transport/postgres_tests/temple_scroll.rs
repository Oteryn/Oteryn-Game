//! WU26-06 Temple Teleport Scroll through the production USE owner
//! (`docs/contracts/TEMPLE_TELEPORT_SCROLL_V1.md`): a use burns the scroll with its use row and
//! moves the actor to the home temple, a logout block refuses with nothing written, and a
//! scroll used after its seven-day delivery deadline burns as expired without moving anyone.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const SCROLL: &str = "oteryn:item.tibia.i25718";
const BAG: u8 = 170;
const FRESH: u8 = 171;
const OLD: u8 = 172;
const FRESH_CLAIM: u8 = 173;
const OLD_CLAIM: u8 = 174;
const START: MovementLocalPosition = MovementLocalPosition {
    x: 7,
    y: 0,
    floor: 7,
};

#[derive(Clone, Copy)]
enum Case {
    Teleport,
    LogoutBlock,
    Expired,
}

#[test]
fn temple_scroll_teleports_home_and_is_consumed() -> TestResult {
    on_big_stack("temple_scroll_teleport", Case::Teleport)
}

#[test]
fn temple_scroll_refused_with_logout_block() -> TestResult {
    on_big_stack("temple_scroll_logout_block", Case::LogoutBlock)
}

#[test]
fn temple_scroll_expires_after_seven_days() -> TestResult {
    on_big_stack("temple_scroll_expired", Case::Expired)
}

fn on_big_stack(name: &'static str, case: Case) -> TestResult {
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(64 << 20)
        .spawn(move || scenario(name, case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "temple scroll scenario panicked")?
        .map_err(Into::into)
}

#[allow(
    clippy::too_many_lines,
    reason = "one composed owner per case keeps the production setup in one place"
)]
fn scenario(name: &'static str, case: Case) -> TestResult {
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(admin, name, false, false).await?;
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
            .initialize_pinned_test_position(deps.actor, START)
            .map_err(debug)?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&harness.pool).await?;
        seed_scrolls(&harness).await?;
        let temple = owner.runtime.lock().await.respawn_position();
        assert_ne!(temple, START, "the fixture start must differ from the temple");
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
        let item = |seed: u8| async move {
            sqlx::query_as::<_, (i64, i16)>(
                "SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(seed).as_slice())
            .fetch_one(pool)
            .await
        };
        let uses = || async {
            sqlx::query_as::<_, (i64, i64, i64)>(
                "SELECT (SELECT count(*) FROM game_item_use_records WHERE cause_kind=481 AND shape=1), \
                        (SELECT count(*) FROM game_temple_scroll_uses WHERE NOT expired), \
                        (SELECT count(*) FROM game_temple_scroll_uses WHERE expired)",
            )
            .fetch_one(pool)
            .await
        };
        let position = || async {
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map(|snapshot| snapshot.position())
                .map_err(debug)
        };
        let use_scroll = async |command_id, seed| {
            owner
                .use_backpack_item(actor, command(command_id), ItemKey::Instance(id(seed)))
                .await
                .disposition
        };

        match case {
            Case::Teleport => {
                // One use burns the scroll with its use row and moves the actor home.
                assert_eq!(use_scroll(1, FRESH).await, UseDisposition::Committed);
                assert_eq!(item(FRESH).await?, (0, 2));
                assert_eq!(uses().await?, (1, 1, 0));
                assert_eq!(position().await?, temple);
                let paired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_use_records u JOIN game_temple_scroll_uses s USING(transaction_id) WHERE u.cause_kind=481 AND u.source_item_instance_id=encode($1,'hex')::uuid AND u.source_definition_production_key=$2 AND s.item_instance_id=u.source_item_instance_id AND s.deadline_at IS NOT NULL AND s.deadline_at>s.observed_at")
                    .bind(id(FRESH).as_slice())
                    .bind(SCROLL)
                    .fetch_one(pool)
                    .await?;
                assert_eq!(paired, 1);
                // The same command again answers from its record: nothing more burns.
                assert_eq!(use_scroll(1, FRESH).await, UseDisposition::Committed);
                assert_eq!(uses().await?, (1, 1, 0));
                assert_eq!(item(OLD).await?, (1, 1));
            }
            Case::LogoutBlock => {
                // A PvE hit starts the in-fight deadline: LOGOUT_INTENT would refuse, so the
                // scroll refuses with nothing burnt, written or moved.
                {
                    let runtime = owner.runtime.lock().await;
                    owner.states.attack.lock().await.record_hit_taken(
                        &runtime,
                        actor,
                        session,
                        owner.owner_now(),
                    );
                }
                assert_eq!(use_scroll(1, FRESH).await, UseDisposition::NotPossible);
                assert_eq!(item(FRESH).await?, (1, 1));
                assert_eq!(uses().await?, (0, 0, 0));
                assert_eq!(position().await?, START);
            }
            Case::Expired => {
                // Delivered eight days ago: the burn's row records the expiry, the scroll is
                // consumed and the actor stays where it is.
                assert_eq!(use_scroll(1, OLD).await, UseDisposition::Committed);
                assert_eq!(item(OLD).await?, (0, 2));
                assert_eq!(uses().await?, (1, 0, 1));
                assert_eq!(position().await?, START);
                let lifetime_ms: i64 = sqlx::query_scalar("SELECT (EXTRACT(EPOCH FROM s.deadline_at-l.effect_started_at)*1000)::bigint FROM game_temple_scroll_uses s JOIN game_character_daily_reward_lines l ON l.item_instance_id=s.item_instance_id WHERE s.item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(OLD).as_slice())
                    .fetch_one(pool)
                    .await?;
                assert_eq!(
                    lifetime_ms,
                    crate::durability::item_use::temple_scroll::TEMPLE_SCROLL_LIFETIME_MS
                );
                // The fresh scroll still teleports.
                assert_eq!(use_scroll(2, FRESH).await, UseDisposition::Committed);
                assert_eq!(uses().await?, (2, 1, 1));
                assert_eq!(position().await?, temple);
            }
        }
        eprintln!("{name}: cases executed");
        Ok(())
    })
}

/// One main backpack holding two scrolls with their delivered daily lines: one delivered an
/// hour ago and one eight days ago.
async fn seed_scrolls(h: &Harness) -> TestResult {
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
    for (ordinal, (seed, claim, age)) in [(FRESH, FRESH_CLAIM, "1 hour"), (OLD, OLD_CLAIM, "8 days")]
        .into_iter()
        .enumerate()
    {
        sqlx::raw_sql(sqlx::AssertSqlSafe(item(seed, SCROLL, 1)))
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::bigint,encode($6,'hex')::uuid)").bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(i64::try_from(ordinal + 1)?).bind(id(246).as_slice()).execute(&mut *tx).await?;
        // The delivery line as the daily claim writes it (migration 2762's interval).
        sqlx::query("WITH instant AS (SELECT clock_timestamp()-$3::text::interval AS at) INSERT INTO game_character_daily_reward_lines(claim_id,line,kind,definition,quantity,status,delivering_transaction_id,item_instance_id,effect_started_at,effect_expires_at) SELECT encode($1,'hex')::uuid,0,'TEMPLE_SCROLL',$4,1,'DELIVERED',encode($1,'hex')::uuid,encode($2,'hex')::uuid,at,at+interval '168 hours' FROM instant")
            .bind(id(claim).as_slice())
            .bind(id(seed).as_slice())
            .bind(age)
            .bind(SCROLL)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
