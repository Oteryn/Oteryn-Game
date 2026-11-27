//! A4/C17b: production spell completion under real Character and Content fences.
use super::death_ml_loss_composed_fixtures::{
    current_fence, production_generation_with_cosmetics, production_generation_with_source,
};
use super::fixture::{Harness, debug, id, run};
use super::item_look_fixtures::large_stack;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::foundation::{
    ChannelContentPin, CharacterId, CharacterLease, GameSessionId, MovementLocalPosition, WorldId,
};
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use crate::gameplay_transport::{
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{FreshAdmissionAuthority, prime_player_status_clock, regression_player_status},
    world_spatial::{EntityRef, SessionVisibility, VisibilityUpdate},
};
use oteryn_gameplay_domain::domain::condition::ConflictKey;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use oteryn_protocol_oteryn::zones_nologout::{PlayerPresence, PlayerStatus};

#[test]
fn a4_invisible_postgres_real_dispatch() -> TestResult {
    invisibility_case::<false, false, false>()
}

#[test]
fn invisible_cast_projects_spell_and_support_cooldowns() -> TestResult {
    invisibility_case::<false, false, true>()
}

#[test]
fn invisible_caster_stays_visible_to_players_hidden_from_monsters() -> TestResult {
    invisibility_case::<true, false, false>()
}

#[test]
fn invisible_player_swing_publishes_exact_rat_damage() -> TestResult {
    invisibility_case::<false, true, false>()
}

fn invisibility_case<
    const WITH_OBSERVER: bool,
    const WITH_ATTACK: bool,
    const WITH_COOLDOWNS: bool,
>() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    large_stack(|| {
        run(async |admin| {
            let h = if WITH_OBSERVER {
                Harness::create_appearance(admin, "a4_invisible").await?
            } else {
                Harness::create_with_admission_seed(admin, "a4_invisible", true, false).await?
            };
            prepare_current_session(&h).await?;
            if WITH_OBSERVER {
                h.seed_appearance_observer().await?;
                super::appearance_change_postgres_fixtures::observer_visibility_tests::admit(&h)
                    .await?;
            }
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = WorldId::decode(&id(42)).map_err(debug)?;
            let source = include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            );
            let (controller, pin) = if WITH_OBSERVER {
                let mut cosmetics = crate::content::native_gameplay::test_support::cosmetics_fixture::cosmetics_test_packet();
                cosmetics["authoring"]["document"]["content_revision"] = "content-1".into();
                production_generation_with_cosmetics(
                    world,
                    "content-1",
                    Some(source),
                    serde_json::to_vec(&cosmetics)?,
                )?
            } else {
                production_generation_with_source(world, "content-1", Some(source))?
            };
            let active = controller.active().ok_or("active")?;
            let native = active.native_gameplay().ok_or("native")?;
            let room = pin.qualified_room();
            let cp = ChannelContentPin::from_activation(
                world,
                1,
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                room.map_revision_digest(),
                (2, 11, -7),
            );
            let mut deps = Dependencies::with_content_and_capacity(
                cp.clone(),
                room.clone(),
                native.spell_book().clone(),
                if WITH_OBSERVER { 4 } else { 2 },
            )?;
            let (actor, session) = (deps.actor, deps.admitted.game_session_id);
            let (rat, guard) = {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                runtime
                    .bind_attacker_lease(
                        actor,
                        session,
                        CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                            .map_err(debug)?,
                    )
                    .map_err(debug)?;
                runtime
                    .install_companion_policies(native.policies().clone())
                    .map_err(debug)?;
                let rat = runtime
                    .realize_native_qualification_spawn(
                        cp.clone(),
                        "oteryn:creature.rat",
                        MovementLocalPosition {
                            x: if WITH_ATTACK { 1 } else { -2 },
                            y: 11,
                            floor: -7,
                        },
                    )
                    .map_err(debug)?
                    .actor;
                let guard = if WITH_OBSERVER {
                    Some(
                        runtime
                            .realize_native_qualification_spawn_for_source(
                                u16::MAX - 1,
                                cp,
                                "oteryn:creature.dwarf_guard",
                                MovementLocalPosition {
                                    x: -1,
                                    y: 12,
                                    floor: -7,
                                },
                            )
                            .map_err(debug)?
                            .actor,
                    )
                } else {
                    None
                };
                (rat, guard)
            };
            let (consumer, clock, _activation, _) = purchase_fixture::current(id(40));
            let premium =
                SpellPremiumCoordinator::new(consumer, Some(clock), None, Some(h.root.clone()));
            deps.combat_room = Some(room.clone());
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            owner.premium_coordinator = Some(&premium);
            owner.admit_premium(id(40), session);
            let formula = native.training_formula().ok_or("training")?;
            let build =
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
                        after: build.clone(),
                        pruned_stance: None,
                    },
                    formula,
                )
                .await
                .map_err(debug)?;
            let CastFactsLoad::Ready { facts, .. } = load_character_cast_facts(
                &h.root,
                &authority,
                current_fence(&owner, session).await?.character_id,
            )
            .await
            else {
                return Err("durable caster facts".into());
            };
            assert_eq!(facts.level, 50);
            {
                let runtime = deps.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                states
                    .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
                    .ok_or("caster")?;
                assert!(states.load_owned_build(&runtime, actor, session, &build));
                states
                    .get_mut(&runtime, actor, session)
                    .ok_or("caster")?
                    .enable_owned_training(&build, formula)
                    .map_err(debug)?;
            }
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            let spell = (1..=u32::try_from(native.spell_book().source_len())?)
                .filter_map(std::num::NonZeroU32::new)
                .find(|i| {
                    native
                        .spell_book()
                        .indexed(*i)
                        .is_some_and(|s| s.name.eq_ignore_ascii_case("Invisibility"))
                })
                .ok_or("spell index")?;
            owner.clock_origin -= std::time::Duration::from_secs(10);
            owner.drain_monster_melee().await;
            let guard_before = {
                let states = owner.states.spell_states.lock().await;
                let ai = &states.monster_melee.creature_ai;
                assert_eq!(ai.state(rat).ok_or("rat AI")?.target(), Some(actor));
                if let Some(guard) = guard {
                    let report = ai.last_report_for_test(guard).ok_or("dwarf guard think")?;
                    assert_eq!(report.target, Some(actor));
                    Some((guard, report.sequence))
                } else {
                    None
                }
            };
            let observer = if WITH_OBSERVER {
                Some(
                    super::appearance_change_postgres_fixtures::observer_visibility_tests::initialize(
                        &deps, &owner,
                    )
                    .await?,
                )
            } else {
                None
            };
            let observation = if let Some(observer) = observer {
                prime_player_status_clock(&owner, deps.admitted);
                let mut reader = deps.admitted;
                reader.runtime_actor = Some(observer);
                reader.game_session_id = GameSessionId::decode(&id(110)).map_err(debug)?;
                reader.continuity.selected_capabilities =
                    SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, 600])
                        .ok_or("player status capability")?;
                let target = EntityRef {
                    identity: actor.placement_identity(),
                    generation: actor.actor_local_generation(),
                };
                let channel = owner
                    .observe_visible_entities(observer)
                    .await
                    .ok_or("visible caster")?;
                let mut view = SessionVisibility::default();
                view.snapshot(&channel, &mut |_| Ok(())).map_err(debug)?;
                assert!(view.shows(target), "the caster is initially visible");
                let visible = regression_player_status(&owner, reader, target, &view)
                    .await
                    .and_then(|bytes| PlayerStatus::decode(&bytes))
                    .ok_or("visible player status")?;
                assert_eq!(visible.presence, PlayerPresence::Online);
                assert!(!visible.no_logout, "the pinned caster tile permits logout");
                assert!(owner.appearance_available());
                let appearance = owner
                    .appearance_observation_projected(
                        observer,
                        reader.game_session_id,
                        true,
                        vec![target],
                    )
                    .await
                    .ok_or("visible caster appearance")?;
                let look = appearance
                    .bindings
                    .first()
                    .ok_or("caster look")?
                    .look
                    .clone();
                let attack = owner
                    .attack_target(observer, reader.game_session_id, 2, Some(target))
                    .await;
                Some((observer, reader, target, view, look, attack))
            } else {
                None
            };
            let cooldown_clock = WITH_COOLDOWNS.then(|| owner.freeze_owner_clock_for_test());
            let outcome = owner
                .cast_spell_completion(
                    actor,
                    session,
                    1,
                    SpellCastIntent {
                        spell,
                        target: SpellTarget::None,
                        aim_at_target: false,
                    },
                )
                .await
                .ok_or("terminal spell completion")?;
            assert_eq!(
                outcome.disposition,
                SpellCastDisposition::Cast,
                "C17b donor Invisible must cast"
            );
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let conditions = &states
                .get(&runtime, actor, session)
                .ok_or("caster")?
                .conditions;
            let instance = conditions
                .get(ConflictKey::Invisible)
                .ok_or("invisibility condition")?;
            let start = instance.started_at();
            assert!(conditions.invisible_at(start + 199_999_999));
            assert!(!conditions.invisible_at(start + 200_000_000));
            drop(states);
            drop(runtime);
            if WITH_COOLDOWNS {
                assert_invisible_cooldowns(&owner, deps.admitted, spell, start).await?;
            }
            drop(cooldown_clock);
            if let Some((observer, reader, target, mut view, look, attack)) = observation {
                let channel = owner
                    .observe_visible_entities(observer)
                    .await
                    .ok_or("observer visibility")?;
                match view.refresh(&channel, &mut |_| Ok(())).map_err(debug)? {
                    VisibilityUpdate::Delta(delta) => assert!(
                        !delta.leave.contains(&target),
                        "Invisible must not send a leave for the caster"
                    ),
                    VisibilityUpdate::Unchanged => {}
                    VisibilityUpdate::Snapshot(_) => {
                        return Err("expected an incremental refresh".into());
                    }
                }
                assert!(
                    view.shows(target),
                    "the observer keeps the caster's incarnation"
                );
                let status = regression_player_status(&owner, reader, target, &view)
                    .await
                    .and_then(|bytes| PlayerStatus::decode(&bytes))
                    .ok_or("invisible caster's player status")?;
                assert_eq!(status.presence, PlayerPresence::Online);
                assert!(!status.no_logout);
                let own = owner
                    .appearance_observation_projected(actor, session, true, vec![target])
                    .await
                    .ok_or("own invisible appearance")?;
                assert!(matches!(
                    own.bindings[0].look,
                    oteryn_protocol_oteryn::appearance::Look::Outfit { .. }
                ));
                for empty_outfit in [false, true] {
                    let projected = owner
                        .appearance_observation_projected(
                            observer,
                            reader.game_session_id,
                            empty_outfit,
                            vec![target],
                        )
                        .await
                        .ok_or("invisible caster appearance")?;
                    assert_eq!(projected.bindings.first().ok_or("caster look")?.look, look);
                }
                assert_eq!(
                    owner
                        .attack_target(observer, reader.game_session_id, 3, Some(target))
                        .await,
                    attack,
                    "Invisible does not change eligibility under the existing PvP rules"
                );
            }
            owner.clock_origin -= std::time::Duration::from_secs(2);
            owner.drain_monster_melee().await;
            {
                let states = owner.states.spell_states.lock().await;
                let ai = &states.monster_melee.creature_ai;
                let rat_target = ai.state(rat).ok_or("rat AI")?.target();
                if WITH_OBSERVER {
                    assert_ne!(
                        rat_target,
                        Some(actor),
                        "the rat drops the invisible caster"
                    );
                } else {
                    assert_eq!(rat_target, None, "the rat loses its only target");
                }
                if let Some((guard, before)) = guard_before {
                    let report = ai.last_report_for_test(guard).ok_or("dwarf guard think")?;
                    assert!(
                        report.sequence > before,
                        "the guard thinks again after the cast"
                    );
                    assert_eq!(
                        report.target,
                        Some(actor),
                        "the guard can target the invisible caster"
                    );
                }
            }
            if let Some(observer) = observer {
                // Stop the real logout path at the durable commit, before actor retirement.
                // An observer can refresh in this window while the old incarnation still
                // has a Channel position.
                let binding = deps.admitted.controller.ok_or("caster controller")?;
                let store = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
                let released = store.release_logout_session(
                    session,
                    &crate::gameplay_transport::canonical_uuid(&binding.account_id),
                    binding.transport,
                ).await?;
                assert!(matches!(
                    released,
                    crate::durability::fresh_admission::ExpiredLossReleaseV1::Released { .. }
                ));
                let raw = owner.observe_visible_entities(observer).await.ok_or("raw projection")?;
                assert!(raw.entities.iter().any(|entity| entity.identity == actor.placement_identity()));
                let named = owner.named_channel_entities(observer, false).await.map_err(debug)?;
                assert!(named.entities.iter().all(|entity| entity.identity != actor.placement_identity()));
                let observer_session = GameSessionId::decode(&id(110)).map_err(debug)?;
                let retired = EntityRef {
                    identity: actor.placement_identity(),
                    generation: actor.actor_local_generation(),
                };
                let observation = owner.appearance_observation_projected(
                    observer,
                    observer_session,
                    true,
                    vec![retired],
                ).await.ok_or("terminal peer cannot abort the observer's appearance projection")?;
                assert_eq!(observation.skipped, vec![retired]);
                eprintln!("PG_EXECUTED invisible_caster_stays_visible_to_players_hidden_from_monsters terminal_gap=verified");
            }
            if WITH_ATTACK {
                let target = EntityRef {
                    identity: rat.placement_identity(),
                    generation: rat.actor_local_generation(),
                };
                let attacker = EntityRef {
                    identity: actor.placement_identity(),
                    generation: actor.actor_local_generation(),
                };
                let initial_health = deps.runtime.lock().await.companion_snapshot(rat).map_err(debug)?.health;
                let cursor = owner.observe_sounds(actor, session, u64::MAX).await.ok_or("presentation cursor")?.0;
                assert_eq!(
                    owner.attack_target(actor, session, 2, Some(target)).await,
                    oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
                );
                let mut health = initial_health;
                for _ in 0..16 {
                    owner.clock_origin -= std::time::Duration::from_secs(2);
                    owner.refresh_attack_weapon(actor, session).await;
                    owner.drain_auto_attacks().await;
                    health = deps.runtime.lock().await.companion_snapshot_including_dead(rat)
                        .map_or(0, |snapshot| snapshot.health);
                    if health < initial_health {
                        break;
                    }
                }
                assert!(health < initial_health, "the invisible caster's swing must damage the exact Rat");
                let (_, events) = owner.observe_sounds(actor, session, cursor).await.ok_or("damage presentation")?;
                use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationValueKind};
                assert!(events.iter().any(|event| matches!(
                    event.kind,
                    PresentationEventKind::ValueText {
                        kind: PresentationValueKind::DamageDealt,
                        target: victim,
                        attacker: Some(source),
                        primary,
                        ..
                    } if victim == target && source == attacker
                        && primary.value == u64::try_from(initial_health - health).unwrap_or_default()
                )), "committed Rat damage must publish the exact attacker, target and health loss: {events:?}");
                eprintln!("PG_EXECUTED invisible_player_swing_publishes_exact_rat_damage");
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

