//! Existing proficiency owner consumption of a frozen Soulpit boss death.
use super::super::weapon_proficiency::{
    LiveProficiencyTrack, ProficiencyAttackEffects, WeaponProficiencyDispatch,
};
use super::*;
use crate::domain::weapon_proficiency::{
    ProficiencyKillContributor, ProficiencySelectionShape, ProficiencyThresholdClass,
};
use crate::foundation::{
    ActorLocalGeneration, ActorLocalId, ActorRef, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    WorldId,
};
use oteryn_gameplay_domain::domain::soulpit::SoulpitStack;
use oteryn_gameplay_entities::carrier::{
    TopDamagePrincipal,
    soulpit::{SoulpitCreatureTag, SoulpitDeathFacts},
};

type TestResult = Result<(), String>;
fn debug(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}
fn id(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

#[test]
fn soulpit_boss_uses_existing_weapon_points_owner_once_without_normal_kill_credit() -> TestResult {
    let session = GameSessionId::decode(&id(1)).map_err(debug)?;
    let world = WorldId::decode(&id(2)).map_err(debug)?;
    let actor = ExactActorRef(ActorRef::from_scoped_owner_parts(
        RuntimeScopeRefV1::instance(world, id(3)).map_err(debug)?,
        ScopeOwnershipGeneration::new(1).map_err(debug)?,
        ActorLocalId(1),
        ActorLocalGeneration(1),
    ));
    let mut reward = super::super::soulpit::runtime::SoulpitBossReward {
        occurrence: id(4),
        death_at_ms: 400_000,
        species: "oteryn:creature.rat".into(),
        experience: 140,
        death: SoulpitDeathFacts {
            actor,
            tag: SoulpitCreatureTag {
                handoff_id: id(5),
                wave: 4,
                monster_id: 28,
                stack: SoulpitStack::Boss,
                is_summon: false,
                boss: None,
            },
            principal: TopDamagePrincipal::Untracked,
            death_at_ms: Some(400_000),
            damage_shares: vec![crate::foundation::RewardDamageShare {
                character: crate::foundation::CharacterId::decode(&id(6)).map_err(debug)?,
                total: 1,
                lease_generation: Some(1),
                session: Some(session),
                last_damage_at_ms: Some(100_000),
            }],
        },
    };
    let mut owner = WeaponProficiencyDispatch::default();
    owner.install_track(
        "weapon",
        LiveProficiencyTrack {
            shape: ProficiencySelectionShape::new(ProficiencyThresholdClass::Standard, &[1; 7])
                .map_err(debug)?,
            progress: 0,
            selections: vec![None; 7],
            modifications: [None, None],
            effects: ProficiencyAttackEffects::default(),
        },
    );
    let contributor = ProficiencyKillContributor {
        character_in_channel: true,
        last_damage_at_ms: 100_000,
        equipped_weapon_item_key: Some("weapon"),
        weapon_has_promoted_proficiency: true,
    };
    assert_eq!(
        reward.credit_weapon(&mut owner, session, contributor, 0),
        Some(1500)
    );
    assert_eq!(
        reward.credit_weapon(&mut owner, session, contributor, 0),
        None
    );
    assert_eq!(owner.track("weapon").ok_or("track")?.progress, 1500);
    reward.occurrence = id(7);
    reward.death.tag.is_summon = true;
    assert_eq!(
        reward.credit_weapon(&mut owner, session, contributor, 0),
        None
    );
    reward.death.tag.is_summon = false;
    reward.death.tag.stack = SoulpitStack::Powerful;
    assert_eq!(
        reward.credit_weapon(&mut owner, session, contributor, 0),
        None
    );
    reward.death.tag.stack = SoulpitStack::Boss;
    let stale = ProficiencyKillContributor {
        last_damage_at_ms: 99_999,
        ..contributor
    };
    assert_eq!(reward.credit_weapon(&mut owner, session, stale, 0), None);
    assert_eq!(
        reward.credit_weapon(
            &mut owner,
            GameSessionId::decode(&id(8)).map_err(debug)?,
            contributor,
            0
        ),
        None
    );
    assert_eq!(
        reward.credit_weapon(
            &mut owner,
            session,
            ProficiencyKillContributor {
                character_in_channel: false,
                ..contributor
            },
            0
        ),
        None
    );
    assert_eq!(owner.track("weapon").ok_or("track")?.progress, 1500);
    assert!(
        owner.checkpoint_due(460_000_000, false),
        "same existing checkpoint owner flushes the1500delta"
    );
    Ok(())
}

#[test]
fn soulpit_lethal_callback_queues_only_stack40_xp_without_ordinary_loot() -> TestResult {
    std::thread::Builder::new()
        .name("soulpit-lethal-owner".into())
        .stack_size(64 << 20)
        .spawn(soulpit_lethal_callback_scenario)
        .map_err(debug)?
        .join()
        .map_err(|_| "Soulpit lethal scenario thread failed".to_string())?
}

fn soulpit_lethal_callback_scenario() -> TestResult {
    use crate::domain::soulpit::BossAbility;
    use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition, NodeId};
    use oteryn_gameplay_entities::carrier::soulpit::SoulpitWaveRequest;
    let world = WorldId::decode(&id(31)).map_err(debug)?;
    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native content")?;
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        native.source_digest(),
        [2; 32],
        [3; 32],
        [4; 32],
        (32375, 31151, 8),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        RuntimeScopeRefV1::instance(world, id(32)).map_err(debug)?,
        NodeId::decode(&id(33)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        pin,
    )
    .map_err(debug)?;
    let actors = {
        let kernel = runtime.borrow_soulpit_kernel().map_err(debug)?;
        kernel
            .carrier
            .spawn_soulpit_wave(
                kernel.continuity,
                kernel.content,
                native.policies(),
                SoulpitWaveRequest {
                    handoff_id: id(34),
                    wave: 4,
                    species: "oteryn:creature.rat".into(),
                    boss_ability: BossAbility::Overpower,
                    now_micros: 0,
                    positions: std::array::from_fn(|index| {
                        Some(MovementLocalPosition {
                            x: if index == 6 {
                                32376
                            } else {
                                32363 + index as i32
                            },
                            y: if index == 6 { 31144 } else { 31133 },
                            floor: 8,
                        })
                    }),
                },
                |_| true,
            )
            .map_err(debug)?
    };
    let ordinary = actors[0].ok_or("ordinary stack")?.actor;
    let boss = actors[6].ok_or("stack40")?.actor;
    let queue = KillSettlementQueue::default();
    let principal = CapturedRewardPrincipal {
        character: crate::foundation::CharacterId::decode(&id(35)).map_err(debug)?,
        lease_generation: 1,
        session: GameSessionId::decode(&id(36)).map_err(debug)?,
        actor: boss,
    };
    for (ordinal, actor) in [ordinary, boss].into_iter().enumerate() {
        runtime
            // Unsequenced owner occurrences cannot contain the binding's NUL delimiter.
            .commit_monster_lab_damage(actor, &[b'a' + ordinal as u8], i64::MAX)
            .map_err(debug)?;
        record_creature_kill(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: &queue,
                rewards: native.reward_table(),
                charms: None,
            },
            actor,
            principal,
            1000,
            Some(oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000)),
        );
        assert_eq!(queue.state().queued.len(), ordinal);
    }
    // Replayed lethality keeps the one queued physical occurrence.
    record_creature_kill(
        &mut runtime,
        KillSink {
            prey: None,
            activity: &[],
            queue: &queue,
            rewards: native.reward_table(),
            charms: None,
        },
        boss,
        principal,
        2000,
        Some(oteryn_simulation_determinism::SemanticTimeMicros::from_micros(2_000_000)),
    );
    let state = queue.state();
    assert_eq!(state.queued.len(), 1);
    let entry = state.queued.front().ok_or("queued stack40 death")?;
    assert!(entry.row.is_none());
    assert!(entry.soulpit_experience.is_some_and(|xp| xp.get() > 0));
    assert!(entry.bosstiary_only.is_none() && entry.prey_checkpoint.is_none());
    assert!(entry.forge.is_none() && entry.echo.is_none() && entry.grizzly.is_none());
    assert!(entry.activity_at_death.is_some());
    assert!(runtime.retains_corpse_projection(boss));
    Ok(())
}

