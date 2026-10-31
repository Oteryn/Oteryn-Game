//! W01c: source melee, ranged arrows and targeted areas reach real players and fenced saves.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::{ComposedFreshAdmission, last_position::AdmissionPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::time::Duration;

#[path = "monster_attacks/invisible_area.rs"]
mod invisible_area;
#[path = "monster_attacks/removal.rs"]
mod removal;

#[test]
fn rat_source_melee_dispatch_damages_player_once_and_checkpoints_vitals_postgres() -> TestResult {
    run_case("monster_attacks_rat", AttackCase::Rat)
}

#[test]
fn hunter_source_arrow_dispatch_damages_distant_player_and_checkpoints_vitals_postgres() -> TestResult {
    run_case("monster_attacks_hunter", AttackCase::Hunter)
}

#[test]
fn fire_devil_source_area_dispatch_damages_target_and_bystander_postgres() -> TestResult {
    run_case("monster_attacks_fire_devil", AttackCase::FireDevil)
}

#[test]
fn fire_devil_source_area_dispatch_damages_invisible_bystander_and_checkpoints_vitals_postgres() -> TestResult {
    run_case("monster_attacks_invisible_area", AttackCase::FireDevilInvisibleBystander)
}

#[test]
fn fire_elemental_source_field_dispatch_commits_owned_ground_once_postgres() -> TestResult {
    run_case("monster_attacks_fire_field", AttackCase::FireElemental)
}

#[test]
fn firestarter_zero_range_source_field_dispatch_commits_distant_ground_postgres() -> TestResult {
    run_case("monster_attacks_zero_range_field", AttackCase::Firestarter)
}

#[test]
fn the_monster_source_removal_dispatch_retires_owned_wall_once_postgres() -> TestResult {
    run_case("monster_attacks_wall_removal", AttackCase::TheMonster)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AttackCase {
    Rat,
    Hunter,
    FireDevil,
    FireDevilInvisibleBystander,
    FireElemental,
    Firestarter,
    TheMonster,
}

impl AttackCase {
    fn creature_key(self) -> &'static str {
        match self {
            Self::Rat => "oteryn:creature.rat",
            Self::Hunter => "oteryn:creature.hunter",
            Self::FireDevil | Self::FireDevilInvisibleBystander => "oteryn:creature.fire_devil",
            Self::FireElemental => "oteryn:creature.fire_elemental",
            Self::Firestarter => "oteryn:creature.firestarter",
            Self::TheMonster => "oteryn:creature.the_monster",
        }
    }
}

fn run_case(tag: &'static str, case: AttackCase) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("MONSTER_AI POSTGRES_CASE_SKIPPED: PostgreSQL is not configured");
        return Ok(());
    }
    // A composed dispatcher future needs the same explicit stack under cargo and nextest.
    std::thread::Builder::new()
        .name("monster-attacks-postgres".into())
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            run(async |admin| {
                let harness = Harness::create_with_admission_seed(
                    admin,
                    tag,
                    false,
                    false,
                )
                .await?;
                let result = if case == AttackCase::TheMonster {
                    removal::scenario(&harness).await
                } else if matches!(case, AttackCase::FireElemental | AttackCase::Firestarter) {
                    field_scenario(&harness, case).await
                } else {
                    scenario(&harness, case).await
                };
                let cleanup = harness.cleanup().await;
                result?;
                cleanup?;
                eprintln!("MONSTER_AI POSTGRES_CASE_COMPLETED {tag}");
                Ok(())
            })
            .map_err(|error| format!("{error:?}"))
        })?
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        .map_err(Into::into)
}