async fn assert_invisible_cooldowns(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    spell: std::num::NonZeroU32,
    start: u64,
) -> TestResult {
    use crate::gameplay_transport::connection::CooldownProjection;
    use oteryn_protocol_oteryn::presentation as wire;
    let actor = admitted.runtime_actor.ok_or("caster")?;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[6, wire::CAPABILITY_PRESENTATION_V1],
    )
    .ok_or("presentation selection")?;
    let (mut server, mut client) = tokio::io::duplex(4096);
    let mut projection = CooldownProjection::default();
    let mut sequence = admitted.continuity.server_sequence;
    projection
        .emit(&mut server, owner, actor, &mut admitted, &mut sequence)
        .await
        .map_err(debug)?;
    drop(server);
    let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut client).await?;
    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?;
    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
    assert_eq!(
        (delta.domain_id, delta.base_revision, delta.new_revision),
        (wire::STATE_DOMAIN_ACTOR_COOLDOWNS, 0, 1)
    );
    let value = wire::decode_actor_cooldowns(delta.payload).map_err(debug)?;
    let definition = owner.spells.indexed(spell).ok_or("authored Invisible")?;
    let support = definition
        .groups
        .iter()
        .find(|group| group.key == "support")
        .ok_or("authored Support group")?;
    let group = u32::try_from(crate::gameplay_transport::spell_catalog::cooldown_group_keys()
        .and_then(|groups| groups.iter().position(|key| key == &support.key))
        .ok_or("Support ordinal")? + 1)?;
    assert_eq!(
        value.entries,
        [
            wire::ActorCooldown {
                kind: wire::ActorCooldownKind::Spell,
                id: spell.get(),
                expires_at_ms: (start + definition.cooldown_micros).div_ceil(1000)
            },
            wire::ActorCooldown {
                kind: wire::ActorCooldownKind::Group,
                id: group,
                expires_at_ms: (start + support.cooldown_micros).div_ceil(1000)
            },
        ]
    );
    Ok(())
}
