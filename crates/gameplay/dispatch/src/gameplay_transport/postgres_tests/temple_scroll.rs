//! WU26-06 Temple Teleport Scroll through the production USE owner
//! (`docs/contracts/TEMPLE_TELEPORT_SCROLL_V1.md`): a use burns the scroll with its use row and
//! moves the actor to the home temple, a logout block refuses with nothing written, and a
//! temple landing taken at commit rolls the burn back: the scroll is kept. A scroll past its
//! seven-day deadline is refused with nothing written, and one Item deadline pass removes every
//! due scroll wherever it is (carried, worn, in the Inbox, in a container in the Inbox, in a
//! depot box, on a House tile, in a corpse, in a container on Ground, containers and corpses
//! nested at any depth), for an online or an offline Character: expiry applies in all custodies.
//! Due scrolls the session-bound pass owns never hold back the others. Instance Ground has no
//! deadline pass: a due scroll there leaves with the Instance at teardown.
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
const INBOX: u8 = 175;
const DELIVERY: u8 = 176;
const DEPOT: u8 = 177;
const PARCEL: u8 = 178;
const NESTED: u8 = 179;
const WORN: u8 = 180;
const CORPSE: u8 = 181;
const IN_CORPSE: u8 = 182;
const GROUND_BAG: u8 = 183;
const ON_GROUND: u8 = 184;
const ON_TILE: u8 = 185;
const MID_BAG: u8 = 189;
const DEEP_BAG: u8 = 190;
const DEEP: u8 = 191;
const INNER_CORPSE: u8 = 192;
const DEEP_IN_CORPSE: u8 = 193;
const INSTANCE_CORPSE: u8 = 194;
const INSTANCE_IN_CORPSE: u8 = 195;
const INSTANCE_GROUND: u8 = 196;
const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
const DUE: [u8; 5] = [OLD, INBOX, DEPOT, NESTED, WORN];
const WORLD_DUE: [u8; 3] = [ON_TILE, IN_CORPSE, ON_GROUND];
/// Three containers deep in the Inbox parcel, and in a corpse inside the corpse on Ground.
const DEEP_DUE: [u8; 2] = [DEEP, DEEP_IN_CORPSE];
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
    Offline,
    Backlog,
    NoLanding,
}

/// The Channel owner's landing with every temple cell and its ring occupied at commit.
struct Occupied;