async fn field_scenario(harness: &Harness, case: AttackCase) -> TestResult {
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
    let active = controller.active().ok_or("active native generation")?;
    let (deps, monster) = dependencies(active, &pin, case)?;
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells();
    // The source keeps all three canonical attacks. A larger test player pool survives
    // their real damage while the field's original schedule gets its first cast.
    let initial_health = crate::gameplay_transport::actor_spell::tests::FACTS.max_health
        .checked_mul(128)
        .ok_or("test player pool overflow")?;
    initialize_player(&owner, deps.actor, deps.admitted.game_session_id, initial_health).await?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let record = native.creature_profiles().records.iter()
        .find(|record| record.profile.target.key == case.creature_key())
        .ok_or("source field creature profile")?;
    let crate::content::ProjectV2AuthoringProfileData::Behavior(behavior) =
        &record.behavior.as_ref().ok_or("source behavior")?.data else {
            return Err("source behavior kind".into());
        };
    let field = behavior.attacks.get(2).ok_or("source field attack")?;
    let (chance, range) = match case {
        AttackCase::FireElemental => (250_000, 7),
        AttackCase::Firestarter => (100_000, 0),
        _ => return Err("unexpected source field case".into()),
    };
    assert_eq!((field.interval_ms, field.chance_ppm), (2_000, chance));
    let source = native.source_item_ability(&record.profile.target, &field.ability)
        .ok_or("canonical source field body")?;
    let crate::content::ProjectV2AuthoringProfileData::Ability(source) = &source.data else {
        return Err("source field Ability kind".into());
    };
    let details = source.details.as_ref().ok_or("source field details")?;
    assert_eq!(details.range_tiles, range);
    assert!(details.needs_target);
    assert_eq!(details.area, Some(crate::content::ProjectV2AbilityArea::Circle { radius_tiles: 1 }));
    assert!(committed_source_fields(harness, monster, case, &field.ability.key).await?.is_empty());
    owner.clock_origin -= Duration::from_secs(10);
    let mut committed = Vec::new();
    for _ in 0..64 {
        owner.drain_monster_melee().await;
        committed = committed_source_fields(harness, monster, case, &field.ability.key).await?;
        if !committed.is_empty() {
            break;
        }
        owner.clock_origin -= Duration::from_millis(2_000);
    }
    if committed.is_empty() {
        let receipts: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT spell_production_key, creature_occurrence, operation_count::text FROM game_spell_item_receipts WHERE cause_kind=1 AND creature_placement=$1 ORDER BY creature_occurrence",
        )
        .bind(monster.placement_identity().as_slice())
        .fetch_all(&harness.pool)
        .await?;
        eprintln!("MONSTER_AI SOURCE_FIELD_RECEIPTS {receipts:?}");
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let table = &states.monster_melee.creature_ai;
        eprintln!(
            "MONSTER_AI SOURCE_FIELD_MISSING ability={} entry=2 ai_sequence={} next_pass={} think_due={:?} think={:?} melee_selection={:?} monster_position={:?} target_position={:?} monster_health={:?} player_health={:?}",
            field.ability.key, states.monster_ai_sequence, states.next_monster_ai_pass_us,
            table.due(monster), table.state(monster), table.native_melee_selection(monster, 0),
            runtime.read_actor_position(monster), runtime.read_actor_position(deps.actor),
            runtime.live_creature_health(monster),
            states.get(&runtime, deps.actor, deps.admitted.game_session_id).map(|state| state.vitals().health),
        );
    }
    assert_eq!(committed.len(), 1, "one target-centered source field is physically committed");
    assert_eq!(committed[0].2, "oteryn:item.tibia.i2118");
    assert_eq!((committed[0].3.as_str(), committed[0].4.as_str()), ("1", "1"));
    assert_eq!(committed[0].6.as_slice(), committed[0].7.as_bytes(),
        "noncorpse Ground retains its exact committed native attack occurrence");
    assert!(committed[0].6.starts_with(b"ai-profile:"));
    assert!((1..=128).contains(&committed[0].6.len()));
    if range == 0 {
        let runtime = owner.runtime.lock().await;
        assert_ne!(runtime.read_actor_position(monster).map_err(debug)?.position(),
            runtime.read_actor_position(deps.actor).map_err(debug)?.position(),
            "canonical zero source range is uncapped, not restricted to the caster cell");
    }
    let sequence = owner.states.spell_states.lock().await.monster_ai_sequence;
    owner.drain_monster_melee().await;
    assert_eq!(owner.states.spell_states.lock().await.monster_ai_sequence, sequence);
    assert_eq!(
        committed_source_fields(harness, monster, case, &field.ability.key).await?, committed,
        "a repeated connection drain does not mint a second field or receipt",
    );
    Ok(())
}

