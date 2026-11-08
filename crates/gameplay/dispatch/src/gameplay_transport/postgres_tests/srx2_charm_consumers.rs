//! SRX-2: real auto-attack damage consumes a killing-hit charm without chaining.
use super::party_xp::{self, Case};
use crate::bestiary_postgres_harness::{TestResult, debug};
use crate::durability::charm_state::{CharacterCharmState, CombatCharmState};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_domain::domain::charm::{BestiaryRaceKey, CharmKey, CharmStage};
use oteryn_gameplay_progression::charm_effects::{
    CharmAttackerFacts, CharmEffectOutcome, CharmHitSource, CharmHookEvent, CharmResult,
};
use oteryn_simulation_determinism::GameplayDecisionRoot;

#[test]
fn srx2_charm_carnage_real_lethal_swing_and_adjacent_damage() -> TestResult {
    party_xp::run(Case::CharmCarnage)
}

#[test]
fn srx2_charm_incoming_hit_hastes_player() -> TestResult {
    party_xp::run(Case::CharmIncoming)
}

#[test]
fn srx2_charm_numb_incoming_hit_paralyses_attacker() -> TestResult {
    party_xp::run(Case::CharmNumb)
}

#[test]
fn srx2_charm_cleanse_incoming_hit_removes_one_negative_condition() -> TestResult {
    party_xp::run(Case::CharmCleanse)
}

#[test]
fn srx2_charm_cripple_real_swing_slows_living_target() -> TestResult {
    party_xp::run(Case::CharmCripple)
}

#[test]
fn srx2_charm_fatal_hold_real_swing_prevents_native_flee() -> TestResult {
    party_xp::run(Case::CharmFatalHold)
}

#[test]
fn srx2_charm_void_inversion_native_source_mana_commit_and_replay() -> TestResult {
    party_xp::run(Case::CharmMana)
}

#[test]
fn srx2_charm_bless_lethal_snapshot_atomic_loss_rollback_and_replay() -> TestResult {
    party_xp::run(Case::CharmBless)
}

