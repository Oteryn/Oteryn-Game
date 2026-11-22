//! Physical fault injection for the server-save cohort and Item owner.
use super::{Harness, TestResult, debug, id, planned_world_reset_fixtures::reset_fixture};
use crate::durability::DurabilityRoot;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_loop_move::{
    ITEM_LOOP_DROP_REF_PREFIX, ItemLoopContent, LoopDestination, LoopGround, LoopMoveRefusal,
    LoopMoveRequest,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    ItemDefinitionFacts, ItemStackClass, ItemWeightLookup, ItemWeightRefused,
};
use crate::durability::server_save_sweep::{
    GroundSweepBinding, GroundSweepFacts, GroundSweepItemFacts, GroundSweepPlacement,
    GroundSweepRoot, GroundSweepTileFacts, ServerSaveCleanup, ServerSaveOwner,
};
use crate::durability::spell_owner_commit::SpellLane;
use crate::foundation::{ChannelId, WorldId};
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration, GameSessionId};
use crate::gameplay_transport::item_fence_of;
use sqlx::{ConnectOptions, Connection, PgConnection, PgPool};

const EPOCH_MAP: &str = "save-test-map-r1";
const MAP: &str = "sha256:0707070707070707070707070707070707070707070707070707070707070707";
const CONTENT: &str = "sha256:0808080808080808080808080808080808080808080808080808080808080808";
const SERVER_DIGEST: [u8; 32] = [8; 32];
const FRAME: [u8; 32] = [9; 32];

#[test]
fn server_save_sweep_cohort_rollback_and_expiry_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::CohortRollback)
}

#[test]
fn server_save_sweep_restart_keeps_later_and_redropped_ground_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Restart)
}

#[test]
fn server_save_sweep_tree_rollback_keeps_custody_and_audit_atomic_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::TreeRollback)
}

#[test]
fn server_save_sweep_lost_ack_replays_original_receipt_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::LostAck)
}

#[test]
fn server_save_sweep_batches_resume_only_unsettled_ground_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Batches)
}

#[test]
fn server_save_sweep_pickup_owner_and_reset_fences_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Fences)
}

#[test]
fn server_save_sweep_due_publisher_and_disabled_cleanup_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Disabled)
}

#[derive(Clone, Copy, Debug)]
enum Case {
    CohortRollback,
    Restart,
    TreeRollback,
    LostAck,
    Batches,
    Fences,
    Disabled,
}

fn run(admin: String, case: Case) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(scenario(admin, case))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "server-save PostgreSQL scenario panicked")?
        .map_err(Into::into)
}

struct Facts(GroundSweepBinding);
impl GroundSweepFacts for Facts {
    fn binding(&self) -> &GroundSweepBinding {
        &self.0
    }

    fn tile(&self, placement: &GroundSweepPlacement) -> GroundSweepTileFacts {
        GroundSweepTileFacts {
            house: Some(false),
            protection_zone: Some(placement.position == position(103)),
        }
    }

    fn item(&self, _root: &GroundSweepRoot) -> GroundSweepItemFacts {
        GroundSweepItemFacts {
            loaded_from_map: Some(false),
            removable: Some(true),
            pickupable: Some(true),
            has_action_id: Some(false),
            has_unique_id: Some(false),
        }
    }
}

struct Scene<'a> {
    harness: &'a Harness,
    facts: Facts,
    lane: SpellLane,
}

