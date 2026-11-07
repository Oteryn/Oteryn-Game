//! Real native HP publication enters the copied source-death dispatcher.
//! The test never invokes a Quest kill consumer or inserts an XP member.
use super::fixture::{Harness, debug, id, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn srx3_source_death_real_malice_lethal_runs_copied_goal_and_preserves_pre_taint_xp_postgres()
-> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-source-death-quest", || {
        run(async |admin| {
            reset();
            // Crystal201189f lib/quests/soul_war.lua:332-354: literal first
            // waiting tile and its real source map; no seeded catalogue recipe.
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsMalice,
                MovementLocalPosition {
                    x: 33679,
                    y: 31599,
                    floor: 14,
                },
            )?;
            let deps: Dependencies = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_source_death_quest", false, false)
                    .await?;
            // Imported prior quest access is a historical fixture fact, following
            // srx3_taints. New admission/callback writes keep all normal guards.
            let mut historical = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *historical)
                .await?;
            sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed) VALUES(encode($1,'hex')::uuid,true,0,NULL,false,false)")
                .bind(id(41).as_slice()).execute(&mut *historical).await?;
            historical.commit().await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            let catalogue =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            let source_rule = format!(
                "{}remorse",
                oteryn_gameplay_quests::quest::soul_war::source_items::TRANSITION_PREFIX
            );
            assert!(
                catalogue.catalogue().transition(&source_rule).is_some(),
                "the real embedded Soul War Quest catalogue must be installed before source work"
            );
            owner.quest_catalogue = Some(std::sync::Arc::new(catalogue.catalogue().clone()));
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsMalice)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            let supported = [
                4,
                6,
                7,
                10,
                11,
                12,
                15,
                17,
                18,
                23,
                30,
                92,
                wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                385,
            ];
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::connection::fresh_capability_selection(
                    &owner, &supported,
                )
                .await
                .ok_or("actual native source capability selection")?;
            assert!(admitted.continuity.selected_capabilities.contains(30));
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let instance = super::activity_runtime::allocate_ready_instance(&h, 215).await?;
            let mut frames = Vec::new();
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                "oteryn:creature.goshnar_s_malice",
                true,
                &mut frames,
            )
            .await?;
            assert!(
                matches!(reply, wire::BossActivityReply::Accepted { .. }),
                "actual Malice source entry: {reply:?}"
            );
            assert!(barrier.is_some());
            let fence = admitted.item_fence.ok_or("actual Instance fence")?;
            assert_eq!(fence.runtime_scope, instance);
            assert!(matches!(instance, RuntimeScopeRefV1::Instance { .. }));
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual source owner")?;
            let player = admitted.runtime_actor.ok_or("actual transferred Player")?;
            let native = scene
                .controller
                .active()
                .and_then(|active| active.native_gameplay())
                .ok_or("actual full native gameplay")?;
            let view = owner.runtime_view(&registered);
            view.refresh_quest_session(admitted.game_session_id, false)
                .await;
            assert!(
                view.refresh_source_party_presence(player, admitted.game_session_id)
                    .await
            );
            let mut captured = None;
            for _ in 0..8 {
                // Consume older real room timers before choosing the actual HP
                // owner time. Do not silence them or fabricate an enqueue clock.
                view.drain_monster_melee().await;
                let mut runtime = view.runtime.lock().await;
                let states = view.states.spell_states.try_lock().map_err(debug)?;
                let attack = view.states.attack.try_lock().map_err(debug)?;
                let observed = view.owner_now();
                if states
                    .soul_war_callbacks
                    .next_native_due()
                    .is_some_and(|due| due <= observed.get())
                {
                    continue;
                }
                let boss = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|row| {
                        runtime.matches_live_creature_identity(
                            row.actor,
                            b"oteryn:creature.goshnar_s_malice",
                        )
                    })
                    .ok_or("actual live source Malice")?
                    .actor;
                let health = runtime.companion_snapshot(boss).map_err(debug)?.health;
                let lease = runtime
                    .bound_attacker_lease(player, admitted.game_session_id)
                    .map_err(debug)?;
                let hit = runtime
                    .borrow_exact_actor_commit_at_us(observed.get())
                    .commit_damage_for_bound_attacker(
                        boss,
                        player,
                        CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(32101).map_err(debug)?,
                        ),
                        0,
                        crate::foundation::OwnerDamageCommand {
                            target: b"oteryn:creature.goshnar_s_malice",
                            occurrence: b"source-malice-goal",
                            binding: b"source-malice-goal-canonical-native-hp-v1",
                            damage: health,
                        },
                    )
                    .map_err(debug)?;
                assert!(
                    hit.applied && hit.health_before > 0 && hit.health_after == 0,
                    "real bound Player lethal, never a health-zero setter"
                );
                let notice = runtime
                    .next_source_creature_death()
                    .map_err(debug)?
                    .ok_or("canonical HP must enqueue source work before rewards")?;
                assert_eq!(notice.actor(), boss);
                assert_eq!(notice.enqueued_at_us(), Some(observed.get()));
                assert!(
                    runtime
                        .source_player_damage_shares(boss)
                        .map_err(debug)?
                        .iter()
                        .any(|share| share.actor == player && share.total > 0)
                );
                let activity = attack.shared_experience_cells();
                crate::gameplay_transport::kill_reward::record_creature_kill(
                    &mut runtime,
                    crate::gameplay_transport::kill_reward::KillSink {
                        prey: Some(&states),
                        activity: &activity,
                        queue: attack.kills(),
                        rewards: native.reward_table(),
                        charms: Some(view.imported_charms),
                    },
                    boss,
                    crate::combat::CapturedRewardPrincipal {
                        character: lease.character_id(),
                        lease_generation: lease.generation(),
                        session: admitted.game_session_id,
                        actor: player,
                    },
                    observed.get() / 1000,
                    Some(observed),
                );
                assert!(
                    attack.kills().has_session(admitted.game_session_id),
                    "actual ordinary reward owner retains this source death before take"
                );
                captured = Some(notice.death_key());
                break;
            }
            let death =
                captured.ok_or("real source timer work must quiesce before the lethal fixture")?;
            // Production dispatcher and ordinary physical drain only. No direct
            // apply_soul_war_kill/Quest helper invocation can mask a missing hook.
            view.drain_monster_melee().await;
            let _ = view.drain_kill_rewards(admitted.game_session_id).await;
            let flags:i16=sqlx::query_scalar("SELECT defeated_bosses FROM game_character_soul_war WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                flags & 1,
                1,
                "copied SoulWarBossesDeath must award Malice through the real deferred caller"
            );
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_soul_war_receipts WHERE character_id=encode($1,'hex')::uuid AND boss_key='malice'")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(count, 1);
            let key = crate::durability::party_kill_xp::encode_death_key(death);
            let xp:(i64,i64)=sqlx::query_as("SELECT d.base_xp,m.raw_share FROM game_party_kill_xp_receipts d JOIN game_party_kill_xp_members m USING(death_key) WHERE d.death_key=$1 AND m.character_id=encode($2,'hex')::uuid")
                .bind(&key).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert!(xp.0 > 0);
            assert_eq!(
                xp.1, xp.0,
                "one actual unshared receiver gains XP before this death adds its first taint"
            );
            view.drain_monster_melee().await;
            let _ = view.drain_kill_rewards(admitted.game_session_id).await;
            let replay:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_soul_war_receipts WHERE character_id=encode($1,'hex')::uuid AND boss_key='malice'")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                replay, 1,
                "copied handler/physical retry must not award twice"
            );
            drop(view);
            drop(owner);
            drop(authority);
            drop(seal);
            drop(evidence);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "[DBG-W11-DEATH] source-goal scenario seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
