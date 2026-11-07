//! Actual native attacks and creature thinks clear the source corridor; no kill flag.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use oteryn_protocol_oteryn::attack::*;

const HEALTH_POTION: &str = "oteryn:item.tibia.i7643";

pub(super) async fn prepare_supplies(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    member: &AdmittedSession,
    build: &crate::durability::character_build::DurableBuildState,
    stock: bool,
) -> TestResult {
    use crate::durability::{
        item_mint::TypedDefinitionRef,
        starter_grant::{StarterGrantOutcome, StartingLoadout},
    };
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("session")?
        .item_fence();
    let loadout = StartingLoadout::resolve(build.vocation(), |id| {
        let key = format!("oteryn:item.tibia.i{id}");
        let revision_ref = native.item_index()?.revision(&key)?.to_owned();
        Some(TypedDefinitionRef {
            family: "Item".into(),
            production_key: key,
            revision_ref,
        })
    })
    .ok_or("official starting loadout")?;
    let outcome = owner
        .root
        .grant_starting_loadout(
            owner.character,
            owner.holder,
            &fence,
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            &loadout,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(
        outcome,
        StarterGrantOutcome::Minted(_) | StarterGrantOutcome::AlreadyGranted
    ));
    if stock {
        for occurrence in [4000, 4001] {
            super::quest_world_callbacks::input_item(
                owner,
                fence,
                &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                    position: oteryn_gameplay_quests::quest::world_callbacks::Position {
                        x: 33215,
                        y: 31671,
                        floor: -13,
                    },
                    item: HEALTH_POTION.into(),
                    count: 100,
                },
                occurrence,
                true,
            )
            .await?;
        }
    }
    Ok(())
}

pub(super) async fn heal(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::*;
    member.item_fence = Some(
        owner
            .current_quest_fence(member.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("session")?
            .item_fence(),
    );
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(2).ok_or("handle")?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", HEALTH_POTION, "definition-r1"))
        .ok_or("potion reference")?;
    let (next, status, result, _) = super::special_potions::exchange(
        owner,
        *member,
        member.continuity.next_command_id,
        &payload,
        Some(reference),
        None,
    )
    .await?;
    *member = next;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    assert_eq!(
        decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    Ok(())
}

pub(super) async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    kind: u32,
    payload: Vec<u8>,
) -> TestResult {
    let sequence = member.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(owner, member, sequence, kind, payload).await?;
    assert_eq!(
        decode_attack_intent_result(&reply).map_err(debug)?,
        AttackIntentDisposition::Ok
    );
    Ok(())
}

pub(super) async fn clear_corridor(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
) -> TestResult {
    let player = member.runtime_actor.ok_or("player")?;
    // Retain the actual source actors before combat. Native AI is free to move
    // them; a later coordinate lookup must not substitute a different creature.
    let corridor = {
        let runtime = owner.runtime.lock().await;
        let mut targets = Vec::new();
        for x in [33223, 33224] {
            let target = runtime
                .visible_entities()
                .creatures
                .into_iter()
                .find(|row| {
                    runtime.read_actor_position(row.actor).is_ok_and(|at| {
                        let p = at.position();
                        p.x == x && p.y == 31659 && p.floor == 13
                    })
                })
                .ok_or("source corridor demon")?;
            let initial = runtime.companion_snapshot(target.actor).map_err(debug)?;
            assert_eq!(
                initial.health, initial.maximum_health,
                "source full health before combat"
            );
            targets.push((x, target, initial));
        }
        targets
    };
    command(
        owner,
        member,
        COMMAND_TYPE_FIGHT_MODES_INTENT,
        encode_fight_modes_intent(&FightModes {
            fight_mode: FightMode::Offensive,
            ..FightModes::DEFAULT
        }),
    )
    .await?;
    let mut observed_incoming_damage = false;
    for (x, target, initial) in corridor {
        let entity = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: target.actor.placement_identity(),
            generation: target.generation,
        };
        command(
            owner,
            member,
            COMMAND_TYPE_ATTACK_TARGET_INTENT,
            encode_attack_target_intent(Some(&entity)).map_err(debug)?,
        )
        .await?;
        let mut killed = false;
        let mut observed_outgoing_damage = false;
        for pass in 0..512 {
            owner
                .refresh_attack_weapon(player, member.game_session_id)
                .await;
            if pass % 8 == 0 {
                let state = owner.states.attack.lock().await.combat_state(
                    player,
                    member.game_session_id,
                    owner.owner_now(),
                );
                eprintln!(
                    "ANNIHILATOR_ATTACK_STATE pass={pass} target_bound={} in_fight={}",
                    state.target.is_some(),
                    state.in_fight
                );
            }
            let before_attack = owner
                .runtime
                .lock()
                .await
                .companion_snapshot(target.actor)
                .map(|s| s.health)
                .unwrap_or(0);
            owner.drain_auto_attacks().await;
            let after_attack = owner
                .runtime
                .lock()
                .await
                .companion_snapshot(target.actor)
                .map(|s| s.health)
                .unwrap_or(0);
            observed_outgoing_damage |= after_attack < before_attack;
            owner.drain_monster_melee().await;
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert!(
                !states.is_dead(player),
                "source encounter killed the participant"
            );
            let player_state = states
                .get(&runtime, player, member.game_session_id)
                .ok_or("player state")?;
            observed_incoming_damage |=
                player_state.health < player_state.character_facts().max_health;
            if runtime
                .companion_snapshot(target.actor)
                .is_ok_and(|s| s.health > 0)
            {
                assert!(
                    states
                        .monster_melee
                        .creature_ai
                        .state(target.actor)
                        .is_some(),
                    "source demon must participate in native AI"
                );
            }
            let needs_healing =
                player_state.character_facts().max_health - player_state.health >= 1200;
            drop(states);
            let remaining = runtime
                .companion_snapshot(target.actor)
                .map(|s| s.health)
                .unwrap_or(0);
            drop(runtime);
            if pass == 9 {
                assert!(
                    observed_outgoing_damage,
                    "real source-map attacks must deal damage"
                );
            }
            if remaining == 0 {
                assert!(
                    owner
                        .states
                        .attack
                        .lock()
                        .await
                        .kills()
                        .has_session(member.game_session_id),
                    "native lethal damage must emit the real reward occurrence"
                );
                killed = true;
                break;
            }
            if needs_healing {
                heal(owner, member).await?;
            }
            if pass % 32 == 0 {
                eprintln!(
                    "ANNIHILATOR_COMBAT_PROGRESS x={x} pass={pass} before_attack={before_attack} after_attack={after_attack} health={remaining}/{}",
                    initial.maximum_health
                );
            }
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_millis(
                    crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS,
                ))
                .ok_or("clock")?;
        }
        assert!(
            killed,
            "source full-health demon survived real combat budget"
        );
        let outcome = owner.drain_kill_rewards(member.game_session_id).await;
        assert!(
            matches!(
                outcome,
                crate::gameplay_transport::kill_reward::DrainOutcome::Drained
            ),
            "native kill drain: {outcome:?}"
        );
        super::annihilator_route::step(
            owner,
            member,
            oteryn_protocol_oteryn::world_spatial::StepDirection::East,
        )
        .await?;
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(player)
            .map_err(debug)?
            .position();
        assert_eq!(
            (at.x, at.y, at.floor),
            (x, 31659, 13),
            "cross the actually cleared source tile"
        );
    }
    assert!(
        observed_incoming_damage,
        "real source AI must damage the participant"
    );
    eprintln!("ANNIHILATOR_COMBAT native_corridor_kills=2 source_ai_thinks=active");
    Ok(())
}
