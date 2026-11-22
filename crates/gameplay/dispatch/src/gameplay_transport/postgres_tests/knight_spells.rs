//! Native combat dispatch with current equipment, entitlement and real spell payment.
use super::fixture::{Harness, debug, id};
use super::{TestResult, parameter_spell_fixtures::scenario};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_protocol_oteryn::actor_spell::{
    SpellCastDisposition, SpellCastIntent, SpellTarget, SpellTargetPosition,
};
use std::num::NonZeroU32;

#[test]
fn flurry_of_blows_hits_the_aimed_wave_and_builds_harmony() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(9)
}

#[test]
fn shield_bash_hits_only_its_target_and_weakens_the_next_attack() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(10)
}

#[test]
fn shield_slam_weakens_adjacent_creatures() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(11)
}

#[test]
fn sweeping_takedown_hits_both_areas_and_spends_harmony() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(12)
}

#[test]
fn divine_empowerment_commits_owned_fields_and_expires_the_damage_bonus() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(13)
}

pub(super) async fn cast_case(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    case: u8,
) -> TestResult {
    let name = match case {
        9 => "Flurry of Blows",
        10 => "Shield Bash",
        11 => "Shield Slam",
        12 => "Sweeping Takedown",
        13 => "Divine Empowerment",
        _ => return Err("unknown spell case".into()),
    };
    let native = owner
        .active_generation
        .and_then(|a| a.native_gameplay())
        .ok_or("native content")?;
    let spell = (1..=u32::try_from(native.spell_book().source_len())?)
        .filter_map(NonZeroU32::new)
        .find(|index| {
            native
                .spell_book()
                .indexed(*index)
                .is_some_and(|spell| spell.name == name)
        })
        .ok_or("active spell")?;
    let center = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let position = SpellTarget::Position(SpellTargetPosition {
        x: center.x + 1,
        y: center.y,
        floor: center.floor.unsigned_abs().try_into()?,
    });
    let intent = SpellCastIntent {
        spell,
        target: if matches!(case, 9 | 10 | 12) {
            position
        } else {
            SpellTarget::None
        },
        aim_at_target: matches!(case, 9 | 12),
    };
    let before = owner
        .observe_vitals(actor, session)
        .await
        .ok_or("before vitals")?
        .1;
    if matches!(case, 10 | 11) {
        let refused = owner
            .cast_spell_dispatch(actor, session, 1, intent)
            .await
            .ok_or("no-shield refusal")?;
        assert_ne!(refused.disposition, SpellCastDisposition::Cast);
        assert_eq!(
            owner
                .observe_vitals(actor, session)
                .await
                .ok_or("refused vitals")?
                .1
                .mana,
            before.mana
        );
        let shield = native
            .item_policy_for_source_id(3420)
            .ok_or("Demon Shield policy")?;
        let definition = &shield.record().production_definition;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($1,'hex')::uuid)")
            .bind(id(112).as_slice()).bind(id(42).as_slice()).bind(&definition.production_key)
            .bind(&definition.revision_ref).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,6,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(112).as_slice()).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    let mut targets = Vec::new();
    if case != 13 {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
        let race = "oteryn:creature.demon";
        let maximum = runtime
            .companion_policy(race)
            .map_err(debug)?
            .maximum_health;
        let other = if case == 12 { 2 } else { -1 };
        runtime
            .realize_activation_spawns(vec![(
                vec![
                    (center.x + 1, center.y, center.floor),
                    (center.x + other, center.y, center.floor),
                ],
                60_000,
                5_000,
                race.into(),
                u64::try_from(maximum)?,
            )])
            .map_err(debug)?;
        for x in [center.x + 1, center.x + other] {
            let target = runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|creature| {
                    runtime
                        .companion_snapshot(creature.actor)
                        .is_ok_and(|c| c.position.x == x)
                })
                .ok_or("spawned attack target")?
                .actor;
            targets.push((target, u64::try_from(maximum)?));
        }
    }
    let cast = owner
        .cast_spell_dispatch(actor, session, 2, intent)
        .await
        .ok_or("native cast outcome")?;
    assert_eq!(cast.disposition, SpellCastDisposition::Cast, "{name}");
    let after = owner
        .observe_vitals(actor, session)
        .await
        .ok_or("after vitals")?
        .1;
    let runtime = owner.runtime.lock().await;
    let now = owner.owner_now().get();
    if case == 13 {
        let states = owner.states.spell_states.lock().await;
        let state = states
            .get(&runtime, actor, session)
            .ok_or("empowered caster")?;
        assert!(matches!(
            state.divine_empowerment_percent(center, now),
            8 | 10 | 12
        ));
        assert_eq!(state.divine_empowerment_percent(center, now + 5_000_000), 0);
        assert!(before.mana - after.mana >= 500);
        return Ok(());
    }
    assert!(after.mana < before.mana);
    let [(first, first_max), (other, other_max)] = targets.as_slice() else {
        return Err("expected two attack targets".into());
    };
    assert!(runtime.live_creature_health(*first).ok_or("first health")? < *first_max);
    let other_hit = runtime.live_creature_health(*other).ok_or("other health")? < *other_max;
    assert_eq!(other_hit, matches!(case, 11 | 12));
    match case {
        9 => assert_eq!(after.harmony, before.harmony + 1),
        10 | 11 => {
            assert_eq!(runtime.shield_attack_damage_percent(*first, now), 50);
            assert_eq!(
                runtime.shield_attack_damage_percent(*other, now),
                if case == 11 { 50 } else { 100 }
            );
            assert_eq!(
                runtime.shield_attack_damage_percent(*first, now + 10_000_000),
                100
            );
        }
        12 => assert_eq!(after.harmony, 0),
        _ => (),
    }
    Ok(())
}
