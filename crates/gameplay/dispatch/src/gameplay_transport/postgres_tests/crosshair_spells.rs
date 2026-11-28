//! C18b: crosshair and legacy position requests through the wire command, with real Character,
//! Equipment and Premium owners. Barrages require a held target.
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
    ChannelContentPin, CharacterLease, MessageType, WorldId, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, actor_spell, attack, decode_command_result,
    encode_client_command, world_spatial_entities::EntityRef,
};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// What one vocation's scenario casts: the durable vocation and experience that reach every
/// spell's level, the weapon the formulas read, and `(key, level, mana)` of each spell.
#[derive(Clone, Copy)]
struct Plan {
    tag: &'static str,
    vocation: &'static str,
    experience: i64,
    weapon: &'static str,
    spells: &'static [(&'static str, u32, i64)],
}

#[derive(Clone, Copy)]
enum Targeting {
    PositionOnly,
    MissingHeldTarget,
    ClickHeldTarget,
    ClickOtherCreature,
    HeldPlayer,
}

/// Each rat's health before every cast: far above any one hit.
const RAT_HEALTH: i64 = 10_000;

const PALADIN: Plan = Plan {
    tag: "crosshair_paladin",
    vocation: "paladin",
    // Level 70 needs 5_246_300 experience (50/3 * (L^3 - 6L^2 + 17L - 12)).
    experience: 6_000_000,
    // A DISTANCE weapon with a known attack: Ethereal Barrage reads attack_skill.
    weapon: "oteryn:item.tibia.i3277",
    spells: &[
        ("candidate:spell/divine_barrage", 70, 175),
        ("candidate:spell/ethereal_barrage", 60, 135),
    ],
};

#[test]
fn crosshair_spells_postgres_paladin_barrages_hit_position_target() -> TestResult {
    scenario(PALADIN, Targeting::ClickHeldTarget)
}

#[test]
fn crosshair_spells_postgres_paladin_barrages_require_held_target() -> TestResult {
    scenario(
        Plan {
            tag: "crosshair_paladin_no_target",
            ..PALADIN
        },
        Targeting::MissingHeldTarget,
    )
}

#[test]
fn crosshair_spells_postgres_paladin_barrages_prefer_held_target() -> TestResult {
    scenario(
        Plan {
            tag: "crosshair_paladin_other_tile",
            ..PALADIN
        },
        Targeting::ClickOtherCreature,
    )
}

#[test]
fn crosshair_spells_postgres_paladin_barrages_accept_held_player_in_both_encodings() -> TestResult {
    scenario(
        Plan { tag: "crosshair_pvp_paladin", ..PALADIN },
        Targeting::HeldPlayer,
    )
}

#[test]
fn crosshair_spells_postgres_monk_thousand_fist_blows_hits_position_target() -> TestResult {
    scenario(
        Plan {
            tag: "crosshair_monk",
            vocation: "monk",
            // Level 120 needs 27_393_800 experience.
            experience: 30_000_000,
            // A FIST weapon with a known attack: Thousand Fist Blows reads attack_skill.
            weapon: "oteryn:item.tibia.i50157",
            spells: &[("candidate:spell/thousand_fist_blows", 120, 145)],
        },
        Targeting::PositionOnly,
    )
}

