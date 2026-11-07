//! A4/C17b: production spell completion under real Character and Content fences.
use super::death_ml_loss_composed::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id, run};
use super::item_look::large_stack;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::foundation::{
    ChannelContentPin, CharacterId, CharacterLease, GameSessionId, MovementLocalPosition, WorldId,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_domain::domain::condition::ConflictKey;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};

#[test]
fn a4_invisible_postgres_real_dispatch() -> TestResult {
    invisibility_case::<false>()
}

#[test]
fn a4_invisible_postgres_other_player_cannot_see_or_target_caster() -> TestResult {
    invisibility_case::<true>()
}

fn invisibility_case<const WITH_OBSERVER: bool>() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
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
                super::appearance_change_postgres::observer_visibility_tests::admit(&h).await?;
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
                use crate::content::native_gameplay::{NativeGameplayInput, PinnedGameplayBytes};
                use crate::content::{
                    ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
                    activate_native_entry_room_with_gameplay,
                };
                use sha2::Digest;
                let mut supplied = NativeGameplayInput::from_manifest(
                    &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../../content/spells.manifest.json"),
                )
                .map_err(debug)?;
                let mut cosmetics = crate::content::native_gameplay::test_support::cosmetics_fixture::cosmetics_test_packet();
                cosmetics["authoring"]["document"]["content_revision"] = "content-1".into();
                let pinned = |bytes: Vec<u8>| PinnedGameplayBytes {
                    sha256: sha2::Sha256::digest(&bytes)
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect(),
                    bytes,
                };
                supplied.cosmetics = pinned(serde_json::to_vec(&cosmetics)?);
                supplied.source_world = Some(pinned(source.to_vec()));
                supplied
                    .build_training
                    .as_mut()
                    .ok_or("build training")?
                    .content_revision = "content-1".into();
                let room = crate::content::qualify_selected_native_gameplay_room(world, &supplied)
                    .map_err(debug)?;
                let issuance = NativeEntryActivationIssuance {
                    world_id: world,
                    activation_sequence: 1,
                    server_artifact_digest: room.compiled().server_digest(),
                    client_artifact_digest: room.compiled().client_digest(),
                    frame_binding_digest: room.frame_binding().digest(),
                };
                let mut controller = ContentActivationController::new();
                let pin = activate_native_entry_room_with_gameplay(
                    &mut controller,
                    &NodeBootQuiescence::before_channel_runtime(),
                    world,
                    &issuance,
                    &supplied,
                )
                .map_err(debug)?;
                (controller, pin)
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
                if WITH_OBSERVER { 3 } else { 2 },
            )?;
            let (actor, session) = (deps.actor, deps.admitted.game_session_id);
            {
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
                runtime
                    .realize_native_qualification_spawn(
                        cp,
                        "oteryn:creature.rat",
                        MovementLocalPosition {
                            x: -2,
                            y: 11,
                            floor: -7,
                        },
                    )
                    .map_err(debug)?;
            }
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
            let rat = deps.runtime.lock().await.visible_entities().creatures[0].actor;
            {
                let states = owner.states.spell_states.lock().await;
                let ai = states
                    .monster_melee
                    .creature_ai
                    .state(rat)
                    .ok_or("rat AI")?;
                assert_eq!(ai.target(), Some(actor));
            }
            let observer = if WITH_OBSERVER {
                Some(
                    super::appearance_change_postgres::observer_visibility_tests::initialize(
                        &deps, &owner,
                    )
                    .await?,
                )
            } else {
                None
            };
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
            if let Some(observer) = observer {
                // SRV6 owner difference: unlike Crystal player.cpp:1383-1396,
                // another player must receive no invisible player entity.
                let channel = owner
                    .observe_visible_entities(observer)
                    .await
                    .ok_or("observer visibility")?;
                let snapshot =
                    crate::gameplay_transport::world_spatial::SessionVisibility::default()
                        .snapshot(&channel, &mut |_| Ok(()))
                        .map_err(debug)?;
                let (_, bytes) =
                    crate::gameplay_transport::world_spatial::encode_visibility_snapshot(
                        &[6],
                        &snapshot,
                    )
                    .map_err(debug)?;
                let received = oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_snapshot(&bytes).map_err(debug)?;
                assert!(
                    !received.entities.iter().any(|entity| entity.entity.identity
                        == actor.placement_identity()
                        && entity.entity.generation == actor.actor_local_generation()),
                    "the spectator must not receive the invisible player entity"
                );
                let entity = oteryn_protocol_oteryn::appearance::EntityRef {
                    identity: actor.placement_identity(),
                    generation: actor.actor_local_generation(),
                };
                let observer_session = GameSessionId::decode(&id(110)).map_err(debug)?;
                assert!(owner.appearance_available());
                let own = owner
                    .appearance_observation_projected(actor, session, true, vec![entity])
                    .await
                    .ok_or("own invisible appearance")?;
                assert!(matches!(
                    own.bindings[0].look,
                    oteryn_protocol_oteryn::appearance::Look::Outfit { .. }
                ));
                let projected = owner
                    .appearance_observation_projected(
                        observer,
                        observer_session,
                        true,
                        vec![entity],
                    )
                    .await
                    .ok_or("invisible appearance projection")?;
                assert!(
                    projected.bindings.is_empty(),
                    "invisible appearance must not leak to another player"
                );
                let legacy = owner
                    .appearance_observation_projected(
                        observer,
                        observer_session,
                        false,
                        vec![entity],
                    )
                    .await
                    .ok_or("legacy invisible appearance projection")?;
                assert!(
                    legacy.bindings.is_empty(),
                    "visibility is independent of empty-outfit capability"
                );
                assert_eq!(
                    owner
                        .attack_target(observer, observer_session, 2, Some(entity))
                        .await,
                    oteryn_protocol_oteryn::attack::AttackIntentDisposition::TargetNotVisible,
                    "a known entity reference cannot bypass invisibility"
                );
            }
            owner.clock_origin -= std::time::Duration::from_secs(2);
            owner.drain_monster_melee().await;
            {
                let states = owner.states.spell_states.lock().await;
                let ai = states
                    .monster_melee
                    .creature_ai
                    .state(rat)
                    .ok_or("rat AI")?;
                if WITH_OBSERVER {
                    assert_ne!(
                        ai.target(),
                        Some(actor),
                        "the rat drops the invisible caster"
                    );
                } else {
                    assert_eq!(
                        ai.target(),
                        None,
                        "a rat without sense_invisible loses the caster"
                    );
                }
            }
            if let Some(observer) = observer {
                owner.clock_origin -= std::time::Duration::from_secs(199);
                let restored = owner
                    .appearance_observation_projected(
                        observer,
                        GameSessionId::decode(&id(110)).map_err(debug)?,
                        true,
                        vec![oteryn_protocol_oteryn::appearance::EntityRef {
                            identity: actor.placement_identity(),
                            generation: actor.actor_local_generation(),
                        }],
                    )
                    .await
                    .ok_or("expired invisibility appearance projection")?;
                assert!(matches!(
                    restored.bindings[0].look,
                    oteryn_protocol_oteryn::appearance::Look::Outfit { .. }
                ));
                let visible = owner
                    .observe_visible_entities(observer)
                    .await
                    .ok_or("restored entities")?;
                let snapshot =
                    crate::gameplay_transport::world_spatial::SessionVisibility::default()
                        .snapshot(&visible, &mut |_| Ok(()))
                        .map_err(debug)?;
                let (_, bytes) =
                    crate::gameplay_transport::world_spatial::encode_visibility_snapshot(
                        &[6],
                        &snapshot,
                    )
                    .map_err(debug)?;
                let received = oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_snapshot(&bytes).map_err(debug)?;
                assert!(
                    received
                        .entities
                        .iter()
                        .any(|entity| entity.entity.identity == actor.placement_identity())
                );
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
