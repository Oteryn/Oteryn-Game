//! C11PKG: actual party, persistent standard stances and qualified Ascetic spender damage.
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, id};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_spell_rules::native_actor_states::StandardStance;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use std::{num::NonZeroU32, sync::Arc, time::Duration};

#[test]
fn c11_monk_real_dispatch_ascetic_party_serene_refund_and_stance_reload_postgres() -> TestResult {
    super::party_buffs::run_party_buffs(Some(("c11_monk", "monk")))
}

async fn cast(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    player: (ExactActorRef, GameSessionId),
    key: &str,
    command: u64,
    target: SpellTarget,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(15))
        .ok_or("clock")?;
    let spell = (1..=owner.spells.source_len())
        .find_map(|i| {
            NonZeroU32::new(u32::try_from(i).ok()?).filter(|i| {
                owner
                    .spells
                    .source_indexed(*i)
                    .is_some_and(|(s, active)| active && s.key == format!("candidate:spell/{key}"))
            })
        })
        .ok_or("monk spell")?;
    let result = owner
        .cast_spell_dispatch(
            player.0,
            player.1,
            command,
            SpellCastIntent {
                spell,
                target,
                aim_at_target: false,
            },
        )
        .await
        .ok_or("cast result")?;
    assert_eq!(result.disposition, SpellCastDisposition::Cast, "{key}");
    Ok(())
}