impl crate::durability::item_use::temple_scroll::TempleLanding for Occupied {
    fn land(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
        Box::pin(async { false })
    }
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
fn temple_scroll_kept_when_the_temple_landing_is_taken_at_commit() -> TestResult {
    on_big_stack("temple_scroll_no_landing", Case::NoLanding)
}

#[test]
fn temple_scroll_vanishes_through_the_item_deadline_owner() -> TestResult {
    on_big_stack("temple_scroll_expired", Case::Expired)
}

#[test]
fn temple_scroll_vanishes_in_every_custody_of_an_offline_character() -> TestResult {
    on_big_stack("temple_scroll_offline", Case::Offline)
}

#[test]
fn temple_scroll_backlog_of_online_scrolls_never_holds_back_stored_ones() -> TestResult {
    on_big_stack("temple_scroll_backlog", Case::Backlog)
}

#[test]
fn temple_scroll_on_instance_ground_leaves_with_the_instance() -> TestResult {
    std::thread::Builder::new()
        .name("temple_scroll_instance".into())
        .stack_size(256 << 20)
        .spawn(|| instance_teardown().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "temple scroll instance scenario panicked")?
        .map_err(Into::into)
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
        seed_scrolls(&harness, matches!(case, Case::Offline)).await?;
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
            sqlx::query_as::<_, (i64, i64)>(
                "SELECT (SELECT count(*) FROM game_item_use_records WHERE cause_kind=481 AND shape=1), \
                        (SELECT count(*) FROM game_temple_scroll_uses)",
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
                assert_eq!(uses().await?, (1, 1));
                assert_eq!(position().await?, temple);
                let paired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_use_records u JOIN game_temple_scroll_uses s USING(transaction_id) JOIN game_spell_item_temporal_schedules p ON p.item_instance_id=s.item_instance_id WHERE u.cause_kind=481 AND u.source_item_instance_id=encode($1,'hex')::uuid AND u.source_definition_production_key=$2 AND s.item_instance_id=u.source_item_instance_id AND s.expires_at_unix_ms=p.expires_at_unix_ms AND s.expires_at_unix_ms>s.observed_at_unix_ms")
                    .bind(id(FRESH).as_slice())
                    .bind(SCROLL)
                    .fetch_one(pool)
                    .await?;
                assert_eq!(paired, 1);
                // The same command again answers from its record: nothing more burns.
                assert_eq!(use_scroll(1, FRESH).await, UseDisposition::Committed);
                assert_eq!(uses().await?, (1, 1));
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
                assert_eq!(uses().await?, (0, 0));
                assert_eq!(position().await?, START);
            }
            Case::NoLanding => {
                // PREPARE found a landing; the commit's move finds none. The burn and its row
                // roll back with the transaction: the scroll is not consumed.
                let inventory = owner
                    .root
                    .read_item_use_inventory(owner.character, owner.holder, fence)
                    .await
                    .map_err(debug)?;
                let occurred_at_unix_ms = i64::try_from(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_millis(),
                )?;
                let request = crate::durability::item_use::ItemUseRequest {
                    kind: crate::durability::item_use::ItemUseKind::TempleScroll,
                    command_id: 1,
                    content_revision: inventory.content_revision.clone(),
                    ruleset_revision: inventory.ruleset_revision.clone(),
                    sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION
                        .to_owned(),
                    transaction_id: crate::operator_files::uuid_v7()?,
                    event_id: crate::operator_files::uuid_v7()?,
                    occurred_at_unix_ms,
                    server_build_id: crate::durability::character_authority::SERVER_BUILD_ID
                        .to_owned(),
                    source_item_instance_id: id(FRESH),
                    source_definition: crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".to_owned(),
                        production_key: SCROLL.to_owned(),
                        revision_ref: "definition-r1".to_owned(),
                    },
                    source_quantity_before: 1,
                    flask: crate::durability::item_use::ItemUseFlaskPlan::None,
                };
                let outcome = owner
                    .root
                    .commit_temple_scroll_use(
                        owner.character,
                        owner.holder,
                        fence,
                        request,
                        &Occupied,
                    )
                    .await
                    .map_err(debug)?;
                assert_eq!(
                    outcome,
                    crate::durability::item_use::temple_scroll::TempleScrollOutcome::NoLanding
                );
                assert_eq!(item(FRESH).await?, (1, 1));
                assert_eq!(uses().await?, (0, 0));
                assert_eq!(position().await?, START);
                // The kept scroll is used normally once its landing is free.
                assert_eq!(use_scroll(2, FRESH).await, UseDisposition::Committed);
                assert_eq!(item(FRESH).await?, (0, 2));
                assert_eq!(uses().await?, (1, 1));
                assert_eq!(position().await?, temple);
            }
            Case::Expired => {
                // A use racing the deadline pass is refused: nothing burns, is written or moves.
                assert_eq!(use_scroll(1, OLD).await, UseDisposition::NotPossible);
                assert_eq!(item(OLD).await?, (1, 1));
                assert_eq!(uses().await?, (0, 0));
                assert_eq!(position().await?, START);
                // One pass removes every due scroll: the carried one under the live session,
                // the stored ones (Inbox, container in the Inbox, depot box) under none.
                owner.drain_source_item_deadlines().await;
                for seed in [OLD, INBOX, DEPOT, NESTED] {
                    assert_eq!(item(seed).await?, (0, 2));
                }
                assert_eq!(item(FRESH).await?, (1, 1));
                assert_eq!(custody(pool, &[OLD, INBOX, DEPOT, NESTED]).await?, (0, vec![2, 2, 4, 5], 3, 0));
                assert_eq!(uses().await?, (0, 0));
                assert_eq!(position().await?, START);
                // The fresh scroll still teleports.
                assert_eq!(use_scroll(2, FRESH).await, UseDisposition::Committed);
                assert_eq!(uses().await?, (1, 1));
                assert_eq!(position().await?, temple);
            }
            Case::Offline => {
                // The Character logs out with due scrolls carried, worn, in the Inbox, in a
                // container in the Inbox (one of them three containers deep) and in a depot
                // box; others lie on a House tile, in a corpse (one in a corpse inside it) and
                // in a backpack on Ground. One pass removes each of them under no session; the
                // fresh scroll and every container stay.
                go_offline(&harness).await?;
                owner.drain_source_item_deadlines().await;
                let due: Vec<u8> = DUE.into_iter().chain(WORLD_DUE).chain(DEEP_DUE).collect();
                for seed in due.iter().copied() {
                    assert_eq!(item(seed).await?, (0, 2));
                }
                for seed in [FRESH, PARCEL, MID_BAG, DEEP_BAG, CORPSE, INNER_CORPSE] {
                    assert_eq!(item(seed).await?, (1, 1));
                }
                assert_eq!(
                    custody(pool, &due).await?,
                    (0, vec![2, 2, 2, 2, 3, 4, 5, 6, 7, 7], 10, 0)
                );
                let inner: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid), (SELECT count(*) FROM game_item_corpse_entry_removal_evidence WHERE item_instance_id=encode($3,'hex')::uuid AND parent_item_instance_id=encode($1,'hex')::uuid AND channel_id=encode($4,'hex')::uuid AND scope_kind=1)")
                    .bind(id(INNER_CORPSE).as_slice())
                    .bind(id(CORPSE).as_slice())
                    .bind(id(DEEP_IN_CORPSE).as_slice())
                    .bind(id(43).as_slice())
                    .fetch_one(pool)
                    .await?;
                assert_eq!(inner, (1, 1));
                let world: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_item_house_reclaim_provenance WHERE item_instance_id=encode($1,'hex')::uuid), (SELECT count(*) FROM game_spell_item_temporal_receipts WHERE character_id IS NULL AND custody_kind IN(2,6,7))")
                    .bind(id(ON_TILE).as_slice())
                    .fetch_one(pool)
                    .await?;
                assert_eq!(world, (0, 4));
                let worn: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid), (SELECT revision::bigint FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid)")
                    .bind(id(41).as_slice())
                    .fetch_one(pool)
                    .await?;
                assert_eq!(worn, (0, 2));
                assert_eq!(uses().await?, (0, 0));
            }
            Case::Backlog => {
                // 501 scrolls carried by the live Character fell due earlier than the stored
                // ones. The session-less pass leaves them to the session-bound pass, and they
                // never fill its window: a one-scroll pass still retires a stored scroll.
                seed_backlog(&harness).await?;
                let retired: i32 = sqlx::query_scalar("SELECT game_temple_scroll_expire_due(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1)")
                    .bind(id(42).as_slice())
                    .bind(id(43).as_slice())
                    .fetch_one(pool)
                    .await?;
                assert_eq!(retired, 1);
                let receipts: (i64, i64) = sqlx::query_as("SELECT count(*), count(*) FILTER (WHERE game_session_id IS NULL AND custody_kind IN(2,4,5)) FROM game_spell_item_temporal_receipts")
                    .fetch_one(pool)
                    .await?;
                assert_eq!(receipts, (1, 1));
                let live: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2 AND i.lifecycle=1")
                    .bind(id(41).as_slice())
                    .bind(SCROLL)
                    .fetch_one(pool)
                    .await?;
                assert_eq!(live, 503);
            }
        }
        eprintln!("{name}: cases executed");
        Ok(())
    })
}

