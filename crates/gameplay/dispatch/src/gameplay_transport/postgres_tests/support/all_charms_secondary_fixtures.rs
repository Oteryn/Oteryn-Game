use crate::bestiary_postgres_harness::{Harness, TestResult, debug, id};

use crate::domain::charm::{BestiaryRaceKey, CharmKey, CharmStage};

use crate::durability::charm_state::{CharacterCharmState, CombatCharmState};

use crate::foundation::{ExactActorRef, GameSessionId};

use crate::gameplay_transport::ComposedFreshAdmission;

use oteryn_gameplay_combat_rules::{attack::*, resolution::*};

use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, FightModes};

use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

use oteryn_simulation_determinism::GameplayDecisionRoot;

use sha2::{Digest, Sha256};

use std::time::Duration;

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let (actor, session) = players[0];
    let character = owner
        .runtime
        .lock()
        .await
        .bound_attacker_lease(actor, session)
        .map_err(debug)?
        .character_id();
    // Test custody provisioning only. All tested reads, installation and damage use real owners.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key, slot, tier) in [
        (112_u8, "oteryn:item.tibia.i3264", 6_i16, 10_i16),
        (114, "oteryn:item.tibia.i6529", 4, 10),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(character.as_bytes().as_slice()).bind(id(42).as_slice()).bind(slot).bind(id(seed).as_slice()).execute(&mut *tx).await?;
        if tier > 0 {
            sqlx::query("INSERT INTO game_item_tiers(item_instance_id,tier) VALUES(encode($1,'hex')::uuid,$2)").bind(id(seed).as_slice()).bind(tier).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    let mut effects = owner
        .read_admission_effects(session)
        .await
        .map_err(|_| "effects")?
        .ok_or("effects")?;
    assert_eq!(
        (
            effects.4.cleave,
            effects.4.reflect_percent,
            effects.4.reflect_flat
        ),
        (10_000, 1_000, 99)
    );
    assert_eq!((effects.4.weapon_tier, effects.4.boots_tier), (10, 10));
    assert_eq!(effects.4.perfect_damage(1, 0), 42);
    // Keep actual fenced cleave/tier equipment, with a 5%/40% critical fixture.
    effects.4.critical_chance = 500;
    effects.4.critical_damage = 4000;
    let baseline = effects.4.clone();
    let runtime = owner.runtime.lock().await;
    owner
        .install_admission_suppressions(&runtime, actor, session, Some(effects))
        .await
        .ok_or("install")?;
    let creatures = runtime.visible_entities().creatures;
    let primary = creatures
        .iter()
        .find(|row| {
            runtime
                .read_actor_position(row.actor)
                .is_ok_and(|p| p.position().y == 11)
        })
        .ok_or("primary")?;
    let flank = creatures
        .iter()
        .find(|row| {
            runtime
                .read_actor_position(row.actor)
                .is_ok_and(|p| p.position().y == 12)
        })
        .ok_or("flank")?
        .actor;
    let target = EntityRef {
        identity: primary.actor.placement_identity(),
        generation: primary.generation,
    };
    let primary = primary.actor;
    drop(runtime);
    let constants = AttackConstants::checked_in().ok_or("constants")?;
    let character = crate::domain::CharacterId::from_bytes(*character.as_bytes()).map_err(debug)?;
    // The same roll must cross Low Blow's exact boundary, including stage III's 9%.
    // Both species are inspected on every attack, independently of assignment order.
    let mut command_floor = 100;
    for (name, bonuses) in [
        ("low_blow", [400, 800, 900]),
        ("savage_blow", [2000, 4000, 4400]),
    ] {
        for (index, bonus) in bonuses.into_iter().enumerate() {
            for race in [
                "oteryn:creature.cave_rat",
                "oteryn:creature.rat",
                "oteryn:creature.orc",
            ] {
                let samples =
                    if name == "savage_blow" && index == 2 && race == "oteryn:creature.cave_rat" {
                        3
                    } else {
                        2
                    };
                for sample in 0..samples {
                    let boundary = sample == 1;
                    let failed = sample == 2;
                    let runtime = owner.runtime.lock().await;
                    let digest = runtime.content_pin().server_artifact_digest();
                    let root = GameplayDecisionRoot::from_bytes(digest);
                    let flank_root = GameplayDecisionRoot::from_bytes(
                        Sha256::new()
                            .chain_update(digest)
                            .chain_update(flank.placement_identity())
                            .finalize()
                            .into(),
                    );
                    let mut states = owner.states.spell_states.lock().await;
                    let state = states.get_mut(&runtime, actor, session).ok_or("state")?;
                    state.facts.level = 100;
                    let mut installed = baseline.clone();
                    // The stage/race matrix uses the real 100% cleave fixture. One
                    // extra case truncates a 0.01% cleave to zero before critical,
                    // proving an assigned Savage Blow cannot create a hit from zero.
                    if failed {
                        installed.cleave = 1;
                    }
                    state.install_equipped_offence(installed);
                    let key = CharmKey::new(format!("oteryn:charm.{name}")).map_err(debug)?;
                    let charms = CombatCharmState::new(
                        character,
                        CharacterCharmState {
                            unlocks: [(
                                key.clone(),
                                CharmStage::new(u8::try_from(index + 1)?).map_err(debug)?,
                            )]
                            .into(),
                            assignments: [(key, BestiaryRaceKey::new(race).map_err(debug)?)].into(),
                        },
                    );
                    let facts = state.character_facts();
                    let skills = state
                        .swing_skills(2, owner.owner_now().get())
                        .flatten()
                        .ok_or("sword skills")?;
                    let weapon = swing_weapon(
                        owner
                            .active_generation
                            .ok_or("active")?
                            .native_gameplay()
                            .ok_or("gameplay")?,
                        |slot| (slot == 6).then_some(("oteryn:item.tibia.i3264", "definition-r1")),
                    )
                    .ok_or("weapon")?;
                    let SwingWeapon::Melee { attack, .. } = weapon else {
                        return Err("melee weapon required".into());
                    };
                    let formula = if matches!(
                        facts.vocation,
                        crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk
                    ) {
                        formulas::player_monk_formula(constants)
                    } else {
                        player_melee_formula(constants)
                    };
                    let (minimum, maximum) = formula
                        .bounds(&crate::spell::formula::FormulaInputs {
                            level: facts.level,
                            magic_level: facts.magic_level,
                            base_power: None,
                            attack_skill: skills.attack,
                            attack_value: attack,
                            attack_factor: constants.fight_mode(FightMode::Offensive).attack_factor,
                            shielding_skill: skills.shielding,
                            shield_defense: None,
                        })
                        .map_err(debug)?;
                    assert!(states.install_charm_state(&runtime, actor, session, charms));
                    drop(states);
                    let threshold = 1000 + if name == "low_blow" { bonus } else { 0 };
                    let wanted = i64::from(threshold) + i64::from(boundary);
                    let command = (command_floor..command_floor + 2_000_000)
                        .find(|command| {
                            swing_draw(
                                &root,
                                actor.placement_identity(),
                                (*session.as_bytes(), *command),
                                0,
                                b"critical-hit",
                                "attack.critical-hit",
                                1,
                                10_000,
                            ) == Some(wanted)
                        })
                        .ok_or("exact critical threshold command")?;
                    command_floor = command + 1;
                    let mut expected = Vec::new();
                    for (victim, target_root, species, extension) in [
                        (primary, &root, "oteryn:creature.rat", false),
                        (flank, &flank_root, "oteryn:creature.cave_rat", true),
                    ] {
                        let lineage = (*session.as_bytes(), command);
                        let raw = swing_draw(
                            target_root,
                            actor.placement_identity(),
                            lineage,
                            0,
                            b"damage-draw",
                            "attack.damage",
                            minimum,
                            maximum,
                        )
                        .ok_or("damage draw")?;
                        let mut offence = baseline.clone();
                        let assigned = race == species;
                        if assigned {
                            if name == "low_blow" {
                                offence.critical_chance += u16::try_from(bonus)?;
                            } else {
                                offence.critical_damage += u16::try_from(bonus)?;
                            }
                        }
                        let mut damage = u32::try_from(if extension && failed {
                            raw / 10_000
                        } else {
                            raw
                        })?;
                        if assigned && !boundary && !failed {
                            assert!(
                                offence.critical(damage, u64::try_from(wanted)?)
                                    > baseline.critical(damage, u64::try_from(wanted)?),
                                "the charm adds damage beyond the same equipment and roll"
                            );
                        }
                        if extension && failed {
                            assert_eq!(damage, 0, "a failed hit gains no charm damage");
                        }
                        if !extension || assigned {
                            damage = offence.critical(damage, u64::try_from(wanted)?);
                        }
                        if !extension {
                            let fatal = swing_draw(
                                &root,
                                actor.placement_identity(),
                                lineage,
                                0,
                                b"fatal-hit",
                                "attack.fatal-hit",
                                0,
                                10_000,
                            )
                            .ok_or("fatal")?;
                            damage = offence.fatal(damage, u64::try_from(fatal)?);
                        }
                        let snapshot = runtime.companion_snapshot(victim).map_err(debug)?;
                        let policy = &snapshot.state.policy;
                        let mitigated = creature_mitigated(
                            target_root,
                            constants,
                            actor.placement_identity(),
                            lineage,
                            0,
                            i64::from(damage),
                            true,
                            policy.defense.ok_or("defense")?,
                            policy.armor.ok_or("armor")?,
                        )
                        .ok_or("mitigation")?;
                        expected.push((
                            victim,
                            u64::try_from(
                                creature_physical_response(policy, mitigated)
                                    .ok_or("physical response")?,
                            )?,
                        ));
                    }
                    let mut attacks = owner.states.attack.lock().await;
                    assert!(attacks.set_modes(
                        &runtime,
                        actor,
                        session,
                        FightModes {
                            fight_mode: oteryn_protocol_oteryn::attack::FightMode::Offensive,
                            ..FightModes::DEFAULT
                        }
                    ));
                    assert_eq!(
                        attacks.set_target(
                            &runtime,
                            owner.rooms().gate_auto_attack(),
                            actor,
                            session,
                            owner.owner_now(),
                            Some(target),
                            command
                        ),
                        AttackIntentDisposition::Ok
                    );
                    drop(attacks);
                    drop(runtime);
                    {
                        let mut runtime = owner.runtime.lock().await;
                        for victim in [primary, flank] {
                            runtime
                                .set_test_creature_health(victim, 10_000)
                                .map_err(debug)?;
                        }
                    }
                    owner.refresh_attack_weapon(actor, session).await;
                    owner.drain_auto_attacks().await;
                    let runtime = owner.runtime.lock().await;
                    for (victim, damage) in &expected {
                        assert_eq!(
                            10_000 - runtime.live_creature_health(*victim).ok_or("health")?,
                            *damage,
                            "{name} stage={} race={race} roll={wanted} secondary={}",
                            index + 1,
                            *victim == flank
                        );
                    }
                    drop(runtime);
                    // Not due: the same lineage cannot apply its critical damage twice.
                    owner.drain_auto_attacks().await;
                    let runtime = owner.runtime.lock().await;
                    for (victim, damage) in &expected {
                        assert_eq!(runtime.live_creature_health(*victim), Some(10_000 - damage));
                    }
                    // Re-select the exact command after the timer is due. Its ordinal-zero
                    // owner receipts still forbid a second HP commit on either target.
                    assert_eq!(
                        owner.states.attack.lock().await.set_target(
                            &runtime,
                            owner.rooms().gate_auto_attack(),
                            actor,
                            session,
                            owner.owner_now(),
                            Some(target),
                            command,
                        ),
                        AttackIntentDisposition::Ok
                    );
                    drop(runtime);
                    owner.clock_origin -= Duration::from_secs(3);
                    owner.refresh_attack_weapon(actor, session).await;
                    owner.drain_auto_attacks().await;
                    let runtime = owner.runtime.lock().await;
                    for (victim, damage) in expected {
                        assert_eq!(
                            runtime.live_creature_health(victim),
                            Some(10_000 - damage),
                            "replayed attack command"
                        );
                    }
                    drop(runtime);
                    owner.clock_origin -= Duration::from_secs(3);
                }
            }
        }
    }
    Ok(())
}
