//! Real combat -> tutorial claim -> corpse take -> equipment for each named reward.
//! Arms through the production Forcefield tutorial owner; Viola's NPC route is separate.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::durability::character_tutorial::TutorialProgressOutcome;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelContentPin, CharacterLease, MovementLocalPosition};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, take_corpse_entry};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::tutorial::{ISLET_TEMPLE, TutorialEvent, corruptor_items};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::item_view::ItemMoveOutcome;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::ExactI64;
use std::time::{Duration, Instant};

#[path = "quest_newhaven_gear/source.rs"]
mod source;

#[test]
fn newhaven_corruptor_named_gear_all_vocations_kill_take_equip_once_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("newhaven-gear", || {
        for vocation in ["knight", "paladin", "sorcerer", "druid", "monk"] {
            scenario(vocation, false)?;
        }
        Ok(())
    })
}

#[test]
fn newhaven_corruptor_partial_native_admission_keeps_entire_reward_armed_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("newhaven-gear-refusal", || scenario("monk", true))
}

const PLAYER: MovementLocalPosition = MovementLocalPosition {
    x: 2,
    y: 11,
    floor: -7,
};
const CREATURE: MovementLocalPosition = MovementLocalPosition {
    x: 1,
    y: 11,
    floor: -7,
};

fn scenario(vocation: &str, partial: bool) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "newhaven_gear", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = source::generation(deps.admitted.world_id, partial)?;
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let room = pin.qualified_room();
        // Combat uses the retained source-qualified Thalom cells. The Channel's
        // respawn pin selects Newhaven tutorial rules independently of current location.
        let channel_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            ISLET_TEMPLE,
        );
        deps = Dependencies::with_content_and_capacity(
            channel_pin.clone(),
            room.clone(),
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            2,
        )?;
        let (actor, session) = (deps.actor, deps.admitted.game_session_id);
        {
            let mut runtime = deps.runtime.lock().await;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            runtime
                .initialize_pinned_test_position(actor, PLAYER)
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
            runtime
                .realize_native_qualification_spawn(
                    channel_pin,
                    "oteryn:creature.the_corruptor",
                    CREATURE,
                )
                .map_err(debug)?;
        }
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(room));
        owner.clock_origin = Instant::now()
            .checked_sub(Duration::from_secs(60))
            .ok_or("clock")?;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        let progression = owner.progression_binding(session).ok_or("progression")?;
        let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
        let formula = native.training_formula().ok_or("training")?;
        h.root
            .commit_character_build(
                &authority,
                &h.node,
                fence,
                BuildChangeRequest {
                    occurrence: BuildOccurrence::from_bytes(id(80)).map_err(debug)?,
                    cause: BuildCause::VocationChoice,
                    before: DurableBuildState::default(),
                    after: convert_vocation(formula, &DurableBuildState::default(), vocation)
                        .map_err(debug)?,
                    pruned_stance: None,
                },
                formula,
            )
            .await
            .map_err(debug)?;
        let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
        // Level eight satisfies the exact source amulet/wand/rod requirements.
        h.root
            .commit_character_experience(
                &authority,
                &h.node,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(81)).map_err(debug)?,
                    amount: ExactI64::new(4_200),
                    context: progression.context.clone(),
                    policy_revision: progression.policy_revision.clone(),
                    reward_revision: progression.reward_revision.clone(),
                    policy: progression.policy.clone(),
                },
            )
            .await
            .map_err(debug)?;
        let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
        h.root
            .initialize_familiar_group(&authority, &h.node, fence)
            .await
            .map_err(debug)?;
        assert!(
            owner
                .tutorial_event(fence, TutorialEvent::Forcefield)
                .await
                .map_err(|()| "tutorial arm")?
                .is_some()
        );
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                active.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &NativeItemWeights(native),
                &crate::durability::starter_grant::StarterTemplate {
                    definition: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i2854".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    ..crate::durability::starter_grant::StarterTemplate::main()
                },
            )
            .await
            .map_err(debug)?;
        {
            let runtime = deps.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    actor,
                    session,
                    crate::spell::cast::CharacterCastFacts {
                        // Only the combat fixture's swing facts are accelerated; the durable
                        // build used by loot/equipment remains the chosen level-eight vocation.
                        level: 1_000_000,
                        max_health: 1_000_000,
                        vocation: match vocation {
                            "knight" => crate::spell::Vocation::Knight,
                            "paladin" => crate::spell::Vocation::Paladin,
                            "sorcerer" => crate::spell::Vocation::Sorcerer,
                            "druid" => crate::spell::Vocation::Druid,
                            "monk" => crate::spell::Vocation::Monk,
                            _ => return Err("fixture vocation".into()),
                        },
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (3, 7_000_000),
                    owner.owner_now(),
                )
                .ok_or("combat state")?;
            let target = runtime.visible_entities().creatures[0];
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(room),
                    actor,
                    session,
                    owner.owner_now(),
                    Some(EntityRef {
                        identity: target.actor.placement_identity(),
                        generation: target.generation
                    }),
                    1
                ),
                AttackIntentDisposition::Ok
            );
        }
        for attempt in 0..64 {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            if owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(session)
            {
                break;
            }
            assert!(attempt < 63, "native Corruptor must die");
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(3))
                .ok_or("clock advance")?;
        }
        let captured = {
            let attack = owner.states.attack.lock().await;
            let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) =
                attack.kills().take(session, 2)
            else {
                return Err("captured real Corruptor death".into());
            };
            taken.entry().ok_or("captured death entry")?.clone()
        };
        let mut stale_claim = super::death_ml_loss_composed::current_fence(&owner, session).await?;
        stale_claim.connection_generation =
            crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
        assert!(
            owner
                .tutorial_corruptor_table(stale_claim, &captured)
                .await
                .is_err()
        );
        assert_eq!(claim_count(&h).await?, 0, "stale authority writes no claim");
        if partial {
            owner.drain_kill_rewards(session).await;
            assert_eq!(claim_count(&h).await?, 0, "no partial vocation grant");
            let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
            assert!(
                matches!(h.root.character_tutorial_progress(&authority, &h.node, fence, None).await.map_err(debug)?,
                TutorialProgressOutcome::Read(Some(stored)) if stored.progress.corruptor)
            );
            assert!(owner.runtime.lock().await.visible_corpses().is_empty());
        } else {
            for attempt in 0..8 {
                if owner.drain_kill_rewards(session).await
                    == crate::gameplay_transport::kill_reward::DrainOutcome::Drained
                {
                    break;
                }
                assert!(
                    attempt < 7,
                    "{vocation}: Corruptor reward settles; missing native gear: {:?}",
                    corruptor_items(vocation)
                        .iter()
                        .filter(|item| native
                            .item_policy(&format!("oteryn:item.tibia.i{item}"), "definition-r1")
                            .is_none())
                        .collect::<Vec<_>>()
                );
                owner.root.maintain_ready_once().await.map_err(debug)?;
            }
            assert_eq!(claim_count(&h).await?, 1);
            let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
            let replay = owner
                .tutorial_corruptor_table(fence, &captured)
                .await
                .map_err(|()| "same-death claim replay")?;
            let reward_tail =
                &replay.entries[replay.entries.len() - corruptor_items(vocation).len()..];
            assert_eq!(
                reward_tail
                    .iter()
                    .map(|entry| entry.item.production_key.clone())
                    .collect::<Vec<_>>(),
                corruptor_items(vocation)
                    .iter()
                    .map(|item| format!("oteryn:item.tibia.i{item}"))
                    .collect::<Vec<_>>()
            );
            owner.drain_kill_rewards(session).await;
            assert_eq!(claim_count(&h).await?, 1, "drain replay grants once");
            let corpse = owner
                .runtime
                .lock()
                .await
                .visible_corpses()
                .first()
                .cloned()
                .ok_or("corpse")?;
            let contents = owner
                .root
                .read_corpse_contents(&authority, corpse.item.item_instance_id)
                .await
                .map_err(debug)?
                .ok_or("contents")?;
            for (n, item) in corruptor_items(vocation).iter().enumerate() {
                let key = format!("oteryn:item.tibia.i{item}");
                let entry = contents
                    .iter()
                    .find(|e| e.item.definition.production_key == key)
                    .ok_or("complete named gear")?;
                let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
                let command = UseCommand {
                    game_session_id: session,
                    command_id: 200 + u64::try_from(n)?,
                    item_fence: Some(fence.item_fence()),
                };
                let mut stale = fence.item_fence();
                stale.connection_generation =
                    crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
                assert!(
                    take_corpse_entry(
                        &owner,
                        actor,
                        UseCommand {
                            command_id: 100 + u64::try_from(n)?,
                            item_fence: Some(stale),
                            ..command
                        },
                        ItemKey::Entity(corpse.identity),
                        ItemKey::Instance(entry.item.item_instance_id)
                    )
                    .await
                    .is_err(),
                    "stale session cannot take named gear"
                );
                take_corpse_entry(
                    &owner,
                    actor,
                    command,
                    ItemKey::Entity(corpse.identity),
                    ItemKey::Instance(entry.item.item_instance_id),
                )
                .await
                .map_err(debug)?;
                // Replay the exact transfer and stale fence; custody remains singular.
                assert!(
                    take_corpse_entry(
                        &owner,
                        actor,
                        command,
                        ItemKey::Entity(corpse.identity),
                        ItemKey::Instance(entry.item.item_instance_id),
                    )
                    .await
                    .is_err(),
                    "repeated take cannot duplicate custody"
                );
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(Duration::from_secs(3))
                    .ok_or("equip clock")?;
                let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
                let definition = native
                    .item_index()
                    .ok_or("index")?
                    .definition_ref("Item", &key, "definition-r1")
                    .ok_or("definition")?;
                let result = owner
                    .hotkey_equip(
                        actor,
                        UseCommand {
                            game_session_id: session,
                            command_id: 300 + u64::try_from(n)?,
                            item_fence: Some(fence.item_fence()),
                        },
                        oteryn_protocol_oteryn::hotkey_equip::Request { definition },
                    )
                    .await;
                assert_eq!(
                    result,
                    Some(ItemMoveOutcome::Moved),
                    "{vocation}: {key} equips"
                );
            }
            assert_eq!(claim_count(&h).await?, 1);
            let worn: i64 = sqlx::query_scalar("SELECT count(DISTINCT item_instance_id) FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert!(
                worn >= i64::try_from(corruptor_items(vocation).len())?,
                "all vocation gear is worn"
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn claim_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_tutorial_corruptor_claims")
            .fetch_one(&h.pool)
            .await?,
    )
}
