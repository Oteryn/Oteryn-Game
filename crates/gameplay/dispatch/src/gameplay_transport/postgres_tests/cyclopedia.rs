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
    u64::from_le_bytes(bytes[11..19].try_into().expect("fixed result field"))
}
#[test]
fn cyclopedia_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Cyclopedia PostgreSQL scenario panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
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
        let world = support::world(&deps, generation)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        support::seed(&h, world.base().digest()).await?;
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
        owner.movement_cells = &cells;
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
        // Insufficient Premium is a typed refusal and writes no discovery state.
        owner.premium_coordinator = None;
        let (_, refused) = support::exchange(&owner, &mut admitted, 3, 73, &request(1, 0)).await?;
        assert_eq!(refused[0], 2);
        owner.premium_coordinator = Some(&premium);
        let before_start = admitted;
        let (_, started) = support::exchange(&owner, &mut admitted, 4, 73, &request(1, 0)).await?;
        assert_eq!(started[0], 0);
        assert_eq!(started[28], 0, "start excludes the game window");
        let progress: String =
            sqlx::query_scalar("SELECT progress::text FROM game_character_cyclopedia")
                .fetch_one(&h.pool)
                .await?;
        let progress: crate::domain::cyclopedia::MapProgress = serde_json::from_str(&progress)?;
        let first_points = progress.exploration.ok_or("durable exploration")?.points;
        assert_eq!(first_points.len(), 8);
        // Simulate process state replacement and a lost ACK. The Channel keeps the GameSession's
        // command stream (SESSION-INGRESS-1), which replays the retained result; the same
        // command still reaches PostgreSQL through the native receipt.
        let commands = std::mem::take(&mut owner.states.command_ingress);
        drop(owner);
        let mut owner = deps.owner(&h, &authority);
        owner.states.command_ingress = commands;
        owner.movement_cells = &cells;
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
        let (_, shuffled) = support::exchange(&owner, &mut admitted, 5, 73, &request(2, 0)).await?;
        assert_eq!(shuffled[0], 0);
        assert!(revision(&shuffled) > revision(&started));
        let progress: String =
            sqlx::query_scalar("SELECT progress::text FROM game_character_cyclopedia")
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
        }
        let (_, discovered) = support::exchange(&owner, &mut admitted, 13, 72, &[]).await?;
        assert_eq!(discovered[9], 1);
        assert_eq!(discovered[28], 0);
        assert_eq!(
            support::exchange(&owner, &mut admitted, 14, 73, &request(4, 0))
                .await?
                .0,
            CommandStatus::Rejected
        );
        // Epoch-zero draw is server-owned. This World has no random winner at epoch zero.
        assert_eq!(initial[27], 1, "fixture World must allow a donation");
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
        let commands = std::mem::take(&mut owner.states.command_ingress);
        drop(owner);
        let mut owner = deps.owner(&h, &authority);
        owner.states.command_ingress = commands;
        owner.movement_cells = &cells;
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
        sqlx::query("UPDATE game_world_cyclopedia SET pool=10000000")
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
        assert_eq!(epoch[27], 2);
        assert_eq!(pool(&epoch), 0);
        let (_, frozen_pool) =
            support::exchange(&owner, &mut admitted, 20, 73, &request(4, 1)).await?;
        assert_eq!(
            frozen_pool[0], 7,
            "current improved area refuses donation atomically"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT balance FROM game_account_bank_balances")
                .fetch_one(&h.pool)
                .await?,
            990
        );
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
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_world_cyclopedia_epochs WHERE donation_winner"
            )
            .fetch_one(&h.pool)
            .await?,
            1
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
