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

/// PR #2771 ANSWER6544065111: the existing percentage owner remains active;
/// another member's Mantra is not a substitute for the absent complete-party owner.
async fn assert_guiding_percentage_without_shared_mantra(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: (ExactActorRef, GameSessionId),
    recipient: (ExactActorRef, GameSessionId),
) -> TestResult {
    use crate::player_lethal::PlayerHitKind;
    let original_mantra = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        states
            .get(&runtime, source.0, source.1)
            .ok_or("Guiding source")?
            .ascetic_attack_bonus(1)
    };
    let mut without_shared_mantra = None;
    for source_mantra in [0, 17, 83] {
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, source.0, source.1)
                .ok_or("Guiding source")?;
            // Explicit equipment projection fixture, installed by the same seam as
            // admission equipment. Party membership and Guiding are read by the real
            // native owner from the committed World and Wheel state above.
            state.install_equipped_mantra(source_mantra);
            assert_eq!(state.ascetic_attack_bonus(1), source_mantra);
        }
        owner
            .refresh_source_party_serene(recipient.0, recipient.1)
            .await;
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let state = states
            .get(&runtime, recipient.0, recipient.1)
            .ok_or("Guiding recipient")?;
        let now = owner.owner_now().get();
        assert_eq!(state.virtue_party_bonuses(now).damage_reduction, 4);
        assert!(state.vitals().health > 100);
        let mut losses = Vec::new();
        for kind in [
            PlayerHitKind::Fire,
            PlayerHitKind::Energy,
            PlayerHitKind::Earth,
            PlayerHitKind::Ice,
        ] {
            let (after, _) = crate::spell::actor_conditions::stage_typed_hit_in_world(
                &runtime,
                recipient.0,
                state,
                100,
                kind,
                now,
            )
            .map_err(debug)?;
            let loss = state.vitals().health - after.vitals().health;
            if source_mantra == 0 {
                let mut without_virtue = state.clone();
                without_virtue
                    .install_virtue_party_bonuses(Default::default(), now)
                    .map_err(debug)?;
                let (plain, _) = crate::spell::actor_conditions::stage_typed_hit_in_world(
                    &runtime,
                    recipient.0,
                    &without_virtue,
                    100,
                    kind,
                    now,
                )
                .map_err(debug)?;
                let plain_loss = state.vitals().health - plain.vitals().health;
                assert!(plain_loss > 0);
                assert_eq!(
                    loss,
                    plain_loss * 96 / 100,
                    "Guiding percentage for {kind:?}"
                );
            }
            losses.push(loss);
        }
        if let Some(expected) = &without_shared_mantra {
            assert_eq!(
                &losses, expected,
                "party Mantra {source_mantra} must not change absorption"
            );
        } else {
            without_shared_mantra = Some(losses);
        }
    }
    let runtime = owner.runtime.lock().await;
    owner
        .states
        .spell_states
        .lock()
        .await
        .get_mut(&runtime, source.0, source.1)
        .ok_or("Guiding source")?
        .install_equipped_mantra(original_mantra);
    Ok(())
}