/// Where the due scrolls are after one pass: their remaining location rows, their receipt custody
/// kinds, the receipts written under no session and the committed depot box count.
async fn custody(pool: &sqlx::PgPool, seeds: &[u8]) -> TestResult<(i64, Vec<i16>, i64, i32)> {
    let items: Vec<Vec<u8>> = seeds.iter().map(|seed| id(*seed).to_vec()).collect();
    Ok(sqlx::query_as(
        "WITH s AS (SELECT encode(b,'hex')::uuid AS item_instance_id FROM unnest($1::bytea[]) b) \
         SELECT (SELECT count(*) FROM s WHERE item_instance_id IN( \
                   SELECT item_instance_id FROM game_item_container_entries \
                   UNION ALL SELECT item_instance_id FROM game_item_nested_entries \
                   UNION ALL SELECT item_instance_id FROM game_character_equipment_slots \
                   UNION ALL SELECT item_instance_id FROM game_item_character_inbox_locations \
                   UNION ALL SELECT item_instance_id FROM game_item_character_depot_locations \
                   UNION ALL SELECT item_instance_id FROM game_item_house_interior_locations \
                   UNION ALL SELECT item_instance_id FROM game_item_corpse_container_entries)), \
                (SELECT coalesce(array_agg(custody_kind ORDER BY custody_kind),'{}') \
                   FROM game_spell_item_temporal_receipts JOIN s USING(item_instance_id)), \
                (SELECT count(*) FROM game_spell_item_temporal_receipts JOIN s USING(item_instance_id) \
                   WHERE game_session_id IS NULL), \
                (SELECT committed FROM game_character_depot_box_counters \
                   WHERE character_id=encode($2,'hex')::uuid AND town_id=1 AND box_number=1)",
    )
    .bind(items)
    .bind(id(41).as_slice())
    .fetch_one(pool)
    .await?)
}