pub(super) async fn bless(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &crate::bestiary_postgres_harness::Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    use crate::ability::creature_bite::CreatureBiteVitals;
    use crate::gameplay_transport::attack::player_block::{IncomingCharm, PlayerBiteVitals};
    let (actor, session) = players[0];
    let progression = owner
        .progression_binding(session)
        .ok_or("progression binding")?;
    let initial = progression.policy.thresholds[119].minimum_experience.get() + 10;
    let character = owner
        .runtime
        .lock()
        .await
        .bound_attacker_lease(actor, session)
        .map_err(debug)?
        .character_id();
    {
        use crate::durability::character_build::{
            BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState,
        };
        use crate::durability::character_progression::{
            ExperienceAwardRequest, ExperienceRewardOccurrence,
        };
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "current gameplay fence")?
            .ok_or("current gameplay fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        slot.commit_experience(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(
                    crate::bestiary_postgres_harness::id(219),
                )
                .map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(initial),
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
            None,
        )
        .await
        .map_err(|error| format!("Bless setup XP award: {error:?}"))?;
        let before = owner
            .root
            .read_character_build_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        let mut skills = before.skills();
        skills[0] = (100, 0);
        let after =
            DurableBuildState::new(before.vocation(), before.magic(), skills).map_err(debug)?;
        let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
            include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/build-training.json"
            ),
            &progression.context.content,
        )
        .map_err(debug)?;
        slot.commit_build(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(crate::bestiary_postgres_harness::id(220))
                    .map_err(debug)?,
                cause: BuildCause::Training,
                before,
                after,
                pruned_stance: None,
            },
            &formula,
            None,
        )
        .await
        .map_err(|error| format!("Bless setup training: {error:?}"))?;
    }
    let death = {
        let mut runtime = owner.runtime.lock().await;
        let source = runtime
            .visible_entities()
            .creatures
            .first()
            .ok_or("rat")?
            .actor;
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let character =
            crate::domain::CharacterId::from_bytes(*character.as_bytes()).map_err(debug)?;
        let key = CharmKey::new("oteryn:charm.bless").map_err(debug)?;
        let parry = CharmKey::new("oteryn:charm.parry").map_err(debug)?;
        let race = BestiaryRaceKey::new("oteryn:creature.rat").map_err(debug)?;
        let state = CombatCharmState::new(
            character,
            CharacterCharmState {
                unlocks: [
                    (key.clone(), CharmStage::new(3).map_err(debug)?),
                    (parry.clone(), CharmStage::new(3).map_err(debug)?),
                ]
                .into(),
                assignments: [(key, race.clone()), (parry, race)].into(),
            },
        );
        let occurrence = (0_u16..1000)
            .find_map(|value| {
                let mut bytes = [0; 16];
                bytes[..2].copy_from_slice(&value.to_le_bytes());
                let occurrence =
                    oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes);
                crate::gameplay_transport::attack::run_charm_hook(
                    owner.imported_charms,
                    &state,
                    *character.as_bytes(),
                    "oteryn:creature.rat",
                    &root,
                    occurrence,
                    CharmHookEvent::IncomingCreatureHit { base_damage: 500 },
                )
                .ok()
                .filter(|outcomes| {
                    outcomes.iter().any(|outcome| {
                        matches!(
                            outcome.result,
                            CharmResult::Applied(CharmEffectOutcome::ReflectDamage { .. })
                                | CharmResult::FailedClosed {
                                    evaluated: CharmEffectOutcome::ReflectDamage { .. },
                                    ..
                                }
                        )
                    })
                })
                .map(|_| occurrence)
            })
            .ok_or("triggering mutual-kill Parry")?;
        let mut states = owner.states.spell_states.lock().await;
        assert!(states.install_charm_state(&runtime, actor, session, state));
        let mut attack = owner.states.attack.lock().await;
        let mut trained = Vec::new();
        let mut vitals = PlayerBiteVitals {
            quest_sessions: None,
            charm: Some(IncomingCharm {
                source,
                catalogue: owner.imported_charms,
                root: root.clone(),
                occurrence,
                sequence: 1,
                race: "oteryn:creature.rat",
                base_damage: 500,
            }),
            conditions: None,
            states: &mut states,
            attack: &mut attack,
            formula: None,
            trained: &mut trained,
        };
        let now = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
            owner.owner_now().get(),
        );
        assert!(vitals.reflect_equipment_damage(
            &mut runtime,
            source,
            actor,
            session,
            Some("physical"),
            500,
            b"srx2-mutual-lethal",
            now
        ));
        assert_eq!(
            runtime
                .companion_snapshot_including_dead(source)
                .map_err(debug)?
                .health,
            0,
            "actual Parry kills the exact source before its admitted incoming lethal hit lands"
        );
        let hit = vitals
            .apply_creature_damage(
                &mut runtime,
                actor,
                session,
                500,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                    owner.owner_now().get(),
                ),
            )
            .ok_or("lethal source hit")?;
        let death = states
            .player_death(&runtime, actor, session)
            .ok_or("pending death")?;
        assert_eq!(hit.death, Some(*death.occurrence.as_bytes()));
        let loss = attack
            .death_charm(&runtime, session, *death.occurrence.as_bytes())
            .ok_or("retained Bless")?;
        assert_eq!((loss.stage, loss.percent), (3, 12));
        assert!(
            attack
                .death_charm(&runtime, players[1].1, *death.occurrence.as_bytes())
                .is_none()
        );
        assert!(
            attack
                .death_charm(&runtime, session, crate::bestiary_postgres_harness::id(221))
                .is_none()
        );
        // Later assignment changes cannot rewrite the exact lethal selection.
        assert!(states.install_charm_state(
            &runtime,
            actor,
            session,
            CombatCharmState::new(character, CharacterCharmState::default())
        ));
        death
    };
    sqlx::raw_sql(sqlx::AssertSqlSafe("CREATE FUNCTION srx2_bless_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'SRX2 death atomicity probe'; END $$; CREATE TRIGGER srx2_bless_failure BEFORE INSERT ON game_character_death_receipts FOR EACH ROW EXECUTE FUNCTION srx2_bless_failure();".to_owned())).execute(&h.pool).await?;
    assert_ne!(
        owner.commit_death(session, &progression, death).await.0,
        crate::gameplay_transport::DeathCommit::Settled
    );
    let unchanged:(i64,i32) = sqlx::query_as("SELECT p.total_experience,b.fist_level FROM game_character_progression_state p JOIN game_character_build_state b USING(character_id) WHERE p.character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        unchanged,
        (initial, 100),
        "receipt failure rolls back XP and skill loss"
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe("DROP TRIGGER srx2_bless_failure ON game_character_death_receipts; DROP FUNCTION srx2_bless_failure();".to_owned())).execute(&h.pool).await?;
    owner.root.maintain_ready_once().await.map_err(debug)?;
    assert_eq!(
        owner.commit_death(session, &progression, death).await.0,
        crate::gameplay_transport::DeathCommit::Settled
    );
    let row:(i64,Vec<u8>) = sqlx::query_as("SELECT experience_lost,command_binding FROM game_character_death_receipts WHERE death_occurrence_id=encode($1,'hex')::uuid")
        .bind(death.occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        u128::try_from(row.0)?,
        crate::domain::death::death_experience_loss_with_bless(120, 0, false, 12).map_err(debug)?
    );
    assert!(
        row.1.starts_with(b"SRX2BL"),
        "immutable receipt retains source/race/stage/percent"
    );
    let after:(i32,i64) = sqlx::query_as("SELECT fist_level,fist_tries FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after,
        (100, 0),
        "D58 / NIGHT-W1 decision19 preserves trained skills on death, including Bless"
    );
    assert_eq!(
        owner.commit_death(session, &progression, death).await.0,
        crate::gameplay_transport::DeathCommit::Settled
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_death_receipts WHERE death_occurrence_id=encode($1,'hex')::uuid")
        .bind(death.occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?,1);
    Ok(())
}

