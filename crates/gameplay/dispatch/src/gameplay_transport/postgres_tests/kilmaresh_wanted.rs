//! Wanted portal STEP, solo shared room admission, cooldown and native exit.
use super::fixture::{Harness, debug, run};
use super::TestResult;
use crate::content::bosses::activity_template::{captured_boss_levers, wanted};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::{SeamRooms, connection::FreshAdmissionAuthority};
use oteryn_protocol_oteryn::{boss_encounter as boss, world_spatial as movement};

#[test]
fn kilmaresh_wanted_step_uses_solo_room_cooldown_and_exit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| exercise().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Kilmaresh Wanted case panicked")??;
    Ok(())
}

fn exercise() -> TestResult {
    run(async |admin| {
        let start = MovementLocalPosition { x: 33886, y: 31478, floor: 6 };
        let scene = super::srx3_soul_war_world_fixture::source_world(start)?;
        let active = scene.controller.active().ok_or("active World")?;
        let native = active.native_gameplay().ok_or("native World")?;
        let candidate = captured_boss_levers().map_err(debug)?.into_iter()
            .find(|row| row.path == wanted::PATH && row.line == 2).ok_or("Neferi source capture")?;
        let template = scene.controller.qualify_activity_template(
            &scene.room, &scene.room,
            native.training_formula().ok_or("active interpretation")?.content_revision(),
            &candidate,
        ).map_err(debug)?;
        let deps = scene.dependencies()?;
        let h = Harness::create_with_admission_seed(admin, "kilmaresh_wanted", false, false).await?;
        let evidence = super::house_transfer::RecoverySource::start(&h).await?;
        super::house_transfer::prepare_owned_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.evidence = &evidence.evidence;
        owner.active_generation = Some(active);
        owner.rooms = SeamRooms::new(None, Some(&scene.room));
        owner.activity_runtimes.register_template(&template).map_err(|_| "register Wanted template")?;
        super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
        crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone())
            .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template)).await?;
        let mut admitted = deps.admitted;
        let supported = [4, 6, 7, 10, 11, 15, 17, 18, 23, 30, 92, 117, 385];
        let (offers, scope, generation) = {
            let runtime = owner.runtime.lock().await;
            (crate::gameplay_transport::world_required_capabilities::with_native_map_presentation_offer(
                crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                Some(active), Some(&scene.room), runtime.content_pin()),
                runtime.binding().runtime_scope(), runtime.binding().scope_generation().get())
        };
        owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &[11, 18, 30], scope, generation, "policy-1", offers,
        ).map_err(debug)?;
        admitted.continuity.selected_capabilities = crate::gameplay_transport::connection::fresh_capability_selection(&owner, &supported)
            .await.ok_or("Wanted capability negotiation")?;
        assert!(admitted.continuity.selected_capabilities.contains(boss::CAPABILITY_BOSS_ENCOUNTER_V1));
        super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 250).await?;
        let peer = super::m25_ahau_push::prepare_peer_and_party(&h, &owner, &mut admitted).await?;
        let instance = super::activity_runtime::allocate_ready_instance(&h, 237).await?;
        let actor = admitted.runtime_actor.ok_or("Wanted player")?;
        let (step, _) = owner.paced_step(actor, admitted.game_session_id, movement::StepDirection::North).await;
        assert_eq!(step.disposition, movement::StepDisposition::Moved);
        owner.committed_paced_movement(admitted, step.moved_to).await;

        let entered = owner.activity_runtimes.current_connection(admitted);
        let fence = entered.item_fence.ok_or("Wanted instance item fence")?;
        assert_eq!(fence.runtime_scope, instance);
        let (lifetime, cooldown, participants): (i64, i64, i64) = sqlx::query_as(
            "SELECT a.deadline-a.opened_at,c.available_at-a.opened_at,
             (SELECT count(*) FROM game_boss_room_participants p WHERE p.admission=a.admission)
             FROM game_boss_room_activities a JOIN game_character_boss_cooldowns c USING (boss_key)
             WHERE a.boss_key='oteryn:boss.neferi_the_spy'",
        ).fetch_one(&h.pool).await?;
        assert_eq!((lifetime, cooldown, participants), (600_000, 36_000_000, 1));

        assert_eq!(owner.activity_runtimes.current_connection(peer).game_session_id, peer.game_session_id);
        assert_eq!(owner.activity_runtimes.current_connection(peer).item_fence.ok_or("peer fence")?.runtime_scope, scope);
        let registered = owner.activity_runtimes.get(instance, fence.scope_ownership_generation.get()).ok_or("Wanted instance")?;
        let view = owner.runtime_view(&registered);
        let actor = entered.runtime_actor.ok_or("Wanted instance player")?;
        let (step, _) = view.paced_step(actor, entered.game_session_id, movement::StepDirection::North).await;
        assert_eq!(step.disposition, movement::StepDisposition::Moved);
        view.committed_paced_movement(entered, step.moved_to).await;
        let returned = owner.activity_runtimes.current_connection(entered);
        let fence = returned.item_fence.ok_or("Wanted return item fence")?;
        assert_eq!(fence.runtime_scope, scope);
        let actor = returned.runtime_actor.ok_or("Wanted returned player")?;
        assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), start);
        assert_eq!(owner.activity_runtimes.current_connection(peer).game_session_id, peer.game_session_id);
        drop(view);
        drop(registered);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