async fn committed_source_fields(
    harness: &Harness,
    monster: ExactActorRef,
    case: AttackCase,
    ability: &str,
) -> TestResult<Vec<(String, String, String, String, String, String, Vec<u8>, String)>> {
    Ok(sqlx::query_as(
        "SELECT r.transaction_id::text, i.item_instance_id::text, i.definition_production_key, i.quantity::text, i.lifecycle::text, i.state_revision::text, g.corpse_ref, r.creature_occurrence FROM game_spell_item_receipts r JOIN game_spell_item_lines l ON l.transaction_id=r.transaction_id JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id AND i.minted_transaction_id=r.transaction_id JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id JOIN game_spell_item_audit_outbox a ON a.transaction_id=r.transaction_id AND a.event_id=r.event_id WHERE r.cause_kind=1 AND r.creature_placement=$1 AND r.creature_definition_key=$2 AND r.spell_production_key=$3 AND l.operation_kind=1 ORDER BY i.item_instance_id",
    )
    .bind(monster.placement_identity().as_slice())
    .bind(case.creature_key())
    .bind(ability)
    .fetch_all(&harness.pool)
    .await?)
}

async fn scenario(harness: &Harness, case: AttackCase) -> TestResult {
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    // Keep the canonical admitted profiles and their source intervals/chances unchanged.
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
    let active = controller.active().ok_or("active native generation")?;
    let (deps, monster) = dependencies(active, &pin, case)?;
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells();
    let maximum = maximum_source_damage(active, case)?;
    let area_case = matches!(case, AttackCase::FireDevil | AttackCase::FireDevilInvisibleBystander);
    let invisible_bystander = case == AttackCase::FireDevilInvisibleBystander;
    // Only the test player's pool is enlarged for the area case. Both source area entries
    // retain their original chances and damage while we wait for both players to be hit.
    let initial_health = if area_case {
        maximum.checked_mul(128)
            .and_then(|n| n.checked_add(crate::gameplay_transport::actor_spell::tests::FACTS.max_health))
            .ok_or("test pool overflow")?
    } else {
        crate::gameplay_transport::actor_spell::tests::FACTS.max_health
    };
    initialize_player(&owner, deps.actor, deps.admitted.game_session_id, initial_health).await?;
    let session = deps.admitted.game_session_id;
    let bystander = if area_case {
        let second_session = GameSessionId::decode(&id(51)).map_err(debug)?;
        let second = {
            let mut runtime = deps.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(second_session).map_err(debug)?;
            let second = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime.initialize_source_pinned_lab_player_position(
                second,
                second_session,
                MovementLocalPosition { x: 4, y: 12, floor: -7 },
            ).map_err(debug)?;
            second
        };
        initialize_player(&owner, second, second_session, initial_health).await?;
        Some((second, second_session))
    } else {
        None
    };
    let fence = owner
        .current_monk_fence(session)
        .await
        .map_err(|()| "current player fence")?
        .ok_or("current player session")?;
    assert!(owner.track_restored(session, deps.actor, &AdmissionPosition::FirstLogin, fence));
    let before = health(&owner, &deps).await?;
    assert!(before > maximum, "one source hit is nonlethal in this fixture");

    // Production scheduling owns the attack. A zero source roll is a valid swing, so allow
    // bounded source intervals until a deterministic positive roll has been observed.
    owner.clock_origin -= Duration::from_secs(10);
    if invisible_bystander {
        // The persisted first player is the hidden area recipient. The second stays
        // visible and is acquired normally; no source profile or protection is changed.
        invisible_area::install(&owner, &deps).await?;
    }
    let mut after = before;
    let mut bystander_hit = bystander.is_none();
    for _ in 0..64 {
        if case != AttackCase::Rat {
            assert_distant(&owner, monster, deps.actor).await?;
        }
        owner.drain_monster_melee().await;
        if invisible_bystander {
            invisible_area::assert_current(
                &owner, &deps, monster, bystander.ok_or("visible source area target")?,
            ).await?;
        }
        after = health(&owner, &deps).await?;
        if case != AttackCase::Rat {
            assert_distant(&owner, monster, deps.actor).await?;
        }
        if let Some((actor, session)) = bystander {
            bystander_hit = player_health(&owner, actor, session).await? < before;
        }
        if after < before && bystander_hit {
            break;
        }
        owner.clock_origin -= Duration::from_millis(2_000);
    }
    assert!(after < before, "the real source attack must damage the admitted player");
    assert!(bystander_hit, "the source area must also hit the second player");
    if !area_case {
        assert!(before - after <= maximum, "one hit stays within its source range");
    }
    {
        let states = owner.states.spell_states.lock().await;
        let think = states
            .monster_melee
            .creature_ai
            .state(monster)
            .ok_or("production monster think state")?;
        assert!(think.target().is_some());
        if bystander.is_none() {
            assert_eq!(think.target(), Some(deps.actor));
        }
        if case != AttackCase::Rat {
            assert!(!states.monster_spells.casts.is_empty(), "the production spell owner executed");
        }
    }
    let sequence = owner.states.spell_states.lock().await.monster_ai_sequence;
    let repeat_area = if invisible_bystander {
        let (actor, session) = bystander.ok_or("visible source area target")?;
        Some((player_health(&owner, actor, session).await?,
            owner.states.spell_states.lock().await.monster_spells.casts.len()))
    } else {
        None
    };
    owner.drain_monster_melee().await;
    assert_eq!(
        owner.states.spell_states.lock().await.monster_ai_sequence,
        sequence,
        "a second connection drain before the shared deadline is not another swing"
    );
    assert_eq!(health(&owner, &deps).await?, after);
    if let Some((visible_health, cast_count)) = repeat_area {
        let visible = bystander.ok_or("visible source area target")?;
        invisible_area::assert_current(&owner, &deps, monster, visible).await?;
        assert_eq!(player_health(&owner, visible.0, visible.1).await?, visible_health);
        assert_eq!(owner.states.spell_states.lock().await.monster_spells.casts.len(), cast_count,
            "an unchanged drain does not publish another area occurrence");
    }

    if case == AttackCase::Rat {
        rat_movement_retry(&mut owner, &deps, monster).await?;
        after = health(&owner, &deps).await?;
    }

    // Existing CHAR-VITALS-1 owns HP durability: its session-fenced transaction writes the
    // actual post-attack pools with the position, and an unchanged retry writes nothing.
    assert_eq!(owner.write_due_last_positions(true).await, 0);
    let stored = saved_vitals(harness).await?;
    assert_eq!(stored.0, Some(i64::from(after)));
    {
        let runtime = deps.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let state = states.get(&runtime, deps.actor, session).ok_or("player vitals")?;
        assert_eq!(stored.1, Some(i64::from(state.vitals().mana)));
    }
    assert_eq!(owner.write_due_last_positions(true).await, 0);
    assert_eq!(saved_vitals(harness).await?, stored, "same pools keep the durable order key");
    Ok(())
}

