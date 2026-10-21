#![allow(clippy::expect_used)]

use super::*;
use crate::combat::charm_effects::{
    CharmDamageKind, CharmEffect, CharmElement, CharmPercent, CharmStageValue,
};
use crate::domain::charm::{CharmBalance, CharmDefinition, CharmSlotEntitlement, CharmStage};
use crate::durability::charm_state::CharacterCharmState;

struct Effects(BTreeMap<String, EffectDefinition>);
impl CharmCatalogueRead for Effects {
    fn charm(&self, key: &str) -> Option<&EffectDefinition> {
        self.0.get(key)
    }
}

fn key() -> CharmKey {
    CharmKey::new("oteryn:charm.wound").expect("charm")
}
fn race(name: &str) -> BestiaryRace {
    BestiaryRace::new(
        format!("oteryn:creature.{name}"),
        "definition-r1",
        vec![1, 2, 3],
    )
    .expect("race")
}
fn catalogue() -> CharmCatalogue {
    CharmCatalogue::new([CharmDefinition {
        key: key(),
        category: CharmCategory::Major,
        stage_costs: [100, 200, 300],
    }])
    .expect("catalogue")
}
fn effects() -> Effects {
    let percent = |value| CharmPercent::from_hundredths(value).expect("percent");
    let definition = EffectDefinition::new(
        key().as_str(),
        EffectCategory::Major,
        CharmStageValue::TriggerChancePercent,
        [percent(100), percent(200), percent(300)],
        CharmEffect::AttackProcDamage {
            damage: CharmDamageKind {
                element: CharmElement::Physical,
                ignores_resistances: false,
                reduced_by_armor: true,
            },
            percent_of_creature_max_health: percent(500),
            damage_cap_level_multiplier: 2,
        },
    )
    .expect("definition");
    Effects(BTreeMap::from([(key().as_str().to_owned(), definition)]))
}
fn content() -> CharmConnectionContent {
    CharmConnectionContent::new(
        "content-1".to_owned(),
        catalogue(),
        vec![race("wolf"), race("rat")],
        &effects(),
    )
    .expect("content")
}
fn snapshot() -> CharacterCharmProgressionSnapshot {
    CharacterCharmProgressionSnapshot {
        character_revision: CharacterRevision::new(8).expect("revision"),
        state: CharacterCharmState {
            unlocks: BTreeMap::from([(key(), CharmStage::FIRST)]),
            assignments: BTreeMap::from([(
                key(),
                BestiaryRaceKey::new("oteryn:creature.rat").expect("rat"),
            )]),
        },
        bestiary_counts: BTreeMap::from([(
            BestiaryRaceKey::new("oteryn:creature.wolf").expect("wolf"),
            2,
        )]),
        balance: CharmBalance {
            points_earned: 200,
            points_spent: 100,
            echoes_earned: 100,
            echoes_spent: 0,
        },
        slot_entitlement: CharmSlotEntitlement::Free,
    }
}

#[test]
fn signed_balances_fail_closed_without_clamping_or_wrapping() {
    assert_eq!(wire_balance(0), Ok(0));
    assert_eq!(
        wire_balance(i128::from(MAX_CHARM_BALANCE)),
        Ok(MAX_CHARM_BALANCE)
    );
    for value in [-1, i128::MIN, i128::MAX, i128::from(MAX_CHARM_BALANCE) + 1] {
        assert_eq!(wire_balance(value), Err(CharmPortUnavailable));
    }
}

#[test]
fn canonical_indices_and_actual_combat_definition_drive_coherent_views() {
    let content = content();
    let views = content.project(snapshot()).expect("views");
    assert_eq!(views.revision.get(), 8);
    assert_eq!(views.bestiary[0].race.get(), 2);
    assert_eq!(
        views.charms.charms[0].assigned_race.map(NonZeroU32::get),
        Some(1)
    );
    assert_eq!(views.charms.charms[0].next_stage_cost, 200);
    assert!(
        content
            .charm(key().as_str())
            .expect("effect")
            .effect_active()
    );
    assert!(!views.charms.charms[0].effect_active);
    assert_eq!(views.charms.charm_points_available, 100);
    assert_eq!(
        views.charms.assignment_slot_limit.map(NonZeroU32::get),
        Some(2)
    );
}

#[test]
fn historical_counters_are_ignored_but_an_obsolete_assignment_is_a_fault() {
    let content = content();
    let mut historical = snapshot();
    let retired = BestiaryRaceKey::new("oteryn:creature.retired").expect("retired");
    historical.bestiary_counts.insert(retired.clone(), 3);
    assert_eq!(
        content.project(historical.clone()),
        content.project(snapshot())
    );
    historical.state.assignments.insert(key(), retired);
    assert_eq!(content.project(historical), Err(CharmPortUnavailable));
    let mut debt = snapshot();
    debt.balance.points_earned = 0;
    assert_eq!(content.project(debt), Err(CharmPortUnavailable));
}

#[test]
fn lowered_thresholds_project_complete_progress_without_rewriting_history() {
    let content = content();
    let mut retained = snapshot();
    let wolf = BestiaryRaceKey::new("oteryn:creature.wolf").expect("wolf");
    retained.bestiary_counts.insert(wolf.clone(), 5);
    let views = content.project(retained.clone()).expect("complete view");
    assert_eq!(views.bestiary[0].kill_count, 3);
    assert_eq!(views.bestiary[0].kill_thresholds, [1, 2, 3]);
    assert_eq!(retained.bestiary_counts[&wolf], 5);
    assert_eq!(
        views.charms,
        content.project(snapshot()).expect("view").charms
    );
}

#[test]
fn generation_binding_rejects_missing_effects_duplicate_races_and_wire_limits() {
    let valid = effects();
    let empty = Effects(BTreeMap::new());
    assert!(CharmConnectionContent::new("r1".into(), catalogue(), vec![], &empty).is_err());
    for races in [
        vec![race("rat"), race("rat")],
        vec![race("rat"); MAX_BESTIARY_VIEW_ENTRIES + 1],
        vec![BestiaryRace::new("oteryn:creature.rat", "r1", vec![1, 2]).expect("two stages")],
    ] {
        assert!(CharmConnectionContent::new("r1".into(), catalogue(), races, &valid).is_err());
    }
    let mut wrong = effects();
    let definition = wrong.0.remove(key().as_str()).expect("definition");
    wrong.0.insert(
        key().as_str().to_owned(),
        EffectDefinition::new(
            "oteryn:charm.other",
            definition.category(),
            CharmStageValue::TriggerChancePercent,
            [
                definition.stage(1).expect("stage"),
                definition.stage(2).expect("stage"),
                definition.stage(3).expect("stage"),
            ],
            definition.effect(),
        )
        .expect("other"),
    );
    assert!(CharmConnectionContent::new("r1".into(), catalogue(), vec![], &wrong).is_err());
}