pub(super) async fn mana(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    use crate::gameplay_transport::attack::charm_combat::source_vitals::CharmSourceVitals;
    use crate::player_lethal::PlayerLethalVitals;
    use oteryn_simulation_determinism::DecisionOccurrenceId;
    use sha2::{Digest, Sha256};
    let (actor, session) = players[0];
    let mut runtime = owner.runtime.lock().await;
    let source = runtime
        .visible_entities()
        .creatures
        .first()
        .ok_or("rat")?
        .actor;
    let character = runtime
        .bound_attacker_lease(actor, session)
        .map_err(debug)?
        .character_id();
    let character = crate::domain::CharacterId::from_bytes(*character.as_bytes()).map_err(debug)?;
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let now = owner.owner_now();
    let key = CharmKey::new("oteryn:charm.void_inversion").map_err(debug)?;
    let state = CombatCharmState::new(
        character,
        CharacterCharmState {
            unlocks: [(key.clone(), CharmStage::new(3).map_err(debug)?)].into(),
            assignments: [(
                key,
                BestiaryRaceKey::new("oteryn:creature.rat").map_err(debug)?,
            )]
            .into(),
        },
    );
    let occurrence = (0..1000)
        .map(|index| format!("srx2-mana:{index}"))
        .find(|text| {
            let digest = Sha256::new()
                .chain_update(b"oteryn.charm.source.v1")
                .chain_update(text.as_bytes())
                .finalize();
            let mut bytes = [0; 16];
            bytes.copy_from_slice(&digest[..16]);
            crate::gameplay_transport::attack::run_charm_hook(
                owner.imported_charms,
                &state,
                *character.as_bytes(),
                "oteryn:creature.rat",
                &root,
                DecisionOccurrenceId::from_bytes(bytes),
                CharmHookEvent::IncomingManaDrain { mana_drained: 40 },
            )
            .is_ok_and(|outcomes| {
                outcomes.iter().any(|outcome| {
                    matches!(
                        outcome.result,
                        CharmResult::Applied(_) | CharmResult::FailedClosed { .. }
                    )
                })
            })
        })
        .ok_or("triggering mana event")?;
    let mut states = owner.states.spell_states.lock().await;
    assert!(states.install_charm_state(&runtime, actor, session, state));
    let mut attack = owner.states.attack.lock().await;
    for starting_mana in [0_u32, 10] {
        let current = states.get(&runtime, actor, session).ok_or("player")?;
        let (empty, debit) = current
            .after_creature_mana_drain(current.vitals().mana)
            .ok_or("initial MP debit")?;
        if debit > 0 {
            assert!(states.commit(&runtime, actor, session, empty));
        }
        if starting_mana > 0 {
            let (filled, credited) = states
                .get(&runtime, actor, session)
                .ok_or("player")?
                .after_mana_gain(u64::from(starting_mana))
                .ok_or("initial MP credit")?;
            if credited > 0 {
                assert!(states.commit(&runtime, actor, session, filled));
            }
        }
        let before = states
            .get(&runtime, actor, session)
            .ok_or("player")?
            .vitals();
        let text = format!("{occurrence}:phase{starting_mana}");
        // Keep the selected trigger identity unchanged for this check: each phase gets a
        // fresh source owner memo, without changing the selected charm probability.
        let mut vitals = CharmSourceVitals {
            states: &mut states,
            attack: &mut attack,
            catalogue: owner.imported_charms,
            root: root.clone(),
        };
        let receipt = vitals
            .apply_source_mana_drain(
                &mut runtime,
                source,
                actor,
                session,
                40,
                &occurrence,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
            )
            .ok_or("native mana commit")?;
        assert_eq!(
            receipt.mana_after,
            starting_mana.saturating_add(40).min(before.max_mana),
            "inversion uses full source amount at zero or low mana"
        );
        assert_eq!(
            vitals.apply_source_mana_drain(
                &mut runtime,
                source,
                actor,
                session,
                40,
                &occurrence,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get())
            ),
            Some(receipt)
        );
        let after = states
            .get(&runtime, actor, session)
            .ok_or("player")?
            .vitals();
        assert_eq!(
            (after.health, after.mana),
            (before.health, receipt.mana_after),
            "replay neither damages HP nor grants MP twice"
        );
        // Advance the existing memo with an ordinary non-triggering zero-capacity drain
        // before the next independent fixture phase.
        if starting_mana == 0 {
            use crate::player_lethal::PlayerLethalVitals;
            states
                .apply_attack_mana_drain(
                    &mut runtime,
                    actor,
                    session,
                    1,
                    &text,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
                )
                .ok_or("separate native source memo")?;
        }
    }
    for name in ["numb", "adrenaline_burst"] {
        let key = CharmKey::new(format!("oteryn:charm.{name}")).map_err(debug)?;
        let state = CombatCharmState::new(
            character,
            CharacterCharmState {
                unlocks: [(key.clone(), CharmStage::new(3).map_err(debug)?)].into(),
                assignments: [(
                    key,
                    BestiaryRaceKey::new("oteryn:creature.rat").map_err(debug)?,
                )]
                .into(),
            },
        );
        let occurrence = (0..1000)
            .map(|index| format!("srx2-mana-{name}:{index}"))
            .find(|text| {
                let digest = Sha256::new()
                    .chain_update(b"oteryn.charm.source.v1")
                    .chain_update(text.as_bytes())
                    .finalize();
                let mut bytes = [0; 16];
                bytes.copy_from_slice(&digest[..16]);
                crate::gameplay_transport::attack::run_charm_hook(
                    owner.imported_charms,
                    &state,
                    *character.as_bytes(),
                    "oteryn:creature.rat",
                    &root,
                    DecisionOccurrenceId::from_bytes(bytes),
                    CharmHookEvent::IncomingManaDrain { mana_drained: 40 },
                )
                .is_ok_and(|outcomes| {
                    outcomes.iter().any(|outcome| {
                        matches!(
                            outcome.result,
                            CharmResult::Applied(_) | CharmResult::FailedClosed { .. }
                        )
                    })
                })
            })
            .ok_or("triggering source mana secondary")?;
        assert!(states.install_charm_state(&runtime, actor, session, state));
        let (filled, credited) = states
            .get(&runtime, actor, session)
            .ok_or("player")?
            .after_mana_gain(50)
            .ok_or("source mana refill")?;
        if credited > 0 {
            assert!(states.commit(&runtime, actor, session, filled));
        }
        let mut vitals = CharmSourceVitals {
            states: &mut states,
            attack: &mut attack,
            catalogue: owner.imported_charms,
            root: root.clone(),
        };
        let receipt = vitals
            .apply_source_mana_drain(
                &mut runtime,
                source,
                actor,
                session,
                40,
                &occurrence,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
            )
            .ok_or("native source mana secondary")?;
        assert!(receipt.mana_after < receipt.mana_before);
        assert_eq!(
            vitals.apply_source_mana_drain(
                &mut runtime,
                source,
                actor,
                session,
                40,
                &occurrence,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get())
            ),
            Some(receipt)
        );
        if name == "numb" {
            assert!(
                runtime
                    .actor_active_speed_delta(source, None, now)
                    .map_err(debug)?
                    < 0
            );
        } else {
            assert!(
                states
                    .get(&runtime, actor, session)
                    .ok_or("player")?
                    .owned_conditions()
                    .speed_delta_at(now.get())
                    > 0
            );
        }
    }

    Ok(())
}