async fn health(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
) -> TestResult<u32> {
    player_health(owner, deps.actor, deps.admitted.game_session_id).await
}

async fn player_health(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<u32> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    Ok(states
        .get(&runtime, actor, session)
        .ok_or("live player state")?
        .vitals()
        .health)
}

async fn initialize_player(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    max_health: u32,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    owner.states.spell_states.lock().await.initialize(
        &runtime, actor, session,
        crate::spell::cast::CharacterCastFacts {
            max_health,
            ..crate::gameplay_transport::actor_spell::tests::FACTS
        },
        (0, 0), owner.owner_now(),
    ).ok_or("initialize player spell state")?;
    Ok(())
}

async fn assert_distant(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    monster: ExactActorRef,
    player: ExactActorRef,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let from = runtime.read_actor_position(monster).map_err(debug)?.position();
    let to = runtime.read_actor_position(player).map_err(debug)?.position();
    assert!(from.x.abs_diff(to.x).max(from.y.abs_diff(to.y)) > 1, "ranged proof excludes melee");
    Ok(())
}

fn maximum_source_damage(active: &crate::content::ActiveGeneration, case: AttackCase) -> TestResult<u32> {
    if case == AttackCase::Rat {
        return Ok(8);
    }
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let record = native.creature_profiles().records.iter()
        .find(|record| record.profile.target.key == case.creature_key())
        .ok_or("source creature profile")?;
    let definitions = record.attack_definitions.as_ref().ok_or("canonical attack closure")?;
    let source = crate::creature_damage_spell::SpellSource::from_native(
        &record.profile.target, 1, &definitions.records, &definitions.profiles, native.source_digest(),
    ).map_err(debug)?;
    if case == AttackCase::Hunter {
        assert_eq!((source.minimum, source.maximum, source.range), (0, 100, 7));
        assert!(source.area.is_none());
        return Ok(source.maximum);
    }
    // Crystal 201189f, data-crystal/monster/demons/fire_devil.lua:93-94,119.
    let crate::content::ProjectV2AuthoringProfileData::Behavior(behavior) =
        &record.behavior.as_ref().ok_or("source area behavior")?.data else {
            return Err("source area behavior kind".into());
        };
    assert!(!behavior.targeting.sense_invisible);
    for (index, chance) in [(1, 200_000), (2, 150_000)] {
        let attack = behavior.attacks.get(index).ok_or("source area schedule")?;
        assert_eq!((attack.interval_ms, attack.chance_ppm), (2_000, chance));
    }
    assert_eq!((source.minimum, source.maximum, source.range), (60, 90, 7));
    assert_eq!(source.element.as_deref(), Some("fire"));
    assert_eq!(source.area, Some(crate::content::ProjectV2AbilityArea::Circle { radius_tiles: 4 }));
    assert!(source.needs_target);
    let second = crate::creature_damage_spell::SpellSource::from_native(
        &record.profile.target, 2, &definitions.records, &definitions.profiles, native.source_digest(),
    ).map_err(debug)?;
    assert_eq!((second.minimum, second.maximum), (20, 50));
    assert_eq!(second.area, Some(crate::content::ProjectV2AbilityArea::Circle { radius_tiles: 2 }));
    Ok(source.maximum.max(second.maximum))
}

