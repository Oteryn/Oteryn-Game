//! M18 selected encounter: real source geometry, admission and source timeout.
use super::fixture::{self, Harness, debug, run};
use super::{Dependencies, TestResult, reset};
use crate::content::NativeEntryStart;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelContentPin, CharacterLease, MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_fence_of;
use oteryn_protocol_oteryn::activity_scope::ActivityScope;
use oteryn_protocol_oteryn::boss_encounter::{self as wire, BossActivityReply};
const BOSS: &str = "oteryn:boss.the_first_dragon";
const CREATURE: &str = "oteryn:creature.the_first_dragon";
const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
const TRIALS: &str = "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s4";
type PublishedRoom = (Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>,String,String,i64);
const LEVER: MovementLocalPosition = MovementLocalPosition {x:33583,y:30993,floor:14};
const ENTRY: MovementLocalPosition = MovementLocalPosition {x:33621,y:31023,floor:14};
const EXIT: MovementLocalPosition = MovementLocalPosition {x:33597,y:30994,floor:14};
#[test]
fn m18_selected_first_dragon_native_instance_timeout_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e|e.to_string()))?.join().map_err(|_|"M18 room thread failed")??;
    Ok(())
}
fn scenario(victory: bool) -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        reset();
        let h =
            Harness::create_with_admission_seed(admin, if victory { "season_first_dragon_win" } else { "season_first_dragon_timeout" }, false, false).await?;
        let evidence = super::house_transfer::RecoverySource::start(&h).await?;
        super::house_transfer::prepare_owned_session(&h).await?;
        let expected = fixture::fence(1)?;
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = expected.runtime_scope
        else {
            return Err("source Channel".into());
        };
        let (controller, activation) =
            super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
        let generation = controller.active().ok_or("active gameplay")?;
        let native = generation.native_gameplay().ok_or("native gameplay")?;
        let source_room = activation.qualified_room().clone();
        let activation_pin = activation.into_channel_pin();
        let candidate = crate::content::bosses::activity_template::captured_boss_levers()
            .map_err(debug)?
            .into_iter()
            .find(|candidate| candidate.path == "content/encounters/bosses/selected-first-dragon.json")
            .ok_or("captured selected First Dragon lever")?;
        let bundle = super::srvenc_source_boss::source_bundle(world_id, channel_id, native, &candidate)?;
        let cells = bundle
            .movement_cells(source_room.movement_cells())
            .map_err(debug)?;
        let room = source_room
            .bundle_view(
                cells.clone(),
                bundle.frame_binding_digest(world_id),
                bundle.map_revision_digest(),
                NativeEntryStart {
                    x: LEVER.x,
                    y: LEVER.y,
                    floor: LEVER.floor,
                },
            )
            .ok_or("source bundle combat room")?;
        let template = controller
            .qualify_activity_template(&source_room, &room, "content-1", &candidate)
            .map_err(|reason| {
                let anchors:Vec<_> = candidate.lever_positions.iter().chain(&candidate.entry_positions)
                    .chain(candidate.boss_position.iter()).chain(candidate.exit_position.iter())
                    .map(|&[x,y,z]| ([x,y,z],room.movement_cells().index().lookup(
                        room.movement_cells().scope(),crate::content::LogicalCell {x,y,z},
                    ))).collect();
                format!("First Dragon source qualification {reason:?}; actual bundle anchor collisions: {anchors:?}")
            })?;
        assert_eq!(template.boss_key(), BOSS);
        assert_eq!(
            template.boss_position(),
            crate::content::LogicalCell {
                x: 33617,
                y: 31023,
                z: 14
            }
        );
        let policy = native
            .policies()
            .policies()
            .iter()
            .find(|policy| policy.definition_key == CREATURE)
            .ok_or("native First Dragon policy")?;
        assert_eq!(policy.reward_boss, Some(true));
        let rewards = native.reward_table().row(CREATURE).map_err(debug)?;
        assert_eq!(
            rewards.corpse_item.production_key,
            "oteryn:item.tibia.i25065"
        );

        let pin = ChannelContentPin::from_activation(
            world_id,
            generation.activation_sequence(),
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            bundle.frame_binding_digest(world_id),
            bundle.map_revision_digest(),
            (LEVER.x, LEVER.y, LEVER.floor),
        );
        assert_ne!(
            activation_pin.frame_binding_digest(),
            pin.frame_binding_digest(),
            "source Content issuance and bundle runtime retain distinct frame bindings"
        );
        let mut deps = Dependencies::with_content(
            pin.clone(),
            source_room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.combat_room = Some(room);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.evidence = &evidence.evidence;
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(
            FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0,
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[wire::CAPABILITY_BOSS_ENCOUNTER_V1],
        )
        .ok_or("boss capability")?;
        let fence = admitted.item_fence.ok_or("current source item fence")?;
        super::item_look::seed_main_backpack(&h, &[]).await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
            .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            assert_eq!(
                runtime
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                LEVER
            );
        }
        deps.initialize(&owner).await;
        owner.admit_premium(fixture::id(40), admitted.game_session_id);
        // Use the same pinned Character interpretation as the real source session;
        // the historical seeded level50/profile-policy fixture is not this policy.
        owner.admit_progression_session(&mut admitted).await;
        assert!(owner.progression_binding(admitted.game_session_id).is_some(),
                "the source session must bind the native content-1 progression policy");
        super::qengine_outfit_stories::earn_level(&owner, &admitted, &authority, 40).await?;
        // Prior personal trials are historical input to this encounter, not a synthetic death.
        let canonical = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
        let catalogue = canonical.catalogue();
        let hash = catalogue.definition_hash(QUEST).ok_or("First Dragon catalogue")?;
        let mut seed = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *seed).await?;
        sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) SELECT character_id,$1,'content-1',$2,character_revision,character_revision FROM game_character_roots")
            .bind(QUEST).bind(hash.as_slice()).execute(&mut *seed).await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$1,$2,2,character_revision FROM game_character_roots")
            .bind(TRIALS).bind(QUEST).execute(&mut *seed).await?;
        seed.commit().await?;
        let mut guard = h.pool.begin().await?;
        sqlx::query("SELECT character_id FROM game_character_roots FOR UPDATE").fetch_all(&mut *guard).await?;
        let character = admitted.item_fence.ok_or("selected source fence")?.character_id;
        assert!(!crate::durability::activity_handoff::fixture_first_dragon_entry_permit(
            &mut guard, character, &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
        ).await?, "complete trials never bypass missing Premium authority");
        assert!(crate::durability::activity_handoff::fixture_first_dragon_entry_permit(
            &mut guard, character, &owner.premium_coordinator,
        ).await?, "the same live Premium coordinator permits the selected encounter");
        guard.rollback().await?;
        owner.quest_catalogue = Some(std::sync::Arc::new(catalogue.clone()));
        owner.refresh_quest_session(admitted.game_session_id, false).await;
        let clock: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&h.pool).await?;
        h.root.set_test_quest_clock(clock).map_err(debug)?;
        owner
            .register_current_runtime()
            .await
            .map_err(|_| "register source runtime")?;
        owner
            .activity_runtimes
            .remember(admitted)
            .map_err(|_| "remember source connection")?;
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
        let catalogue =
            crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone());
        catalogue
            .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template))
            .await?;
        catalogue
            .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template))
            .await?;
        let published: PublishedRoom = sqlx::query_as(
            "SELECT origin_frame_digest,origin_map_digest,server_artifact_digest,activation_frame_digest,content_revision,template_revision,count(*) OVER () FROM game_boss_room_publications WHERE world_id=encode($1,'hex')::uuid AND boss_key=$2",
        ).bind(world_id.as_bytes().as_slice()).bind(BOSS).fetch_one(&h.pool).await?;
        assert_eq!(
            published.0.as_slice(),
            pin.frame_binding_digest().as_slice()
        );
        assert_eq!(published.1.as_slice(), pin.map_revision_digest().as_slice());
        assert_eq!(
            published.2.as_slice(),
            pin.server_artifact_digest().as_slice()
        );
        assert_eq!(
            published.3.as_slice(),
            activation_pin.frame_binding_digest().as_slice()
        );
        assert_eq!(published.4, "content-1");
        assert_eq!(published.5, template.template_revision());
        assert_eq!(
            published.6, 1,
            "replaying the sealed source publication is idempotent"
        );
        let instance = super::activity_runtime::allocate_ready_instance(&h, 88).await?;
        let assignment = h
            .root
            .read_runtime_scope_assignment(instance)
            .await
            .map_err(debug)?
            .ok_or("ready Instance allocation")?;
        assert!(
            owner
                .activity_runtimes
                .get(instance, assignment.ownership_generation)
                .is_none(),
            "only the production template caller may instantiate the destination"
        );
        let old = admitted;
        let (_, reply, barrier) =
            super::boss_activity::exchange(&owner, &mut admitted, BOSS, true).await?;
        let BossActivityReply::Accepted {
            handoff,
            deadline_unix_ms: Some(_),
        } = reply
        else {
            return Err(format!("source Sugar entry refused: {reply:?}").into());
        };
        let barrier = barrier.ok_or("actual scope barrier")?;
        let RuntimeScopeRefV1::Instance { instance_id, .. } = instance else {
            return Err("Instance allocation".into());
        };
        assert_eq!(
            barrier.scope,
            ActivityScope::Instance {
                world: *world_id.as_bytes(),
                instance: instance_id
            }
        );
        assert_eq!(barrier.handoff, handoff);
        assert_eq!(barrier.source_session, *old.game_session_id.as_bytes());
        assert_eq!(barrier.session, *admitted.game_session_id.as_bytes());
        let destination = owner
            .resolve_runtime_view(admitted)
            .await
            .map_err(|_| "resolve production Instance")?
            .ok_or("Instance owner switched")?;
        assert_eq!(
            destination
                .bundle_map()
                .ok_or("Instance map")?
                .overlay
                .runtime_scope(),
            instance
        );
        assert!(
            destination
                .bundle_map()
                .ok_or("Instance map")?
                .overlay
                .base()
                .tile(33583, 30993, -14)
                .is_none(),
            "the Instance cannot expose the source lever outside its bounds"
        );
        {
            let runtime = destination.runtime.lock().await;
            assert_eq!(runtime.binding().runtime_scope(), instance);
            assert_eq!(
                runtime
                    .read_actor_position(admitted.runtime_actor.ok_or("Instance player")?)
                    .map_err(debug)?
                    .position(),
                ENTRY
            );
            let creatures = runtime.visible_entities().creatures;
            assert_eq!(creatures.len(), 1);
            assert_eq!(
                runtime
                    .companion_snapshot(creatures[0].actor)
                    .map_err(debug)?
                    .state
                    .policy
                    .definition_key,
                CREATURE
            );
        }
        let timers: (i64,i64) = sqlx::query_as("SELECT a.deadline-a.opened_at,c.available_at-a.opened_at FROM game_boss_room_activities a JOIN game_character_boss_cooldowns c USING(boss_key) WHERE a.admission=encode($1,'hex')::uuid")
            .bind(handoff.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(timers, (1_200_000,72_000_000));
        if victory {
            settle_selected_boss(&destination, &admitted).await?;
            let completed: (i64,i64,bool) = sqlx::query_as("SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),(SELECT value FROM game_character_quest_tracks WHERE track_key=$2),(SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key=$3)")
                .bind("oteryn:quest-progress/chosen-source/the_first_dragon_quest/s5")
                .bind("oteryn:quest-progress/chosen-source/the_first_dragon_quest/s6").bind(QUEST).fetch_one(&h.pool).await?;
            assert_eq!(completed,(1,1,true));
            let exit_after: i64 = sqlx::query_scalar("SELECT exit_deadline / 1000 + 1 FROM game_boss_room_activities WHERE admission=encode($1,'hex')::uuid")
                .bind(handoff.as_slice()).fetch_one(&h.pool).await?;
            h.root.advance_test_quest_clock(exit_after - clock).map_err(debug)?;
        } else {
            h.root.advance_test_quest_clock(1201).map_err(debug)?;
        }
        let (_, returned, barrier) = super::boss_activity::exchange_command(
            &owner,
            &mut admitted,
            BOSS,
            true,
            if victory { wire::COMMAND_TYPE_BOSS_DEFEATED } else { wire::COMMAND_TYPE_BOSS_TIMEOUT },
        )
        .await?;
        assert!(matches!(returned, BossActivityReply::Accepted { .. }));
        assert!(matches!(
            barrier.ok_or("return barrier")?.scope,
            ActivityScope::Channel { .. }
        ));
        let returned_fence = admitted.item_fence.ok_or("returned source fence")?;
        assert_eq!(returned_fence.runtime_scope, expected.runtime_scope);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(admitted.runtime_actor.ok_or("returned player")?)
                .map_err(debug)?
                .position(),
            EXIT
        );
        owner.activity_lifecycle_pass().await.map_err(|_| "retire selected activity")?;
        assert!(owner.activity_runtimes.get(instance, assignment.ownership_generation).is_none(),
            "the terminal private room must be retired after its last participant returns");
        let cooldown: i64 = sqlx::query_scalar("SELECT available_at FROM game_character_boss_cooldowns WHERE boss_key=$1")
            .bind(BOSS).fetch_one(&h.pool).await?;
        assert!(cooldown > (clock + 1201) * 1000, "returning never clears the personal cooldown");
        if !victory {
            let minted: i64 = sqlx::query_scalar("SELECT count(*) FROM game_boss_reward_items").fetch_one(&h.pool).await?;
            assert_eq!(minted, 0, "timeout never invents rewards or a quest kill");
            let deaths: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE track_key LIKE '%the_first_dragon_quest/s5'").fetch_one(&h.pool).await?;
            assert_eq!(deaths, 0);
        }
        eprintln!("M18_PG_EXECUTED selected_first_dragon_instance victory={} duration={:?}", victory, started.elapsed());
        Ok(())
    })
}

#[test]
fn m18_selected_first_dragon_native_victory_completes_once_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|e|e.to_string()))?.join().map_err(|_|"M18 room thread failed")??;
    Ok(())
}
async fn settle_selected_boss(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    assert!(owner.refresh_source_party_presence(actor, session).await);
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    {
        let mut runtime = owner.runtime.lock().await;
        let creature = CREATURE;
        let target = runtime.visible_entities().creatures.first().ok_or("native First Dragon")?.actor;
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(session, CommandId::new(22109).map_err(debug)?),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: creature.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"qengine-native-trial-death",
                    damage: health,
                },
            )
            .map_err(debug)?;
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink {
                    prey: None,
                    activity: &[],
                    queue: &queue,
                    rewards: native.reward_table(),
                    charms: None,
                },
                target,
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session,
                    actor,
                },
                now,
                Some(observed),
            );
        }
    }
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    Ok(())
}