/// The Character's session ends: it is offline for the deadline pass.
async fn go_offline(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn into_inbox(tx: &mut sqlx::PgConnection, seed: u8) -> TestResult {
    sqlx::query("INSERT INTO game_item_character_inbox_locations(item_instance_id,world_id,character_id,ordinal) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,coalesce(max(ordinal),0)+1 FROM game_item_character_inbox_locations WHERE character_id=encode($3,'hex')::uuid")
        .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(tx).await?;
    Ok(())
}

/// 501 more scrolls in the live Character's main backpack, each due two days ago.
async fn seed_backlog(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "WITH s AS MATERIALIZED (SELECT g,game_character_uuid_v7() AS item FROM generate_series(1,501) g), \
          i AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) \
                SELECT item,encode($1,'hex')::uuid,'Item',$3,'definition-r1',1,1,game_character_uuid_v7() FROM s), \
          e AS (INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) \
                SELECT encode($2,'hex')::uuid,encode($4,'hex')::uuid,item,encode($1,'hex')::uuid,100+g,game_character_uuid_v7() FROM s) \
         INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,content_digest) \
         SELECT item,encode($5,'hex')::uuid,1,$6,floor(extract(epoch FROM clock_timestamp())*1000)::bigint-172800000,'Item',$3,'definition-r1',sha256(convert_to($3||'|definition-r1','UTF8')) FROM s",
    )
    .bind(id(42).as_slice())
    .bind(id(41).as_slice())
    .bind(SCROLL)
    .bind(id(BAG).as_slice())
    .bind(id(DELIVERY).as_slice())
    .bind(crate::durability::item_use::temple_scroll::TEMPLE_SCROLL_LIFETIME_MS)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// One main backpack holding two scrolls, a scroll in the Character Inbox, a container in the
/// Inbox holding a scroll and a scroll in depot box 1 of town 1 (and, for the offline case, a
/// scroll in the left hand, one on a House tile, one in a corpse on Ground, one in a backpack
/// on Ground, one three containers deep in the Inbox and one in a corpse inside the corpse), each with its deadline on the Item deadline owner as the daily claim writes it: the
/// carried fresh scroll is due in seven days less an hour, every other one was due a day ago.
async fn seed_scrolls(h: &Harness, offline: bool) -> TestResult {
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
    sqlx::raw_sql(sqlx::AssertSqlSafe(item(PARCEL, "oteryn:item.tibia.i2854", 1)))
        .execute(&mut *tx)
        .await?;
    into_inbox(&mut tx, PARCEL).await?;
    let lifetime = crate::durability::item_use::temple_scroll::TEMPLE_SCROLL_LIFETIME_MS;
    let mut scrolls = vec![(FRESH, lifetime - 3_600_000)];
    scrolls.extend(DUE.into_iter().filter(|seed| offline || *seed != WORN).map(|seed| (seed, -86_400_000)));
    if offline {
        scrolls.extend(WORLD_DUE.map(|seed| (seed, -86_400_000)));
        // A live corpse and a backpack lying on this Channel's Ground.
        for (seed, key) in [(CORPSE, "oteryn:item.tibia.i4240"), (GROUND_BAG, "oteryn:item.tibia.i2854")] {
            sqlx::raw_sql(sqlx::AssertSqlSafe(item(seed, key, 1)))
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,'\\x01'::bytea,'map-r1','content-1','\\x01'::bytea,$5)")
                .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(vec![seed, 0, 7]).bind(9_170_000 + i64::from(seed)).execute(&mut *tx).await?;
        }
        // Two more backpacks under the Inbox parcel, one in the other, and a corpse inside the
        // corpse on Ground: the deep scrolls lie at the bottom of each chain.
        for (seed, key) in [(MID_BAG, "oteryn:item.tibia.i2854"), (DEEP_BAG, "oteryn:item.tibia.i2854"), (INNER_CORPSE, "oteryn:item.tibia.i4240")] {
            sqlx::raw_sql(sqlx::AssertSqlSafe(item(seed, key, 1)))
                .execute(&mut *tx)
                .await?;
        }
        for (seed, parent, ordinal, placed) in [(MID_BAG, PARCEL, 2_i64, 198), (DEEP_BAG, MID_BAG, 1, 199)] {
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::bigint,encode($5,'hex')::uuid)")
                .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(parent).as_slice()).bind(ordinal).bind(id(placed).as_slice()).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO game_item_corpse_container_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,2,encode($4,'hex')::uuid)")
            .bind(id(INNER_CORPSE).as_slice()).bind(id(42).as_slice()).bind(id(CORPSE).as_slice()).bind(id(201).as_slice()).execute(&mut *tx).await?;
        scrolls.extend(DEEP_DUE.map(|seed| (seed, -86_400_000)));
    }
    for (seed, due_in_ms) in scrolls {
        sqlx::raw_sql(sqlx::AssertSqlSafe(item(seed, SCROLL, 1)))
            .execute(&mut *tx)
            .await?;
        match seed {
            INBOX => into_inbox(&mut tx, seed).await?,
            DEPOT => {
                sqlx::query("INSERT INTO game_item_character_depot_locations(item_instance_id,world_id,character_id,town_id,box_number,ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,1)")
                    .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_depot_box_counters(character_id,town_id,box_number,next_ordinal,committed) VALUES(encode($1,'hex')::uuid,1,1,2,1)")
                    .bind(id(41).as_slice()).execute(&mut *tx).await?;
            }
            NESTED => {
                sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                    .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(PARCEL).as_slice()).bind(id(247).as_slice()).execute(&mut *tx).await?;
            }
            ON_TILE => {
                sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'\\x01'::bytea,1,encode($4,'hex')::uuid)")
                    .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(187).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
                    .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(id(187).as_slice()).execute(&mut *tx).await?;
            }
            IN_CORPSE => {
                sqlx::query("INSERT INTO game_item_corpse_container_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                    .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(CORPSE).as_slice()).bind(id(186).as_slice()).execute(&mut *tx).await?;
            }
            ON_GROUND => {
                sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                    .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(GROUND_BAG).as_slice()).bind(id(188).as_slice()).execute(&mut *tx).await?;
            }
            DEEP => {
                sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                    .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(DEEP_BAG).as_slice()).bind(id(200).as_slice()).execute(&mut *tx).await?;
            }
            DEEP_IN_CORPSE => {
                sqlx::query("INSERT INTO game_item_corpse_container_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                    .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(INNER_CORPSE).as_slice()).bind(id(202).as_slice()).execute(&mut *tx).await?;
            }
            WORN => {
                sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,6,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
                    .bind(id(41).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(248).as_slice()).execute(&mut *tx).await?;
            }
            _ => {
                sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::bigint,encode($6,'hex')::uuid)").bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(i64::from(seed - FRESH + 1)).bind(id(249 + (seed - FRESH)).as_slice()).execute(&mut *tx).await?;
            }
        }
        // The deadline as `schedule_delivered_scroll` writes it (migration 2762).
        sqlx::query("INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,content_digest) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3,floor(extract(epoch FROM clock_timestamp())*1000)::bigint+$4,'Item',$5,'definition-r1',sha256(convert_to($5||'|definition-r1','UTF8')))")
            .bind(id(seed).as_slice())
            .bind(id(DELIVERY).as_slice())
            .bind(lifetime)
            .bind(due_in_ms)
            .bind(SCROLL)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("INSERT INTO game_character_inbox_counters(character_id,next_ordinal,committed) SELECT character_id,max(ordinal)+1,count(*) FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid GROUP BY character_id ON CONFLICT(character_id) DO UPDATE SET next_ordinal=EXCLUDED.next_ordinal,committed=EXCLUDED.committed")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

