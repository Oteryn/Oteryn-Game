use super::appearance_change_postgres_fixtures::observer_visibility_tests as observer;

use super::death_ml_loss_composed_fixtures::{current_fence, production_generation_with_source};

use super::fixture::{Harness, debug, id, join_two, run};

use super::{Dependencies, TestResult, prepare_current_session};

use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};

use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};

use crate::foundation::{
    ChannelContentPin, CharacterLease, GameSessionId, MessageType, MovementLocalPosition,
    decode_wire_envelope,
};

use crate::gameplay_transport::capabilities::{SelectedCapabilities, test_command};

use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};

use crate::gameplay_transport::item_fence_of;

use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

use oteryn_protocol_oteryn::{CommandStatus, actor_spell, attack, decode_command_result};

use std::time::Duration;

#[path = "pvp_live/bundle_fixture.rs"]
mod bundle_fixture;

#[path = "pvp_live/expert_cases.rs"]
mod expert_cases;

use tokio::io::{AsyncWriteExt, DuplexStream};

const SPLASH_SECOND_MEMBER: u8 = 2;
const SPLASH_SECOND_SHOOTER: u8 = 100 + SPLASH_SECOND_MEMBER;
const SPLASH_SECOND_SESSION: u8 = 112;

async fn admit_splash_shooter(h: &Harness) -> TestResult {
    let account = id(120 + SPLASH_SECOND_MEMBER);
    sqlx::query("INSERT INTO game_character_account_guards VALUES (encode($1,'hex')::uuid)")
        .bind(account.as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_roots
           (character_id, account_id, world_id, lifecycle, character_revision, profile_revision,
            ruleset_revision, content_revision, starter_template_revision, name, sex, creation_vocation)
         SELECT encode($1,'hex')::uuid, encode($2,'hex')::uuid, world_id, lifecycle, 1,
                profile_revision, ruleset_revision, content_revision, starter_template_revision,
                'Fixture Splash Shooter', 2, 'knight'
         FROM game_character_roots WHERE character_id=encode($3,'hex')::uuid",
    )
    .bind(id(SPLASH_SECOND_SHOOTER).as_slice())
    .bind(account.as_slice())
    .bind(id(101).as_slice())
    .execute(&h.pool)
    .await?;
    super::boss_rewards_fixtures::privacy::seed_peer_creation(h, SPLASH_SECOND_MEMBER).await?;
    observer::admit_peer(
        h,
        SPLASH_SECOND_SHOOTER,
        SPLASH_SECOND_SESSION,
        SPLASH_SECOND_SESSION,
        Some([SPLASH_SECOND_SESSION; 32]),
    )
    .await
}

async fn command(
    client: &mut DuplexStream,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let frame = test_command(id, kind, payload);
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    loop {
        let frame = super::item_look_fixtures::read_frame(client).await?;
        let env = decode_wire_envelope(&frame).map_err(debug)?;
        if env.message_type() == MessageType::LivenessProbe {
            ack(client, env.payload()).await?;
        }
        if env.message_type() == MessageType::CommandResult {
            let result = decode_command_result(env.payload()).map_err(debug)?;
            assert_eq!(result.command_id, id);
            return Ok(result.payload.to_vec());
        }
    }
}

async fn ack(client: &mut DuplexStream, payload: &[u8]) -> TestResult {
    let id = oteryn_protocol_oteryn::decode_liveness_probe(payload).map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_liveness_ack(1, id, 0).map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    Ok(())
}

async fn wait_live(client: &mut DuplexStream, duration: Duration) -> TestResult {
    let deadline = tokio::time::Instant::now() + duration;
    loop {
        match tokio::time::timeout_at(deadline, super::item_look_fixtures::read_frame(client)).await {
            Ok(Ok(frame)) => {
                let env = decode_wire_envelope(&frame).map_err(debug)?;
                if env.message_type() == MessageType::LivenessProbe {
                    ack(client, env.payload()).await?;
                }
            }
            Ok(Err(error)) => return Err(error),
            Err(_) => return Ok(()),
        }
    }
}

async fn health(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
) -> TestResult<u32> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    Ok(states
        .get(
            &runtime,
            actor,
            GameSessionId::decode(&id(110)).map_err(debug)?,
        )
        .ok_or("target state")?
        .vitals()
        .health)
}

