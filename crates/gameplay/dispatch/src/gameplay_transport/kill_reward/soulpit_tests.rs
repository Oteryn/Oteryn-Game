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