async fn saved_vitals(harness: &Harness) -> TestResult<(Option<i64>, Option<i64>, i64)> {
    Ok(sqlx::query_as(
        "SELECT health::bigint, mana::bigint, write_sequence::bigint FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

fn dependencies(
    active: &crate::content::ActiveGeneration,
    pin: &crate::content::NativeEntryContentPin,
    case: AttackCase,
) -> TestResult<(Dependencies, ExactActorRef)> {
    let mut deps = Dependencies::new()?;
    deps.room = pin.qualified_room().clone();
    deps.combat_room = Some(deps.room.clone());
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let channel_pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        pin.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        deps.room.map_revision_digest(),
        (2, 11, -7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        channel_pin,
    )
    .map_err(debug)?;
    native.install_companion_policies(&mut runtime).map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_source_pinned_lab_player_position(
            deps.actor,
            deps.admitted.game_session_id,
            MovementLocalPosition { x: 3, y: 12, floor: -7 },
        )
        .map_err(debug)?;
    let maximum_health = u64::try_from(
        runtime.companion_policy(case.creature_key()).map_err(debug)?.maximum_health,
    )?;
    runtime
        .begin_activation_spawn_sources(vec![(
            vec![((if matches!(case, AttackCase::Rat | AttackCase::TheMonster) { 2 } else { -1 }, 12, -7), 60_000, case.creature_key().into(), maximum_health, false)],
            5_000,
        )])
        .map_err(debug)?;
    assert_eq!(runtime.realize_activation_spawn_window(0).map_err(debug)?.realized, 1);
    let monster = runtime.spawn_point_creatures()[0].2.ok_or("spawned source monster")?;
    runtime.companion_snapshot(monster).map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    Ok((deps, monster))
}

/// A due, unreachable canonical Rat melee gains exactly one movement retry through
/// the production drain. The original source may roll zero; owner tests separately
/// prove the fixed-positive damage receipt, while this case proves the real callsite.
async fn rat_movement_retry(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    monster: ExactActorRef,
) -> TestResult {
    use crate::movement::CardinalStep;

    let before = health(owner, deps).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        let player = runtime.read_actor_position(deps.actor).map_err(debug)?.position();
        assert_eq!(player, MovementLocalPosition { x: 3, y: 12, floor: -7 });
        let mut at = runtime.read_actor_position(monster).map_err(debug)?.position();
        assert_eq!(at.floor, -7);
        assert!((2..=4).contains(&at.x) && (11..=13).contains(&at.y));
        // The qualified Thalom rectangle is open. Use y=11 as a corridor so the
        // actual native Movement steps never pass through the player's occupied cell.
        if at.x == player.x && at.y > 11 {
            at = step_in_source_fixture(&mut runtime, &deps.room, monster, CardinalStep::West)?;
        }
        while at.y > 11 {
            at = step_in_source_fixture(&mut runtime, &deps.room, monster, CardinalStep::North)?;
        }
        while at.x > 0 {
            at = step_in_source_fixture(&mut runtime, &deps.room, monster, CardinalStep::West)?;
        }
        let at = step_in_source_fixture(&mut runtime, &deps.room, monster, CardinalStep::South)?;
        assert_eq!(at, MovementLocalPosition { x: 0, y: 12, floor: -7 });
    }

    // Each production full Think advances the shared source clock by its original
    // 1000ms. Translate before both drains to invalidate any already queued chase
    // snapshot; neither tick can close the held target's three-cell gap.
    for tick in 0..2 {
        if tick == 1 {
            let mut runtime = owner.runtime.lock().await;
            let mut at = runtime.read_actor_position(monster).map_err(debug)?.position();
            // A fresh post-Think chase may have made one step; re-establish a
            // three-cell gap with real translations before the second full tick.
            while at.y > 11 {
                at = step_in_source_fixture(&mut runtime, &deps.room, monster, CardinalStep::North)?;
            }
            while at.x > 0 {
                at = step_in_source_fixture(&mut runtime, &deps.room, monster, CardinalStep::West)?;
            }
            while at.y < 13 {
                at = step_in_source_fixture(&mut runtime, &deps.room, monster, CardinalStep::South)?;
            }
            assert_eq!(at, MovementLocalPosition { x: 0, y: 13, floor: -7 });
        }
        owner.clock_origin -= Duration::from_millis(2_000);
        owner.drain_monster_melee().await;
        assert_eq!(health(owner, deps).await?, before, "out-of-reach source melee cannot damage");
        let runtime = owner.runtime.lock().await;
        let at = runtime.read_actor_position(monster).map_err(debug)?.position();
        assert!(at.x.abs_diff(3).max(at.y.abs_diff(12)) > 1,
            "a fresh chase step still leaves the canonical source melee out of reach");
    }

    let (selection, think_due, pass_due, swing_due, global_sequence) = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let table = &states.monster_melee.creature_ai;
        assert!(!table.has_movement_melee_retry(&runtime));
        let selection = table.native_melee_selection(monster, 0)
            .ok_or("canonical Rat source selection")?;
        assert_eq!(selection.target, None, "due melee still lacks reach");
        assert_eq!(table.state(monster).and_then(|state| state.target()), Some(deps.actor));
        (selection, table.due(monster), states.next_monster_ai_pass_us,
            states.monster_melee.next_swing_pass_us, states.monster_ai_sequence)
    };
    {
        let mut runtime = owner.runtime.lock().await;
        let rat = runtime.read_actor_position(monster).map_err(debug)?.position();
        assert!((-1..=1).contains(&rat.x) && (12..=14).contains(&rat.y));
        // Approach the current caster from its east through qualified open cells;
        // a source chase may have made one step after the out-of-reach Think.
        let mut at = step_in_source_fixture(&mut runtime, &deps.room, deps.actor, CardinalStep::West)?;
        while at.y < rat.y {
            at = step_in_source_fixture(&mut runtime, &deps.room, deps.actor, CardinalStep::South)?;
        }
        while at.x > rat.x + 1 {
            at = step_in_source_fixture(&mut runtime, &deps.room, deps.actor, CardinalStep::West)?;
        }
        assert_eq!(at, MovementLocalPosition { x: rat.x + 1, y: rat.y, floor: -7 });
        let states = owner.states.spell_states.lock().await;
        assert!(states.monster_melee.creature_ai.has_movement_melee_retry(&runtime),
            "held target translation alone wakes the pending extra melee");
        let now = owner.owner_now().get();
        assert!(now < pass_due && now < swing_due && think_due.is_some_and(|due| now < due),
            "retry precedes the shared pass, swing and full-Think deadlines");
    }
    owner.drain_monster_melee().await;
    let after = health(owner, deps).await?;
    assert!(after <= before && before - after <= 8, "canonical Rat roll remains 0..=8");
    let retried = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let table = &states.monster_melee.creature_ai;
        let retried = table.native_melee_selection(monster, 0)
            .ok_or("movement-selected canonical Rat melee")?;
        assert_eq!(retried.occurrence.sequence, selection.occurrence.sequence + 1);
        assert_eq!(retried.target, Some(deps.actor));
        assert_eq!(table.due(monster), think_due);
        assert_eq!(states.next_monster_ai_pass_us, pass_due);
        assert_eq!(states.monster_melee.next_swing_pass_us, swing_due);
        assert_eq!(states.monster_ai_sequence, global_sequence);
        assert!(!table.has_movement_melee_retry(&runtime));
        retried
    };
    owner.drain_monster_melee().await;
    assert_eq!(health(owner, deps).await?, after, "unchanged poll cannot damage again");
    {
        let states = owner.states.spell_states.lock().await;
        assert_eq!(states.monster_melee.creature_ai.native_melee_selection(monster, 0), Some(retried));
        assert_eq!(states.monster_melee.creature_ai.due(monster), think_due);
        assert_eq!(states.monster_ai_sequence, global_sequence);
    }
    Ok(())
}

fn step_in_source_fixture(
    runtime: &mut ChannelRuntimeV1,
    room: &crate::content::QualifiedNativeEntryRoom,
    actor: ExactActorRef,
    step: crate::movement::CardinalStep,
) -> TestResult<MovementLocalPosition> {
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let selection = crate::movement::MovementEngineeringSelection {
        owner_context: before.context(),
        content_scope: room.movement_cells().scope(),
    };
    let moved = crate::movement::step_cardinal(
        &mut runtime.borrow_movement_position(), actor, before, &selection,
        room.movement_cells().index(), step,
    ).map_err(debug)?;
    assert!(!runtime.position_occupied_by_other(actor, moved.position()).map_err(debug)?);
    Ok(moved.position())
}