pub(super) async fn control(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    players: &[(ExactActorRef, GameSessionId)],
    name: &str,
) -> TestResult {
    let (actor, session) = players[0];
    let victim = {
        let runtime = owner.runtime.lock().await;
        let victim = *runtime.visible_entities().creatures.first().ok_or("rat")?;
        let character = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?
            .character_id();
        let character =
            crate::domain::CharacterId::from_bytes(*character.as_bytes()).map_err(debug)?;
        let key = CharmKey::new(format!("oteryn:charm.{name}")).map_err(debug)?;
        let state = CombatCharmState::new(
            character,
            CharacterCharmState {
                unlocks: [(key.clone(), CharmStage::new(3).map_err(debug)?)].into(),
                assignments: [(
                    key,
                    BestiaryRaceKey::new("oteryn:creature.rat").map_err(debug)?,
                )]
                .into(),
            },
        );
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let command = (1..1000)
            .find(|command| {
                let occurrence = oteryn_gameplay_combat_rules::resolution::swing_occurrence(
                    actor.placement_identity(),
                    (*session.as_bytes(), *command),
                    0,
                    b"committed-hit",
                );
                crate::gameplay_transport::attack::run_charm_hook(
                    owner.imported_charms,
                    &state,
                    *character.as_bytes(),
                    "oteryn:creature.rat",
                    &root,
                    occurrence,
                    CharmHookEvent::CommittedHit {
                        source: CharmHitSource::CharacterAttack,
                        attacker: CharmAttackerFacts {
                            level: 30,
                            max_health: 150,
                            max_mana: 50,
                        },
                        creature_max_health: 20,
                        health_before: 20,
                        health_after: 19,
                    },
                )
                .is_ok_and(|outcomes| {
                    outcomes.iter().any(|outcome| {
                        matches!(
                            outcome.result,
                            CharmResult::Applied(_) | CharmResult::FailedClosed { .. }
                        )
                    })
                })
            })
            .ok_or("triggering attack")?;
        let mut states = owner.states.spell_states.lock().await;
        assert!(states.install_charm_state(&runtime, actor, session, state));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("player")?
            .facts
            .level = 30;
        assert_eq!(
            owner.states.attack.lock().await.set_target(
                &runtime,
                owner.rooms().gate_auto_attack(),
                actor,
                session,
                owner.owner_now(),
                Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                    identity: victim.actor.placement_identity(),
                    generation: victim.generation
                }),
                command
            ),
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        );
        victim.actor
    };
    owner.refresh_attack_weapon(actor, session).await;
    owner.drain_auto_attacks().await;
    {
        let mut runtime = owner.runtime.lock().await;
        let after = runtime.companion_snapshot(victim).map_err(debug)?.health;
        assert!(
            (1..20).contains(&after),
            "a nonlethal real swing must land: {after}"
        );
        if name == "cripple" {
            assert!(
                runtime
                    .actor_active_speed_delta(victim, None, owner.owner_now())
                    .map_err(debug)?
                    < 0,
                "Cripple must slow the native creature"
            );
            return Ok(());
        }
        runtime
            .commit_monster_lab_damage(victim, b"srx2-flee-threshold", after - 1)
            .map_err(debug)?;
        owner
            .states
            .attack
            .lock()
            .await
            .clear_target(actor, session);
    }
    owner.clock_origin -= std::time::Duration::from_secs(1);
    owner.drain_monster_melee().await;
    assert!(
        !owner
            .states
            .spell_states
            .lock()
            .await
            .monster_melee
            .creature_ai
            .state(victim)
            .ok_or("native AI")?
            .is_fleeing(),
        "Fatal Hold must suppress the low-health native flee decision"
    );
    owner.clock_origin -= std::time::Duration::from_secs(31);
    owner.drain_monster_melee().await;
    assert!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .monster_melee
            .creature_ai
            .state(victim)
            .ok_or("native AI")?
            .is_fleeing(),
        "native fleeing returns after the thirty-second hold expires"
    );
    Ok(())
}

