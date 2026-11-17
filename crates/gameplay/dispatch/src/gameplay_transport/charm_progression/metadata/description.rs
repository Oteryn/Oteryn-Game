//! Own wording derived from the exact parameters used by the admitted Charm evaluator.
use oteryn_gameplay_domain::domain::charm_data::{CharmEffect as E, CharmPercent};
fn percent(p: CharmPercent) -> String {
    let n = p.hundredths();
    format!("{}.{:02}%", n / 100, n % 100)
}
pub(super) fn describe(effect: E) -> String {
    let damage = match effect {
        E::AttackProcDamage { damage, .. }
        | E::AttackProcResourceDamage { damage, .. }
        | E::KillAreaDamage { damage, .. }
        | E::ReflectDamageTaken { damage } => Some(damage),
        _ => None,
    };
    let mut text = match effect {
        E::AttackProcDamage {
            damage,
            percent_of_creature_max_health,
            damage_cap_level_multiplier,
        } => format!(
            "On a qualifying hit, deal {} of the creature's maximum health as {:?} damage, capped at {} times your level.",
            percent(percent_of_creature_max_health),
            damage.element,
            damage_cap_level_multiplier
        ),
        E::AttackProcResourceDamage {
            damage,
            resource,
            percent_of_own_maximum,
            damage_cap_percent_of_creature_max_health,
        } => format!(
            "On a qualifying hit, deal {} of your maximum {:?} as {:?} damage, capped at {} of the creature's maximum health.",
            percent(percent_of_own_maximum),
            resource,
            damage.element,
            percent(damage_cap_percent_of_creature_max_health)
        ),
        E::KillAreaDamage {
            damage,
            percent_of_creature_max_health,
            damage_cap_level_multiplier,
        } => format!(
            "After your killing hit, strike nearby creatures for {} of the defeated creature's maximum health as {:?} damage, capped at {} times your level.",
            percent(percent_of_creature_max_health),
            damage.element,
            damage_cap_level_multiplier
        ),
        E::ParalyseCreatureOnAttack { duration_ms } => {
            format!("A qualifying attack can slow the creature for {duration_ms} ms.")
        }
        E::ParalyseCreatureAfterItsAttack { duration_ms } => {
            format!("A creature attack can slow that creature for {duration_ms} ms.")
        }
        E::HasteAfterHit { duration_ms } => {
            format!("A creature hit can grant you haste for {duration_ms} ms.")
        }
        E::PreventCreatureFlee { duration_ms } => {
            format!("A qualifying hit can prevent the creature from fleeing for {duration_ms} ms.")
        }
        E::ReflectDamageTaken { damage } => format!(
            "A creature hit can reflect its damage back as {:?} damage.",
            damage.element
        ),
        E::DodgeAttack => "A qualifying creature attack can be avoided entirely.".into(),
        E::CleanseAfterHit => "Clear harmful conditions after a qualifying creature hit.".into(),
        E::DeathLossReduction => {
            "Reduce experience and skill loss when this creature kills you.".into()
        }
        E::SkinningChanceBonus => {
            "Increase your chance of success when skinning this creature.".into()
        }
        E::CreatureProductBonus => "Increase creature-product loot from this creature.".into(),
        E::CriticalHitChance => "Add to critical-hit chance against this creature.".into(),
        E::CriticalExtraDamage => {
            "Add to the extra damage of critical hits against this creature.".into()
        }
        E::LifeLeech => "Add life leech when damaging this creature.".into(),
        E::ManaLeech => "Add mana leech when damaging this creature.".into(),
        E::ManaDrainInversion => "A creature's mana drain can restore mana instead.".into(),
    };
    if let Some(damage) = damage {
        text.push_str(if damage.ignores_resistances {
            " Creature resistances are ignored."
        } else {
            " Creature resistances apply."
        });
        text.push_str(if damage.reduced_by_armor {
            " Armor reduces this damage."
        } else {
            " Armor does not reduce this damage."
        });
    }
    text
}