// The target has no serving connection in this fixture, so its driver must retry death
// settlement as its production cadence would. The caster's serving cadence shares the
// max-one durability holder; a busy pass is Retry, not a non-durable death.
async fn settle_death_when_ready(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    death: crate::gameplay_transport::actor_spell::PlayerDeath,
) -> TestResult<crate::gameplay_transport::SessionDeath> {
    use crate::gameplay_transport::SessionDeath;
    Ok(tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let settled = owner.settle_death(actor, session, death, false).await.0;
            if settled != SessionDeath::Retry {
                return settled;
            }
            let now = tokio::time::Instant::now();
            owner
                .root
                .wait_holder_retry(now, now + Duration::from_millis(25))
                .await;
        }
    })
    .await?)
}

pub(super) async fn wait_for_login_protection_to_expire(
    pool: &sqlx::PgPool,
    session: GameSessionId,
) -> TestResult {
    // Wait on the same PostgreSQL clock used by the PvP legality read. The original
    // admission decision time is validated against the committed admission evidence.
    sqlx::query(
        "SELECT pg_sleep(GREATEST(0, r.authorization_decided_at + 10
                                    - extract(epoch FROM clock_timestamp()))::double precision)
         FROM game_durability_reconnect_sessions s
         JOIN game_durability_fresh_admission_receipts r
           ON r.replay_key = COALESCE(s.fresh_replay_key, s.initial_fresh_replay_key)
         WHERE s.game_session_id = encode($1, 'hex')::uuid",
    )
    .bind(session.as_bytes().as_slice())
    .fetch_one(pool)
    .await?;
    Ok(())
}

async fn paid_splash(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    (actor, session): (crate::foundation::ExactActorRef, GameSessionId),
    (victim, _victim_session): (crate::foundation::ExactActorRef, GameSessionId),
    (second_shooter, second_session): (crate::foundation::ExactActorRef, GameSessionId),
    content: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult {
    use oteryn_protocol_oteryn::pvp_expert::{ExpertFightModes, ExpertMode};

    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    for (character, weapon, projectile, ammo) in
        [(41, 70, 71, 3449), (SPLASH_SECOND_SHOOTER, 72, 73, 3447)]
    {
        for (seed, item, count, slot) in [(weapon, 3350, 1, 6_i16), (projectile, ammo, 10, 10)] {
            sqlx::raw_sql(sqlx::AssertSqlSafe(
                crate::durability::item_fee_burn::test_support::item(
                    seed, &format!("oteryn:item.tibia.i{item}"), "definition-r1", count,
                ),
            )).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($4,'hex')::uuid)")
                .bind(id(character).as_slice()).bind(id(42).as_slice()).bind(slot).bind(id(seed).as_slice()).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    assert_eq!(owner.expert_fight_modes(actor, session, 1, ExpertFightModes {
        modes: attack::FightModes { secure: false, ..attack::FightModes::DEFAULT },
        expert: ExpertMode::RedFist,
    }).await.map_err(debug)?, attack::AttackIntentDisposition::Ok);
    let rat = {
        let mut runtime = owner.runtime.lock().await;
        runtime.install_companion_policies(content.policies().clone()).map_err(debug)?;
        let state = runtime.companion_policy("oteryn:creature.rat").map_err(debug)?;
        let health = u64::try_from(state.maximum_health)?;
        runtime.realize_activation_spawns(vec![(
            vec![(2, 12, -7)], 60_000, 0, "oteryn:creature.rat".into(), health,
        )]).map_err(debug)?;
        let visible = runtime.visible_entities();
        let rat = visible.creatures.first().ok_or("rat")?;
        runtime.set_test_creature_health(rat.actor, 10_000).map_err(debug)?;
        EntityRef { identity: rat.actor.placement_identity(), generation: rat.generation }
    };
    for (source, source_session) in [(actor, session), (second_shooter, second_session)] {
        assert_eq!(owner.select_attack_target(source, source_session, 2, Some(rat)).await,
            attack::AttackIntentDisposition::Ok);
        owner.refresh_attack_weapon(source, source_session).await;
    }
    let count = |seed| async move {
        sqlx::query_scalar::<_, i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(seed).as_slice()).fetch_one(&h.pool).await
    };
    let before = health(owner, victim).await?;
    let now = owner.owner_now();
    crate::gameplay_transport::attack::ranged::lose_shot_ack_for(second_session);
    owner.drain_auto_attacks_at(now).await;
    assert_eq!((count(71).await?, count(73).await?), (9, 9), "both shots committed");
    assert_eq!(health(owner, victim).await?, before, "the later payment fenced the pending splash");
    assert!(owner.spell_lane.acquire().await.is_err(), "later shot is parked");
    owner.drain_auto_attacks_at(now).await;
    let after = health(owner, victim).await?;
    assert!(after < before, "the earlier paid splash reached its player target");
    owner.drain_auto_attacks_at(now).await;
    assert_eq!(health(owner, victim).await?, after, "the splash was installed exactly once");
    assert_eq!((count(71).await?, count(73).await?), (9, 9));
    Ok(())
}

async fn melee_fault(
    client: &mut DuplexStream,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    command_id: u64,
    payload: &[u8],
    point: crate::gameplay_transport::pvp_live::faults::Point,
) -> TestResult {
    use crate::gameplay_transport::pvp_live::faults::{Point, arm};
    let mut pending = if point == Point::AfterCommit {
        Point::SwingHolderBusy
    } else {
        point
    };
    let mut consumed = arm(pending);
    let result = command(
        client,
        command_id,
        attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
        payload,
    )
    .await?;
    assert_eq!(
        attack::decode_attack_intent_result(&result).map_err(debug)?,
        attack::AttackIntentDisposition::Ok
    );
    // The serving loop alone drives combat and reconciliation. An explicit owner
    // drain here would change the cadence and contend for the harness's single
    // database holder. Observe the real fault and lane instead.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match consumed.try_recv() {
                Ok(()) if pending != point => {
                    // The accepted target must still swing after a busy checkout.
                    pending = point;
                    consumed = arm(point);
                }
                Ok(()) => break,
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                    wait_live(client, Duration::from_millis(25)).await?;
                }
                Err(error) => return Err(error.into()),
            }
        }
        loop {
            match owner.spell_lane.acquire().await {
                Ok(permit) => {
                    drop(permit);
                    return Ok::<_, Box<dyn std::error::Error>>(());
                }
                Err(unresolved) => drop(unresolved),
            }
            wait_live(client, Duration::from_millis(25)).await?;
        }
    })
    .await
    .map_err(|_| format!("live melee did not reach and reconcile {point:?}"))??;
    Ok(())
}

