//! LOOP-MAP-1: admitted command bytes -> native fenced durable state -> result bytes.
use super::fixture::{self as harness_fixture, Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use crate::gameplay_transport::guild_intent::GuildReply;
use oteryn_protocol_oteryn::CommandStatus;
#[path = "cyclopedia/atlas.rs"]
mod atlas;
#[path = "cyclopedia/speed.rs"]
mod speed;
#[path = "cyclopedia/fixture.rs"]
pub(super) mod support;

fn request(operation: u8, amount: u32) -> Vec<u8> {
    let mut b = vec![operation];
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&amount.to_le_bytes());
    b
}
fn count(bytes: &[u8]) -> u8 {
    bytes[10]
}
fn revision(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes[1..9].try_into().expect("fixed result field"))
}
fn pool(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes[12..20].try_into().expect("fixed result field"))
}
fn percentage(bytes: &[u8]) -> u8 {
    bytes[11]
}
#[test]
fn cyclopedia_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Cyclopedia PostgreSQL scenario panicked")??;
    Ok(())
}
#[test]
fn cyclopedia_postgres_canonical_conversion_preserves_completed_legacy_progress() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "canonical Cyclopedia scenario panicked")??;
    Ok(())
}
fn scenario(canonical: bool) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "cyclopedia_wire", true, false).await?;
        eprintln!("POSTGRES_STARTED LOOP-MAP-1 cyclopedia_postgres_real_dispatch");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activation_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let world = support::world_with_subareas(&deps, generation, true)?;
        let cells = world
            .movement_cells(activation_pin.qualified_room().movement_cells())
            .map_err(debug)?;
        support::seed(&h, world.base().digest()).await?;
        // Map-only seed digests are replaced by the actual runtime's artifacts for STEP.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE game_world_reset_epochs SET server_digest=$1,client_digest=$2,frame_digest=$3",
        )
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(activation_pin.frame_binding().digest().as_slice())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        if canonical {
            // Retained pre-conversion World state, including an already selected active day.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_world_cyclopedia(world_id,bundle_digest,catalogue_digest,area_id,pool) VALUES(encode($1,'hex')::uuid,$2,$3,0,25000000)")
                .bind(id(42).as_slice()).bind(world.base().digest().as_slice())
                .bind(crate::domain::cyclopedia::BasicArea::content_digest().as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_world_cyclopedia_epochs(world_id,area_id,epoch,improved,donation_winner,pool_before) VALUES(encode($1,'hex')::uuid,0,0,true,false,25000000)")
                .bind(id(42).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }

        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activation_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (1, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        runtime
            .realize_activation_spawns(vec![(
                vec![(1, 1, 7)],
                60_000,
                5_000,
                "canary:creature/rat".into(),
                20,
            )])
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let premium =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = (&cells).into();
        owner.active_generation = Some(generation);
        owner.premium_coordinator = Some(&premium);
        deps.initialize(&owner).await;
        assert!(
            owner.cyclopedia_available(),
            "authored area must be admitted"
        );
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let offers = crate::gameplay_transport::cyclopedia::offered(
            owner.offered_capabilities(),
            owner.cyclopedia_available(),
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[4, 6, 57]).ok_or("capability selection")?;
        // Unnegotiated clients cannot enter the handler or observe private map facts.
        let mut old = admitted;
        old.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert_eq!(
            support::exchange(&owner, &mut old, 1, 72, &[]).await?.0,
            CommandStatus::Rejected
        );
        // Even a refused command advances this GameSession's command stream.
        let selected = admitted.continuity.selected_capabilities;
        admitted.continuity = old.continuity;
        admitted.continuity.selected_capabilities = selected;
        assert_eq!(admitted.continuity.next_command_id, 2);
        let (status, initial) = support::exchange(&owner, &mut admitted, 2, 72, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(initial[0], 0);
        assert_eq!(count(&initial), 0);
        assert_eq!(percentage(&initial), 0);
        let automatic: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT progress->'subareas'->'2'->'exploration' FROM game_character_cyclopedia",
        )
        .fetch_optional(&h.pool)
        .await?;
        assert!(
            automatic.is_some_and(|value| value.is_object()),
            "T186: query activates discovery without Start"
        );
        if canonical {
            eprintln!("POSTGRES_STARTED WORLD-SRC W12 canonical_conversion");
            assert_eq!(
                pool(&initial),
                25_000_000,
                "legacy pool imports without another reset debit"
            );
            assert_eq!(
                initial[28], 2,
                "an already active legacy day survives conversion"
            );

            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_character_cyclopedia(character_id,bundle_digest,catalogue_digest,progress,occurrence) VALUES(encode($1,'hex')::uuid,$2,$3,'{\"discovered\":true,\"exploration\":null}',encode($4,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET catalogue_digest=EXCLUDED.catalogue_digest,progress=EXCLUDED.progress,occurrence=EXCLUDED.occurrence")
                .bind(admitted.item_fence.ok_or("fence")?.character_id.as_bytes().as_slice()).bind(world.base().digest().as_slice())
                .bind(crate::domain::cyclopedia::BasicArea::V1_CONTENT_DIGEST.as_slice()).bind(id(99).as_slice())
                .execute(&mut *tx).await?;
            tx.commit().await?;
            let (_, credited) = support::exchange(&owner, &mut admitted, 3, 72, &[]).await?;
            assert_eq!(
                credited[9], 1,
                "canonical current subarea inherits legacy completion"
            );
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .actor_active_speed_delta(
                        deps.actor,
                        Some(admitted.game_session_id),
                        owner.owner_now(),
                    )
                    .map_err(debug)?,
                195,
                "T187: one fully discovered area contributes the content maximum"
            );
            let (_, donated) =
                support::exchange_inventory(&owner, &mut admitted, 4, 73, &request(4, 1)).await?;
            assert_eq!(donated[0], 0);
            let progress: serde_json::Value =
                sqlx::query_scalar("SELECT progress FROM game_character_cyclopedia")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                progress["legacy"]["discovered"], true,
                "legacy source remains retained"
            );
            assert_eq!(progress["subareas"]["2"]["discovered"], true);
            assert_eq!(
                progress["subareas"]["3"]["discovered"], true,
                "all canonical subareas retain old completion"
            );
            assert_eq!(pool(&donated), 25_000_001);
            let retained: i64 =
                sqlx::query_scalar("SELECT pool FROM game_world_cyclopedia WHERE area_id=0")
                    .fetch_one(&h.pool)
                    .await?;
            let spendable: i64 = sqlx::query_scalar(
                "SELECT sum(pool)::bigint FROM game_world_cyclopedia WHERE area_id>0",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(retained, 25_000_000, "legacy gold source is retained");
            assert_eq!(
                spendable, 25_000_001,
                "legacy gold becomes spendable exactly once"
            );

            for (proposed, reason) in [
                (
                    {
                        let mut p = progress.clone();
                        p["legacy"]["discovered"] = false.into();
                        p
                    },
                    "legacy discovery cannot regress",
                ),
                (
                    {
                        let mut p = progress.clone();
                        p["subareas"]["2"]["discovered"] = false.into();
                        p
                    },
                    "canonical discovery cannot regress",
                ),
                (
                    {
                        let mut p = progress.clone();
                        p["subareas"]
                            .as_object_mut()
                            .ok_or("subareas object")?
                            .remove("2");
                        p
                    },
                    "canonical discovery cannot regress",
                ),
            ] {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SAVEPOINT refusal").execute(&mut *tx).await?;
                sqlx::query("SET CONSTRAINTS game_cyclopedia_state_consistent IMMEDIATE")
                    .execute(&mut *tx)
                    .await?;
                let error = sqlx::query("UPDATE game_character_cyclopedia SET progress=$1")
                    .bind(proposed)
                    .execute(&mut *tx)
                    .await
                    .err()
                    .ok_or("monotonic guard accepted regression")?;
                let database = error.as_database_error().ok_or("database refusal")?;
                assert_eq!(database.code().as_deref(), Some("23514"));
                assert_eq!(database.message(), reason);
                sqlx::query("ROLLBACK TO SAVEPOINT refusal")
                    .execute(&mut *tx)
                    .await?;
                tx.rollback().await?;
            }
            let mut tx = h.pool.begin().await?;
            sqlx::query("SAVEPOINT duplicate_import")
                .execute(&mut *tx)
                .await?;
            let error=sqlx::query("INSERT INTO game_world_cyclopedia(world_id,bundle_digest,catalogue_digest,area_id,pool,legacy_imported) SELECT world_id,bundle_digest,catalogue_digest,24,25000000,true FROM game_world_cyclopedia WHERE area_id=1")
                .execute(&mut *tx).await.err().ok_or("a second area imported legacy gold")?;
            let database = error.as_database_error().ok_or("database import refusal")?;
            assert_eq!(database.code().as_deref(), Some("23505"));
            assert_eq!(
                database.constraint(),
                Some("game_cyclopedia_one_legacy_import")
            );
            sqlx::query("ROLLBACK TO SAVEPOINT duplicate_import")
                .execute(&mut *tx)
                .await?;
            tx.rollback().await?;
            let points: Vec<_> = (0..7)
                .map(|x| serde_json::json!({"x":x,"y":0,"floor":-7}))
                .collect();
            let changed_geometry = serde_json::json!({"legacy":{"discovered":false,"exploration":null},"subareas":{
                "2":{"discovered":false,"exploration":{"points":points,"found":1}},
                "3":{"discovered":true,"exploration":null}
            }});
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_character_cyclopedia SET bundle_digest=$1,progress=$2")
                .bind([9u8; 32].as_slice())
                .bind(changed_geometry)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            let (_, reset_points) = support::exchange(&owner, &mut admitted, 5, 72, &[]).await?;
            assert_eq!(
                reset_points[10], 0,
                "new geometry invalidates unfinished points"
            );
            assert_eq!(
                percentage(&reset_points),
                50,
                "completed canonical subareas survive changed geometry"
            );
            let (_, restarted) =
                support::exchange(&owner, &mut admitted, 6, 73, &request(1, 0)).await?;
            assert_eq!(
                restarted[0], 0,
                "a stale unfinished row can start discovery again"
            );
            {
                let mut runtime = owner.runtime.lock().await;
                let mut movement = runtime.borrow_movement_position();
                let mut current = movement.read(deps.actor).map_err(debug)?;
                let target = MovementLocalPosition {
                    x: 11,
                    y: 8,
                    floor: 7,
                };
                while current.position() != target {
                    let at = current.position();
                    let next = if at.x != target.x {
                        MovementLocalPosition {
                            x: at.x + (target.x - at.x).signum(),
                            ..at
                        }
                    } else {
                        MovementLocalPosition {
                            y: at.y + (target.y - at.y).signum(),
                            ..at
                        }
                    };
                    current = movement.commit_cardinal(current, next).map_err(debug)?;
                }
            }
            let (status, small) = support::exchange(&owner, &mut admitted, 7, 72, &[]).await?;
            assert_eq!(
                status,
                CommandStatus::Accepted,
                "a canonical subarea with exactly7candidates is admitted by the durable writer"
            );
            assert_eq!(
                small[9], 1,
                "seven-candidate subarea retains its completed state"
            );
            speed::verify(
                &mut owner,
                &deps,
                &mut admitted,
                &h,
                &authority,
                generation,
                activation_pin.frame_binding().digest(),
            )
            .await?;
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            eprintln!("POSTGRES_CASE_COMPLETED WORLD_SRC W12 canonical_conversion");
            return Ok(());
        }

        // Discovery and POI reset require no Premium entitlement.
        owner.premium_coordinator = None;
        let (_, refused) = support::exchange(&owner, &mut admitted, 3, 73, &request(1, 0)).await?;
        assert_eq!(refused[0], 0);
        owner.premium_coordinator = Some(&premium);
        let before_start = admitted;
        let (_, started) = support::exchange(&owner, &mut admitted, 4, 73, &request(1, 0)).await?;
        assert_eq!(started[0], 0);
        assert_eq!(started[29], 0, "start excludes the game window");
        let progress: String = sqlx::query_scalar(
            "SELECT (progress->'subareas'->'2')::text FROM game_character_cyclopedia",
        )
        .fetch_one(&h.pool)
        .await?;
        let progress: crate::domain::cyclopedia::MapProgress = serde_json::from_str(&progress)?;
        let first_points = progress.exploration.ok_or("durable exploration")?.points;
        assert_eq!(first_points.len(), 7);
        assert!(first_points.iter().enumerate().all(|(index, point)| {
            first_points[..index].iter().all(|other| {
                point.floor != other.floor
                    || point.x.abs_diff(other.x) >= 4
                    || point.y.abs_diff(other.y) >= 4
            })
        }));
        // Simulate process state replacement and a lost ACK. The Channel keeps the GameSession's
        // command stream (SESSION-INGRESS-1), which replays the retained result; the same
        // command still reaches PostgreSQL through the native receipt.
        let commands = std::mem::take(
            &mut std::sync::Arc::get_mut(&mut owner.states)
                .expect("W02: isolated fixture state")
                .command_ingress,
        );
        drop(owner);
        let mut owner = deps.owner(&h, &authority);
        std::sync::Arc::get_mut(&mut owner.states)
            .expect("W02: isolated fixture state")
            .command_ingress = commands;
        owner.movement_cells = (&cells).into();
        owner.active_generation = Some(generation);
        owner.premium_coordinator = Some(&premium);
        let (status, replay) =
            support::exchange(&owner, &mut admitted, 4, 73, &request(1, 0)).await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(started, replay);
        let GuildReply::Payload(replay) = owner
            .cyclopedia_command(before_start, 4, 73, &request(1, 0))
            .await
        else {
            return Err("start receipt".into());
        };
        assert_eq!(started, replay);
        // DISCOVERY3: a character row authored by the v1 catalogue remains readable.
        // The next mutating action rewrites its owner digest to the current v2 catalogue.
        // The state guard demands a same-transaction receipt, so the legacy row is staged with
        // triggers off, as a v1 owner commit would have left it.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_cyclopedia SET catalogue_digest=$1")
            .bind(crate::domain::cyclopedia::BasicArea::V1_CONTENT_DIGEST.as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        let (_, shuffled) = support::exchange(&owner, &mut admitted, 5, 73, &request(2, 0)).await?;
        assert_eq!(shuffled[0], 0);
        assert!(revision(&shuffled) > revision(&started));
        let catalogue_digest: Vec<u8> =
            sqlx::query_scalar("SELECT catalogue_digest FROM game_character_cyclopedia")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            catalogue_digest,
            crate::domain::cyclopedia::BasicArea::content_digest(),
            "a v1 row is upgraded by the v2 mutating owner"
        );
        let progress: String = sqlx::query_scalar(
            "SELECT (progress->'subareas'->'2')::text FROM game_character_cyclopedia",
        )
        .fetch_one(&h.pool)
        .await?;
        let progress: crate::domain::cyclopedia::MapProgress = serde_json::from_str(&progress)?;
        let points = progress.exploration.ok_or("shuffled exploration")?.points;
        assert_ne!(points, first_points);
        for (index, point) in points.iter().take(7).enumerate() {
            {
                let mut runtime = owner.runtime.lock().await;
                let mut movement = runtime.borrow_movement_position();
                let mut current = movement.read(deps.actor).map_err(debug)?;
                let target = MovementLocalPosition {
                    x: point.x,
                    y: point.y,
                    floor: -point.floor,
                };
                while current.position() != target {
                    let at = current.position();
                    let next = if at.x != target.x {
                        MovementLocalPosition {
                            x: at.x + (target.x - at.x).signum(),
                            ..at
                        }
                    } else {
                        MovementLocalPosition {
                            y: at.y + (target.y - at.y).signum(),
                            ..at
                        }
                    };
                    current = movement.commit_cardinal(current, next).map_err(debug)?;
                }
            }
            let (_, found) =
                support::exchange(&owner, &mut admitted, 6 + index as u64, 73, &request(3, 0))
                    .await?;
            assert_eq!(found[0], 0);
            assert_eq!(count(&found), index as u8 + 1);
            assert_eq!(found[9], u8::from(index == 6));
            assert_eq!(percentage(&found), if index == 6 { 50 } else { 0 });
        }
        let (_, discovered) = support::exchange(&owner, &mut admitted, 13, 72, &[]).await?;
        assert_eq!(discovered[9], 1);
        assert_eq!(percentage(&discovered), 50);
        assert_eq!(discovered[29], 0);
        assert_eq!(
            support::exchange(&owner, &mut admitted, 14, 73, &request(4, 0))
                .await?
                .0,
            CommandStatus::Rejected
        );
        // Epoch selection has no random winner at all.
        assert_eq!(initial[28], 1, "fixture World must allow a donation");
        let before_donation = admitted;
        let (_, donated) =
            support::exchange_inventory(&owner, &mut admitted, 15, 73, &request(4, 15)).await?;
        assert_eq!(donated[0], 0);
        assert_eq!(pool(&donated), 15);
        let balance: i64 = sqlx::query_scalar("SELECT balance FROM game_account_bank_balances")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(balance, 990, "five backpack coins, ten bank gold");
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=4"
            )
            .fetch_one(&h.pool)
            .await?,
            1
        );
        // Restart replay cannot charge again, even after the consumed coins disappeared.
        let commands = std::mem::take(
            &mut std::sync::Arc::get_mut(&mut owner.states)
                .expect("W02: isolated fixture state")
                .command_ingress,
        );
        drop(owner);
        let mut owner = deps.owner(&h, &authority);
        std::sync::Arc::get_mut(&mut owner.states)
            .expect("W02: isolated fixture state")
            .command_ingress = commands;
        owner.movement_cells = (&cells).into();
        owner.active_generation = Some(generation);
        owner.premium_coordinator = Some(&premium);
        assert_eq!(
            support::exchange(&owner, &mut admitted, 15, 73, &request(4, 15)).await?,
            (CommandStatus::DuplicateReplay, donated.clone())
        );
        let GuildReply::Payload(retry) = owner
            .cyclopedia_command(before_donation, 15, 73, &request(4, 15))
            .await
        else {
            return Err("donation receipt".into());
        };
        assert_eq!(retry, donated);
        let (_, reloaded) = support::exchange(&owner, &mut admitted, 16, 72, &[]).await?;
        assert_eq!(reloaded[9], 1);
        assert_eq!(pool(&reloaded), 15);
        let (_, poor) = support::exchange(&owner, &mut admitted, 17, 73, &request(4, 1000)).await?;
        assert_eq!(poor[0], 6);
        assert_eq!(pool(&poor), 15);
        let mut stale = admitted;
        let mut fence = stale.item_fence.ok_or("fence")?;
        fence.character_lease_generation += 1;
        stale.item_fence = Some(fence);
        assert!(matches!(
            owner
                .cyclopedia_command(stale, 18, 73, &request(4, 1))
                .await,
            GuildReply::Rejected
        ));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT balance FROM game_account_bank_balances")
                .fetch_one(&h.pool)
                .await?,
            990
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_account_bank_entries WHERE kind=5"
            )
            .fetch_one(&h.pool)
            .await?,
            1
        );
        // Explicit already-activated next-epoch snapshot, as in the reset/map fixtures.
        // This tests the map consumer and restart selection; reset publication has its own lane.
        let mut snapshot = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *snapshot)
            .await?;
        sqlx::query("UPDATE game_world_cyclopedia SET pool=25000000 WHERE area_id=1")
            .execute(&mut *snapshot)
            .await?;
        snapshot.commit().await?;
        sqlx::query("INSERT INTO game_world_reset_epochs SELECT world_id,1,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month FROM game_world_reset_epochs WHERE reset_epoch=0").execute(&h.pool).await?;
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=1")
            .execute(&h.pool)
            .await?;
        assert_eq!(
            support::exchange(&owner, &mut admitted, 18, 72, &[])
                .await?
                .0,
            CommandStatus::Rejected,
            "old boot cannot cross the active epoch"
        );
        owner.reset_epoch = 1;
        let (_, epoch) = support::exchange(&owner, &mut admitted, 19, 72, &[]).await?;
        assert_eq!(epoch[28], 2);
        assert_eq!(pool(&epoch), 15_000_000, "donation excess survives reset");
        // PAR-W12: Crystal `discovery_system.lua:191` DONATION_GOAL = 10,000,000 (MATCH, `>=`).
        let (winner, before): (bool, i64) = sqlx::query_as(
            "SELECT donation_winner,pool_before FROM game_world_cyclopedia_epochs WHERE epoch=1",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!((winner, before), (true, 25_000_000));
        let (_, frozen_pool) =
            support::exchange(&owner, &mut admitted, 20, 73, &request(4, 1)).await?;
        assert_eq!(
            frozen_pool[0], 0,
            "active boosts still accept next-cycle donations"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT balance FROM game_account_bank_balances")
                .fetch_one(&h.pool)
                .await?,
            989
        );
        assert_eq!(pool(&frozen_pool), 15_000_001);
        let (receipt_before,receipt_after):(i64,i64)=sqlx::query_as("SELECT pool_before,pool_after FROM game_character_cyclopedia_receipts WHERE donation=1").fetch_one(&h.pool).await?;
        assert_eq!((receipt_before, receipt_after), (15_000_000, 15_000_001));
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_account_bank_entries WHERE kind=5"
            )
            .fetch_one(&h.pool)
            .await?,
            2
        );

        sqlx::query("INSERT INTO game_world_reset_epochs SELECT world_id,2,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month FROM game_world_reset_epochs WHERE reset_epoch=1").execute(&h.pool).await?;
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=2")
            .execute(&h.pool)
            .await?;
        owner.reset_epoch = 2;
        let (_, epoch_two) = support::exchange(&owner, &mut admitted, 21, 72, &[]).await?;
        assert_eq!(epoch_two[28], 2);
        assert_eq!(pool(&epoch_two), 5_000_001);
        let (improved, winner, before): (bool, bool, i64) = sqlx::query_as(
            "SELECT improved,donation_winner,pool_before FROM game_world_cyclopedia_epochs WHERE epoch=2",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!((improved, winner, before), (true, true, 15_000_001));
        {
            let mut runtime = owner.runtime.lock().await;
            let (point, _, live) = runtime.spawn_point_creatures()[0];
            assert!(runtime.kill_creature_for_test(live.ok_or("home rat")?));
            runtime
                .drive_spawn_respawns(1000, |_, _| false)
                .map_err(debug)?;
            assert_eq!(
                runtime.pending_respawn(point).map(|p| p.due_micros),
                Some(30_001_000),
                "native 60-second home delay gets the persisted factor two"
            );
        }
        sqlx::query("INSERT INTO game_world_reset_epochs SELECT world_id,3,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month FROM game_world_reset_epochs WHERE reset_epoch=2").execute(&h.pool).await?;
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=3")
            .execute(&h.pool)
            .await?;
        owner.reset_epoch = 3;
        let (_, epoch_three) = support::exchange(&owner, &mut admitted, 22, 72, &[]).await?;
        assert_eq!(epoch_three[28], 1);
        assert_eq!(pool(&epoch_three), 5_000_001);
        let (improved, winner): (bool, bool) = sqlx::query_as(
            "SELECT improved,donation_winner FROM game_world_cyclopedia_epochs WHERE epoch=3",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!((improved, winner), (false, false));
        let reset_id = admitted.continuity.next_command_id;
        let (_, reset_pois) =
            support::exchange(&owner, &mut admitted, reset_id, 73, &request(2, 0)).await?;
        assert_eq!(
            reset_pois[0], 0,
            "T187: completed subarea POIs can be reset"
        );
        assert_eq!(reset_pois[9], 0, "the reset starts a fresh POI run");
        assert_eq!(count(&reset_pois), 0);
        assert_eq!(percentage(&reset_pois), 0);
        let reset_progress: serde_json::Value =
            sqlx::query_scalar("SELECT progress->'subareas'->'2' FROM game_character_cyclopedia")
                .fetch_one(&h.pool)
                .await?;
        let reset_progress: crate::domain::cyclopedia::MapProgress =
            serde_json::from_value(reset_progress)?;
        assert!(
            !reset_progress.discovered,
            "T187: reset removes completion and speed eligibility"
        );
        let points = reset_progress
            .exploration
            .ok_or("reset exploration")?
            .points;
        // Movement consumes the admitted equipment owner, unlike map-only requests.
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activation_pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &admitted.item_fence.ok_or("equipment fence")?,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        for (index, point) in points.iter().enumerate() {
            let from_left = point.x > 0
                && world
                    .base()
                    .tile((point.x - 1) as u16, point.y as u16, point.floor as i8)
                    .is_some_and(|tile| tile.walkable());
            let direction = if from_left {
                oteryn_protocol_oteryn::world_spatial::StepDirection::East
            } else {
                oteryn_protocol_oteryn::world_spatial::StepDirection::West
            };
            {
                let mut runtime = owner.runtime.lock().await;
                let mut movement = runtime.borrow_movement_position();
                let mut current = movement.read(deps.actor).map_err(debug)?;
                let target = MovementLocalPosition {
                    x: point.x + if from_left { -1 } else { 1 },
                    y: point.y,
                    floor: -point.floor,
                };
                while current.position() != target {
                    let at = current.position();
                    let next = if at.x != target.x {
                        MovementLocalPosition {
                            x: at.x + (target.x - at.x).signum(),
                            ..at
                        }
                    } else {
                        MovementLocalPosition {
                            y: at.y + (target.y - at.y).signum(),
                            ..at
                        }
                    };
                    current = movement.commit_cardinal(current, next).map_err(debug)?;
                }
            }
            owner.clock_origin -= std::time::Duration::from_secs(2);
            // Each exchange closes its socket; the next admission restores movement state.
            deps.initialize(&owner).await;
            super::maintain_ready(&h.root).await?;
            let step_id = admitted.continuity.next_command_id;
            let (status, step) = support::exchange(
                &owner,
                &mut admitted,
                step_id,
                1,
                &oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction),
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                step,
                oteryn_protocol_oteryn::world_spatial::encode_step_result(
                    oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
                )
            );
            let found: Option<i32> = sqlx::query_scalar(
                "SELECT (progress->'subareas'->'2'->'exploration'->>'found')::integer FROM game_character_cyclopedia",
            ).fetch_one(&h.pool).await?;
            if index == 6 {
                assert_eq!(
                    found, None,
                    "T186: real STEP completes exploration without Find"
                );
            } else {
                assert_eq!(
                    found.map(|bits| bits & (1 << index)),
                    Some(1 << index),
                    "T186: real STEP discovers a POI without Find"
                );
            }
        }
        assert_eq!(
            owner
                .states
                .tutorial_sessions
                .take(admitted.game_session_id),
            Some(
                oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(
                    "Subarea discovered! All seven points of interest found.".into(),
                )
            ),
            "T187: the seventh real STEP queues the completion banner"
        );
        atlas::verify(&owner, &mut admitted).await?;
        // A stale socket reserves its command before the Bank prelude refuses it, so its
        // unresolved original ends this GameSession's command stream (SESSION-INGRESS-1):
        // exercise it last.
        let mut stale = admitted;
        let mut fence = stale.item_fence.ok_or("fence")?;
        fence.character_lease_generation += 1;
        stale.item_fence = Some(fence);
        let stale_id = stale.continuity.next_command_id;
        assert!(
            support::exchange(&owner, &mut stale, stale_id, 73, &request(4, 1))
                .await
                .is_err(),
            "stale session cannot even construct its admitted snapshot"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT balance FROM game_account_bank_balances")
                .fetch_one(&h.pool)
                .await?,
            989
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_account_bank_entries WHERE kind=5"
            )
            .fetch_one(&h.pool)
            .await?,
            2
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_world_cyclopedia_epochs WHERE donation_winner"
            )
            .fetch_one(&h.pool)
            .await?,
            2
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!("POSTGRES_CASE_COMPLETED WORLD_SRC W12 donations_and_dispatch");
        Ok(())
    })
}
