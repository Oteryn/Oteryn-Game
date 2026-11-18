//! The shared native boss lever and timeout transfer both staged Characters.
use super::fixture::{self, Harness, debug, run};
use super::{Dependencies, TestResult};
use crate::content::NativeEntryStart;
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelContentPin, CharacterId, CharacterLease, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, FirstEntryOutcome,
};
use crate::gameplay_transport::item_fence_of;
use oteryn_protocol_oteryn::boss_encounter::{self as wire, BossActivityReply};

const BOSS: &str = "oteryn:boss.the_last_lore_keeper";
const LEVER: MovementLocalPosition = MovementLocalPosition {
    x: 32019,
    y: 32844,
    floor: 14,
};
const PEER: MovementLocalPosition = MovementLocalPosition {
    x: 32020,
    y: 32844,
    floor: 14,
};
const ENTRY: MovementLocalPosition = MovementLocalPosition {
    x: 31985,
    y: 32851,
    floor: 14,
};
const EXIT: MovementLocalPosition = MovementLocalPosition {
    x: 32035,
    y: 32859,
    floor: 14,
};

#[test]
fn fknow_lore_keeper_two_player_entry_and_exit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run("fknow-lore-keeper", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        super::reset();
        let expected = fixture::fence(1)?;
        let world = expected.runtime_scope.world_id();
        let channel = crate::foundation::ChannelId::decode(&fixture::id(43)).map_err(debug)?;
        let (controller, activation) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("active native content")?;
        let native = active.native_gameplay().ok_or("native policies")?;
        let parent = activation.qualified_room().clone();
        let activation_pin = activation.into_channel_pin();
        let bundle = super::srvenc_source_boss::source_bundle_for_layout(
            world,
            channel,
            native,
            [[31968, 32821, 14], [32004, 32865, 15]],
            &[[32019, 32844, 14], [32020, 32844, 14], [32035, 32859, 14]],
            [32019, 32844, 14],
        )?;
        let cells = bundle
            .movement_cells(parent.movement_cells())
            .map_err(debug)?;
        let room = parent
            .bundle_view(
                cells.clone(),
                bundle.frame_binding_digest(world),
                bundle.map_revision_digest(),
                NativeEntryStart {
                    x: LEVER.x,
                    y: LEVER.y,
                    floor: LEVER.floor,
                },
            )
            .ok_or("source bundle room")?;
        let template = active
            .qualify_lore_keeper_activity_template(&parent, &room, "content-1")
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            world,
            active.activation_sequence(),
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            (LEVER.x, LEVER.y, LEVER.floor),
        );
        let mut deps = Dependencies::with_content(
            pin.clone(),
            parent,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.combat_room = Some(room);
        let h =
            Harness::create_with_admission_seed(admin, "fknow_lore_keeper", true, false).await?;
        let evidence = super::house_transfer::RecoverySource::start(&h).await?;
        super::house_transfer::prepare_owned_session(&h).await?;
        h.seed_appearance_observer().await?;
        let peer_session = super::activity_deadline_paging::admit_created_member(
            &h,
            &evidence.evidence,
            1,
            fixture::id(101),
        )
        .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.evidence = &evidence.evidence;
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();

        let store = FreshAdmissionStore::from_root(h.root.clone());
        let mut leader = deps.admitted;
        let leader_current = store.current_session_at(leader.game_session_id).await?.0;
        let leader_fence = item_fence_of(leader_current).ok_or("leader fence")?;
        leader.item_fence = Some(leader_fence);
        leader.first_entry = FirstEntryOutcome::Positioned;
        leader.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[wire::CAPABILITY_BOSS_ENCOUNTER_V1],
        )
        .ok_or("boss capability")?;

        let peer_current = store.current_session_at(peer_session).await?.0;
        let peer_transport = peer_current.current_transport().ok_or("peer transport")?;
        let peer_fence = item_fence_of(peer_current).ok_or("peer fence")?;
        let peer_account: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(account_id) FROM game_durability_reconnect_sessions
             WHERE game_session_id=encode($1,'hex')::uuid",
        )
        .bind(peer_session.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        let peer_actor = {
            let mut runtime = owner.runtime.lock().await;
            let reserved = runtime.reserve_fresh_session(peer_session).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
            for (actor, fence, at) in [(deps.actor, leader_fence, LEVER), (actor, peer_fence, PEER)]
            {
                runtime
                    .bind_attacker_lease(
                        actor,
                        fence.game_session_id,
                        CharacterLease::new(
                            CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                            fence.character_lease_generation,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
                runtime
                    .initialize_pinned_test_position(actor, at)
                    .map_err(debug)?;
            }
            actor
        };
        let peer = AdmittedSession {
            game_session_id: peer_session,
            runtime_actor: Some(peer_actor),
            controller: Some(ControllerBinding {
                transport: peer_transport,
                account_id: peer_account.try_into().map_err(|_| "peer account UUID")?,
            }),
            item_fence: Some(peer_fence),
            ..leader
        };
        deps.initialize(&owner).await;
        deps.initialize_player(&owner, peer_actor, peer_session)
            .await;
        owner
            .register_current_runtime()
            .await
            .map_err(|_| "register source owner")?;
        for member in [leader, peer] {
            owner
                .activity_runtimes
                .remember(member)
                .map_err(|_| "remember source connection")?;
        }
        owner
            .activity_runtimes
            .register_template(&template)
            .map_err(|_| "register source template")?;
        super::boss_activity::issue_content_and_reset_with_runtime_pin(
            &h,
            &activation_pin,
            &pin,
            bundle.map_revision(),
        )
        .await?;
        ActivityHandoffOwner::new(h.root.clone())
            .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template))
            .await?;
        let instance = super::activity_runtime::allocate_ready_instance(&h, 88).await?;
        let clock: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        h.root.set_test_quest_clock(clock).map_err(debug)?;

        let (_, reply, barrier) =
            super::boss_activity::exchange(&owner, &mut leader, BOSS, true).await?;
        let BossActivityReply::Accepted {
            handoff,
            deadline_unix_ms: Some(_),
        } = reply
        else {
            return Err(format!("Lore Keeper party lever: {reply:?}").into());
        };
        assert!(barrier.is_some());
        let destination = owner
            .resolve_runtime_view(leader)
            .await
            .map_err(|_| "resolve Lore Keeper Instance")?
            .ok_or("private Instance")?;
        {
            let runtime = destination.runtime.lock().await;
            assert_eq!(runtime.binding().runtime_scope(), instance);
            let census = runtime.positioned_actor_census().map_err(debug)?;
            assert_eq!(
                census
                    .iter()
                    .filter(|(_, at, session)| session.is_some() && at.position() == ENTRY)
                    .count(),
                2
            );
            assert_eq!(
                census
                    .iter()
                    .filter(|(_, _, session)| session.is_none())
                    .count(),
                5
            );
        }
        let timers: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT a.deadline-a.opened_at,c.available_at-a.opened_at
             FROM game_boss_room_activities a JOIN game_boss_room_participants p USING(admission)
             JOIN game_character_boss_cooldowns c ON c.character_id=p.character_id AND c.boss_key=a.boss_key
             WHERE a.admission=encode($1,'hex')::uuid",
        ).bind(handoff.as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(timers, vec![(1_800_000, 1_209_600_000); 2]);

        h.root.advance_test_quest_clock(1801).map_err(debug)?;
        let (_, reply, barrier) = super::boss_activity::exchange_command(
            &owner,
            &mut leader,
            BOSS,
            true,
            wire::COMMAND_TYPE_BOSS_TIMEOUT,
        )
        .await?;
        assert!(
            matches!(reply, BossActivityReply::Accepted { .. }),
            "{reply:?}"
        );
        assert!(barrier.is_some());
        owner
            .activity_lifecycle_pass()
            .await
            .map_err(|_| "return due Lore Keeper party")?;
        let returned: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_boss_room_participants WHERE admission=encode($1,'hex')::uuid
             AND returned_at IS NOT NULL",
        ).bind(handoff.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(returned, 2);
        let runtime = owner.runtime.lock().await;
        let census = runtime.positioned_actor_census().map_err(debug)?;
        assert_eq!(
            census
                .iter()
                .filter(|(_, at, session)| session.is_some() && at.position() == EXIT)
                .count(),
            2
        );
        Ok(())
    })
}
