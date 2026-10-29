//! A4b: utility runes through the production USE owner and durable source transaction.
use super::death_ml_loss_composed::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id, run};
use super::item_look::{large_stack, seed_main_backpack};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelContentPin, CharacterId, CharacterLease, MovementLocalPosition, WorldId,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use crate::gameplay_transport::{item_fence_of, item_view::ItemKey};
use oteryn_protocol_oteryn::actor_spell::SpellTargetPosition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

#[test]
fn a4b_paralyze_rune_postgres_real_dispatch() -> TestResult {
    large_stack(|| utility_rune(3165))
}
#[test]
fn a4b_convince_rune_postgres_real_dispatch() -> TestResult {
    large_stack(|| utility_rune(3177))
}
#[test]
fn a4b_chameleon_rune_postgres_real_dispatch() -> TestResult {
    large_stack(|| utility_rune(3178))
}

fn utility_rune(rune_id: u32) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "a4b_utility", false, false).await?;
        prepare_current_session(&h).await?;
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
        )?;
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
            if rune_id == 3178 {
                (12, 12, -7)
            } else {
                (2, 11, -7)
            },
        );
        let mut deps =
            Dependencies::with_content(cp.clone(), room.clone(), native.spell_book().clone())?;
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
                        x: 1,
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
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        let progression = owner.progression_binding(session).ok_or("progression")?;
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
        let trained =
            DurableBuildState::new(build.vocation(), (18, 0), build.skills()).map_err(debug)?;
        h.root
            .commit_character_build(
                &authority,
                &h.node,
                current_fence(&owner, session).await?,
                BuildChangeRequest {
                    occurrence: BuildOccurrence::from_bytes(id(92)).map_err(debug)?,
                    cause: BuildCause::Training,
                    before: build,
                    after: trained.clone(),
                    pruned_stance: None,
                },
                formula,
            )
            .await
            .map_err(debug)?;
        let build = trained;
        h.root
            .commit_character_experience(
                &authority,
                &h.node,
                current_fence(&owner, session).await?,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(27_393_800),
                    context: progression.context.clone(),
                    policy_revision: progression.policy_revision.clone(),
                    reward_revision: progression.reward_revision.clone(),
                    policy: progression.policy.clone(),
                },
            )
            .await
            .map_err(debug)?;
        let crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
            facts, ..
        } = crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
            &h.root,
            &authority,
            current_fence(&owner, session).await?.character_id,
        )
        .await
        else {
            return Err("durable caster facts".into());
        };
        assert_eq!(facts.level, 120);
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
        let source = format!("oteryn:item.tibia.i{rune_id}");
        seed_main_backpack(&h, &[(&source, 2)]).await?;
        // The admitted source map has a movable top Item (3446) at this tile.
        let target = MovementLocalPosition {
            x: if rune_id == 3178 { 12 } else { 1 },
            y: 11,
            floor: -7,
        };
        h.root
            .initialize_familiar_group(&authority, &h.node, current_fence(&owner, session).await?)
            .await
            .map_err(debug)?;
        if rune_id == 3178 {
            // Prepare the source-map fixture through its real fenced initialization owner.
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("map initialization lane")?;
            let runtime = deps.runtime.lock().await;
            let proof = crate::spell::source_map_initialization::CurrentNativeMapInitialization::from_current_owner(&runtime, room, native).map_err(debug)?;
            h.root
                .initialize_native_map_current_owner(&permit, &authority, &h.node, &proof)
                .await
                .map_err(debug)?;
        }
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?
            .0;
        let outcome = owner
            .use_rune(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 1,
                    item_fence: item_fence_of(current),
                },
                UseIntent {
                    target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
                    use_with: None,
                    use_with_map_item: None,
                    use_with_position: Some(SpellTargetPosition {
                        x: target.x,
                        y: target.y,
                        floor: target.floor,
                    }),
                },
                Some(ItemKey::Instance(id(71))),
                true,
                false,
            )
            .await
            .ok_or("terminal USE")?;
        assert_eq!(
            outcome.disposition,
            UseDisposition::Committed,
            "rune {rune_id}"
        );
        let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(quantity, 1, "one successful cast consumes exactly one rune");
        {
            let runtime = deps.runtime.lock().await;
            let rat = runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|c| c.actor != actor)
                .ok_or("rat")?
                .actor;
            let snapshot = runtime.companion_snapshot(rat).map_err(debug)?;
            match rune_id {
                3165 => {
                    let condition = snapshot.state.conditions.instances().iter().find(|c| c.definition().condition_type() == oteryn_gameplay_domain::domain::condition::ConditionType::Paralysis).ok_or("paralyze installed")?;
                    // The admitted donor formula is normalized to its fixed speed floor.
                    assert_eq!(
                        condition.definition().values(),
                        oteryn_gameplay_domain::domain::condition::ConditionValues::Speed {
                            paralysis: true,
                            duration_ms: 6000,
                            range: oteryn_gameplay_domain::domain::condition::SpeedRange {
                                a_min: 0,
                                b_min: 40,
                                a_max: 0,
                                b_max: 40
                            },
                        }
                    );
                    assert_eq!(
                        condition.remaining_duration_ms(condition.started_at()),
                        Some(6000)
                    );
                    assert_eq!(
                        i64::from(condition.application_base_speed()) + condition.speed_delta(),
                        40
                    );
                }
                3177 => {
                    let master = snapshot.state.master.ok_or("convince ownership")?;
                    assert_eq!(master.actor, actor);
                    assert_eq!(master.session, session);
                }
                3178 => {
                    let states = owner.states.spell_states.lock().await;
                    let store = states
                        .get(&runtime, actor, session)
                        .ok_or("caster")?
                        .owned_conditions();
                    let condition = store
                        .instances()
                        .iter()
                        .find(|c| c.definition().source_item_appearance().is_some())
                        .ok_or("Chameleon Item outfit")?;
                    assert_eq!(
                        condition
                            .definition()
                            .source_item_appearance()
                            .ok_or("Item appearance")?
                            .definition_key(),
                        "oteryn:item.tibia.i3446"
                    );
                    assert_eq!(
                        condition.remaining_duration_ms(condition.started_at()),
                        Some(200_000)
                    );
                }
                _ => return Err("unknown rune".into()),
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
