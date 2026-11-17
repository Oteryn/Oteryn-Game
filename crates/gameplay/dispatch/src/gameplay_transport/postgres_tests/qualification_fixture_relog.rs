//! MP's native admission, logout position save and re-admission before provisioning.
use super::*;
use crate::gameplay_transport::connection::LogoutReply;
use oteryn_protocol_oteryn::logout::LogoutResult;

// Exercise the exact helper used by the node's root-only qualification socket.
#[allow(
    dead_code,
    reason = "this mount uses the served-policy provisioning entry point"
)]
#[path = "../../../../../../apps/game-server/src/qualification_spell_character.rs"]
mod provision;

#[test]
fn qualification_fixture_accepts_native_admission_logout_and_relog() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("qualification-relog", || {
        scenario("qualification_relog", false)
    })
}

fn scenario(tag: &str, provision_before_relog: bool) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, tag, false, false).await?;
        let result: TestResult = async {
        let request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (generation, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        super::tutorial_vitals_fixture::publish_native_activation(&h, &pin).await?;
        super::tutorial_vitals_fixture::native_asset_revisions(
            &h,
            generation.active().ok_or("active generation")?,
        )
        .await?;
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            generation.active().and_then(|active| active.native_gameplay()).ok_or("native gameplay")?.spell_book().clone(),
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = generation.active();
        super::tutorial_vitals_fixture::admit(&owner, &deps, &request).await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let store = FreshAdmissionStore::from_root(h.root.clone());
        admitted.item_fence = crate::gameplay_transport::item_fence_of(
            store
                .current_session(admitted.game_session_id)
                .await
                .map_err(debug)?,
        );
        owner.admit_progression_session(&mut admitted).await;
        assert!(
            owner
                .progression_binding(admitted.game_session_id)
                .is_some()
        );
        let character = fence(1)?.character_id;
        assert_eq!(
            h.root
                .read_character_progression(&authority, character)
                .await
                .map_err(debug)?
                .ok_or("initial progression")?
                .character_revision
                .get(),
            1
        );
        // The shipped post-binding training settlement writes a receipt even with no gain.
        owner.admit_loop_training(&mut admitted).await;
        assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned);
        owner
            .prepare_admission_prey(admitted)
            .await
            .map_err(|()| "Prey admission refused")?;
        let before = h
            .root
            .read_character_progression(&authority, character)
            .await
            .map_err(debug)?
            .ok_or("settled progression")?;
        assert_eq!(before.character_revision.get(), 2);
        assert_eq!(before.level, 1);
        assert_eq!(before.total_experience.get(), 0);
        if provision_before_relog {
            let policy = owner.active_generation.and_then(|active| active.progression()).ok_or("policy")?
                .policy_for("profile-1", "ruleset-1", "content-1").map_err(debug)?;
            let current = store.current_session(admitted.game_session_id).await.map_err(debug)?;
            provision::prepare_character_with_policy(&h.root, &authority, &h.node, &current,
                provision::QualificationBuildSpec { vocation: "knight", level: 100, context: policy.context.clone(), xp_occurrence: id(183), build_occurrence: id(184) }, policy).await?;
        }
        let position_fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "position fence unavailable")?
            .ok_or("position fence absent")?;
        assert!(owner.track_restored(
            admitted.game_session_id,
            deps.actor,
            &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
            position_fence
        ));
        assert_eq!(
            owner.logout_session(admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        let after_logout = h
            .root
            .read_character_progression(&authority, character)
            .await
            .map_err(debug)?
            .ok_or("logout progression")?;
        assert!(after_logout.character_revision.get() > before.character_revision.get());
        if provision_before_relog {
            assert_eq!(after_logout.level, 100);
        } else {
            assert_eq!(after_logout.level, before.level);
            assert_eq!(after_logout.total_experience, before.total_experience);
            assert_eq!(h.root.read_character_build_state(&authority, character).await.map_err(debug)?,
                crate::durability::character_build::DurableBuildState::default());
        }
        let saved: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(saved, 1, "logout saved the actor's position");
        let (session, relog_request) = super::full_world_support::relogin_request(&h).await?;
        let current = store.current_session(session).await.map_err(debug)?;
        if provision_before_relog {
            assert!(owner.premium_coordinator.is_none(), "Platform access is unavailable");
            let actor = {
                let mut runtime = deps.runtime.lock().await;
                let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
                runtime.commit_fresh_session(reserved).map_err(debug)?
            };
            let transport = current.current_transport().ok_or("relog transport")?;
            let (outcome, _, _) = Box::pin(owner.initialize_first_entry(&relog_request, session, transport, actor)).await;
            // Prove which old Err(()) fires: current durable/actor facts load, but
            // the canonical familiar's caster qualification has no Premium projection.
            let native = owner.active_generation.and_then(|active| active.native_gameplay()).ok_or("native")?;
            let access = owner.refresh_and_apply_spell_access(actor, session).await;
            let owned = owner.read_familiar_lifecycle_inputs(actor, session, native, &access).await.map_err(|()| "familiar inputs refused")?;
            let index = owner.active_familiar_index("knight").ok_or("canonical knight familiar")?;
            let (spell, _) = owner.spells.source_indexed(index).ok_or("familiar spell")?;
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, actor, session).ok_or("relog spell state")?;
            assert_eq!(owned.caster(state, spell, native, crate::spell::harmony::HarmonyMultiplier::ONE, owner.owner_now().get()).err(),
                Some(crate::spell::owned_cast_facts::OwnedFactsError::UnavailablePremium));
            drop(states);
            drop(runtime);
            let stored = h.root.read_character_familiar_state(&authority, character).await.map_err(debug)?;
            assert!(stored.state().familiar_definition.is_none());
            assert!(stored.state().cooldowns.is_empty());
            assert!(stored.state().lifecycle.as_ref().is_none_or(|life| !life.open && life.familiar_remaining_ms == 0));
            assert!(matches!(outcome, FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled),
                "provisioned Character without a familiar must log in without Platform: {outcome:?}");
            return Ok(());
        }
        let mut readmitted = admitted;
        readmitted.game_session_id = session;
        readmitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        owner.admit_progression_session(&mut readmitted).await;
        assert!(
            owner.progression_binding(session).is_some(),
            "the real binder accepts the unchanged level/XP at the new root revision"
        );
        assert_eq!(
            h.root
                .read_current_character(&authority, character)
                .await
                .map_err(debug)?
                .revision,
            after_logout.character_revision
        );
        assert_eq!(
            h.root
                .read_character_progression(&authority, character)
                .await
                .map_err(debug)?,
            Some(after_logout.clone())
        );
        owner.admit_loop_training(&mut readmitted).await;
        assert_eq!(readmitted.first_entry, FirstEntryOutcome::Positioned);
        let before = h
            .root
            .read_character_progression(&authority, character)
            .await
            .map_err(debug)?
            .ok_or("re-admitted progression")?;
        assert_eq!(
            before.character_revision.get(),
            after_logout.character_revision.get() + 1
        );
        assert_eq!(before.level, 1);
        assert_eq!(before.total_experience.get(), 0);
        let policy = owner
            .active_generation
            .and_then(|active| active.progression())
            .ok_or("served progression")?
            .policy_for("profile-1", "ruleset-1", "content-1")
            .map_err(debug)?;
        eprintln!(
            "QUALIFICATION_RELOG logout_revision={} revision={} level={} xp={} binding=Bound",
            after_logout.character_revision.get(),
            before.character_revision.get(),
            before.level,
            before.total_experience.get()
        );
        provision::prepare_character_with_policy(
            &h.root,
            &authority,
            &h.node,
            &current,
            provision::QualificationBuildSpec {
                vocation: "knight",
                level: 100,
                context: policy.context.clone(),
                xp_occurrence: id(181),
                build_occurrence: id(182),
            },
            policy,
        )
        .await?;
        let after = h
            .root
            .read_character_progression(&authority, character)
            .await
            .map_err(debug)?
            .ok_or("provisioned progression")?;
        assert_eq!(after.level, 100);
        assert_eq!(after.character_revision.get(), before.character_revision.get() + 2);
        assert_eq!(
            h.root
                .read_character_build_state(&authority, character)
                .await
                .map_err(debug)?
                .vocation(),
            "knight"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}

#[test]
fn qualification_fixture_familiar_login_without_platform_is_playable() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("qualification-familiar-login", || {
        scenario("qualification_familiar_login", true)
    })
}
