//! Durable daily choice through the admitted command route, including process-cache replacement.
use super::cyclopedia_fixtures::support;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::daily_boost::*;

async fn assert_parked_trade_query(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    expected: &[u8],
) -> TestResult {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
    use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
    use oteryn_protocol_oteryn::{
        ClientCommandValue, decode_command_result, encode_client_command,
    };
    use tokio::io::{AsyncWriteExt, DuplexStream};

    async fn query(
        stream: &mut DuplexStream,
        generation: u64,
        id: u64,
    ) -> TestResult<(CommandStatus, Vec<u8>)> {
        let frame = encode_client_command(
            generation,
            &ClientCommandValue {
                command_id: id,
                command_type: COMMAND_TYPE_DAILY_BOOST_QUERY,
                payload: &[],
            },
        )
        .map_err(debug)?;
        write_frame(stream, &frame).await?;
        loop {
            let frame = read_frame(stream).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(reply.command_id, id);
                return Ok((reply.status, reply.payload.to_vec()));
            }
        }
    }

    let (mut server, mut client) = tokio::io::duplex(4096);
    let before = *admitted;
    let serving = async move { serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await };
    let driving = async move {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        owner
            .states
            .test_daily_boost_parked_trade
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let generation = before.continuity.connection_generation;
        let rejected = query(&mut client, generation, 5).await;
        assert_eq!(
            rejected?,
            (
                CommandStatus::Rejected,
                DailyBoostRefusal::HolderBusy.encode().to_vec()
            )
        );
        super::maintain_authority_ready(owner).await;
        assert_eq!(
            query(&mut client, generation, 6).await?,
            (CommandStatus::Accepted, expected.to_vec())
        );
        client.shutdown().await?;
        TestResult::Ok(())
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    if let crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current)
    | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(current, _) = end
    {
        *admitted = current;
    }
    result
}

#[test]
fn daily_boost_postgres_real_dispatch_restart_and_epoch_fence() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(Case::Restart).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "daily boost PostgreSQL scenario panicked")??;
    Ok(())
}

#[test]
fn selected_world_login_probe_returns_boost_and_online_before_admission() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(Case::PreAdmission).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "world login probe PostgreSQL scenario panicked")?
        .map_err(Into::into)
}

async fn pre_admission_case(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    world: crate::foundation::WorldId,
    pool: &sqlx::PgPool,
) -> TestResult {
    use oteryn_protocol_oteryn::daily_boost::login;
    let request = login::encode_request(*world.as_bytes());
    // The World respawn owner commits this independently of player admission.
    owner
        .initialize_daily_boost()
        .await
        .map_err(|()| "boost init")?;
    let (mut stream, mut peer) = tokio::io::duplex(8192);
    assert!(crate::gameplay_transport::world_login_info::route(&mut stream, &request, owner).await);
    drop(stream);
    let reply = crate::gameplay_transport::tcp_tls::read_frame(&mut peer)
        .await
        .map_err(debug)?;
    let info = login::decode_reply(&reply).ok_or("world reply")?;
    assert_eq!(info.world_id, *world.as_bytes());
    let info = info.info.ok_or("available world")?;
    assert_eq!(info.players_online, 1);
    let first = info.boost.ok_or("boosted races")?;
    let choice = owner
        .runtime
        .lock()
        .await
        .daily_boost()
        .cloned()
        .ok_or("committed selection")?;
    assert_eq!(
        (first.creature.key.as_str(), first.boss.key.as_str()),
        (choice.creature.as_str(), choice.boss.as_str())
    );
    assert!(!first.creature.name.is_empty() && !first.boss.name.is_empty());
    assert!(first.effective_from < first.effective_until);

    // Keep the old committed epoch as the active pointer, with no retirement in
    // progress, after its Berlin boundary has elapsed.
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_world_reset_epochs SET activated_at=statement_timestamp()-interval '2 days'
                 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let (mut stream, mut peer) = tokio::io::duplex(8192);
    assert!(crate::gameplay_transport::world_login_info::route(&mut stream, &request, owner).await);
    drop(stream);
    let reply = crate::gameplay_transport::tcp_tls::read_frame(&mut peer)
        .await
        .map_err(debug)?;
    let info = login::decode_reply(&reply)
        .and_then(|reply| reply.info)
        .ok_or("available world")?;
    assert_eq!(info.players_online, 1);
    assert!(
        info.boost.is_none(),
        "an expired boost must be omitted before retirement starts"
    );

    let (mut stream, mut peer) = tokio::io::duplex(8192);
    let unknown = id(97);
    assert!(
        crate::gameplay_transport::world_login_info::route(
            &mut stream,
            &login::encode_request(unknown),
            owner
        )
        .await
    );
    drop(stream);
    let reply = crate::gameplay_transport::tcp_tls::read_frame(&mut peer)
        .await
        .map_err(debug)?;
    let reply = login::decode_reply(&reply).ok_or("wrong-world reply")?;
    assert_eq!(reply.world_id, unknown);
    assert_eq!(reply.info, None);
    Ok(())
}

