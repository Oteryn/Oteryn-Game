//! Actual familiar/master links and canonical creature HP; not Drome profile qualification.
use super::*;
use crate::carrier::tibiadrome_modifiers::DromeModifier;
fn setup(bind_familiar: bool) -> TestResult<(ChannelRuntimeV1, ExactActorRef, ExactActorRef)> {
    let mut familiar = fixture_policy("oteryn:creature.fixture_familiar", "Fixture Familiar", 101);
    familiar.is_familiar = true;
    let (mut runtime, master, session) = fixture_with_policies(
        8,
        vec![fixture_policy("oteryn:creature.rat", "Rat", 101), familiar],
    )?;
    runtime.set_tibiadrome(master, true).map_err(debug)?;
    let selected = [DromeModifier::GoingDownWithMe, DromeModifier::Somersault];
    runtime
        .bind_tibiadrome_modifiers(master, [5; 16], selected)
        .map_err(debug)?;
    let familiar = runtime
        .create_companion(
            master,
            session,
            "oteryn:creature.fixture_familiar",
            at(102, 100, 7),
            None,
            Some(1000),
            1,
        )
        .map_err(debug)?;
    if bind_familiar {
        runtime
            .bind_tibiadrome_modifiers(familiar, [5; 16], selected)
            .map_err(debug)?;
    }
    let target = ordinary(&mut runtime, at(103, 100, 7))?;
    runtime
        .scale_tibiadrome_opponent(target, 0)
        .map_err(debug)?;
    runtime
        .bind_tibiadrome_modifiers(target, [5; 16], selected)
        .map_err(debug)?;
    let hp = runtime.live_creature_health(target).ok_or("live target")?;
    runtime
        .borrow_exact_actor_commit()
        .commit_damage(
            target,
            OwnerDamageCommand {
                target: b"oteryn:creature.rat",
                occurrence: b"familiar-hit",
                binding: b"familiar-hit\0fixture:drome-source",
                damage: hp as i64,
            },
        )
        .map_err(debug)?;
    runtime.record_tibiadrome_source_hit(familiar, target);
    let mut death = runtime.borrow_combat_death();
    let receipt = death.committed_lethal_receipt(target).map_err(debug)?;
    death.project_committed_lethal(receipt).map_err(debug)?;
    Ok((runtime, familiar, target))
}
#[test]
fn srx3_drome_familiar_last_hitter_uses_master_membership_without_substituting_master() -> TestResult
{
    let (runtime, familiar, target) = setup(false)?;
    assert_eq!(runtime.tibiadrome_last_hitter(target), Some(familiar));
    Ok(())
}
#[test]
fn srx3_drome_familiar_death_blast_uses_canonical_hp_and_replay() -> TestResult {
    let (mut runtime, familiar, target) = setup(true)?;
    let receipt = runtime
        .commit_tibiadrome_familiar_death_blast(familiar, target, 1)
        .map_err(debug)?;
    assert_eq!((receipt.health_before, receipt.health_after), (101, 61));
    assert_eq!(runtime.live_creature_health(familiar), Some(61));
    assert!(
        !runtime
            .commit_tibiadrome_familiar_death_blast(familiar, target, 1)
            .map_err(debug)?
            .applied
    );
    assert_eq!(runtime.live_creature_health(familiar), Some(61));
    Ok(())
}

#[test]
fn srx3_drome_familiar_blast_stream_remains_bounded_beyond_canonical_receipt_window() -> TestResult
{
    let (mut runtime, familiar, mut target) = setup(true)?;
    let limit = COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX as u64;
    for ordinal in 1..=limit + 2 {
        runtime
            .set_test_creature_health(familiar, 101)
            .map_err(debug)?;
        let result = runtime
            .commit_tibiadrome_familiar_death_blast(familiar, target, ordinal)
            .map_err(debug)?;
        assert_eq!(result.health_after, 61);
        if ordinal < limit + 2 {
            runtime.remove_arena_opponent(target).map_err(debug)?;
            target = ordinary(&mut runtime, at(103, 100, 7))?;
            runtime
                .scale_tibiadrome_opponent(target, 0)
                .map_err(debug)?;
            runtime
                .bind_tibiadrome_modifiers(
                    target,
                    [5; 16],
                    [DromeModifier::GoingDownWithMe, DromeModifier::Somersault],
                )
                .map_err(debug)?;
            runtime
                .borrow_exact_actor_commit()
                .commit_damage(
                    target,
                    OwnerDamageCommand {
                        target: b"oteryn:creature.rat",
                        occurrence: b"familiar-hit",
                        binding: b"familiar-hit\0fixture:drome-source",
                        damage: 101,
                    },
                )
                .map_err(debug)?;
            runtime.record_tibiadrome_source_hit(familiar, target);
            let mut deaths = runtime.borrow_combat_death();
            let receipt = deaths.committed_lethal_receipt(target).map_err(debug)?;
            deaths.project_committed_lethal(receipt).map_err(debug)?;
        }
    }
    assert!(
        !runtime
            .commit_tibiadrome_familiar_death_blast(familiar, target, limit + 2)
            .map_err(debug)?
            .applied
    );
    let index = runtime
        .carrier
        .validate_ref(&runtime.continuity, familiar.0)
        .map_err(debug)?;
    let Slot::CreatureOccupied { committed, .. } = &runtime.carrier.slots[index] else {
        return Err("familiar slot".into());
    };
    assert!(committed.entries().len() <= COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX);
    Ok(())
}

#[test]
fn srx3_drome_familiar_zero_percent_result_cannot_reapply_after_healing() -> TestResult {
    let (mut runtime, familiar, target) = setup(true)?;
    runtime
        .set_test_creature_health(familiar, 2)
        .map_err(debug)?;
    let result = runtime
        .commit_tibiadrome_familiar_death_blast(familiar, target, 1)
        .map_err(debug)?;
    assert_eq!((result.health_before, result.health_after), (2, 2));
    runtime
        .commit_source_creature_heal_batch(
            [1; 32],
            &[(familiar, "oteryn:creature.fixture_familiar".into(), 101, 99)],
        )
        .map_err(debug)?;
    assert_eq!(runtime.live_creature_health(familiar), Some(101));
    match runtime.commit_tibiadrome_familiar_death_blast(familiar, target, 1) {
        Ok(replay) => assert!(!replay.applied),
        Err(error) => assert_eq!(error, CarrierError::StaleAttackerSequence),
    }
    assert_eq!(
        runtime.live_creature_health(familiar),
        Some(101),
        "a settled floor(40%*2)=0 must not become a new40HP hit after healing"
    );
    Ok(())
}

#[test]
fn srx3_drome_familiar_blast_refuses_lost_master_membership_and_foreign_run() -> TestResult {
    for lost_membership in [true, false] {
        let (mut runtime, familiar, target) = setup(true)?;
        if lost_membership {
            let master = runtime
                .companion_snapshot(familiar)
                .map_err(debug)?
                .state
                .master
                .ok_or("master")?;
            runtime.set_tibiadrome(master.actor, false).map_err(debug)?;
        } else {
            runtime
                .bind_tibiadrome_modifiers(
                    target,
                    [6; 16],
                    [DromeModifier::GoingDownWithMe, DromeModifier::Somersault],
                )
                .map_err(debug)?;
        }
        assert_eq!(
            runtime.commit_tibiadrome_familiar_death_blast(familiar, target, 1),
            Err(CarrierError::InvalidDamage)
        );
        assert_eq!(runtime.live_creature_health(familiar), Some(101));
    }
    Ok(())
}
