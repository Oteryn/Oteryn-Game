//! P05: durable equipment/active bounty inputs consumed by real auto attacks.
use super::*;
use crate::gameplay_transport::task_board::bounty_talisman as effects;
use oteryn_gameplay_combat_rules::resolution::equipped_offence::{EquippedOffence, leech_amount};
use oteryn_gameplay_combat_rules::{attack::*, resolution::*};
use oteryn_simulation_determinism::GameplayDecisionRoot;
use std::time::Duration;

#[path = "bounty_talisman/equipment.rs"]
mod equipment;

const TARGET_HEALTH: i64 = 1_000;

pub(in super::super) async fn qualify_talisman(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let (actor, session) = players[0];
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let character = fence.character_id;
    let item = crate::bestiary_postgres_harness::id(119);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i51978','definition-r1',1,1,encode($1,'hex')::uuid)")
        .bind(item.as_slice()).bind(owner.world_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,10,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(character.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(item.as_slice()).execute(&mut *tx).await?;
    for key in task_board::TALISMAN_PATHS {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,355,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_bytes().as_slice()).bind(key).bind(task_board::QUEST).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    equipment::prepare(owner, h, character, item).await?;

    // Retain one qualified snapshot only for choosing deterministic test occurrences;
    // production drains below independently read the actual changed durable owner.
    let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
    let active = owner
        .bounty_talisman_for_session(session)
        .await
        .map_err(|()| "active talisman read")?;
    assert_eq!(
        active.bonuses("oteryn:creature.rat"),
        [5000, 5000, 5000, 10000]
    );
    drop(permit);
    let constants = AttackConstants::checked_in().ok_or("attack constants")?;
    let creature = {
        let runtime = owner.runtime.lock().await;
        runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|row| {
                runtime.companion_snapshot(row.actor).is_ok_and(|snapshot| {
                    snapshot.state.policy.definition_key == "oteryn:creature.rat"
                })
            })
            .ok_or("rat")?
            .actor
    };
    let mut next_command = 100_u64;
    for (label, slot, status, race, qualifies) in [
        (
            "extra slot and active matching bounty",
            10_i16,
            BountyStatus::Active,
            "oteryn:creature.rat",
            true,
        ),
        (
            "non-extra slot cannot activate",
            7,
            BountyStatus::Active,
            "oteryn:creature.rat",
            false,
        ),
        (
            "different active bounty cannot activate",
            10,
            BountyStatus::Active,
            "oteryn:creature.cave_rat",
            false,
        ),
        (
            "completed bounty cannot activate",
            10,
            BountyStatus::Completed,
            "oteryn:creature.rat",
            false,
        ),
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let body: serde_json::Value = sqlx::query_scalar(
            "SELECT body FROM game_character_task_boards WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(character.as_bytes().as_slice())
        .fetch_one(&mut *tx)
        .await?;
        let mut board: TaskBoardData = serde_json::from_value(body)?;
        board.bounty.state = status;
        let bounty = board
            .bounty
            .active
            .as_mut()
            .ok_or("active bounty fixture")?;
        bounty.race = race.into();
        bounty.current = if status == BountyStatus::Completed {
            bounty.required
        } else {
            bounty.required - 1
        };
        sqlx::query("UPDATE game_character_task_boards SET body=$2 WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_bytes().as_slice()).bind(serde_json::to_value(board)?).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_equipment_slots SET slot=$2 WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(item.as_slice()).bind(slot).execute(&mut *tx).await?;
        tx.commit().await?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(3))
            .ok_or("clock")?;
        owner.refresh_attack_weapon(actor, session).await;
        let expected;
        {
            let mut runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, actor, session)
                .ok_or("player state")?;
            state.facts.level = 100;
            state.health = 1;
            state.install_equipped_offence(EquippedOffence {
                life_leech: 300,
                ..EquippedOffence::default()
            });
            let facts = state.character_facts();
            let skills = state
                .swing_skills(0, owner.owner_now().get())
                .flatten()
                .ok_or("fist skills")?;
            let inputs = crate::spell::formula::FormulaInputs {
                level: facts.level,
                magic_level: facts.magic_level,
                base_power: None,
                attack_skill: skills.attack,
                attack_value: constants.fist.attack,
                attack_factor: constants.fight_mode(FightMode::Offensive).attack_factor,
                shielding_skill: skills.shielding,
                shield_defense: None,
            };
            let formula = if matches!(
                facts.vocation,
                crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk
            ) {
                oteryn_gameplay_combat_rules::attack::formulas::player_monk_formula(constants)
            } else {
                player_fist_formula(constants)
            };
            let (minimum, maximum) = formula.bounds(&inputs).map_err(debug)?;
            let root =
                GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
            let policy = runtime
                .companion_snapshot(creature)
                .map_err(debug)?
                .state
                .policy;
            let chosen = (next_command..next_command + 100_000)
                .find_map(|command| {
                    let lineage = (*session.as_bytes(), command);
                    let critical = swing_draw(
                        &root,
                        actor.placement_identity(),
                        lineage,
                        0,
                        b"critical-hit",
                        "attack.critical-hit",
                        1,
                        10_000,
                    )?;
                    let procs = effects::hit_bonuses(
                        &active,
                        "oteryn:creature.rat",
                        &root,
                        swing_occurrence(
                            actor.placement_identity(),
                            lineage,
                            0,
                            b"bounty-talisman",
                        ),
                        0,
                    )?;
                    if critical <= 500 || procs != [5000, 5000] {
                        return None;
                    }
                    let damage = swing_draw(
                        &root,
                        actor.placement_identity(),
                        lineage,
                        0,
                        b"damage-draw",
                        "attack.damage",
                        minimum,
                        maximum,
                    )?;
                    let mitigated = creature_mitigated(
                        &root,
                        constants,
                        actor.placement_identity(),
                        lineage,
                        0,
                        damage,
                        true,
                        policy.defense?,
                        policy.armor?,
                    )?;
                    let landed = creature_physical_response(&policy, mitigated)?;
                    (landed >= 10).then_some((command, landed))
                })
                .ok_or("qualified talisman damage/life occurrence")?;
            next_command = chosen.0 + 1;
            expected = if qualifies {
                chosen.1 * 150 / 100
            } else {
                chosen.1
            };
            assert!(expected > 0);
            drop(states);
            runtime
                .set_test_creature_health(creature, TARGET_HEALTH)
                .map_err(debug)?;
            let row = runtime
                .visible_entities()
                .creatures
                .into_iter()
                .find(|row| row.actor == creature)
                .ok_or("visible rat")?;
            let mut attack = owner.states.attack.lock().await;
            assert!(attack.set_modes(
                &runtime,
                actor,
                session,
                oteryn_protocol_oteryn::attack::FightModes {
                    fight_mode: oteryn_protocol_oteryn::attack::FightMode::Offensive,
                    ..oteryn_protocol_oteryn::attack::FightModes::DEFAULT
                }
            ));
            assert_eq!(
                attack.set_target(
                    &runtime,
                    owner.rooms.gate_auto_attack(),
                    actor,
                    session,
                    owner.owner_now(),
                    Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                        identity: creature.placement_identity(),
                        generation: row.generation,
                    }),
                    chosen.0
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
        }
        owner.drain_auto_attacks().await;
        let runtime = owner.runtime.lock().await;
        let real = TARGET_HEALTH.unsigned_abs()
            - runtime.live_creature_health(creature).ok_or("living rat")?;
        assert_eq!(
            real,
            u64::try_from(expected)?,
            "{label}: actual HP delta includes exactly the admitted damage bonus"
        );
        let states = owner.states.spell_states.lock().await;
        let state = states.get(&runtime, actor, session).ok_or("player state")?;
        assert_eq!(
            state.health,
            1 + leech_amount(u32::try_from(real)?, if qualifies { 5300 } else { 300 }),
            "{label}: combined ordinary and talisman life skill uses actual committed HP loss"
        );
        assert_eq!(
            state.equipped_offence().life_leech,
            300,
            "the temporary proc cannot persist in worn equipment"
        );
    }
    println!(
        "<BOUNTY_TALISMAN> POSTGRES_CASE_COMPLETED: real_damage_and_combined_life_equipment_race_completed_gates"
    );
    qualify_bestiary_rollback(owner, h, actor, session, creature).await?;
    Ok(())
}

/// Another real kill may complete a party recipient's bounty while an earlier
/// independent Bestiary descendant is retrying. Its sealed first decision wins.
async fn qualify_bestiary_rollback(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    creature: ExactActorRef,
) -> TestResult {
    use crate::durability::bestiary_progress::{
        BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest, BestiaryProgressError,
    };
    let mut fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_task_boards SET body=jsonb_set(jsonb_set(body,'{bounty,state}','\"Active\"'::jsonb),'{bounty,active,current}','24'::jsonb) WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let progression = owner
        .progression_binding(session)
        .ok_or("bound progression")?;
    let race = owner
        .reward_table()
        .row("oteryn:creature.rat")
        .map_err(debug)?
        .race
        .clone()
        .ok_or("admitted rat Bestiary")?;
    let request = BestiaryKillRequest {
        occurrence: BestiaryKillOccurrence::from_bytes(crate::bestiary_postgres_harness::id(120))
            .map_err(debug)?,
        race: race.clone(),
        context: progression.context.clone(),
        policy_revision: progression.policy_revision.clone(),
        reward_revision: progression.reward_revision.clone(),
    };
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    fence.expected_character_revision = slot
        .cursor(owner.root, owner.character)
        .await
        .map_err(debug)?;
    let captured = owner
        .root
        .capture_bestiary_talisman(
            owner.character,
            owner.holder,
            fence,
            request.occurrence,
            &race,
        )
        .await
        .map_err(debug)?;
    assert_eq!(captured.bonuses(race.key())[3], 10_000);
    let mut substituted = request.clone();
    substituted.occurrence =
        BestiaryKillOccurrence::from_bytes(crate::bestiary_postgres_harness::id(121))
            .map_err(debug)?;
    assert!(
        matches!(
            slot.commit_bestiary_with_talisman(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                substituted,
                Some(captured.clone())
            )
            .await,
            Err(BestiaryProgressError::AuthorityRejected)
        ),
        "a sealed capture cannot be reused for another death"
    );
    sqlx::query("CREATE FUNCTION talisman_test_rollback() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'TALISMAN_TEST_ROLLBACK'; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER talisman_test_rollback BEFORE INSERT ON game_character_bestiary_kill_receipts FOR EACH ROW EXECUTE FUNCTION talisman_test_rollback()")
        .execute(&h.pool).await?;
    assert!(matches!(
        slot.commit_bestiary_with_talisman(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request.clone(),
            Some(captured.clone())
        )
        .await,
        Err(BestiaryProgressError::Unavailable(_))
    ));
    drop(slot);
    let committed: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_bestiary_kill_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        committed, 0,
        "fault rolls back the entire first Bestiary mutation"
    );
    let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        revision,
        fence.expected_character_revision.get().to_string()
    );
    sqlx::query("DROP TRIGGER talisman_test_rollback ON game_character_bestiary_kill_receipts")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP FUNCTION talisman_test_rollback()")
        .execute(&h.pool)
        .await?;
    owner.root.maintain_ready_once().await?;

    // Two actual lethal swings share one queue and one drain. The first advances
    // Character revision and completes the bounty before the second captures it.
    let other = owner
        .runtime
        .lock()
        .await
        .visible_entities()
        .creatures
        .into_iter()
        .find(|row| row.actor != creature)
        .ok_or("second living creature")?
        .actor;
    for (index, target) in [creature, other].into_iter().enumerate() {
        {
            let mut runtime = owner.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .get_mut(&runtime, actor, session)
                .ok_or("player state")?
                .facts
                .level = 1;
            if index == 1 {
                let position = runtime.read_actor_position(actor).map_err(debug)?;
                let mut next = position.position();
                next.y += 1;
                runtime
                    .borrow_movement_position()
                    .commit_cardinal(position, next)
                    .map_err(debug)?;
            }
            runtime.set_test_creature_health(target, 1).map_err(debug)?;
            let row = runtime
                .visible_entities()
                .creatures
                .into_iter()
                .find(|row| row.actor == target)
                .ok_or("visible target")?;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(3))
                .ok_or("clock")?;
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    owner.rooms.gate_auto_attack(),
                    actor,
                    session,
                    owner.owner_now(),
                    Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                        identity: target.placement_identity(),
                        generation: row.generation,
                    }),
                    1_000_000 + u64::try_from(index)?
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
        }
        for pass in 0..256 {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            if owner.states.attack.lock().await.kills().counts().0 == index + 1 {
                break;
            }
            assert!(
                pass < 255,
                "each real kill must reach the same pending queue"
            );
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(3))
                .ok_or("clock")?;
        }
    }
    assert_eq!(owner.states.attack.lock().await.kills().counts().0, 2);
    equipment::unequip_after_queued_deaths(owner, h, actor, session).await?;
    assert_eq!(owner.states.attack.lock().await.kills().counts().0, 0);
    let increments: Vec<(String, i16)> = sqlx::query_as("SELECT race_key,kill_count_increment FROM game_character_bestiary_kill_receipts WHERE character_id=encode($1,'hex')::uuid ORDER BY race_key")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        increments,
        [
            ("oteryn:creature.cave_rat".into(), 1),
            ("oteryn:creature.rat".into(), 2)
        ],
        "the second queued death captures the current revision after the first has committed"
    );
    println!(
        "<BOUNTY_TALISMAN> POSTGRES_CASE_COMPLETED: two_queued_deaths_one_drain_current_capture"
    );
    let state: String = sqlx::query_scalar("SELECT body->'bounty'->>'state' FROM game_character_task_boards WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        state, "Completed",
        "the distinct real death completed the bounty"
    );
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_bestiary_with_talisman(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request.clone(),
            Some(captured),
        )
        .await
        .map_err(debug)?;
    let BestiaryKillOutcome::Committed(saved) = outcome else {
        return Err("retry must commit its original sealed death".into());
    };
    assert_eq!(
        (saved.kill_count_before, saved.kill_count_after),
        (2, 4),
        "the earlier rolled-back death still doubles after a later real kill completed the bounty"
    );
    assert_eq!(
        slot.commit_bestiary(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request.clone()
        )
        .await
        .map_err(debug)?,
        BestiaryKillOutcome::AlreadyCommitted(saved),
        "the existing DB receipt remains replay authority without any capture"
    );
    let mut uncaptured = request;
    uncaptured.occurrence =
        BestiaryKillOccurrence::from_bytes(crate::bestiary_postgres_harness::id(122))
            .map_err(debug)?;
    let BestiaryKillOutcome::Committed(next) = slot
        .commit_bestiary(owner.root, owner.character, owner.holder, fence, uncaptured)
        .await
        .map_err(debug)?
    else {
        return Err("ordinary uncaptured kill".into());
    };
    assert_eq!(
        (next.kill_count_before, next.kill_count_after),
        (4, 5),
        "legacy API must still enforce the current Completed gate"
    );
    drop(slot);
    println!(
        "<BOUNTY_TALISMAN> POSTGRES_CASE_COMPLETED: sealed_bestiary_rollback_other_death_completion_retry"
    );
    equipment::equip_after_unworn_death(owner, h, actor, session).await?;
    Ok(())
}