impl<'a> Scene<'a> {
    async fn new(harness: &'a Harness) -> TestResult<Self> {
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let facts = Facts(GroundSweepBinding {
            world,
            channel,
            generation: 1,
            map_revision: MAP.into(),
            content_revision: CONTENT.into(),
            frame: FRAME.to_vec(),
        });
        let mut tx = harness.pool.begin().await?;
        sqlx::query("INSERT INTO game_content_activations
            (world_id,channel_id,activation_sequence,server_artifact_digest,client_artifact_digest,frame_binding_digest,issued_by,issued_at)
            VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3,$3,$4,session_user,
              floor(extract(epoch FROM statement_timestamp()))::bigint)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(SERVER_DIGEST.as_slice()).bind(FRAME.as_slice())
            .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_world_reset_epochs
            (world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,
             calendar,calendar_revision,activated_at,reset_month)
            SELECT encode($1,'hex')::uuid,0,$2,$3,$2,$2,$4,'TIBIA_SERVER_SAVE_BERLIN_06',1,
              t,to_char((t AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM')
            FROM (SELECT statement_timestamp()-interval '2 days' AS t) old_epoch")
            .bind(id(42).as_slice()).bind(SERVER_DIGEST.as_slice()).bind(EPOCH_MAP).bind(FRAME.as_slice())
            .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES (encode($1,'hex')::uuid,0)")
            .bind(id(42).as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Self {
            harness,
            facts,
            lane: SpellLane::new(world, channel),
        })
    }

    fn owner(&self) -> ServerSaveOwner<'_> {
        ServerSaveOwner {
            node: &self.harness.node,
            epoch: 0,
            epoch_map_revision: EPOCH_MAP,
            server_digest: SERVER_DIGEST,
        }
    }

    async fn poll(
        &self,
        cleanup: ServerSaveCleanup,
    ) -> TestResult<crate::durability::server_save_sweep::ServerSavePoll> {
        Ok(self
            .harness
            .root
            .publish_due_server_save(&self.facts.0, &self.owner(), cleanup)
            .await?)
    }

    async fn step(&self, root: &DurabilityRoot) -> TestResult {
        let permit = self
            .lane
            .acquire()
            .await
            .map_err(|_| "Channel lane unresolved")?;
        root.run_server_save_sweep(&self.owner(), &permit, &self.facts)
            .await?;
        Ok(())
    }

    async fn fresh_owner(&self) -> TestResult<DurabilityRoot> {
        let options = self.harness.pool.connect_options();
        let root = DurabilityRoot::connect_test_runtime(options.to_url_lossy().as_str())?;
        assert!(root.maintain_ready_once().await?);
        Ok(root)
    }

    async fn reopened(&self) -> TestResult<PgConnection> {
        // The admin fixture pool supplies only the local ephemeral database address.
        let options = self.harness.pool.connect_options();
        let connection = PgConnection::connect_with(&options).await?;
        Ok(connection)
    }

    async fn slot(&self, open: bool) -> TestResult<String> {
        // Controlled database clock boundary for the crash cases. The publisher's
        // real 06:00 and duplicate-poll behavior are qualified separately.
        Ok(sqlx::query_scalar("INSERT INTO game_server_save_occurrences
          (world_id,reset_epoch,scheduled_at,freeze_until,map_revision,server_digest,frame_digest,cleanup_enabled)
          SELECT world_id,reset_epoch,clock_timestamp()-interval '1 minute',
           clock_timestamp()+CASE WHEN $1 THEN interval '10 minutes' ELSE interval '-1 second' END,
           map_revision,server_digest,frame_digest,true FROM game_world_reset_epochs
          RETURNING occurrence_id::text")
            .bind(open).fetch_one(&self.harness.pool).await?)
    }

    async fn publish_reset(&self) -> TestResult {
        let ok: bool = sqlx::query_scalar(
            "SELECT game_world_reset_publish(
            encode($1,'hex')::uuid,0,$2,$3,$2,$2,$4,'[]',1)",
        )
        .bind(id(42).as_slice())
        .bind(SERVER_DIGEST.as_slice())
        .bind(EPOCH_MAP)
        .bind(FRAME.as_slice())
        .fetch_one(&self.harness.pool)
        .await?;
        assert!(ok);
        Ok(())
    }
}

async fn scenario(admin: String, case: Case) -> TestResult {
    let started = std::time::Instant::now();
    let harness =
        Harness::create_with_admission_seed(admin, "server_save_recovery", true, false).await?;
    let result = qualify(&harness, case).await;
    let cleanup = harness.cleanup().await;
    result.and(cleanup)?;
    eprintln!("SERVER SAVE SWEEP PG {case:?} {:?}", started.elapsed());
    Ok(())
}

async fn qualify(harness: &Harness, case: Case) -> TestResult {
    let mut scene = Scene::new(harness).await?;
    if matches!(case, Case::Restart) {
        super::prepare_current_session(harness).await?;
        super::item_look_fixtures::seed_main_backpack(harness, &[]).await?;
        sqlx::query(
            "INSERT INTO game_character_equipment_state(character_id,revision)
            VALUES(encode($1,'hex')::uuid,1)",
        )
        .bind(id(41).as_slice())
        .execute(&harness.pool)
        .await?;
    } else {
        reset_fixture::publish_ready(harness, 1).await?;
    }
    let root = &harness.root;
    let pool = &harness.pool;
    seed_ground(pool, 101, None).await?;
    seed_ground(pool, 103, None).await?;
    if matches!(case, Case::Disabled) {
        return disabled(&scene).await;
    }
    let occurrence = scene.slot(true).await?;

    if matches!(case, Case::CohortRollback) {
        fault(pool, Fault::Cohort, true).await?;
        assert!(scene.step(root).await.is_err());
        fault(pool, Fault::Cohort, false).await?;
        assert_eq!(
            readback(&scene, &occurrence).await?,
            [2, 0, 0, 0, 0, 0, 0, 0]
        );
        scene.step(root).await?;
        assert_eq!(
            readback(&scene, &occurrence).await?,
            [2, 0, 0, 0, 0, 1, 1, 0]
        );
        scene.step(root).await?;
        let expired = scene.slot(false).await?;
        seed_ground(pool, 104, None).await?;
        scene.step(root).await?;
        assert_eq!(readback(&scene, &expired).await?, [2, 0, 1, 1, 0, 0, 0, 1]);
        return Ok(());
    }

    if matches!(case, Case::TreeRollback) {
        seed_ground(pool, 102, Some(101)).await?;
        seed_ground(pool, 105, Some(102)).await?;
    }
    if matches!(case, Case::Batches | Case::Fences) {
        seed_ground(pool, 102, None).await?;
    }
    scene.step(root).await?;
    let fresh = scene.fresh_owner().await?;

    match case {
        Case::Restart => {
            seed_ground(pool, 104, None).await?;
            pickup_then_redrop(&scene, &fresh).await?;
            assert_eq!(
                readback(&scene, &occurrence).await?,
                [3, 0, 0, 0, 0, 0, 1, 0]
            );
            assert_eq!(disposition(&scene, &occurrence, 101).await?, "skipped");
        }
        Case::TreeRollback => {
            // By the root UPDATE, both children have been updated and unlinked.
            fault(pool, Fault::Root, true).await?;
            assert!(scene.step(root).await.is_err());
            fault(pool, Fault::Root, false).await?;
            assert_eq!(
                readback(&scene, &occurrence).await?,
                [2, 2, 0, 0, 0, 1, 1, 0]
            );
            assert_eq!(lifecycle(&scene, 105).await?, 1);
            fault(pool, Fault::Progress, true).await?;
            assert!(scene.step(root).await.is_err());
            fault(pool, Fault::Progress, false).await?;
            assert_eq!(
                readback(&scene, &occurrence).await?,
                [2, 2, 0, 0, 0, 1, 1, 0]
            );
            scene.step(&fresh).await?;
            assert_eq!(
                readback(&scene, &occurrence).await?,
                [1, 0, 1, 1, 1, 0, 1, 0]
            );
            for item in [101, 102, 105] {
                assert_eq!(lifecycle(&scene, item).await?, 2);
            }
        }
        Case::LostAck => {
            // Deliberately discard the worker result and reload using a new holder.
            let _lost = scene.step(root).await;
            let before = readback(&scene, &occurrence).await?;
            assert_eq!(before, [1, 0, 1, 1, 1, 0, 1, 0]);
            let prior = scene
                .poll(ServerSaveCleanup::default())
                .await?
                .latest_retirement;
            assert!(prior.is_some());
            scene.step(&fresh).await?;
            assert_eq!(
                scene
                    .poll(ServerSaveCleanup::default())
                    .await?
                    .latest_retirement,
                prior
            );
            assert_eq!(readback(&scene, &occurrence).await?, before);
            let view = fresh
                .read_item_loop_ground_for_scope(crate::foundation::RuntimeScopeRefV1::channel(
                    scene.facts.0.world,
                    scene.facts.0.channel,
                ))
                .await?;
            assert_eq!(
                view.iter()
                    .map(|row| row.item.item_instance_id)
                    .collect::<Vec<_>>(),
                [id(103)]
            );
        }
        Case::Batches => {
            scene.step(root).await?;
            assert_eq!(
                readback(&scene, &occurrence).await?,
                [2, 0, 1, 1, 1, 1, 1, 0]
            );
            seed_ground(pool, 104, None).await?;
            scene.step(&fresh).await?;
            scene.step(&fresh).await?;
            assert_eq!(
                readback(&scene, &occurrence).await?,
                [2, 0, 2, 2, 2, 0, 1, 0]
            );
        }
        Case::Fences => {
            reset_fixture::replace(
                harness,
                scene.facts.0.world,
                scene.facts.0.channel,
                &harness.node,
            )
            .await?;
            assert!(scene.step(root).await.is_err());
            assert_eq!(
                readback(&scene, &occurrence).await?,
                [3, 0, 0, 0, 0, 2, 1, 0]
            );
            scene.facts.0.generation = 2;
            assert!(
                scene.step(root).await.is_err(),
                "replacement is not ready yet"
            );
            reset_fixture::publish_ready(harness, 2).await?;
            scene.step(&fresh).await?;
            assert_eq!(
                readback(&scene, &occurrence).await?,
                [2, 0, 1, 1, 1, 1, 1, 0]
            );
            scene.publish_reset().await?;
            scene.step(&fresh).await?;
            assert_eq!(
                readback(&scene, &occurrence).await?,
                [2, 0, 1, 1, 1, 1, 1, 0]
            );
        }
        Case::CohortRollback | Case::Disabled => {
            return Err("early-return case reached retirement".into());
        }
    }
    assert_eq!(lifecycle(&scene, 103).await?, 1);
    Ok(())
}

async fn disabled(scene: &Scene<'_>) -> TestResult {
    let pool = &scene.harness.pool;
    assert!(
        scene
            .harness
            .root
            .planned_world_reset(scene.facts.0.world)
            .await?
            .is_some()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_server_save_occurrences")
            .fetch_one(pool)
            .await?,
        0
    );
    let disabled = ServerSaveCleanup::default();
    for _ in 0..2 {
        assert!(!scene.poll(disabled).await?.needs_pass);
    }
    let (occurrence, enabled, zones, hour): (String, bool, bool, String) =
        sqlx::query_as("SELECT occurrence_id::text,cleanup_enabled,clean_protection_zones,
          to_char(scheduled_at AT TIME ZONE 'Europe/Berlin','HH24:MI:SS') FROM game_server_save_occurrences")
        .fetch_one(pool).await?;
    assert_eq!((enabled, zones, hour.as_str()), (false, false, "06:00:00"));
    assert!(
        !scene
            .poll(ServerSaveCleanup {
                enabled: true,
                ..disabled
            })
            .await?
            .needs_pass,
        "the first published policy owns the slot"
    );
    scene.step(&scene.harness.root).await?;
    assert_eq!(
        readback(scene, &occurrence).await?,
        [2, 0, 0, 0, 0, 0, 0, 0]
    );
    Ok(())
}

fn position(item: u8) -> [u8; 10] {
    [0, 0, 0, item, 0, 0, 0, 42, 0, 7]
}

async fn seed_ground(pool: &PgPool, item: u8, parent: Option<u8>) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances
        (item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
        .bind(id(item).as_slice()).bind(id(42).as_slice())
        .bind(if item == 101 || item == 102 { "oteryn:item.tibia.i2854" } else { "oteryn:item.tibia.i3031" })
        .execute(&mut *tx).await?;
    if let Some(parent) = parent {
        sqlx::query(
            "INSERT INTO game_item_nested_entries VALUES (encode($1,'hex')::uuid,
          encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($1,'hex')::uuid)",
        )
        .bind(id(item).as_slice())
        .bind(id(42).as_slice())
        .bind(id(parent).as_slice())
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,
           runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,
           native_room_placement_context,stack_ordinal)
           VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$9,$5,$6,$7,$8)")
           .bind(id(item).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(position(item).as_slice())
           .bind(MAP).bind(CONTENT).bind(FRAME.as_slice()).bind(i64::from(item))
           .bind([ITEM_LOOP_DROP_REF_PREFIX,id(item).as_slice()].concat()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

#[derive(Clone, Copy)]
enum Fault {
    Cohort,
    Root,
    Progress,
}

async fn fault(pool: &PgPool, fault: Fault, install: bool) -> TestResult {
    if install {
        sqlx::query(
            "CREATE OR REPLACE FUNCTION reject_server_save() RETURNS trigger LANGUAGE plpgsql AS $$
          BEGIN
            IF TG_TABLE_NAME='game_item_instances' THEN
              IF NEW.lifecycle<>2 OR NOT EXISTS(SELECT 1 FROM game_item_ground_locations g
                 WHERE g.item_instance_id=OLD.item_instance_id) THEN RETURN NEW; END IF;
            END IF;
            RAISE EXCEPTION 'injected server-save crash' USING ERRCODE='23514';
          END $$",
        )
        .execute(pool)
        .await?;
    }
    let sql = match (fault, install) {
        (Fault::Cohort, true) => {
            "CREATE TRIGGER reject_server_save BEFORE INSERT ON game_server_save_ground
           FOR EACH ROW EXECUTE FUNCTION reject_server_save()"
        }
        (Fault::Root, true) => {
            "CREATE TRIGGER reject_server_save BEFORE UPDATE ON game_item_instances
           FOR EACH ROW EXECUTE FUNCTION reject_server_save()"
        }
        (Fault::Progress, true) => {
            "CREATE TRIGGER reject_server_save BEFORE UPDATE ON game_server_save_ground
           FOR EACH ROW WHEN (NEW.disposition='retired') EXECUTE FUNCTION reject_server_save()"
        }
        (Fault::Root, false) => "DROP TRIGGER reject_server_save ON game_item_instances",
        (Fault::Cohort | Fault::Progress, false) => {
            "DROP TRIGGER reject_server_save ON game_server_save_ground"
        }
    };
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn readback(scene: &Scene<'_>, occurrence: &str) -> TestResult<Vec<i64>> {
    let mut connection = scene.reopened().await?;
    Ok(sqlx::query_scalar("SELECT unnest(ARRAY[
      (SELECT count(*) FROM game_item_ground_locations),
      (SELECT count(*) FROM game_item_nested_entries),
      (SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=7),
      (SELECT count(*) FROM game_spell_item_audit_outbox a JOIN game_spell_item_receipts r USING(transaction_id) WHERE r.cause_kind=7),
      (SELECT count(*) FROM game_server_save_ground WHERE occurrence_id=$1::uuid AND disposition='retired'),
      (SELECT count(*) FROM game_server_save_ground WHERE occurrence_id=$1::uuid AND disposition='pending'),
      (SELECT count(*) FROM game_server_save_ground WHERE occurrence_id=$1::uuid AND disposition='preserved'),
      (SELECT count(*) FROM game_server_save_channels WHERE occurrence_id=$1::uuid AND state='missed')
    ])").bind(occurrence).fetch_all(&mut connection).await?)
}

async fn disposition(scene: &Scene<'_>, occurrence: &str, item: u8) -> TestResult<String> {
    let mut connection = scene.reopened().await?;
    Ok(sqlx::query_scalar("SELECT disposition FROM game_server_save_ground WHERE occurrence_id=$1::uuid AND item_id=encode($2,'hex')::uuid")
       .bind(occurrence).bind(id(item).as_slice()).fetch_one(&mut connection).await?)
}

async fn lifecycle(scene: &Scene<'_>, item: u8) -> TestResult<i16> {
    let mut connection = scene.reopened().await?;
    Ok(sqlx::query_scalar(
        "SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(item).as_slice())
    .fetch_one(&mut connection)
    .await?)
}

struct LoopContent;
impl ItemWeightLookup for LoopContent {
    fn generation_digest(&self) -> [u8; 32] {
        SERVER_DIGEST
    }

    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        self.loop_facts(definition)
            .and_then(|facts| facts.weight)
            .ok_or(ItemWeightRefused::Unknown)
    }
}

impl ItemLoopContent for LoopContent {
    fn loop_content_digest(&self) -> [u8; 32] {
        SERVER_DIGEST
    }

    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        if definition.family != "Item" || definition.production_key != "oteryn:item.tibia.i2854" {
            return None;
        }
        Some(ItemDefinitionFacts {
            definition: definition.clone(),
            weight: Some(1),
            stack: ItemStackClass::NonStackable,
            container_capacity: Some(20),
            container_slot_equip_pattern: true,
        })
    }
}

async fn pickup_then_redrop(scene: &Scene<'_>, replacement: &DurabilityRoot) -> TestResult {
    let h = scene.harness;
    let root = &h.root;
    let current = FreshAdmissionStore::from_root(root.clone())
        .current_session_at(GameSessionId::decode(&id(50)).map_err(debug)?)
        .await?
        .0;
    let fence = item_fence_of(current).ok_or("current Item fence")?;
    let stale = fence;
    // Install the successor connection in the fixture. Its command and the
    // predecessor's command both go through the real player Item writer.
    let successor: String = sqlx::query_scalar(
        "UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1,
           current_transport_ref=$2 WHERE game_session_id=encode($1,'hex')::uuid
           RETURNING current_generation::text",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind([15_u8; 16].as_slice())
    .fetch_one(&h.pool)
    .await?;
    let fence = crate::durability::item_transfer::CurrentCharacterItemFence {
        connection_generation: ConnectionGeneration::new(successor.parse()?).map_err(debug)?,
        ..fence
    };
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(debug)?;
    let permit = scene
        .lane
        .acquire()
        .await
        .map_err(|_| "Channel lane unresolved")?;
    let pick = LoopMoveRequest {
        item: id(101),
        count: None,
        pickup_cell: Some(position(101).to_vec()),
        destination: LoopDestination::MainBackpack,
    };
    let command = |id| -> TestResult<CommandRef> {
        Ok(CommandRef::new(
            fence.game_session_id,
            CommandId::new(id).map_err(debug)?,
        ))
    };
    let moves = async move {
        assert_eq!(
            root.commit_item_loop_move(
                &permit,
                &authority,
                &h.node,
                &stale,
                command(11)?,
                &LoopContent,
                pick.clone()
            )
            .await?,
            Err(LoopMoveRefusal::Rejected)
        );
        let picked = root
            .commit_item_loop_move(
                &permit,
                &authority,
                &h.node,
                &fence,
                command(11)?,
                &LoopContent,
                pick,
            )
            .await?
            .map_err(debug)?;
        assert_eq!(picked.operation, 5);
        let dropped = root
            .commit_item_loop_move(
                &permit,
                &authority,
                &h.node,
                &fence,
                command(12)?,
                &LoopContent,
                LoopMoveRequest {
                    item: id(101),
                    count: None,
                    pickup_cell: None,
                    destination: LoopDestination::Ground(LoopGround {
                        spatial_position: position(101).to_vec(),
                        map_revision: MAP.into(),
                        content_revision: CONTENT.into(),
                        placement_context: FRAME.to_vec(),
                    }),
                },
            )
            .await?
            .map_err(debug)?;
        assert_eq!(dropped.operation, 4);
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (moved, swept) = tokio::join!(moves, scene.step(replacement));
    moved?;
    swept
}