#[test]
fn srx3_handoff_proficiency_equipment_keeps_exact_session_and_frozen_history() -> TestResult {
    let world = WorldId::decode(&id(71)).map_err(debug)?;
    let actor = ExactActorRef(ActorRef::from_scoped_owner_parts(
        RuntimeScopeRefV1::instance(world, id(72)).map_err(debug)?,
        ScopeOwnershipGeneration::new(1).map_err(debug)?,
        ActorLocalId(1),
        ActorLocalGeneration(1),
    ));
    let next_actor = ExactActorRef(ActorRef::from_scoped_owner_parts(
        RuntimeScopeRefV1::instance(world, id(73)).map_err(debug)?,
        ScopeOwnershipGeneration::new(1).map_err(debug)?,
        ActorLocalId(1),
        ActorLocalGeneration(1),
    ));
    let session = GameSessionId::decode(&id(74)).map_err(debug)?;
    let next_session = GameSessionId::decode(&id(75)).map_err(debug)?;
    let source = KillSettlementQueue::default();
    let destination = KillSettlementQueue::default();
    source.proficiency_equipment(actor, session, 9, Some("oteryn:item.tibia.i3264"));
    assert_eq!(
        source.activity_proficiency_equipment(actor, next_session),
        None
    );
    assert_eq!(
        source.activity_proficiency_equipment(next_actor, session),
        None
    );
    let (revision, weapon) = source
        .activity_proficiency_equipment(actor, session)
        .ok_or("exact source equipment")?;
    for _ in 0..2 {
        destination.proficiency_equipment(next_actor, next_session, revision, weapon.as_deref());
    }
    assert_eq!(
        destination.activity_proficiency_equipment(next_actor, next_session),
        Some((9, Some("oteryn:item.tibia.i3264".into())))
    );
    assert_eq!(
        destination.state().proficiency_weapons.len(),
        1,
        "retry keeps one current equipment witness"
    );
    assert_eq!(
        source.activity_proficiency_equipment(actor, session),
        Some((revision, weapon)),
        "old frozen deaths retain their source witness"
    );
    destination.proficiency_equipment(next_actor, next_session, 8, Some("wrong-older-weapon"));
    assert_eq!(
        destination
            .activity_proficiency_equipment(next_actor, next_session)
            .map(|row| row.0),
        Some(9)
    );
    Ok(())
}