pub(super) async fn incoming(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    players: &[(ExactActorRef, GameSessionId)],
    charm_name: &str,
) -> TestResult {
    use crate::foundation::owner_timer::SemanticTimeMicros;
    use crate::gameplay_transport::attack::charm_combat::source_vitals::CharmSourceVitals;
    use crate::player_lethal::{PlayerHitKind, PlayerLethalVitals};
    use sha2::{Digest, Sha256};
    let (actor, session) = players[0];
    let mut runtime = owner.runtime.lock().await;
    let creature = runtime
        .visible_entities()
        .creatures
        .first()
        .ok_or("rat")?
        .actor;
    let character = runtime
        .bound_attacker_lease(actor, session)
        .map_err(debug)?
        .character_id();
    let character = crate::domain::CharacterId::from_bytes(*character.as_bytes()).map_err(debug)?;
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let now = owner.owner_now();
    let mut states = owner.states.spell_states.lock().await;
    let mut attack = owner.states.attack.lock().await;
    let source_health_before = runtime.companion_snapshot(creature).map_err(debug)?.health;
    let magnitude = if charm_name == "parry" { 2 } else { 1 };
    let before = crate::spell::actor_conditions::movement_speed(
        states.get(&runtime, actor, session).ok_or("player")?,
        now.get(),
        0,
    );
    for (index, name) in [charm_name].into_iter().enumerate() {
        let key = CharmKey::new(format!("oteryn:charm.{name}")).map_err(debug)?;
        let state = CombatCharmState::new(
            character,
            CharacterCharmState {
                unlocks: [(key.clone(), CharmStage::new(3).map_err(debug)?)].into(),
                assignments: [(
                    key,
                    BestiaryRaceKey::new("oteryn:creature.rat").map_err(debug)?,
                )]
                .into(),
            },
        );
        let occurrence = (0_u16..1000)
            .find_map(|value| {
                let text = format!("srx2-native-source-hit:{value}");
                let digest = Sha256::new()
                    .chain_update(b"oteryn.charm.source.v1")
                    .chain_update(text.as_bytes())
                    .finalize();
                let mut bytes = [0; 16];
                bytes.copy_from_slice(&digest[..16]);
                let occurrence =
                    oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes);
                crate::gameplay_transport::attack::run_charm_hook(
                    owner.imported_charms,
                    &state,
                    *character.as_bytes(),
                    "oteryn:creature.rat",
                    &root,
                    occurrence,
                    CharmHookEvent::IncomingCreatureHit { base_damage: 1 },
                )
                .ok()
                .filter(|outcomes| {
                    outcomes.iter().any(|outcome| {
                        matches!(
                            outcome.result,
                            CharmResult::Applied(_) | CharmResult::FailedClosed { .. }
                        )
                    })
                })
                .map(|_| text)
            })
            .ok_or("triggering incoming hit")?;
        assert!(states.install_charm_state(&runtime, actor, session, state));
        if name == "cleanse" {
            use oteryn_gameplay_domain::domain::condition::{ConditionValues, SpeedRange};
            use oteryn_gameplay_foundation::foundation::condition as native;
            let definition = native::ConditionDefinition::new(
                "fixture:srx2.paralysis",
                1,
                ConditionValues::Speed {
                    paralysis: true,
                    range: SpeedRange {
                        a_min: -1000,
                        a_max: -1000,
                        b_min: 0,
                        b_max: 0,
                    },
                    duration_ms: 60_000,
                },
            )
            .ok_or("negative condition")?;
            let stamp = runtime.issue_owner_work().map_err(debug)?;
            let fence = runtime.owner_fence().map_err(debug)?;
            let base_speed = u16::try_from(
                states
                    .get(&runtime, actor, session)
                    .ok_or("player")?
                    .owned_base_speed(),
            )?;
            let facts = native::ApplicationFacts {
                now: now.get(),
                base_speed,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                    [92; 16],
                ),
            };
            assert!(states.install_owned_player_self_use_test_condition(
                &runtime,
                fence,
                stamp,
                actor,
                session,
                &definition,
                &facts
            ));
            assert!(
                states
                    .get(&runtime, actor, session)
                    .ok_or("player")?
                    .owned_conditions()
                    .speed_delta_at(now.get())
                    < 0
            );
        }
        if matches!(name, "adrenaline_burst" | "parry") {
            use oteryn_gameplay_domain::domain::condition::ConditionValues;
            use oteryn_gameplay_foundation::foundation::condition as native;
            let (filled, credited) = states
                .get(&runtime, actor, session)
                .ok_or("player")?
                .after_mana_gain(50)
                .ok_or("shield MP")?;
            if credited > 0 {
                assert!(states.commit(&runtime, actor, session, filled));
            }
            let definition = native::ConditionDefinition::new(
                "fixture:srx2.shield",
                1,
                ConditionValues::ManaShield {
                    duration_ms: 60_000,
                },
            )
            .ok_or("mana shield")?;
            let stamp = runtime.issue_owner_work().map_err(debug)?;
            let fence = runtime.owner_fence().map_err(debug)?;
            let facts = native::ApplicationFacts {
                now: now.get(),
                base_speed: 220,
                mana_shield_capacity: 50,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                    [93; 16],
                ),
            };
            assert!(states.install_owned_player_self_use_test_condition(
                &runtime,
                fence,
                stamp,
                actor,
                session,
                &definition,
                &facts
            ));
        }
        let _ = index;
        let mut vitals = CharmSourceVitals {
            states: &mut states,
            attack: &mut attack,
            catalogue: owner.imported_charms,
            root: root.clone(),
        };
        let committed = vitals
            .apply_source_typed_attack_damage(
                &mut runtime,
                creature,
                actor,
                session,
                magnitude,
                PlayerHitKind::Physical,
                &occurrence,
                SemanticTimeMicros::from_micros(now.get()),
            )
            .ok_or("native source physical spell commit")?;
        assert_eq!(
            committed.applied,
            if matches!(name, "adrenaline_burst" | "parry") {
                0
            } else {
                magnitude
            },
            "fully mana-shielded source hit still activates incoming charms"
        );
        let reflected_health = runtime.companion_snapshot(creature).map_err(debug)?.health;
        let replay = vitals
            .apply_source_typed_attack_damage(
                &mut runtime,
                creature,
                actor,
                session,
                magnitude,
                PlayerHitKind::Physical,
                &occurrence,
                SemanticTimeMicros::from_micros(now.get()),
            )
            .ok_or("native source spell replay")?;
        assert_eq!(committed, replay);
        assert_eq!(
            runtime.companion_snapshot(creature).map_err(debug)?.health,
            reflected_health,
            "source receipt replay cannot reflect twice"
        );
        if name == "adrenaline_burst" {
            let after = crate::spell::actor_conditions::movement_speed(
                states.get(&runtime, actor, session).ok_or("player")?,
                now.get(),
                0,
            );
            assert!(
                after > before,
                "Adrenaline Burst must change native player movement speed"
            );
        } else if name == "parry" {
            assert!(
                runtime.companion_snapshot(creature).map_err(debug)?.health < source_health_before,
                "native source spell Parry must reflect to the actual source"
            );
            // A distinct fully shielded source hit changes MP and the shared vitals
            // revision, even with zero HP loss. It must remain a new Parry occurrence.
            let next_occurrence = (1000_u16..2000)
                .find_map(|value| {
                    let text = format!("srx2-native-source-hit:{value}");
                    let digest = Sha256::new()
                        .chain_update(b"oteryn.charm.source.v1")
                        .chain_update(text.as_bytes())
                        .finalize();
                    let mut bytes = [0; 16];
                    bytes.copy_from_slice(&digest[..16]);
                    crate::gameplay_transport::attack::run_charm_hook(
                        owner.imported_charms,
                        states.charm_state(&runtime, actor, session)?,
                        *character.as_bytes(),
                        "oteryn:creature.rat",
                        &root,
                        oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes),
                        CharmHookEvent::IncomingCreatureHit {
                            base_damage: u64::from(magnitude),
                        },
                    )
                    .ok()
                    .filter(|outcomes| {
                        outcomes.iter().any(|outcome| {
                            matches!(
                                outcome.result,
                                CharmResult::Applied(_) | CharmResult::FailedClosed { .. }
                            )
                        })
                    })
                    .map(|_| text)
                })
                .ok_or("second triggering shielded Parry")?;
            let second = CharmSourceVitals {
                states: &mut states,
                attack: &mut attack,
                catalogue: owner.imported_charms,
                root: root.clone(),
            }
            .apply_source_typed_attack_damage(
                &mut runtime,
                creature,
                actor,
                session,
                magnitude,
                PlayerHitKind::Physical,
                &next_occurrence,
                SemanticTimeMicros::from_micros(now.get()),
            )
            .ok_or("second fully shielded source spell")?;
            assert_eq!(second.applied, 0);
            assert!(
                second.vitals_revision > committed.vitals_revision,
                "MP-only absorption advances the same authoritative vitals revision"
            );
            assert!(
                runtime.companion_snapshot(creature).map_err(debug)?.health < reflected_health,
                "a later fully shielded hit must not be suppressed by the reflection fence"
            );
        } else if name == "cleanse" {
            assert_eq!(
                states
                    .get(&runtime, actor, session)
                    .ok_or("player")?
                    .owned_conditions()
                    .speed_delta_at(now.get()),
                0,
                "Cleanse must remove the selected negative condition"
            );
        } else {
            assert!(
                runtime
                    .actor_active_speed_delta(creature, None, now)
                    .map_err(debug)?
                    < 0,
                "Numb must slow the attacking creature in the native actor store"
            );
        }
    }
    Ok(())
}