pub(super) async fn scenario(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let player = players[0];
    let fence = owner
        .current_quest_fence(player.1)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let native = owner
        .active_generation
        .ok_or("active")?
        .native_gameplay()
        .ok_or("native")?;
    let world = *fence.runtime_scope.world_id().as_bytes();
    // Fixture custody only. Promotion, Wheel and stance changes below use their actual writers.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    use crate::durability::item_fee_burn::test_support::{item, request};
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "{}{}",
        item(210, "oteryn:item.tibia.i2854", "rev-1", 1),
        item(212, "oteryn:item.tibia.i3043", "rev-1", 2)
    )))
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(id(210).as_slice()).bind(world.as_slice()).bind(id(214).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(id(210).as_slice()).bind(id(212).as_slice()).bind(world.as_slice()).bind(id(215).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i50157','definition-r1',1,1,encode($1,'hex')::uuid)")
        .bind(id(112).as_slice()).bind(world.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,5,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_slice()).bind(id(112).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_equipment_state SET combat_mode=1 WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    use crate::durability::npc_purchase::{NpcPurchaseRequest, PurchaseOutcome, PurchaseService};
    let purchase = NpcPurchaseRequest {
        command: CommandRef::new(player.1, CommandId::new(99).map_err(debug)?),
        occurrence: id(200),
        transaction_id: id(201),
        event_id: id(202),
        account: id(62),
        npc_key: "oteryn:npc.eremo".into(),
        npc_identity: id(203),
        npc_generation: 1,
        service: PurchaseService::Promotion("oteryn:service.promotion.eremo".into()),
        coins: request(1, 20_000)?.change,
        expected_backpack: Some(id(210)),
        occurred_at_unix_ms: 1_790_000_000_000,
    };
    let premium = owner.premium_coordinator.ok_or("premium")?;
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_npc_purchase(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            purchase,
            premium,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, PurchaseOutcome::Committed(_)),
        "{outcome:?}"
    );
    let data =
        oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded().map_err(debug)?;
    use crate::durability::character_wheel::{
        WheelChangeFacts, WheelChangeRequest, WheelCommitOutcome, WheelOccurrence, WheelRuleset,
        WheelSlots,
    };
    let rules =
        Arc::new(WheelRuleset::from_catalogue(data.revision(), data.wheel()).map_err(debug)?);
    owner.wheel_ruleset = Some(Arc::clone(&rules));
    let mut points = [0; 36];
    for (index, slot) in data.wheel()["topology"]
        .as_array()
        .ok_or("topology")?
        .iter()
        .enumerate()
    {
        if slot["domain"] == "blue" {
            points[index] = u16::try_from(slot["capacity"].as_u64().ok_or("capacity")?)?;
        }
    }
    assert_eq!(points.iter().map(|p| u32::from(*p)).sum::<u32>(), 1000);
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_wheel(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            WheelChangeRequest {
                occurrence: WheelOccurrence::from_bytes(id(204)).map_err(debug)?,
                expected_wheel_revision: 0,
                slots: WheelSlots::new(points).map_err(debug)?,
                vessels: None,
            },
            Arc::clone(&rules),
            WheelChangeFacts { at_temple: false },
            premium,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, WheelCommitOutcome::Committed(_)),
        "{outcome:?}"
    );
    owner.admit_wheel_session(player.1).await;
    // Durable standard slot replacement and admission reload use the actual persisted key.
    for (command, key, stance) in [
        (100, "virtue_of_justice", StandardStance::Justice),
        (101, "virtue_of_sustain", StandardStance::Sustain),
        (102, "virtue_of_harmony", StandardStance::Harmony),
    ] {
        cast(owner, player, key, command, SpellTarget::None).await?;
        let loaded = owner
            .root
            .read_character_stance(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(loaded.key(), Some(key));
        let facts = {
            let runtime = owner.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .get(&runtime, player.0, player.1)
                .ok_or("monk")?
                .character_facts()
        };
        let mut reloaded = crate::spell::cast::PlayerSpellState::new(facts, 0, 0)
            .ok_or("fresh admission state")?;
        reloaded
            .load_owned_stance(&loaded, owner.spells)
            .map_err(debug)?;
        assert_eq!(reloaded.standard_stance(), Some(stance));
    }
    owner.refresh_source_party_serene(player.0, player.1).await;
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert_eq!(
            states
                .get(&runtime, player.0, player.1)
                .ok_or("monk")?
                .virtue_party_bonuses(owner.owner_now().get())
                .damage_reduction,
            3
        );
    }
    // PKG8 Q5: a current Guiding Presence allocation scales both the ally and Serene copy.
    // A full green domain reaches Guiding Presence through the actual topology;
    // restore the original blue Ascetic allocation before the spender assertions.
    for (guided, expected_revision, occurrence, percent) in [(true, 1, 205, 4), (false, 2, 206, 3)]
    {
        for (index, slot) in data.wheel()["topology"]
            .as_array()
            .ok_or("topology")?
            .iter()
            .enumerate()
        {
            points[index] = if slot["domain"] == if guided { "green" } else { "blue" } {
                u16::try_from(slot["capacity"].as_u64().ok_or("capacity")?)?
            } else {
                0
            };
        }
        let current = owner
            .current_quest_fence(player.1)
            .await
            .map_err(|_| "wheel fence")?
            .ok_or("wheel fence")?;
        let outcome = owner
            .revision_sequencer
            .acquire(current.character_id)
            .await
            .commit_wheel(
                owner.root,
                owner.character,
                owner.holder,
                current,
                WheelChangeRequest {
                    occurrence: WheelOccurrence::from_bytes(id(occurrence)).map_err(debug)?,
                    expected_wheel_revision: expected_revision,
                    slots: WheelSlots::new(points).map_err(debug)?,
                    vessels: None,
                },
                Arc::clone(&rules),
                // Both domain replacements remove points and require the temple fact.
                WheelChangeFacts { at_temple: true },
                premium,
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(outcome, WheelCommitOutcome::Committed(_)),
            "{outcome:?}"
        );
        for recipient in [player, players[1]] {
            owner
                .refresh_source_party_serene(recipient.0, recipient.1)
                .await;
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert_eq!(
                states
                    .get(&runtime, recipient.0, recipient.1)
                    .ok_or("party actor")?
                    .virtue_party_bonuses(owner.owner_now().get())
                    .damage_reduction,
                percent,
                "Guiding={guided}, recipient={:?}",
                recipient.0
            );
        }
    }
    // Eight adjacent monsters stay fixed; moving the knight across the screen boundary toggles Serene.
    let target;
    let entity;
    {
        let mut runtime = owner.runtime.lock().await;
        for (actor, dx, dy) in [(player.0, 1, 1), (players[1].0, 22, 1)] {
            let mut snapshot = runtime.read_actor_position(actor).map_err(debug)?;
            for _ in 0..dx {
                let mut p = snapshot.position();
                p.x += 1;
                snapshot = runtime
                    .borrow_movement_position()
                    .commit_cardinal(snapshot, p)
                    .map_err(debug)?;
            }
            for _ in 0..dy {
                let mut p = snapshot.position();
                p.y += 1;
                snapshot = runtime
                    .borrow_movement_position()
                    .commit_cardinal(snapshot, p)
                    .map_err(debug)?;
            }
        }
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
        runtime
            .realize_activation_spawns(vec![(
                (1..=3)
                    .flat_map(|x| (0..=2).map(move |y| (x, y, 7)))
                    .filter(|p| *p != (2, 1, 7))
                    .collect(),
                60_000,
                5_000,
                "oteryn:creature.rat".into(),
                20,
            )])
            .map_err(debug)?;
        let visible = runtime.visible_entities();
        let row = visible
            .creatures
            .iter()
            .find(|r| {
                runtime
                    .read_actor_position(r.actor)
                    .is_ok_and(|p| p.position().x == 3 && p.position().y == 1)
            })
            .ok_or("rat")?;
        target = row.actor;
        entity = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: target.placement_identity(),
            generation: row.generation,
        };
    }
    assert_eq!(
        owner
            .select_attack_target(player.0, player.1, 109, Some(entity))
            .await,
        oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
    );
    for (index, serene) in [true, false].into_iter().enumerate() {
        if !serene {
            let mut runtime = owner.runtime.lock().await;
            let mut snapshot = runtime.read_actor_position(players[1].0).map_err(debug)?;
            for _ in 0..20 {
                let mut p = snapshot.position();
                p.x -= 1;
                snapshot = runtime
                    .borrow_movement_position()
                    .commit_cardinal(snapshot, p)
                    .map_err(debug)?;
            }
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(2))
            .ok_or("clock")?;
        owner.refresh_source_party_serene(player.0, player.1).await;
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, player.0, player.1).ok_or("monk")?;
            assert_eq!(
                state
                    .virtue_party_bonuses(owner.owner_now().get())
                    .damage_reduction,
                0
            );
        }
        owner
            .refresh_source_party_serene(players[1].0, players[1].1)
            .await;
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert_eq!(
                states
                    .get(&runtime, players[1].0, players[1].1)
                    .ok_or("knight")?
                    .virtue_party_bonuses(owner.owner_now().get())
                    .damage_reduction,
                if serene { 0 } else { 3 }
            );
        }
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .set_test_creature_health(target, 1_000_000)
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, player.0, player.1).ok_or("monk")?;
            let monk = state.monk.as_mut().ok_or("monk state")?;
            assert_eq!(monk.serene(), serene);
            monk.commit_fill().map_err(debug)?;
            state.health = 1;
        }
        let expected_bounds = {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, player.0, player.1).ok_or("monk")?;
            let facts = state.character_facts();
            use crate::content::ReferenceItemField as F;
            let policy = native
                .item_policy("oteryn:item.tibia.i50157", "definition-r1")
                .ok_or("fist policy")?;
            let F::Known(weapon) = &policy.record().semantics.weapon else {
                return Err("known weapon".into());
            };
            let F::Known(attack) = weapon.attack else {
                return Err("known attack".into());
            };
            let caster = crate::spell::CasterState {
                vocation: facts.vocation,
                level: facts.level,
                magic_level: facts.magic_level,
                premium: crate::spell::CasterPremium::from_current(true),
                mana: facts.max_mana,
                max_mana: facts.max_mana,
                soul: 100,
                learned: Default::default(),
                attack_skill: 10,
                attack_value: u32::try_from(attack.0)?,
                attack_factor: 1.0,
                shielding_skill: 10,
                melee_weapon: true,
                shield_defense: None,
                harmony_multiplier: crate::spell::harmony::HarmonyMultiplier::with_ascetic(
                    facts.level,
                    5,
                    true,
                    serene,
                    3,
                )
                .ok_or("multiplier")?,
            };
            let spell = (1..=owner.spells.source_len())
                .filter_map(|i| {
                    NonZeroU32::new(u32::try_from(i).ok()?)
                        .and_then(|i| owner.spells.source_indexed(i))
                })
                .map(|(s, _)| s)
                .find(|s| s.key == "candidate:spell/tiger_clash")
                .ok_or("spender")?;
            let mut bounds = [0; 2];
            for (i, bound) in bounds.iter_mut().enumerate() {
                let resolution = crate::spell::resolve_cast(
                    spell,
                    &caster,
                    &Default::default(),
                    owner.owner_now(),
                    true,
                    &mut |lo, hi| if i == 0 { lo } else { hi },
                )
                .map_err(debug)?;
                *bound = resolution
                    .effects
                    .iter()
                    .find_map(|e| {
                        if let crate::spell::ResolvedEffect::Damage { magnitude, .. } = e {
                            Some(*magnitude)
                        } else {
                            None
                        }
                    })
                    .ok_or("damage bound")?;
            }
            let colours = [
                ("green".to_owned(), 0),
                ("red".to_owned(), 0),
                ("purple".to_owned(), 0),
                ("blue".to_owned(), 3),
            ]
            .into_iter()
            .collect();
            let flat = native
                .wheel_profile()
                .ok_or("Wheel profile")?
                .flat_stats(&colours)
                .map_err(debug)?
                .0;
            // The cast mitigates at the target after the roll and before the Wheel flat
            // (spell magnitude_owner.rs, Canary Creature::blockHit): armor above 3 draws its
            // reduction, a positive armor up to 3 blocks one point, then the creature's
            // mitigation percent truncates. The rat's armor 2 makes both exact.
            let rat = native
                .policies()
                .policies()
                .iter()
                .find(|p| p.definition_key == "oteryn:creature.rat")
                .ok_or("rat policy")?;
            let armor = rat.armor.ok_or("rat armor")?;
            assert!(armor <= 3, "a drawn armor reduction needs its own bounds");
            assert!(rat.resistances.iter().all(|r| r.damage_type != "physical"));
            let mitigation = rat.mitigation.ok_or("rat mitigation")?;
            let percent = (mitigation.numerator as f64 / mitigation.denominator as f64).min(30.0);
            bounds.map(|b| {
                let value = (b - i64::from(armor > 0)).max(0);
                (value as f64 - value as f64 * percent / 100.0).trunc() as i64 + i64::from(flat)
            })
        };
        let before = owner
            .runtime
            .lock()
            .await
            .companion_snapshot(target)
            .map_err(debug)?
            .health;
        cast(
            owner,
            player,
            "tiger_clash",
            110 + index as u64,
            SpellTarget::AttackTarget,
        )
        .await?;
        let runtime = owner.runtime.lock().await;
        let damage = before - runtime.companion_snapshot(target).map_err(debug)?.health;
        assert!(
            (expected_bounds[0]..=expected_bounds[1]).contains(&damage),
            "stage 3 spender: damage {damage}, bounds {expected_bounds:?}, serene {serene}"
        );
        let states = owner.states.spell_states.lock().await;
        assert_eq!(
            states
                .get(&runtime, player.0, player.1)
                .ok_or("monk")?
                .vitals()
                .harmony,
            1
        );
        let healed = states
            .get(&runtime, player.0, player.1)
            .ok_or("healed monk")?;
        assert_eq!(healed.character_facts().level, 1340);
        // Canary flat F=318: spent@5 795..915 plus refund@1 668..768.
        // The real dispatcher must apply both rolls to the monk in the same cast.
        assert!(
            (1464..=1684).contains(&healed.vitals().health),
            "spent + refund HP {}",
            healed.vitals().health
        );
        eprintln!(
            "C11_DISPATCH serene={serene} damage={damage} ascetic_stage=3 healing_hp={}",
            healed.vitals().health
        );
    }
    Ok(())
}