fn instance_teardown() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("TEMPLE_SCROLL_INSTANCE POSTGRES_CASE_SKIPPED: no configured admin");
        return Ok(());
    };
    let runtime = crate::bestiary_postgres_harness::runtime()?;
    let harness = runtime.block_on(Harness::create_with_admission_seed(
        admin,
        "temple_scroll_instance",
        true,
        false,
    ))?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(instance_case(&harness))
    }));
    let cleanup = runtime.block_on(harness.cleanup());
    match result {
        Ok(result) => {
            result?;
            cleanup
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

/// Instance Ground has no deadline pass. A due scroll lying on the entered Instance's Ground and
/// one in a corpse there are left alone by the Channel pass, and leave with every other Instance
/// Ground Item when the completed Instance is torn down (activity retirement).
#[allow(
    clippy::too_many_lines,
    reason = "one Instance lifecycle from entry to teardown keeps the production setup in one place"
)]
async fn instance_case(h: &Harness) -> TestResult {
    use crate::durability::activity_handoff::{
        ActivityHandoffOwner, ActivityReturnOutcome, ActivityReturnReason, ActivityReturnRequest,
    };
    use crate::durability::item_decay_retire::ActivityGroundRetirement;
    use crate::durability::spell_owner_commit::SpellLane;
    use crate::foundation::{AuthenticatedTransportRefV1, GameSessionId};
    let entered = super::activity_runtime::enter_fixture(h).await?;
    let scope = entered.fence.runtime_scope;
    let instance = scope.instance_id().ok_or("Instance scope")?;
    let world = *scope.world_id().as_bytes();
    seed_instance_ground(
        h,
        world,
        instance,
        entered.fence.scope_ownership_generation.get(),
    )
    .await?;
    let pool = &h.pool;
    let items: Vec<Vec<u8>> = [INSTANCE_CORPSE, INSTANCE_IN_CORPSE, INSTANCE_GROUND]
        .iter()
        .map(|seed| id(*seed).to_vec())
        .collect();
    let state = || async {
        sqlx::query_as::<_, (i64, i64)>(
            "SELECT count(*) FILTER (WHERE i.lifecycle=1), count(*) FILTER (WHERE i.lifecycle=2 AND i.quantity=0) \
               FROM game_item_instances i JOIN unnest($1::bytea[]) b ON i.item_instance_id=encode(b,'hex')::uuid",
        )
        .bind(items.clone())
        .fetch_one(pool)
        .await
    };
    // The Channel pass of the same World holds no custody on Instance Ground.
    let retired: i32 = sqlx::query_scalar("SELECT game_temple_scroll_expire_due(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,500)")
        .bind(world.as_slice())
        .bind(id(43).as_slice())
        .fetch_one(pool)
        .await?;
    assert_eq!(retired, 0);
    assert_eq!(state().await?, (3, 0));

    // The participant returns; the completed Instance is torn down.
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let owner = ActivityHandoffOwner::new(h.root.clone());
    let placement = owner
        .read_return_placement(&authority, &h.node, entered.fence)
        .await?
        .ok_or("return placement")?;
    let request = ActivityReturnRequest {
        return_id: id(100),
        handoff_id: entered.receipt.handoff_id,
        command_id: None,
        source: entered.fence,
        destination_game_session_id: GameSessionId::decode(&id(101)).map_err(debug)?,
        destination_transport: Some(
            AuthenticatedTransportRefV1::decode(&[101; 16]).map_err(debug)?,
        ),
        destination_generation: placement.destination_generation,
        reason: ActivityReturnReason::IdleLogout,
        current_vitals: entered.request.participants[0].source_position.vitals,
        current_rest: entered.request.participants[0].source_position.rest,
    };
    match owner
        .return_participant(&authority, &h.node, request)
        .await?
    {
        ActivityReturnOutcome::Returned(_) => {}
        other => return Err(format!("Instance return refused: {other:?}").into()),
    }
    let proof = owner
        .retirement_ready(
            &authority,
            &h.node,
            entered.fence.runtime_scope,
            entered.fence.scope_ownership_generation,
        )
        .await?
        .ok_or("complete returned activity")?;
    let lane = SpellLane::for_scope(scope);
    let permit = lane
        .acquire()
        .await
        .map_err(|_| "Instance lane unresolved")?;
    let mut steps = 0;
    while let ActivityGroundRetirement::Progress(_) = h
        .root
        .retire_activity_ground_step(&permit, &authority, &h.node, &proof)
        .await
        .map_err(debug)?
    {
        steps += 1;
        assert!(steps <= 3, "bounded Instance teardown must advance");
    }
    assert_eq!(steps, 3);
    assert_eq!(state().await?, (0, 3));
    let left: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_item_ground_locations WHERE scope_kind=2 AND instance_id=encode($2,'hex')::uuid), \
                (SELECT count(*) FROM game_item_corpse_container_entries e JOIN unnest($1::bytea[]) b ON e.item_instance_id=encode(b,'hex')::uuid), \
                (SELECT count(*) FROM game_item_decay_retire_receipts r JOIN unnest($1::bytea[]) b ON r.item_instance_id=encode(b,'hex')::uuid WHERE r.cause_kind='ACTIVITY_RETIREMENT'), \
                (SELECT count(*) FROM game_spell_item_temporal_receipts r JOIN unnest($1::bytea[]) b ON r.item_instance_id=encode(b,'hex')::uuid)",
    )
    .bind(items.clone())
    .bind(instance.as_slice())
    .fetch_one(pool)
    .await?;
    assert_eq!(left, (0, 0, 3, 0));
    eprintln!("temple_scroll_instance: cases executed");
    Ok(())
}