fn scenario(plan: Plan, targeting: Targeting) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            run(async move |admin| {
                for native_frame in [false, true] {
                    scenario_inner(admin.clone(), plan, targeting, native_frame).await?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "crosshair spell thread panicked")?
        .map_err(Into::into)
}

async fn scenario_inner(
    admin: String,
    plan: Plan,
    targeting: Targeting,
    native_frame: bool,
) -> TestResult {
    use super::appearance_change_postgres_fixtures::observer_visibility_tests as observer;
    use oteryn_protocol_oteryn::pvp_expert::{ExpertFightModes, ExpertMode};

    let pvp = matches!(targeting, Targeting::HeldPlayer);
    let h = Harness::create_with_admission_seed(admin, plan.tag, false, false).await?;
    prepare_current_session(&h).await?;
    if pvp {
        h.seed_pvp_target().await?;
        observer::admit(&h).await?;
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = production_generation_with_source(
        world,
        "content-1",
        Some(include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        )),
    )
    .map_err(debug)?;
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let bundle = native_frame.then(|| {
        crate::map::spell_tiles::tests::movement_arena::rectangle((0, 0), (16, 16), -7)
    });
    let room = if let Some(bundle) = &bundle {
        pin.qualified_room()
            .bundle_view(
                bundle.movement_cells(pin.qualified_room().movement_cells()).map_err(debug)?,
                bundle.frame_binding_digest(world),
                bundle.map_revision_digest(),
                crate::content::NativeEntryStart { x: 2, y: 11, floor: 7 },
            )
            .ok_or("bundle room")?
    } else {
        pin.qualified_room().clone()
    };
    let cp = ChannelContentPin::from_activation(
        world,
        1,
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (2, 11, if native_frame { 7 } else { -7 }),
    );
    let mut deps = Dependencies::with_content_and_capacity(
        cp,
        room.clone(),
        native.spell_book().clone(),
        5,
    )?;
    deps.combat_room = Some(room);
    // Every spell here is premium: the account holds a Current Premium.
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    owner.premium_coordinator = Some(&premium);
    let mut admitted = deps.admitted;
    let actor = deps.actor;
    let session = admitted.game_session_id;
    owner.admit_premium(id(40), session);
    owner.admit_progression_session(&mut admitted).await;
    let progression = owner.progression_binding(session).ok_or("progression")?;
    let formula = native.training_formula().ok_or("training")?;
    let build =
        convert_vocation(formula, &DurableBuildState::default(), plan.vocation).map_err(debug)?;
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: build.clone(),
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
                amount: oteryn_simulation_determinism::ExactI64::new(plan.experience),
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    // The caster's facts are the durable ones, so they equal the Character owner's.
    let CastFactsLoad::Ready { facts, .. } = load_character_cast_facts(
        &h.root,
        &authority,
        current_fence(&owner, session).await?.character_id,
    )
    .await
    else {
        return Err("durable caster facts".into());
    };
    for (key, level, mana) in plan.spells {
        assert!(i64::from(facts.level) >= i64::from(*level), "{key} level");
        assert!(i64::from(facts.max_mana) >= *mana, "{key} max mana");
    }
    let (target, other, target_position, other_position, target_ref);
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
        let p = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        assert_eq!(p.floor, if native_frame { 7 } else { -7 });
        target_position = crate::foundation::MovementLocalPosition {
            x: p.x + 1,
            y: p.y,
            floor: p.floor,
        };
        // The rats are three tiles apart, outside each other's 5-by-5 spell area.
        other_position = crate::foundation::MovementLocalPosition {
            x: p.x - 2,
            ..target_position
        };
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
        let health = runtime
            .companion_policy("oteryn:creature.rat")
            .map_err(debug)?
            .maximum_health;
        runtime
            .realize_activation_spawns(vec![(
                vec![
                    (target_position.x, target_position.y, target_position.floor),
                    (other_position.x, other_position.y, other_position.floor),
                ],
                60_000,
                5_000,
                "oteryn:creature.rat".into(),
                u64::try_from(health)?,
            )])
            .map_err(|error| format!("realize crosshair target spawn: {error:?}"))?;
        let visible = runtime.visible_entities();
        let rat_at = |position| {
            visible
                .creatures
                .iter()
                .find(|rat| rat.position == position)
        };
        let rat = rat_at(target_position).ok_or("first rat")?;
        target = rat.actor;
        target_ref = EntityRef {
            identity: target.placement_identity(),
            generation: rat.generation,
        };
        other = rat_at(other_position).ok_or("second rat")?.actor;
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &build));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("caster")?
            .enable_owned_training(&build, formula)
            .map_err(debug)?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
    {
        let mut tx = h.pool.begin().await?;
        // Fixture custody only; tested equipment reads use the production fences.
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
            .bind(id(112).as_slice()).bind(id(42).as_slice()).bind(plan.weapon).execute(&mut *tx).await?;
        // Slot 5 is the weapon hand the attack formulas read.
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$4,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(112).as_slice()).bind(5_i16).execute(&mut *tx).await?;
        // Combat mode 1 is Attack; the attack formulas refuse an unknown mode.
        sqlx::query("UPDATE game_character_equipment_state SET combat_mode=1 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    h.root
        .initialize_familiar_group(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
        )
        .await
        .map_err(debug)?;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?
            .0;
    admitted.item_fence = item_fence_of(current);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7])
            .ok_or("chat capabilities")?;
    let player = if pvp {
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(101).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
            .bind(id(42).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'crosshair-pvp-fixture')")
            .bind(id(42).as_slice()).execute(&h.pool).await?;
        let victim = observer::initialize(&deps, &owner).await?;
        let target_session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
        let target_ref = {
            let mut runtime = deps.runtime.lock().await;
            let before = runtime.read_actor_position(victim).map_err(debug)?;
            runtime.borrow_movement_position().commit_cardinal(before, crate::foundation::MovementLocalPosition {
                x: target_position.x - 2, ..target_position
            }).map_err(debug)?;
            let visible = runtime.visible_entities();
            let row = visible.players.iter().find(|entry| entry.actor == victim).ok_or("visible player")?;
            EntityRef { identity: victim.placement_identity(), generation: row.generation }
        };
        super::pvp_live_fixtures::wait_for_login_protection_to_expire(&h.pool, session).await?;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7, 17, 21, 87]).ok_or("PvP capabilities")?;
        assert_eq!(owner.expert_fight_modes(actor, session, admitted.continuity.next_command_id,
            ExpertFightModes { modes: attack::FightModes { secure: false, ..attack::FightModes::DEFAULT }, expert: ExpertMode::RedFist }).await.map_err(debug)?,
            attack::AttackIntentDisposition::Ok);
        Some((victim, target_session, target_ref))
    } else {
        None
    };
    let position = if matches!(targeting, Targeting::ClickOtherCreature) {
        other_position
    } else {
        target_position
    };
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    let casts = plan.spells.iter().flat_map(|spell| {
        [false, true].into_iter().take(if pvp { 2 } else { 1 }).map(move |hotkey| (spell, hotkey))
    });
    for (index, ((key, _, mana), hotkey)) in casts.enumerate() {
        let spell = (1..=deps.spells.source_len())
            .find_map(|i| {
                std::num::NonZeroU32::new(u32::try_from(i).ok()?).filter(|i| {
                    deps.spells
                        .source_indexed(*i)
                        .is_some_and(|(s, active)| active && s.key == *key)
                })
            })
            .ok_or("spell index")?;
        let (definition, _) = deps.spells.source_indexed(spell).ok_or("spell profile")?;
        let policy = &definition.authored.as_ref().ok_or("cast policy")?.header.targeting;
        let needs_target = !matches!(targeting, Targeting::PositionOnly);
        assert_eq!(policy.cast_at_position, Some(true), "{key}");
        assert_eq!(policy.needs_target, needs_target, "{key}");
        if index > 0 {
            // Clear the 2 s attack group cooldown of the previous cast.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(5))
                .ok_or("clock advance")?;
        }
        {
            let mut runtime = deps.runtime.lock().await;
            for rat in [target, other] {
                runtime
                    .set_test_creature_health(rat, RAT_HEALTH)
                    .map_err(debug)?;
            }
        }
        if matches!(
            targeting,
            Targeting::ClickHeldTarget | Targeting::ClickOtherCreature | Targeting::HeldPlayer
        ) {
            assert_eq!(
                owner
                    .select_attack_target(
                        actor,
                        session,
                        admitted.continuity.next_command_id,
                        Some(player.map_or(target_ref, |player| player.2))
                    )
                    .await,
                attack::AttackIntentDisposition::Ok,
                "{key} select held target"
            );
        }
        let before = owner
            .observe_vitals(actor, session)
            .await
            .ok_or("before vitals")?
            .1
            .mana;
        let victim_health = if let Some((victim, target_session, _)) = player {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, victim, target_session).ok_or("victim")?;
            state.restore_saved_vitals(state.character_facts().max_health, 0);
            Some(state.vitals().health)
        } else {
            None
        };
        let payload = actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
            spell,
            target: if hotkey {
                actor_spell::SpellTarget::AttackTarget
            } else {
                actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
                    x: position.x, y: position.y, floor: position.floor.abs(),
                })
            },
            aim_at_target: false,
        });
        let results = exchange(
            &owner,
            &mut admitted,
            &[(
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                payload,
            )],
        )
        .await?;
        let disposition = actor_spell::decode_spell_cast_result(&results[0].1).map_err(debug)?;
        let damaged = {
            let runtime = deps.runtime.lock().await;
            let health = u64::try_from(RAT_HEALTH)?;
            let mut damaged = [false; 2];
            for (hit, rat) in damaged.iter_mut().zip([target, other]) {
                let remaining = runtime.live_creature_health(rat).ok_or("rat survives")?;
                assert!(remaining <= health, "{key} unexpectedly healed a rat");
                *hit = remaining < health;
            }
            if let (Some((victim, target_session, _)), Some(health)) = (player, victim_health) {
                let states = owner.states.spell_states.lock().await;
                let after = states.get(&runtime, victim, target_session).ok_or("victim after cast")?.vitals().health;
                vec![after < health]
            } else {
                damaged.to_vec()
            }
        };
        let after = owner
            .observe_vitals(actor, session)
            .await
            .ok_or("after vitals")?
            .1
            .mana;
        observed.push((
            *key,
            disposition,
            damaged,
            i64::from(before) - i64::from(after),
        ));
        let outcome = if matches!(targeting, Targeting::MissingHeldTarget) {
            (
                actor_spell::SpellCastDisposition::TargetRequired,
                vec![false, false],
                0,
            )
        } else {
            (
                actor_spell::SpellCastDisposition::Cast,
                if pvp { vec![true] } else { vec![true, false] },
                *mana,
            )
        };
        expected.push((*key, outcome.0, outcome.1, outcome.2));
    }
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await?;
    assert_eq!(
        observed, expected,
        "disposition, target damage, and mana spent (native frame: {native_frame})"
    );
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u32, Vec<u8>)],
) -> TestResult<Vec<(CommandStatus, Vec<u8>)>> {
    let session = *admitted;
    let first_command_id = admitted.continuity.next_command_id;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async move { serve_admitted(&mut server, session, owner, IDLE_LIVENESS).await };
    let driving = async move {
        loop {
            let frame = super::item_look_fixtures::read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let mut results = Vec::new();
        for (offset, (kind, payload)) in commands.iter().enumerate() {
            let frame = encode_client_command(
                session.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: first_command_id + offset as u64,
                    command_type: *kind,
                    payload,
                },
            )
            .map_err(debug)?;
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            loop {
                let frame = super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    results.push((result.status, result.payload.to_vec()));
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok(results)
    };
    let (end, results): (_, TestResult<_>) =
        tokio::time::timeout(Duration::from_secs(45), join_two(serving, driving)).await?;
    match end {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current) => {
            *admitted = current
        }
        other => return Err(format!("crosshair serving ended: {other:?}").into()),
    }
    results
}