pub(super) async fn scenario(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    eprintln!("POSTGRES_STARTED case=c11_monk");
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
        item(210, "oteryn:item.tibia.i2854", "definition-r1", 1),
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
    let mut coins = request(1, 20_000)?.change;
    coins.backpack.definition.revision_ref = "definition-r1".into();
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
        coins,
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
    {
        let runtime = owner.runtime.lock().await;
        owner.states.spell_states.lock().await.get_mut(&runtime, player.0, player.1)
            .ok_or("mantra holder")?.install_equipped_mantra(17);
    }
    // The distant recipient must be a Monk to consume the shared mantra through Ascetic.
    let recipient_fence = owner.current_quest_fence(players[2].1).await.map_err(debug)?
        .ok_or("mantra recipient fence")?;
    let before = owner.root.read_character_build_state(owner.character, recipient_fence.character_id)
        .await.map_err(debug)?;
    let formula = native.training_formula().ok_or("training formula")?;
    let build = crate::durability::character_build::convert_vocation(formula, &before, "monk")
        .map_err(debug)?;
    owner.revision_sequencer.acquire(recipient_fence.character_id).await.commit_build(
        owner.root, owner.character, owner.holder, recipient_fence,
        crate::durability::character_build::BuildChangeRequest {
            occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(id(217))
                .map_err(debug)?,
            cause: crate::durability::character_build::BuildCause::VocationTrial,
            before, after: build.clone(), pruned_stance: None,
        }, formula, None,
    ).await.map_err(debug)?;
    let crate::gameplay_transport::CastFactsLoad::Ready { facts, .. } =
        crate::gameplay_transport::load_character_cast_facts(
            owner.root, owner.character, recipient_fence.character_id,
        ).await
    else { return Err("mantra recipient facts".into()); };
    {
        let mut recipient = crate::spell::cast::PlayerSpellState::new(facts, 0, 0)
            .ok_or("mantra recipient state")?;
        recipient.load_owned_build(&build).map_err(debug)?;
        recipient.make_playable(owner.owner_now()).map_err(debug)?;
        let runtime = owner.runtime.lock().await;
        *owner.states.spell_states.lock().await.get_mut(&runtime, players[2].0, players[2].1)
            .ok_or("mantra recipient")? = recipient;
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
        if guided {
            assert_guiding_percentage_without_shared_mantra(owner, player, players[1]).await?;
        }
        owner.refresh_source_party_serene(players[2].0, players[2].1).await;
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let recipient = states.get(&runtime, players[2].0, players[2].1).ok_or("mantra recipient")?;
        assert_eq!(recipient.virtue_party_bonuses(owner.owner_now().get()).mantra, 0, "no local shared Mantra projection");
        assert_eq!(recipient.ascetic_attack_bonus_at(1, owner.owner_now().get()), 0, "Crystal Ascetic ignores other members Mantra");
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
            // Crystal 201189f player_wheel.cpp:4194-4210 and
            // game.cpp:7681-7685,7708-7721: own Mantra adds a separate flat
            // contribution after Wheel flat damage. HarmonyMultiplier above
            // already owns Serene and does not contain equipment Mantra.
            let own_mantra = state.ascetic_attack_bonus(1);
            assert_eq!(own_mantra, 17, "the spender fixture retains its own equipment Mantra");
            let ascetic_flat = i64::from(own_mantra) * 3;
            bounds.map(|b| b + i64::from(flat) + ascetic_flat)
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
    amber_equipment_and_cast(owner, h, player).await?;
    Ok(())
}

async fn amber_equipment_and_cast(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, h: &Harness,
    player: (ExactActorRef, GameSessionId),
) -> TestResult {
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::equipment_move::move_equipment;
    use crate::gameplay_transport::item_move::{EquipmentMove, EquipmentMoveResult};
    use crate::gameplay_transport::item_view::ItemKey;
    use crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue;
    use oteryn_protocol_oteryn::item_view::EquipmentSlot;
    let key = "oteryn:item.tibia.i50239";
    let fence = owner.current_quest_fence(player.1).await.map_err(|_| "fence")?.ok_or("fence")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
        .bind(id(216).as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(key).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,encode($3,'hex')::uuid)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(id(210).as_slice()).bind(id(216).as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let catalogue = ProductionProficiencyCatalogue::embedded().ok_or("catalogue")?;
    let mut track = catalogue.empty_track(key).ok_or("amber track")?;
    track.progress = track.shape.track_shape().final_progress();
    track.selections = vec![None, Some(0), Some(0), Some(0), Some(1), None, None];
    owner.states.proficiencies.lock().map_err(|_| "tracks")?.entry(player.1).or_default().install_track(key, track);
    owner.proficiency_catalogue = Some(catalogue);
    for (equipped, command) in [(true, 120), (false, 122)] {
        let fence = owner.current_quest_fence(player.1).await.map_err(|_| "fence")?.ok_or("fence")?;
        let item = ItemKey::Instance(id(216));
        assert_eq!(move_equipment(owner, player.0, UseCommand {
            game_session_id: player.1, command_id: command, item_fence: Some(fence.item_fence()),
        }, if equipped { EquipmentMove::Equip { item, to: EquipmentSlot::RightHand } }
           else { EquipmentMove::Unequip { item, from: EquipmentSlot::RightHand } }).await,
           EquipmentMoveResult::Committed);
        let spell = owner.spells.spells.iter().find(|s| s.key == "candidate:spell/devastating_knockout").ok_or("knockout")?;
        // After unequipping, inspect magic through a spell that needs no weapon.
        let projection_spell = if equipped { spell } else {
            owner.spells.spells.iter().find(|s| s.key == "candidate:spell/light").ok_or("light")?
        };
        let access = owner.refresh_spell_access(player.0, player.1).await;
        let fence = owner.current_quest_fence(player.1).await.map_err(|_| "fence")?.ok_or("fence")?;
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, player.0, player.1).ok_or("state")?;
            let mut tx = h.pool.begin().await?;
            let mut owned = crate::gameplay_transport::spell_access_facts::load_owned_cast_facts_in_transaction(
                &mut tx, owner.root, owner.character, owner.holder, &fence.item_fence(),
                CommandRef::new(player.1, CommandId::new(command + 10).map_err(debug)?),
                &runtime, player.0, state, owner.active_generation.ok_or("generation")?,
                &access, owner.owner_now().get(),
            ).await.map_err(debug)?;
            let before = owned.numerical_caster(state, projection_spell, owner.active_generation.ok_or("generation")?.native_gameplay().ok_or("native")?,
                crate::spell::harmony::HarmonyMultiplier::with_ascetic(state.character_facts().level, 0, false, false, 0).ok_or("harmony")?, owner.owner_now().get()).map_err(debug)?.magic_level;
            owner.install_cast_proficiency(spell, &mut owned);
            assert_eq!(owned.access.proficiency_skills[7], if equipped { 2 } else { 0 });
            let caster = owned.numerical_caster(state, projection_spell, owner.active_generation.ok_or("generation")?.native_gameplay().ok_or("native")?,
                crate::spell::harmony::HarmonyMultiplier::with_ascetic(state.character_facts().level, 0, false, false, 0).ok_or("harmony")?, owner.owner_now().get()).map_err(debug)?;
            assert_eq!(caster.magic_level, before + if equipped { 2 } else { 0 });
            let augments = &owned.magnitude().ok_or("magnitude")?.item_augments;
            use oteryn_gameplay_content::content::item_spell_augments::ItemSpellAugmentKind as K;
            assert_eq!(augments.iter().filter(|a| a.kind == K::BaseDamage).map(|a| a.value).sum::<u32>(), if equipped { 500 } else { 0 });
            tx.rollback().await?;
        }
        if equipped { cast(owner, player, "devastating_knockout", 121, SpellTarget::AttackTarget).await?; }
    }
    eprintln!("POSTGRES_EXECUTED case=c11r_amber_equipment_and_cast");
    Ok(())
}
