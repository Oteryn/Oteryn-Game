//! The real server-save writer consumes only the retiring epoch's contributions.
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, planned_world_reset::reset_fixture};
use crate::durability::world_event::changes;
use crate::foundation::{ChannelId, WorldId};

#[test]
fn world_change_contributions_project_only_at_committed_server_save() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("world-change-contributions", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "world_change_contributions", true, false)
                .await?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let (_, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
            world,
            Some(include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        );
        let room = pin.qualified_room();
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
        assert!(
            h.root
                .record_content_activation(
                    &crate::durability::content_activation::ContentActivationRequest {
                        world_id: world,
                        channel_id: channel,
                        activation_sequence: 1,
                        previous_sequence: None,
                        server_artifact_digest: room.compiled().server_digest(),
                        client_artifact_digest: room.compiled().client_digest(),
                        frame_binding_digest: room.frame_binding().digest(),
                    },
                )
                .await?
        );
        assert!(sqlx::query_scalar::<_, bool>("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
            .bind(world.as_bytes().as_slice()).bind(room.compiled().server_digest().as_slice())
            .bind(room.movement_cells().scope().map_revision.as_str()).bind(room.compiled().client_digest().as_slice())
            .bind(room.frame_binding().digest().as_slice()).fetch_one(&h.pool).await?);
        reset_fixture::publish_ready(&h, 1).await?;
        let before = h.root.world_changes(world, 0).await?.ok_or("epoch zero")?;
        assert!(!before.awash && !before.steamship_open && !before.swamp_epidemic);
        sqlx::query("INSERT INTO game_world_change_tallies(world_id,reset_epoch,resource,tally) VALUES(encode($1,'hex')::uuid,0,'coal',202),(encode($1,'hex')::uuid,0,'steamship_coal',199),(encode($1,'hex')::uuid,0,'medicine',100)")
            .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
        assert_eq!(h.root.world_changes(world, 0).await?, Some(before));

        assert!(sqlx::query_scalar::<_, bool>("SELECT game_world_reset_publish(encode($1,'hex')::uuid,0,$2,$3,$2,$4,$5,'[]'::jsonb,1)")
            .bind(world.as_bytes().as_slice()).bind(room.compiled().server_digest().as_slice())
            .bind(room.movement_cells().scope().map_revision.as_str()).bind(room.compiled().client_digest().as_slice())
            .bind(room.frame_binding().digest().as_slice()).fetch_one(&h.pool).await?);
        let retiring = h
            .root
            .retiring_world_reset(world)
            .await?
            .ok_or("retiring reset")?;
        reset_fixture::replace(&h, world, channel, &h.node).await?;
        for _ in 0..2 {
            assert_eq!(
                h.root
                    .finish_world_reset(&authority, &h.node, &retiring, room, room)
                    .await?,
                Some(1)
            );
            let selected = h.root.world_changes(world, 1).await?.ok_or("saved day")?;
            assert!(selected.awash);
            assert!(!selected.steamship_open && !selected.swamp_epidemic);
        }
        let mut tx = h.pool.begin().await?;
        let today = changes::tally(
            &mut tx,
            world,
            1,
            oteryn_gameplay_world_change::owner::ContributionKind::Coal,
        )
        .await?;
        assert_eq!(today, 0);
        tx.rollback().await?;
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn junkar_coal_donation_debits_once_and_stays_separate_from_pump_coal() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("junkar-coal-donation", junkar_scenario)
}

fn junkar_scenario() -> TestResult {
    use super::{Dependencies, prepare_current_session};
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use oteryn_gameplay_world_change::owner::{COAL_ITEM, ContributionKind};
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "junkar_coal", true, false).await?;
        prepare_current_session(&h).await?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let room = pin.qualified_room().clone();
        let (content_pin, ..) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            content_pin.clone(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        deps.initialize(&owner).await;
        let actor = deps.actor;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        super::boss_activity::issue_content_and_reset(&h, &content_pin).await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        let epoch = owner.reset_epoch;
        assert_eq!(epoch, 0);
        let [npc] =
            super::classic_quests::install(&owner, &deps.admitted, ["oteryn:npc.junkar_thais"])
                .await?;
        super::item_look::seed_main_backpack(&h, &[(COAL_ITEM, 1)]).await?;
        let session = deps.admitted.game_session_id;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?
            .item_fence();
        for (ordinal, text) in [(1, "hi"), (2, "donate coal"), (2, "donate coal")] {
            let result = owner
                .npc_talk(
                    actor,
                    UseCommand {
                        game_session_id: session,
                        command_id: ordinal,
                        item_fence: Some(fence),
                    },
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert_eq!(result.disposition, NpcIntentDisposition::Ok, "{text}");
        }
        assert_eq!(
            super::qengine_voucher_stories::quantity(&h, 12600).await?,
            0
        );
        let mut tx = h.pool.begin().await?;
        assert_eq!(
            changes::tally(&mut tx, world, epoch, ContributionKind::SteamshipCoal).await?,
            1
        );
        assert_eq!(
            changes::tally(&mut tx, world, epoch, ContributionKind::Coal).await?,
            0
        );
        tx.rollback().await?;
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
