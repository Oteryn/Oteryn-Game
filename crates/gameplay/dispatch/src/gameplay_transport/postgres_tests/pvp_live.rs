//! Existing admitted-session transport -> PvP rules -> PostgreSQL -> result bytes and reload.
use super::appearance_change_postgres::observer_visibility_tests as observer;
use super::death_ml_loss_composed::{current_fence, production_generation_with_source};
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
use tokio::io::{AsyncWriteExt, DuplexStream};

#[test]
fn pvp_live_wire_melee_spell_death_replay_and_reload() -> TestResult {
    super::item_look::large_stack(|| scenario(2))
}
#[test]
fn pvp_live_optional_world_refuses_player_harm_without_durable_writes() -> TestResult {
    super::item_look::large_stack(|| scenario(1))
}

#[test]
fn pvp_live_protection_zone_refuses_player_harm() -> TestResult {
    super::item_look::large_stack(|| scenario(4))
}
#[test]
fn pvp_live_no_pvp_tile_refuses_player_harm() -> TestResult {
    super::item_look::large_stack(|| scenario(5))
}

#[test]
fn pvp_live_bundle_melee_spell_death_replay_and_reload() -> TestResult {
    super::item_look::large_stack(|| scenario(6))
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
        let frame = super::item_look::read_frame(client).await?;
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
        match tokio::time::timeout_at(deadline, super::item_look::read_frame(client)).await {
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
fn scenario(mode: i16) -> TestResult {
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
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let source = include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        );
        let (controller, pin) = production_generation_with_source(
            world,
            "content-1",
            (mode != 5 && mode != 6).then_some(source.as_slice()),
        )?;
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let bundle = if mode == 5 || mode == 6 {
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
        let floor = if mode == 5 || mode == 6 { 7 } else { -7 };
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
        let mut deps = Dependencies::with_content(cp, room.clone(), native.spell_book().clone())?;
        deps.combat_room = Some(room);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        if let Some(cells) = cells.as_ref() {
            owner.movement_cells = cells;
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
        sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',$2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'pvp-live-fixture')").bind(id(42).as_slice()).bind(if mode==1 {1_i16}else{2_i16}).execute(&h.pool).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        admitted.item_fence = item_fence_of(current);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 17, 21])
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
        let original_health = health(&owner, target).await?;
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serving = async { serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await };
        let driving = async {
            loop {
                let f = super::item_look::read_frame(&mut client).await?;
                if decode_wire_envelope(&f).map_err(debug)?.message_type()
                    == MessageType::SnapshotCommit
                {
                    break;
                }
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
                attack::COMMAND_TYPE_FIGHT_MODES_INTENT,
                &attack::encode_fight_modes_intent(&modes),
            )
            .await?;
            assert_eq!(
                attack::decode_attack_intent_result(&result).map_err(debug)?,
                attack::AttackIntentDisposition::Ok
            );
            wait_live(&mut client, Duration::from_secs(10)).await?;
            use crate::gameplay_transport::pvp_live::faults::{Point, arm};
            arm(Point::BeforeCommit);
            let result = command(
                &mut client,
                3,
                attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
                &payload,
            )
            .await?;
            assert_eq!(
                attack::decode_attack_intent_result(&result).map_err(debug)?,
                attack::AttackIntentDisposition::Ok
            );
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            drop(
                owner
                    .spell_lane_permit()
                    .await
                    .ok_or("resolve rolled-back hit")?,
            );
            assert_eq!(health(&owner, target).await?, original_health);
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pvp_harm")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(count, 0, "a rolled-back attempt leaves no durable harm");
            arm(Point::AfterCommit);
            wait_live(&mut client, Duration::from_millis(2100)).await?;
            let result = command(
                &mut client,
                4,
                attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
                &payload,
            )
            .await?;
            assert_eq!(
                attack::decode_attack_intent_result(&result).map_err(debug)?,
                attack::AttackIntentDisposition::Ok
            );
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            drop(
                owner
                    .spell_lane_permit()
                    .await
                    .ok_or("resolve committed hit")?,
            );
            assert!(
                health(&owner, target).await? < original_health,
                "live melee changes player HP"
            );
            let result = command(
                &mut client,
                5,
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
                    floor,
                }),
                aim_at_target: true,
            };
            let result = command(
                &mut client,
                6,
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
                7,
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
                8,
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
                owner
                    .settle_death(target, target_session, death, false)
                    .await
                    .0,
                crate::gameplay_transport::SessionDeath::Durable
            );
            assert_eq!(
                owner
                    .settle_death(target, target_session, death, false)
                    .await
                    .0,
                crate::gameplay_transport::SessionDeath::Durable
            );
            // SESSION-INGRESS-1 replays the latest retained CommandId from its durable
            // receipt instead of re-executing the cast after death.
            let harm_before: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_pvp_harm")
                    .fetch_one(&h.pool)
                    .await?;
            let frame = test_command(
                8,
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                &actor_spell::encode_spell_cast_intent(&cast),
            );
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            loop {
                let f = super::item_look::read_frame(&mut client).await?;
                let env = decode_wire_envelope(&f).map_err(debug)?;
                if env.message_type() == MessageType::LivenessProbe {
                    ack(&mut client, env.payload()).await?;
                }
                if env.message_type() == MessageType::CommandResult {
                    let replay = decode_command_result(env.payload()).map_err(debug)?;
                    assert_eq!(replay.command_id, 8);
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
            assert_eq!(harm_after, harm_before, "a replayed cast writes no new harm");
            client.shutdown().await?;
            Ok(())
        };
        let (_, result) =
            tokio::time::timeout(Duration::from_secs(45), join_two(serving, driving)).await?;
        result?;
        let receipt_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pvp_harm")
            .fetch_one(&h.pool)
            .await?;
        if mode != 2 && mode != 6 {
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
