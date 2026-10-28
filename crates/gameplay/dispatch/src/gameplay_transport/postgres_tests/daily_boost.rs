//! Durable daily choice through the admitted command route, including process-cache replacement.
use super::cyclopedia::support;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::daily_boost::*;

#[test]
fn daily_boost_postgres_real_dispatch_restart_and_epoch_fence() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "daily boost PostgreSQL scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "daily_boost_wire", true, false).await?;
        eprintln!(
            "POSTGRES_STARTED BOOSTED-1 daily_boost_postgres_real_dispatch_restart_and_epoch_fence"
        );
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
            super::death_ml_loss_composed::production_generation_with_pin(
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
        owner.movement_cells = &cells;
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
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
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &crate::gameplay_transport::daily_boost::offered(
                owner.offered_capabilities(),
                owner.daily_boost_available(),
            ),
            &[CAPABILITY_DAILY_BOOST_V1],
        )
        .ok_or("capability")?;
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
        let commands = std::mem::take(&mut owner.states.command_ingress);
        drop(owner);
        deps.runtime = tokio::sync::Mutex::new(make_runtime()?);
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = &cells;
        owner.active_generation = Some(generation);
        owner.states.command_ingress = commands;
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
            crate::gameplay_transport::guild_intent::GuildReply::Unavailable
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
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_daily_boosts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 2);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