#[test]
fn daily_boost_query_after_logout_and_relogin_under_holder_contention_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("daily-boost-relog", || {
        scenario(Case::Relog)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Case {
    Restart,
    PreAdmission,
    Relog,
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "daily_boost_wire", true, false).await?;
        eprintln!("POSTGRES_STARTED BOOSTED-1 daily_boost case={case:?}");
        prepare_current_session(&h).await?;
        // Verify the allocated migration can be reapplied without dropping choices.
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../apps/game-server/migrations/0333_daily_boost.sql"
            ))
            .to_owned(),
        ))
        .execute(&h.pool)
        .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activation_pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
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
        let make_runtime = || -> TestResult<ChannelRuntimeV1> {
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                4,
                pin.clone(),
            )
            .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            Ok(runtime)
        };
        let runtime = make_runtime()?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = (&cells).into();
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        if case == Case::PreAdmission {
            let result = pre_admission_case(&owner, deps.admitted.world_id, &h.pool).await;
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            return result;
        }
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert_eq!(
            support::exchange(
                &owner,
                &mut admitted,
                1,
                COMMAND_TYPE_DAILY_BOOST_QUERY,
                &[]
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        let supported = if case == Case::Relog {
            &[
                oteryn_protocol_oteryn::logout::CAPABILITY_LOGOUT_V1,
                CAPABILITY_DAILY_BOOST_V1,
            ][..]
        } else {
            &[CAPABILITY_DAILY_BOOST_V1][..]
        };
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::connection::fresh_capability_selection(&owner, supported)
                .await
                .ok_or("capability")?;
        for &capability in supported {
            assert!(
                admitted
                    .continuity
                    .selected_capabilities
                    .contains(capability)
            );
        }
        let (status, first) = support::exchange(
            &owner,
            &mut admitted,
            2,
            COMMAND_TYPE_DAILY_BOOST_QUERY,
            &[],
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        let choice = owner
            .runtime
            .lock()
            .await
            .daily_boost()
            .cloned()
            .ok_or("daily choice")?;
        assert_eq!(
            first,
            encode_view(&DailyBoostView {
                reset_epoch: 0,
                creature_key: choice.creature.clone(),
                boss_key: choice.boss.clone()
            })
            .map_err(debug)?
        );
        if case == Case::Relog {
            let result = logout_relogin_query(&h, &owner, admitted, first).await;
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            return result;
        }
        let native = generation.native_gameplay().ok_or("native content")?;
        let profile = |key: &str| {
            native
                .creature_profiles()
                .records
                .iter()
                .find(|r| r.profile.target.key == key)
                .ok_or("selected profile")
        };
        let crate::content::ProjectV2AuthoringProfileData::Creature(creature) =
            &profile(&choice.creature)?.profile.data
        else {
            return Err("creature profile".into());
        };
        assert!(creature.bestiary.is_some());
        let crate::content::ProjectV2AuthoringProfileData::Creature(boss) =
            &profile(&choice.boss)?.profile.data
        else {
            return Err("boss profile".into());
        };
        assert!(
            boss.bosstiary
                .as_ref()
                .map(|b| b.category.as_str())
                .or_else(|| boss
                    .details
                    .as_ref()?
                    .bosstiary
                    .as_ref()
                    .map(|b| b.category.as_str()))
                .is_some_and(|category| category.eq_ignore_ascii_case("archfoe"))
        );
        eprintln!(
            "BOOSTED_NATIVE_BOSS {} settlement={:?}",
            choice.boss,
            native.reward_table().row(&choice.boss).err()
        );
        assert!(
            !native
                .reward_table()
                .is_daily_boss_equipment("oteryn:item.tibia.i3031")
        );
        assert!(
            native
                .reward_table()
                .is_daily_boss_equipment("oteryn:item.tibia.i3051"),
            "native ring policy participates in boss equipment rolls"
        );
        // Replace all transport state and the runtime's cached selection, then load the same
        // durable row with deliberately different pools: committed choices are never redrawn.
        let commands = std::mem::take(
            &mut std::sync::Arc::get_mut(&mut owner.states)
                .expect("W02: isolated fixture state")
                .command_ingress,
        );
        drop(owner);
        deps.runtime = tokio::sync::Mutex::new(make_runtime()?);
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = (&cells).into();
        owner.active_generation = Some(generation);
        std::sync::Arc::get_mut(&mut owner.states)
            .expect("W02: isolated fixture state")
            .command_ingress = commands;
        deps.initialize(&owner).await;
        let (status, restarted) = support::exchange(
            &owner,
            &mut admitted,
            3,
            COMMAND_TYPE_DAILY_BOOST_QUERY,
            &[],
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(first, restarted);
        let permit = owner.spell_lane_permit().await.ok_or("permit")?;
        let scope =
            crate::foundation::RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id);
        let binding = || crate::durability::daily_boost::DailyBoostBinding {
            scope,
            generation: 1,
            bundle: world.base().digest(),
            epoch: 0,
        };
        let loaded = h
            .root
            .prepare_daily_boost(
                &authority,
                &h.node,
                &permit,
                binding(),
                (
                    vec!["different creature".into()],
                    vec!["different boss".into()],
                ),
            )
            .await
            .map_err(debug)?;
        assert_eq!(loaded, choice);
        let mut stale = binding();
        stale.epoch = 1;
        assert!(
            h.root
                .prepare_daily_boost(
                    &authority,
                    &h.node,
                    &permit,
                    stale,
                    (vec!["other".into()], vec!["other boss".into()])
                )
                .await
                .is_err()
        );
        drop(permit);
        assert_eq!(
            support::exchange(
                &owner,
                &mut admitted,
                4,
                COMMAND_TYPE_DAILY_BOOST_QUERY,
                &[0]
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        assert_parked_trade_query(&owner, &mut admitted, &first).await?;
        let mut reset_tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *reset_tx)
            .await?;
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) SELECT world_id,1,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,clock_timestamp(),reset_month FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0")
            .bind(id(42).as_slice()).execute(&mut *reset_tx).await?;
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=1 WHERE world_id=encode($1,'hex')::uuid").bind(id(42).as_slice()).execute(&mut *reset_tx).await?;
        reset_tx.commit().await?;
        assert!(matches!(
            owner
                .daily_boost_wire(admitted, 5, COMMAND_TYPE_DAILY_BOOST_QUERY, &[])
                .await,
            Err(DailyBoostRefusal::SelectionUnavailable)
        ));
        let permit = owner.spell_lane_permit().await.ok_or("new epoch permit")?;
        let mut next = binding();
        next.epoch = 1;
        let tomorrow = h
            .root
            .prepare_daily_boost(
                &authority,
                &h.node,
                &permit,
                next,
                (
                    vec![choice.creature.clone(), "other creature".into()],
                    vec![choice.boss.clone(), "other boss".into()],
                ),
            )
            .await
            .map_err(debug)?;
        assert_eq!(tomorrow.creature, "other creature");
        assert_eq!(tomorrow.boss, "other boss");
        drop(permit);
        // Multiple epochs can occur within a day: the older choice must remain excluded
        // even though it is no longer the immediately previous epoch.
        for (epoch, age, expected_creature, expected_boss) in [
            (2_u64, "29 days 23 hours", "fresh creature", "fresh boss"),
            (
                3,
                "30 days 1 hour",
                choice.creature.as_str(),
                choice.boss.as_str(),
            ),
        ] {
            sqlx::query("UPDATE game_world_daily_boosts SET selected_at=clock_timestamp()-$1::text::interval WHERE world_id=encode($2,'hex')::uuid AND reset_epoch=0")
                .bind(age).bind(id(42).as_slice()).execute(&h.pool).await?;
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) SELECT world_id,$2::text::numeric,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,clock_timestamp(),reset_month FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0")
                .bind(id(42).as_slice()).bind(epoch.to_string()).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$2::text::numeric WHERE world_id=encode($1,'hex')::uuid")
                .bind(id(42).as_slice()).bind(epoch.to_string()).execute(&mut *tx).await?;
            tx.commit().await?;
            let permit = owner.spell_lane_permit().await.ok_or("history permit")?;
            let mut next = binding();
            next.epoch = epoch;
            if epoch == 2 {
                // Exhaustion must not silently reintroduce a recently boosted candidate.
                for pools in [
                    (vec![choice.creature.clone()], vec!["fresh boss".into()]),
                    (vec!["fresh creature".into()], vec![choice.boss.clone()]),
                ] {
                    let mut exhausted = binding();
                    exhausted.epoch = epoch;
                    assert!(
                        h.root
                            .prepare_daily_boost(&authority, &h.node, &permit, exhausted, pools)
                            .await
                            .is_err()
                    );
                }
            }
            let pools = (
                vec![
                    choice.creature.clone(),
                    tomorrow.creature.clone(),
                    "fresh creature".into(),
                ],
                vec![
                    choice.boss.clone(),
                    tomorrow.boss.clone(),
                    "fresh boss".into(),
                ],
            );
            let selected = h
                .root
                .prepare_daily_boost(&authority, &h.node, &permit, next, pools)
                .await
                .map_err(debug)?;
            assert_eq!(
                selected.creature, expected_creature,
                "30-day creature exclusion at epoch {epoch}"
            );
            assert_eq!(
                selected.boss, expected_boss,
                "30-day boss exclusion at epoch {epoch}"
            );
            eprintln!("POSTGRES_HISTORY SV08 epoch={epoch} age={age} verified");
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_daily_boosts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 4);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn logout_relogin_query(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    expected: Vec<u8>,
) -> TestResult {
    use crate::durability::fresh_admission::FreshAdmissionStore;
    use crate::foundation::GameSessionState;
    use crate::gameplay_transport::composed_admission::{
        CONDITION_RELEASE_PROBE, HolderCheckoutProbe,
    };
    use crate::gameplay_transport::composed_admission::FIRST_ENTRY_CHECKOUT_PROBE;
    use crate::gameplay_transport::daily_boost::{
        DAILY_BOOST_CHECKOUT_PROBE, DAILY_BOOST_RUNTIME_PROBE,
    };
    use oteryn_protocol_oteryn::logout::{
        COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result,
    };
    use std::{cell::Cell, sync::Arc, time::Duration};

    let store = FreshAdmissionStore::from_root(h.root.clone());
    let previous = admitted.game_session_id;
    let logout_checkout = Arc::new(HolderCheckoutProbe::default());
    let next = admitted.continuity.next_command_id;
    let release_started = tokio::time::Instant::now();
    let (result, released_holder) = tokio::time::timeout(
        Duration::from_secs(30),
        super::fixture::join_two(
            CONDITION_RELEASE_PROBE.scope(
                Cell::new(Some(Arc::clone(&logout_checkout))),
                support::exchange(owner, &mut admitted, next, COMMAND_TYPE_LOGOUT_INTENT, &[]),
            ),
            logout_checkout.contend(&h.root),
        ),
    )
    .await?;
    released_holder?;
    let (status, payload) = result?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        decode_logout_result(&payload).map_err(debug)?,
        LogoutResult::Accepted
    );
    assert!(release_started.elapsed() < crate::durability::ROOT_RECOVERY_WINDOW);
    let admission_checkout = Arc::new(HolderCheckoutProbe::default());
    let (fresh, released_holder) = tokio::time::timeout(
        Duration::from_secs(30),
        super::fixture::join_two(
            FIRST_ENTRY_CHECKOUT_PROBE.scope(
                Cell::new(Some(Arc::clone(&admission_checkout))),
                super::full_world_support::readmit_after_logout(h, owner, admitted),
            ),
            admission_checkout.contend(&h.root),
        ),
    )
    .await?;
    released_holder?;
    let mut fresh = fresh?;
    assert_ne!(fresh.game_session_id, previous);
    assert_eq!(
        fresh.item_fence.ok_or("fresh fence")?.character_id,
        admitted.item_fence.ok_or("previous fence")?.character_id
    );
    assert_eq!(fresh.continuity.next_command_id, 1);
    let query_checkout = Arc::new(HolderCheckoutProbe::default());
    let query_started = Arc::new(tokio::sync::Notify::new());
    let runtime_held = Arc::new(tokio::sync::Notify::new());
    let contend = async {
        query_started.notified().await;
        let runtime = owner.runtime.lock().await;
        tokio::time::pause();
        runtime_held.notify_one();
        tokio::task::yield_now().await;
        tokio::time::advance(crate::gameplay_transport::COMMAND_HOLDER_WINDOW).await;
        tokio::time::resume();
        drop(runtime);
        query_checkout.contend(&h.root).await
    };
    let (result, released_holder) = tokio::time::timeout(
        Duration::from_secs(30),
        super::fixture::join_two(
            DAILY_BOOST_CHECKOUT_PROBE.scope(
                Cell::new(Some(Arc::clone(&query_checkout))),
                DAILY_BOOST_RUNTIME_PROBE.scope(
                    (Arc::clone(&query_started), Arc::clone(&runtime_held)),
                    support::exchange(owner, &mut fresh, 1, COMMAND_TYPE_DAILY_BOOST_QUERY, &[]),
                ),
            ),
            contend,
        ),
    )
    .await?;
    released_holder?;
    let (status, view) = result?;
    assert_eq!(
        status,
        CommandStatus::Accepted,
        "fresh command 1: {:?}",
        DailyBoostRefusal::decode(&view)
    );
    assert_eq!(view, expected);
    assert_eq!(
        store.current_session(previous).await?.session_state(),
        GameSessionState::Terminal
    );
    Ok(())
}
