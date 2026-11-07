//! Attack refresh must retain the session's uncheckpointed kill credit.
use super::*;

#[test]
fn proficiency_replay_memory_retains_the_latest_64_deaths() {
    let catalogue = ProductionProficiencyCatalogue::embedded().expect("catalogue");
    let key = "oteryn:item.tibia.i3264";
    let mut owner = WeaponProficiencyDispatch::default();
    owner.install_track(key, catalogue.empty_track(key).expect("track"));
    for tag in 0..65 {
        assert!(
            owner
                .dispatch_kill(KillDispatch {
                    death_occurrence: [tag; 16],
                    death_at_ms: 400_000,
                    creature_is_summon: false,
                    points: Some(1),
                    contributor: ProficiencyKillContributor {
                        character_in_channel: true,
                        last_damage_at_ms: 399_999,
                        equipped_weapon_item_key: Some(key),
                        weapon_has_promoted_proficiency: true,
                    },
                })
                .is_some()
        );
        assert_eq!(owner.seen_deaths.len(), usize::from(tag + 1).min(64));
    }
    assert!(!owner.seen_deaths.contains(&[0; 16]));
    assert!(owner.seen_deaths.contains(&[1; 16]));
    assert!(owner.seen_deaths.contains(&[64; 16]));
}

#[test]
fn attack_refresh_retains_pending_progress_and_death_deduplication() {
    let mut owner = WeaponProficiencyDispatch::default();
    let track = LiveProficiencyTrack {
        shape: ProficiencySelectionShape::new(ProficiencyThresholdClass::Standard, &[1; 7])
            .expect("shape"),
        progress: 0,
        selections: vec![None; 7],
        modifications: [None, None],
        effects: ProficiencyAttackEffects::default(),
    };
    owner.install_track("weapon", track.clone());
    let kill = KillDispatch {
        death_occurrence: [7; 16],
        death_at_ms: 400_000,
        creature_is_summon: false,
        points: Some(100),
        contributor: ProficiencyKillContributor {
            character_in_channel: true,
            last_damage_at_ms: 399_999,
            equipped_weapon_item_key: Some("weapon"),
            weapon_has_promoted_proficiency: true,
        },
    };
    assert_eq!(owner.dispatch_kill(kill), Some(100));
    assert!(!owner.checkpoint_due(400_000_000, false));
    assert!(owner.checkpoint_due(460_000_000, false));
    assert!(owner.checkpoint_due(400_000_000, true));
    owner.install_track("weapon", track.clone());
    assert_eq!(owner.track("weapon").expect("track").progress, 100);
    assert_eq!(owner.dispatch_kill(kill), None);
    // A different writer advanced the committed track. Retain our own delta on top.
    owner.install_track(
        "weapon",
        LiveProficiencyTrack {
            progress: 25,
            ..track
        },
    );
    assert_eq!(owner.track("weapon").expect("track").progress, 125);
}