pub(super) fn scenario(mode: i16) -> TestResult {
    // One mechanic uses one database bootstrap at a time on the owner's shared cluster.
    static CASES: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _case = CASES
        .lock()
        .map_err(|_| "PvP test bootstrap gate poisoned")?;
    run(async move |admin| {
        let mut h =
            Harness::create_with_admission_seed(admin, "pvp_live_wire", false, false).await?;
        prepare_current_session(&h).await?;
        h.seed_pvp_target().await?;
        observer::admit(&h).await?;
        if mode == 16 {
            admit_splash_shooter(&h).await?;
        }
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let source = include_bytes!(
            "../../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        );
        let bundle_fixture = matches!(mode, 5 | 6 | 15);
        let (controller, pin) = production_generation_with_source(
            world,
            "content-1",
            (!bundle_fixture).then_some(source.as_slice()),
        )?;
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let bundle = if bundle_fixture {
            Some(bundle_fixture::world(world, mode == 5)?)
        } else {
            None
        };
        let cells = bundle
            .as_ref()
            .map(|bundle| {
                bundle
                    .movement_cells(pin.qualified_room().movement_cells())
                    .map_err(debug)
            })
            .transpose()?;
        let floor = if bundle_fixture { 7 } else { -7 };
        let room = match (&bundle, &cells) {
            (Some(bundle), Some(cells)) => pin
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    bundle.frame_binding_digest(world),
                    bundle.map_revision_digest(),
                    crate::content::NativeEntryStart { x: 2, y: 11, floor },
                )
                .ok_or("bundle combat room")?,
            _ => pin.qualified_room().clone(),
        };
        let cp = ChannelContentPin::from_activation(
            world,
            1,
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (2, if mode == 4 { 0 } else { 11 }, floor),
        );
        let mut deps = if mode == 16 {
            // Two shooters, an unarmoured victim, a rat and spawn admission headroom.
            Dependencies::with_content_and_capacity(
                cp.clone(),
                room.clone(),
                native.spell_book().clone(),
                7,
            )?
        } else {
            Dependencies::with_content(cp.clone(), room.clone(), native.spell_book().clone())?
        };
        if mode == 13 {
            let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
                world,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                3,
                cp,
            )
            .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            assert_eq!(
                runtime.commit_fresh_session(reservation).map_err(debug)?,
                deps.actor
            );
            deps.runtime = tokio::sync::Mutex::new(runtime);
        }
        deps.combat_room = Some(room);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        if let Some(cells) = cells.as_ref() {
            owner.movement_cells = (cells).into();
        }
        if mode == 15 {
            // Player-origin ticks use the fenced field owner and its bundle capability floor.
            owner.required_capabilities =
                crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                    &[11, 30],
                    crate::foundation::RuntimeScopeRefV1::channel(world, deps.admitted.channel_id),
                    1,
                    "policy-1",
                    owner.offered_capabilities(),
                )
                .map_err(debug)?;
        }
        let mut admitted = deps.admitted;
        let actor = deps.actor;
        let session = admitted.game_session_id;
        owner.admit_progression_session(&mut admitted).await;
        let progression = owner.progression_binding(session).ok_or("progression")?;
        let formula = native.training_formula().ok_or("formula")?;
        let druid =
            convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
        h.root
            .commit_character_build(
                &authority,
                &h.node,
                current_fence(&owner, session).await?,
                BuildChangeRequest {
                    occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                    cause: BuildCause::VocationChoice,
                    before: DurableBuildState::default(),
                    after: druid.clone(),
                    pruned_stance: None,
                },
                formula,
            )
            .await
            .map_err(debug)?;
        h.root
            .commit_character_experience(
                &authority,
                &h.node,
                current_fence(&owner, session).await?,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(98_800),
                    context: progression.context.clone(),
                    policy_revision: progression.policy_revision.clone(),
                    reward_revision: progression.reward_revision.clone(),
                    policy: progression.policy.clone(),
                },
            )
            .await
            .map_err(debug)?;
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        crate::spell::cast::CharacterCastFacts {
                            level: 20,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (0, 0),
                        owner.owner_now()
                    )
                    .is_some()
            );
            assert!(states.load_owned_build(&runtime, actor, session, &druid));
            states
                .get_mut(&runtime, actor, session)
                .ok_or("caster")?
                .enable_owned_training(&druid, formula)
                .map_err(debug)?;
        }
        let target = observer::initialize(&deps, &owner).await?;
        let target_session = GameSessionId::decode(&id(110)).map_err(debug)?;
        let mut target_admitted = admitted;
        target_admitted.game_session_id = target_session;
        target_admitted.runtime_actor = Some(target);
        owner.admit_progression_session(&mut target_admitted).await;
        {
            let mut runtime = deps.runtime.lock().await;
            let before = runtime.read_actor_position(target).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_cardinal(
                    before,
                    MovementLocalPosition {
                        x: 1,
                        y: if mode == 4 { 0 } else { 11 },
                        floor,
                    },
                )
                .map_err(debug)?;
        }
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1),(encode($2,'hex')::uuid,1)").bind(id(41).as_slice()).bind(id(101).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')").bind(id(42).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',$2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'pvp-live-fixture')").bind(id(42).as_slice()).bind(if mode==1 || mode==12 {1_i16}else if mode==8 {3_i16}else{2_i16}).execute(&h.pool).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        admitted.item_fence = item_fence_of(current);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            if mode == 9 {
                &[6, 17, 21][..]
            } else if mode == 15 {
                &[4, 6, 11, 15, 17, 18, 21, 30, 87][..]
            } else {
                &[6, 17, 21, 87][..]
            },
        )
        .ok_or("capabilities")?;
        let entity = {
            let runtime = deps.runtime.lock().await;
            let p = runtime
                .visible_entities()
                .players
                .into_iter()
                .find(|p| p.actor == target)
                .ok_or("visible player")?;
            EntityRef {
                identity: target.placement_identity(),
                generation: p.generation,
            }
        };
        if matches!(mode, 15 | 16) {
            wait_for_login_protection_to_expire(&h.pool, session).await?;
        }
        if mode == 16 {
            let second_shooter =
                observer::initialize_peer(&deps, &owner, SPLASH_SECOND_SHOOTER, SPLASH_SECOND_SESSION)
                    .await?;
            let second_session = GameSessionId::decode(&id(SPLASH_SECOND_SESSION)).map_err(debug)?;
            paid_splash(
                &h,
                &owner,
                (actor, session),
                (target, target_session),
                (second_shooter, second_session),
                native,
            )
            .await?;
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            return Ok(());
        }
        let original_health = health(&owner, target).await?;
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serving = async { serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await };
        let driving = async {
            let mut snapshot_body = Vec::new();
            loop {
                let f = super::item_look_fixtures::read_frame(&mut client).await?;
                let env = decode_wire_envelope(&f).map_err(debug)?;
                if env.message_type() == MessageType::SnapshotChunk {
                    let (_, bytes) =
                        oteryn_protocol_oteryn::decode_snapshot_chunk_framing(env.payload())
                            .map_err(debug)?;
                    snapshot_body.extend_from_slice(bytes);
                }
                if env.message_type() == MessageType::SnapshotCommit {
                    break;
                }
            }
            if mode >= 7 {
                let domains =
                    oteryn_protocol_oteryn::decode_snapshot_body(&snapshot_body).map_err(debug)?;
                let expert = domains.iter().find(|d| d.domain_id == 52);
                assert_eq!(expert.is_some(), mode != 9);
                if let Some(domain) = expert {
                    let state = oteryn_protocol_oteryn::pvp_expert::decode_expert_fight_modes(
                        domain.payload,
                    )
                    .map_err(debug)?;
                    let combat = domains
                        .iter()
                        .find(|d| d.domain_id == 10)
                        .ok_or("combat snapshot")?;
                    let combat =
                        attack::decode_actor_combat_state(combat.payload).map_err(debug)?;
                    assert_eq!(
                        combat.modes, state.modes,
                        "domain 10 and 52 share one mode tuple"
                    );
                    assert_eq!(
                        state.expert,
                        if mode == 8 {
                            oteryn_protocol_oteryn::pvp_expert::ExpertMode::RedFist
                        } else {
                            oteryn_protocol_oteryn::pvp_expert::ExpertMode::Dove
                        }
                    );
                }
            }
            if mode >= 7 {
                expert_cases::drive(
                    &mut client,
                    &owner,
                    &h.pool,
                    actor,
                    target,
                    session,
                    entity,
                    mode,
                )
                .await?;
                client.shutdown().await?;
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
            let target_command = if mode != 2 && mode != 6 {
                let result = command(
                    &mut client,
                    1,
                    attack::COMMAND_TYPE_FIGHT_MODES_INTENT,
                    &attack::encode_fight_modes_intent(&attack::FightModes {
                        secure: false,
                        ..attack::FightModes::DEFAULT
                    }),
                )
                .await?;
                assert_eq!(
                    attack::decode_attack_intent_result(&result).map_err(debug)?,
                    attack::AttackIntentDisposition::Ok
                );
                wait_live(&mut client, Duration::from_secs(10)).await?;
                2
            } else {
                1
            };
            let payload = attack::encode_attack_target_intent(Some(&entity)).map_err(debug)?;
            // Secure mode or post-login immunity (and Optional World) refuses with no harm.
            let result = command(
                &mut client,
                target_command,
                attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
                &payload,
            )
            .await?;
            assert_eq!(
                attack::decode_attack_intent_result(&result).map_err(debug)?,
                attack::AttackIntentDisposition::Rejected
            );
            if mode != 2 && mode != 6 {
                client.shutdown().await?;
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
            let modes = attack::FightModes {
                secure: false,
                ..attack::FightModes::DEFAULT
            };
            let result = command(
                &mut client,
                2,
                oteryn_protocol_oteryn::pvp_expert::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
                &oteryn_protocol_oteryn::pvp_expert::encode_expert_fight_modes(
                    &oteryn_protocol_oteryn::pvp_expert::ExpertFightModes {
                        modes,
                        expert: oteryn_protocol_oteryn::pvp_expert::ExpertMode::RedFist,
                    },
                ),
            )
            .await?;
            assert_eq!(
                attack::decode_attack_intent_result(&result).map_err(debug)?,
                attack::AttackIntentDisposition::Ok
            );
            wait_live(&mut client, Duration::from_secs(10)).await?;
            use crate::gameplay_transport::pvp_live::faults::Point;
            melee_fault(&mut client, &owner, 3, &payload, Point::BeforeCommit).await?;
            assert_eq!(health(&owner, target).await?, original_health);
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pvp_harm")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(count, 0, "a rolled-back attempt leaves no durable harm");
            let forfeited: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid AND adventurer_forfeited)")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert!(
                !forfeited,
                "rolled-back aggression keeps Adventurer's Blessing"
            );
            melee_fault(&mut client, &owner, 4, &payload, Point::AfterCommit).await?;
            assert!(
                health(&owner, target).await? < original_health,
                "live melee changes player HP"
            );
            let forfeiture: (bool, bool) = sqlx::query_as("SELECT (SELECT adventurer_forfeited FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid), (SELECT adventurer_forfeited FROM game_character_pvp_state WHERE character_id=encode($2,'hex')::uuid)")
                .bind(id(41).as_slice()).bind(id(101).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                forfeiture,
                (true, false),
                "first aggression forfeits only the attacker's Adventurer's Blessing"
            );
            let before_hold = health(&owner, target).await?;
            let result = command(
                &mut client,
                5,
                attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
                &payload,
            )
            .await?;
            assert_eq!(
                attack::decode_attack_intent_result(&result).map_err(debug)?,
                attack::AttackIntentDisposition::Ok
            );
            tokio::time::timeout(Duration::from_secs(6), async {
                loop {
                    let hits: i64 = sqlx::query_scalar(
                        "SELECT count(DISTINCT ordinal) FROM game_character_pvp_harm WHERE command_id=5 AND damage>0",
                    )
                    .fetch_one(&h.pool)
                    .await?;
                    if hits >= 2 {
                        return Ok::<_, Box<dyn std::error::Error>>(());
                    }
                    wait_live(&mut client, Duration::from_millis(25)).await?;
                }
            })
            .await
            .map_err(|_| "held ATTACK did not commit two distinct melee swings")??;
            assert!(health(&owner, target).await? < before_hold);
            let result = command(
                &mut client,
                6,
                attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
                &attack::encode_attack_target_intent(None).map_err(debug)?,
            )
            .await?;
            assert_eq!(
                attack::decode_attack_intent_result(&result).map_err(debug)?,
                attack::AttackIntentDisposition::Ok
            );
            let spell = (1..=deps.spells.source_len())
                .find_map(|i| {
                    std::num::NonZeroU32::new(u32::try_from(i).ok()?).filter(|i| {
                        deps.spells.source_indexed(*i).is_some_and(|(s, active)| {
                            active && s.key == "candidate:spell/flame_strike"
                        })
                    })
                })
                .ok_or("flame strike")?;
            let before = health(&owner, target).await?;
            let cast = actor_spell::SpellCastIntent {
                spell,
                target: actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
                    x: 1,
                    y: 11,
                    floor: floor.abs(),
                }),
                aim_at_target: true,
            };
            let result = command(
                &mut client,
                7,
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                &actor_spell::encode_spell_cast_intent(&cast),
            )
            .await?;
            assert_eq!(
                actor_spell::decode_spell_cast_result(&result).map_err(debug)?,
                actor_spell::SpellCastDisposition::Cast
            );
            assert!(
                health(&owner, target).await? < before,
                "real spell dispatch damages the player"
            );
            // The real root has one holder. Wait for the cadence's current pass to return it.
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            while !owner.refresh_pvp_locks(actor, session).await {
                if tokio::time::Instant::now() >= deadline {
                    return Err("PvP lock refresh did not obtain the durability holder".into());
                }
                wait_live(&mut client, Duration::from_millis(25)).await?;
            }
            let result = command(
                &mut client,
                8,
                oteryn_protocol_oteryn::logout::COMMAND_TYPE_LOGOUT_INTENT,
                &[],
            )
            .await?;
            assert!(matches!(
                oteryn_protocol_oteryn::logout::decode_logout_result(&result).map_err(debug)?,
                oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. }
            ));
            // A retained compact contributor must accept a later real hit without
            // deleting its unexpired identity (migration 0081's retention guard).
            sqlx::query("UPDATE game_character_pvp_ledger SET compact=true,compact_kind=1,buckets='[]'::jsonb WHERE victim_character_id=encode($1,'hex')::uuid AND contributor_character_id=encode($2,'hex')::uuid")
                .bind(id(101).as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
            {
                let mut states = owner.states.spell_states.lock().await;
                crate::gameplay_transport::actor_spell::tests::wound(
                    &mut states,
                    target,
                    target_session,
                    1,
                );
            }
            wait_live(&mut client, Duration::from_secs(2)).await?;
            let result = command(
                &mut client,
                9,
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                &actor_spell::encode_spell_cast_intent(&cast),
            )
            .await?;
            assert_eq!(
                actor_spell::decode_spell_cast_result(&result).map_err(debug)?,
                actor_spell::SpellCastDisposition::Cast
            );
            let compact: bool = sqlx::query_scalar("SELECT compact AND compact_kind=1 FROM game_character_pvp_ledger WHERE victim_character_id=encode($1,'hex')::uuid AND contributor_character_id=encode($2,'hex')::uuid")
                .bind(id(101).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert!(
                compact,
                "a live hit preserves the compact Damage contributor"
            );
            let death = {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                states
                    .player_death(&runtime, target, target_session)
                    .ok_or("PvP lethal hit records death")?
            };
            assert_eq!(
                settle_death_when_ready(&owner, target, target_session, death).await?,
                crate::gameplay_transport::SessionDeath::Durable
            );
            assert_eq!(
                settle_death_when_ready(&owner, target, target_session, death).await?,
                crate::gameplay_transport::SessionDeath::Durable
            );
            // SESSION-INGRESS-1 replays the latest retained CommandId from its durable
            // receipt instead of re-executing the cast after death.
            let harm_before: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_pvp_harm")
                    .fetch_one(&h.pool)
                    .await?;
            let frame = test_command(
                9,
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                &actor_spell::encode_spell_cast_intent(&cast),
            );
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            loop {
                let f = super::item_look_fixtures::read_frame(&mut client).await?;
                let env = decode_wire_envelope(&f).map_err(debug)?;
                if env.message_type() == MessageType::LivenessProbe {
                    ack(&mut client, env.payload()).await?;
                }
                if env.message_type() == MessageType::CommandResult {
                    let replay = decode_command_result(env.payload()).map_err(debug)?;
                    assert_eq!(replay.command_id, 9);
                    assert_eq!(replay.status, CommandStatus::DuplicateReplay);
                    assert_eq!(
                        actor_spell::decode_spell_cast_result(replay.payload).map_err(debug)?,
                        actor_spell::SpellCastDisposition::Cast
                    );
                    break;
                }
            }
            let harm_after: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_pvp_harm")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                harm_after, harm_before,
                "a replayed cast writes no new harm"
            );
            client.shutdown().await?;
            Ok(())
        };
        let (_, result) =
            tokio::time::timeout(Duration::from_secs(45), join_two(serving, driving)).await?;
        result?;
        let receipt_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pvp_harm")
            .fetch_one(&h.pool)
            .await?;
        if matches!(mode, 7 | 11 | 13..=15) {
            assert!(receipt_count > 0);
            assert!(health(&owner, target).await? < original_health);
        } else if mode != 2 && mode != 6 {
            assert_eq!(receipt_count, 0);
            assert_eq!(health(&owner, target).await?, original_health);
        } else {
            assert!(receipt_count >= 2);
            let points:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_unjustified_points WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                points, 1,
                "the live ledger feeds the existing death transaction once"
            );
            // Death may extend a contributor's lock after its local ordinary hit expires.
            *crate::gameplay_transport::pvp_live::lock_mut(
                &mut *owner.states.spell_states.lock().await,
                actor,
                session,
            ) = Default::default();
            assert!(
                owner
                    .hold_step_within(actor, session, Duration::from_millis(1))
                    .await
            );
        }
        let reload_fence = current_fence(&owner, session).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        if mode == 2 || mode == 6 {
            h.restart_appearance_root().await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let stored = h
                .root
                .read_pvp_state(&a, &h.node, reload_fence)
                .await
                .map_err(debug)?
                .ok_or("reloaded state")?;
            assert!(stored.logout_block_remaining_ms.is_some());
            assert!(stored.pz_block_remaining_ms.is_some());
            assert!(stored.kill_block_remaining_ms.is_some());
            let reloaded = deps.owner(&h, &a);
            assert!(reloaded.refresh_pvp_locks(actor, session).await);
            assert!(crate::gameplay_transport::pvp_live::logout_blocked(
                &*reloaded.states.spell_states.lock().await,
                actor,
                session,
                reloaded.owner_now()
            ));
        }
        h.cleanup().await
    })
}