pub(super) async fn carnage(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &crate::bestiary_postgres_harness::Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let (actor, session) = players[0];
    let (victim, neighbour, before, command) = {
        let mut runtime = owner.runtime.lock().await;
        let creatures = runtime.visible_entities().creatures;
        let victim = *creatures.first().ok_or("rat")?;
        let neighbour = creatures.get(1).ok_or("adjacent cave rat")?.actor;
        let position = runtime
            .companion_snapshot(victim.actor)
            .map_err(debug)?
            .position;
        let neighbour_snapshot = runtime.companion_snapshot(neighbour).map_err(debug)?;
        assert_eq!(position.floor, neighbour_snapshot.position.floor);
        assert_eq!(
            position.x.abs_diff(neighbour_snapshot.position.x)
                + position.y.abs_diff(neighbour_snapshot.position.y),
            1
        );
        runtime
            .commit_monster_lab_damage(victim.actor, b"srx2-lethal-fixture", 19)
            .map_err(debug)?;
        let character = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?
            .character_id();
        let character =
            crate::domain::CharacterId::from_bytes(*character.as_bytes()).map_err(debug)?;
        let key = CharmKey::new("oteryn:charm.carnage").map_err(debug)?;
        let state = CombatCharmState::new(
            character,
            CharacterCharmState {
                unlocks: [(key.clone(), CharmStage::new(3).map_err(debug)?)].into(),
                assignments: [(
                    key,
                    BestiaryRaceKey::new("oteryn:creature.rat").map_err(debug)?,
                )]
                .into(),
            },
        );
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        // Select a deterministic triggering lineage; the real owner still evaluates and commits it.
        let command = (1..1000)
            .find(|command| {
                let occurrence = oteryn_gameplay_combat_rules::resolution::swing_occurrence(
                    actor.placement_identity(),
                    (*session.as_bytes(), *command),
                    0,
                    b"committed-hit",
                );
                crate::gameplay_transport::attack::run_charm_hook(
                    owner.imported_charms,
                    &state,
                    *character.as_bytes(),
                    "oteryn:creature.rat",
                    &root,
                    occurrence,
                    CharmHookEvent::CommittedHit {
                        source: CharmHitSource::CharacterAttack,
                        attacker: CharmAttackerFacts {
                            level: 100,
                            max_health: 150,
                            max_mana: 50,
                        },
                        creature_max_health: 20,
                        health_before: 1,
                        health_after: 0,
                    },
                )
                .is_ok_and(|outcomes| {
                    outcomes.iter().any(|outcome| {
                        matches!(
                            outcome.result,
                            CharmResult::Applied(CharmEffectOutcome::AreaDamageAroundKill { .. })
                                | CharmResult::FailedClosed {
                                    evaluated: CharmEffectOutcome::AreaDamageAroundKill { .. },
                                    ..
                                }
                        )
                    })
                })
            })
            .ok_or("triggering lineage")?;
        let mut states = owner.states.spell_states.lock().await;
        assert!(states.install_charm_state(&runtime, actor, session, state));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("player")?
            .facts
            .level = 100;
        (victim, neighbour, neighbour_snapshot.health, command)
    };
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            owner.states.attack.lock().await.set_target(
                &runtime,
                owner.rooms().gate_auto_attack(),
                actor,
                session,
                owner.owner_now(),
                Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                    identity: victim.actor.placement_identity(),
                    generation: victim.generation,
                }),
                command,
            ),
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        );
    }
    owner.refresh_attack_weapon(actor, session).await;
    owner.drain_auto_attacks().await;
    let runtime = owner.runtime.lock().await;
    assert!(
        !runtime.contains_live_creature(victim.actor),
        "the ordinary swing killed the target"
    );
    let after = runtime.companion_snapshot(neighbour).map_err(debug)?.health;
    assert!(
        after < before,
        "Carnage must damage the cardinal neighbour: {before} -> {after}"
    );
    drop(runtime);
    owner.drain_auto_attacks().await;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .companion_snapshot(neighbour)
            .map_err(debug)?
            .health,
        after,
        "a repeated owner pass cannot replay the killing proc"
    );
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained,
        "production reward drain settles the already committed lethal source"
    );
    let races: Vec<String> = sqlx::query_scalar(
        "SELECT s.race_key FROM game_corpse_source_races s \
         JOIN game_item_mint_receipts m ON m.item_instance_id=s.item_instance_id \
           AND m.transaction_id=s.mint_transaction_id \
         WHERE m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        races,
        vec!["oteryn:creature.rat"],
        "real lethal mint retains the exact source race for later skinning"
    );
    Ok(())
}

#[test]
fn srx2_charm_parry_native_source_spell_damage_and_replay() -> TestResult {
    party_xp::run(Case::CharmParry)
}