/// A corpse holding a due scroll and a due scroll of its own on the entered Instance's Ground,
/// placed in the activity's ownership generation as an Instance drop is.
async fn seed_instance_ground(
    h: &Harness,
    world: [u8; 16],
    instance: [u8; 16],
    generation: u64,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key) in [
        (INSTANCE_CORPSE, "oteryn:item.tibia.i4240"),
        (INSTANCE_IN_CORPSE, SCROLL),
        (INSTANCE_GROUND, SCROLL),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,game_character_uuid_v7())")
            .bind(id(seed).as_slice())
            .bind(world.as_slice())
            .bind(key)
            .execute(&mut *tx)
            .await?;
    }
    for seed in [INSTANCE_CORPSE, INSTANCE_GROUND] {
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,scope_kind,instance_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,encode($3,'hex')::uuid,$4::text::numeric,$5,'\\x01'::bytea,'map-r1','content-1','\\x01'::bytea,$6)")
            .bind(id(seed).as_slice())
            .bind(world.as_slice())
            .bind(instance.as_slice())
            .bind(generation.to_string())
            .bind(vec![seed, 0, 7])
            .bind(9_170_000 + i64::from(seed))
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("INSERT INTO game_item_corpse_container_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
        .bind(id(INSTANCE_IN_CORPSE).as_slice())
        .bind(world.as_slice())
        .bind(id(INSTANCE_CORPSE).as_slice())
        .bind(id(197).as_slice())
        .execute(&mut *tx)
        .await?;
    for seed in [INSTANCE_IN_CORPSE, INSTANCE_GROUND] {
        sqlx::query("INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,content_digest) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3,floor(extract(epoch FROM clock_timestamp())*1000)::bigint-86400000,'Item',$4,'definition-r1',sha256(convert_to($4||'|definition-r1','UTF8')))")
            .bind(id(seed).as_slice())
            .bind(id(DELIVERY).as_slice())
            .bind(crate::durability::item_use::temple_scroll::TEMPLE_SCROLL_LIFETIME_MS)
            .bind(SCROLL)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
