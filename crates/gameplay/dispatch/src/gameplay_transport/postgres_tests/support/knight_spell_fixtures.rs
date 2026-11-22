//! Shared physical and durable setup for the native combat spell regressions.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::actor_spell::{
    SpellCastDisposition, SpellCastIntent, SpellTarget, SpellTargetPosition,
};
use std::num::NonZeroU32;

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
        floor: center.floor,
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
    assert!(owner.ensure_source_map_initialized().await);
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
    if case == 13 {
        assert_known_field_lifetime(h, owner, session).await?;
        assert_eq!(before.mana - after.mana, 500);
        return Ok(());
    }
    let runtime = owner.runtime.lock().await;
    let now = owner.owner_now().get();
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
            let damage_percent = |target: ExactActorRef, now| {
                let snapshot = runtime.companion_snapshot(target).map_err(debug)?;
                Ok::<_, String>(match snapshot.state.conditions.attributes_at(now) {
                    Some(crate::domain::condition::ConditionValues::Attributes {
                        damage_dealt_percent,
                        ..
                    }) => damage_dealt_percent,
                    _ => 100,
                })
            };
            assert_eq!(damage_percent(*first, now)?, 50);
            assert_eq!(
                damage_percent(*other, now)?,
                if case == 11 { 50 } else { 100 }
            );
            assert_eq!(damage_percent(*first, now + 10_000_000)?, 100);
            drop(runtime);
            assert_next_two_bites(owner, actor, session, *first).await?;
        }
        12 => assert_eq!(after.harmony, 0),
        _ => (),
    }
    Ok(())
}

async fn assert_known_field_lifetime(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> TestResult {
    let lifetimes: Vec<i64> = sqlx::query_scalar(
        "SELECT l.expires_at_unix_ms-r.occurred_at_unix_ms
           FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
           JOIN game_item_ground_locations g USING(item_instance_id)
          WHERE r.game_session_id=encode($1,'hex')::uuid AND l.operation_kind=1
            AND l.definition_production_key='oteryn:item.tibia.i40450'",
    )
    .bind(session.as_bytes().as_slice())
    .fetch_all(&h.pool)
    .await?;
    assert!(!lifetimes.is_empty() && lifetimes.len() <= 9);
    assert!(lifetimes.iter().all(|lifetime| *lifetime == 5000));

    // Advance only the fixture's recorded expiry. The real item owner performs the removal.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_spell_item_lines l SET expires_at_unix_ms=1
           FROM game_spell_item_receipts r WHERE l.transaction_id=r.transaction_id
            AND r.game_session_id=encode($1,'hex')::uuid AND l.operation_kind=1
            AND l.definition_production_key='oteryn:item.tibia.i40450'",
    )
    .bind(session.as_bytes().as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    owner
        .states
        .spell_states
        .lock()
        .await
        .next_item_deadline_pass_us = 0;
    owner.root.await_ready().await?;
    owner.drain_source_item_deadlines().await;
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
           JOIN game_item_ground_locations g USING(item_instance_id)
          WHERE r.game_session_id=encode($1,'hex')::uuid AND l.operation_kind=1
            AND l.definition_production_key='oteryn:item.tibia.i40450'",
    ).bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(remaining, 0);
    Ok(())
}

async fn assert_next_two_bites(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    issuer: ExactActorRef,
) -> TestResult {
    use crate::ability::creature_bite::{
        AiAbilityAdapter, CreatureBiteDefinition, CreatureBiteLedger, ReentryProtection,
        commit_ai_bite,
    };
    use crate::gameplay_transport::attack::player_block::PlayerBiteVitals;
    use oteryn_simulation_determinism::SemanticTimeMicros;

    let mut runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let mut attack = owner.states.attack.lock().await;
    let mut trained = Vec::new();
    let mut vitals = PlayerBiteVitals {
        quest_sessions: None,
        conditions: None,
        charm: None,
        states: &mut states,
        attack: &mut attack,
        formula: None,
        trained: &mut trained,
    };
    let mut ledger = CreatureBiteLedger::default();
    let definition = CreatureBiteDefinition::new(2_000_000, 40).ok_or("bite definition")?;
    let revisions = crate::ability::RevisionSet::new(
        "ruleset:shield-test",
        "content:shield-test",
        "world:shield-test",
        "formula:shield-test",
        "simulation:v1",
    )
    .map_err(debug)?;
    let now = owner.owner_now().get();
    for (sequence, elapsed, expected) in [(1, 0, 20), (1, 0, 20), (2, 2_000_000, 40)] {
        let result = commit_ai_bite(
            &mut ledger,
            &mut runtime,
            &mut vitals,
            AiAbilityAdapter::bite(issuer, sequence, actor, session, 0, 0),
            definition,
            revisions.clone(),
            ReentryProtection {
                protected_until: None,
            },
            SemanticTimeMicros::from_micros(now + elapsed),
        )
        .map_err(debug)?;
        assert_eq!(result.requested, expected);
    }
    Ok(())
}
