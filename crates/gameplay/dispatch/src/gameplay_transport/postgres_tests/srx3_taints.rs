//! SRX-3: command 133 reads private taints through the real admitted connection.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_fence_of;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1;

#[test]
fn srx3_taints_postgres_native_query_private_expiry_and_session_fence() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-taints", || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "srx3_taints", true, false).await?;
            prepare_current_session(&h).await?;
            // Historical state setup follows soul_war.rs's expiry fixture. Only the
            // admin setup transaction bypasses receipt triggers; the dispatched read
            // runs with normal runtime privileges, guards and constraints.
            // A different retained Character must never leak into this view.
            use crate::durability::guild_chat_fixture as characters;
            let other =
                crate::domain::CharacterId::from_bytes(characters::sid(90, 1)).map_err(debug)?;
            characters::historical_character(
                &h.pool,
                90,
                other,
                characters::sid(90, 2),
                super::fixture::id(42),
                "Tainted Other",
            )
            .await?;
            let mut historical = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *historical)
                .await?;
            sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed) VALUES(encode($1,'hex')::uuid,true,31,floor(extract(epoch FROM clock_timestamp()))::bigint,false,false)")
                .bind(other.as_bytes().as_slice()).execute(&mut *historical).await?;
            historical.commit().await?;
            let deps = Dependencies::new()?;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await?
                    .0;
            admitted.item_fence = item_fence_of(current);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[CAPABILITY_BOSS_ENCOUNTER_V1],
            )
            .ok_or("boss capability")?;
            let character = admitted
                .item_fence
                .ok_or("current item fence")?
                .character_id;
            let (status, bytes) =
                super::boss_encounter::exchange_kind(&owner, &mut admitted, 1, 133, &[16, 1])
                    .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert!(bytes.ends_with(&[58, 8, 8, 0, 16, 0, 24, 0, 32, 0]));
            let mut historical = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *historical)
                .await?;
            sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed) VALUES(encode($1,'hex')::uuid,true,31,floor(extract(epoch FROM clock_timestamp()))::bigint,false,false)")
                .bind(character.as_bytes().as_slice()).execute(&mut *historical).await?;
            sqlx::query("INSERT INTO game_character_rotten_blood(character_id,regular_taints,child_flags,final_taint,completed,last_cleanse_at) VALUES(encode($1,'hex')::uuid,3,15,true,false,NULL)")
                .bind(character.as_bytes().as_slice()).execute(&mut *historical).await?;
            historical.commit().await?;
            let (status, bytes) =
                super::boss_encounter::exchange_kind(&owner, &mut admitted, 2, 133, &[16, 1])
                    .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert!(bytes.ends_with(&[58, 8, 8, 5, 16, 3, 24, 15, 32, 1]));
            // Expiry is projected using server time without rewriting the Quest state.
            let mut historical = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *historical)
                .await?;
            sqlx::query("UPDATE game_character_soul_war SET first_taint_at=first_taint_at-1209600 WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_bytes().as_slice()).execute(&mut *historical).await?;
            historical.commit().await?;
            let (_, bytes) =
                super::boss_encounter::exchange_kind(&owner, &mut admitted, 3, 133, &[16, 1])
                    .await?;
            assert!(bytes.ends_with(&[58, 8, 8, 0, 16, 3, 24, 15, 32, 1]));
            let (_, legacy) =
                super::boss_encounter::exchange_kind(&owner, &mut admitted, 4, 133, &[]).await?;
            assert!(!legacy.ends_with(&[58, 8, 8, 0, 16, 3, 24, 15, 32, 1]));
            let mut stale = admitted.item_fence.ok_or("item fence")?;
            stale.game_session_id =
                crate::foundation::GameSessionId::decode(&characters::sid(99, 1)).map_err(debug)?;
            if !owner.root.is_ready() {
                owner.root.request_ready();
            }
            if owner.root.has_ready_demand() {
                owner.root.maintain_ready_once().await.map_err(debug)?;
            }
            assert!(
                owner
                    .root
                    .read_boss_cooldown_list(owner.character, owner.holder, stale, &[16, 1])
                    .await?
                    .is_none()
            );
            eprintln!("SRX3_TAINTS POSTGRES_CASE_COMPLETED");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn srx3_taints_postgres_census_over_one_work_page_keeps_every_current_player() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-taints-census", || {
        run(async |admin| {
            reset();
            let started = std::time::Instant::now();
            let h = crate::bestiary_postgres_harness::Harness::create(
                admin,
                "srx3_taints_census",
                false,
            )
            .await?;
            let world =
                crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?;
            let channel =
                crate::foundation::ChannelId::decode(&super::fixture::id(43)).map_err(debug)?;
            let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
                world,
                channel,
                h.node.fact().node_id(),
                h.node.fact().registration_revision(),
                1,
                1,
                "runtime-scope-assignment:1",
                65,
                crate::content::channel_content_pin::ChannelContentPin::test(world),
            )
            .map_err(debug)?;
            let mut actors = Vec::new();
            // Every member is admitted by the actual durable Fresh writer. This
            // is not a fabricated roster or 65 aliases for one current session.
            for index in 0_u8..65 {
                // Each signed Fresh request uses the actual current database
                // clock; a slow 65-member setup must not reuse an expired JWT.
                let now: i64 = sqlx::query_scalar(
                    "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
                ).fetch_one(&h.pool).await?;
                let current = crate::world_party_runtime_postgres_cases::committed_fresh_player(
                    &h,
                    60 + index,
                    125 + index,
                    190 + index,
                    now,
                )
                .await
                .map_err(|error| format!("genuine fresh player {index}: {error}"))?;
                let session = current.current_game_session_id();
                let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
                let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
                runtime
                    .bind_attacker_lease(actor, session, current.current_character_lease())
                    .map_err(debug)?;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                actors.push((actor, session));
            }
            eprintln!(
                "SRX3_TAINT_CENSUS_FRESH_ACTORS_READY count={}",
                actors.len()
            );
            let seal = h.recovery.seal_current().map_err(debug)?;
            let recovery = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let lane = crate::durability::spell_owner_commit::SpellLane::for_scope(
                runtime.binding().runtime_scope(),
            );
            let permit = lane
                .acquire()
                .await
                .map_err(|_| "unresolved actual scope lane")?;
            let mut context = (&h.root, &recovery, &h.node, &runtime, &permit, &actors);
            let facts = h.root.try_issue_semantic_pass()?.run_with_context(&mut context,
                move |holder, deadline, context| Box::pin(async move {
                    use crate::durability::spell_item_transaction::{
                        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
                    };
                    let (root, recovery, node, runtime, permit, actors) = context;
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let authority = assert_spell_item_scope_in_transaction(
                        &mut tx, permit, root, recovery, node,
                        runtime.binding().runtime_scope(), runtime.binding().scope_generation().get(),
                    ).await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    // First prove BOTH pages are independently genuine/current;
                    // the final failure can then only concern census composition.
                    eprintln!("SRX3_TAINT_CENSUS_SCOPE_PROVEN");
                    let first = crate::durability::quest_state::soul_war::read_current_soul_war_taints_in_transaction(
                        &mut tx, root, &authority, runtime, &actors[..64],
                    ).await?;
                    let last = crate::durability::quest_state::soul_war::read_current_soul_war_taints_in_transaction(
                        &mut tx, root, &authority, runtime, &actors[64..],
                    ).await?;
                    assert_eq!((first.len(), last.len()), (64, 1));
                    eprintln!("SRX3_TAINT_CENSUS_REAL_PAGES_CONFIRMED first=64 last=1");
                    let facts = crate::durability::quest_state::soul_war::read_current_soul_war_taints_in_transaction(
                        &mut tx, root, &authority, runtime, actors,
                    ).await?;
                    tx.rollback().await?;
                    Ok(facts)
                })).await?;
            assert_eq!(
                facts.len(),
                65,
                "a full source census must cross the work-page boundary"
            );
            for (fact, &(actor, session)) in facts.iter().zip(&actors) {
                assert_eq!((fact.actor, fact.session), (actor, session));
                assert_eq!(fact.count, 0);
                assert_eq!(fact.observed_at_seconds, facts[0].observed_at_seconds);
            }
            eprintln!(
                "SRX3_TAINT_CENSUS_COMPLETED players={} elapsed={:?}",
                facts.len(),
                started.elapsed()
            );
            drop(runtime);
            drop(permit);
            drop(lane);
            drop(recovery);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn srx3_taints_postgres_visible_grace_keeps_data_without_restoring_input() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-taints-grace", || {
        run(async |admin| {
            reset();
            let started = std::time::Instant::now();
            let h = Harness::create_with_admission_seed(admin, "srx3_taints_grace", true, false)
                .await?;
            prepare_current_session(&h).await?;
            let deps = Dependencies::new()?;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session(deps.admitted.game_session_id)
                    .await?;
            {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .bind_attacker_lease(
                        deps.actor,
                        deps.admitted.game_session_id,
                        current.current_character_lease(),
                    )
                    .map_err(debug)?;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
            }
            let seal = h.recovery.seal_current().map_err(debug)?;
            let recovery = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let owner = deps.owner(&h, &recovery);
            deps.initialize(&owner).await;
            let controller = crate::gameplay_transport::connection::ControllerBinding {
                transport: current
                    .current_transport()
                    .ok_or("actual fresh transport")?,
                account_id: super::fixture::id(40),
            };
            assert_eq!(
                owner
                    .commit_control_loss(deps.admitted.game_session_id, deps.actor, controller)
                    .await,
                crate::gameplay_transport::connection::ControlLossResult::Recorded
            );
            let permit = owner.spell_lane_permit().await.ok_or("actual scope lane")?;
            let runtime = deps.runtime.lock().await;
            {
                let states = owner.states.spell_states.lock().await;
                let selected = crate::gameplay_transport::soul_war_callbacks::source_player_census(
                    &runtime, &states,
                )
                .map_err(|_| "source physical census")?;
                assert!(
                    selected.contains(&(deps.actor, deps.admitted.game_session_id)),
                    "the production source census must retain this living avatar in genuine committed grace"
                );
            }
            let actors = [(deps.actor, deps.admitted.game_session_id)];
            let mut context = (&owner, &*runtime, &permit, &actors);
            let facts = h.root.try_issue_semantic_pass()?.run_with_context(&mut context,
                move |holder, deadline, context| Box::pin(async move {
                    use crate::durability::spell_item_transaction::{assert_spell_item_scope_in_transaction, begin_spell_owner_transaction};
                    let (owner, runtime, permit, actors) = context;
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let authority = assert_spell_item_scope_in_transaction(&mut tx, permit, owner.root, owner.character,
                        owner.holder, runtime.binding().runtime_scope(), runtime.binding().scope_generation().get())
                        .await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    assert!(crate::durability::admission_journal::party_target_binding::current_colocated_party_character_for_scope_in_transaction(
                        &mut tx, owner.root, &authority, runtime, actors[0].0, actors[0].1).await?.is_none());
                    let facts = crate::durability::quest_state::soul_war::read_current_soul_war_taints_in_transaction(
                        &mut tx, owner.root, &authority, runtime, *actors).await?;
                    tx.rollback().await?;
                    Ok(facts)
                })).await?;
            assert_eq!(facts.len(), 1);
            assert_eq!((facts[0].actor, facts[0].session), actors[0]);
            assert_eq!(
                facts[0].character,
                *current.current_character_lease().character_id().as_bytes()
            );
            assert!(
                runtime
                    .player_control_facts(actors[0].0, actors[0].1)
                    .map_err(debug)?
                    .control_loss
                    .is_some()
            );
            eprintln!(
                "SRX3_TAINT_VISIBLE_GRACE_COMPLETED elapsed={:?}",
                started.elapsed()
            );
            drop(runtime);
            drop(owner);
            drop(recovery);
            drop(seal);
            h.cleanup().await
        })
    })
}
