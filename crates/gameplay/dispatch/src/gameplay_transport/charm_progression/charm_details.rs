//! Names and descriptions follow the same ordered catalogue used by progression and combat.
use crate::content::charm_source::CanonicalCharmCatalogue;
use crate::domain::charm::CharmCategory;
use oteryn_gameplay_progression::charm_effects::{
    CharmCatalogueRead, CharmDamageKind, CharmEffect, CharmPercent, CharmStageValue,
};
use oteryn_protocol_oteryn::charm_progression::catalogue::{Card, PAGE_SIZE, Page};
use std::fmt::Write;
fn percent(p: CharmPercent) -> String {
    format!("{}.{:02}%", p.hundredths() / 100, p.hundredths() % 100)
}
fn damage(d: CharmDamageKind) -> String {
    format!(
        "{:?} damage ({}resistances; {}armor)",
        d.element,
        if d.ignores_resistances {
            "ignores "
        } else {
            "uses "
        },
        if d.reduced_by_armor {
            "uses "
        } else {
            "ignores "
        }
    )
}
fn description(effect: CharmEffect) -> String {
    use CharmEffect::*;
    match effect {
        AttackProcDamage {
            damage: d,
            percent_of_creature_max_health: p,
            damage_cap_level_multiplier: cap,
        } => format!(
            "On an attack, deal {} of the creature's maximum health as {}; capped at {cap} times your level before mitigation.",
            percent(p),
            damage(d)
        ),
        AttackProcResourceDamage {
            damage: d,
            resource,
            percent_of_own_maximum: p,
            damage_cap_percent_of_creature_max_health: cap,
        } => format!(
            "On an attack, deal {} of your maximum {resource:?} as {}; capped at {} of the creature's maximum health.",
            percent(p),
            damage(d),
            percent(cap)
        ),
        KillAreaDamage {
            damage: d,
            percent_of_creature_max_health: p,
            damage_cap_level_multiplier: cap,
        } => format!(
            "On your lethal hit, deal {} of the defeated creature's maximum health to surrounding creatures as {}; capped at {cap} times your level.",
            percent(p),
            damage(d)
        ),
        ParalyseCreatureOnAttack { duration_ms } => {
            format!("Paralyse the creature on an attack for {duration_ms} ms.")
        }
        ParalyseCreatureAfterItsAttack { duration_ms } => {
            format!("Paralyse the creature after its attack for {duration_ms} ms.")
        }
        HasteAfterHit { duration_ms } => {
            format!("Gain haste after being hit for {duration_ms} ms.")
        }
        PreventCreatureFlee { duration_ms } => {
            format!("Prevent the creature from fleeing for {duration_ms} ms.")
        }
        ReflectDamageTaken { damage: d } => format!(
            "Reflect base damage taken to the creature as {}.",
            damage(d)
        ),
        DodgeAttack => "Avoid the creature's attack.".into(),
        CleanseAfterHit => "Cleanse a harmful condition after this creature hits you.".into(),
        DeathLossReduction => "Reduce experience loss when this creature kills you.".into(),
        SkinningChanceBonus => {
            "Increase your chance of obtaining a product when skinning this creature.".into()
        }
        CreatureProductBonus => {
            "Increase the drop chance of creature products from this creature.".into()
        }
        CriticalHitChance => "Increase critical hit chance against this creature.".into(),
        CriticalExtraDamage => "Increase critical damage against this creature.".into(),
        LifeLeech => "Increase life leech against this creature.".into(),
        ManaLeech => "Increase mana leech against this creature.".into(),
        ManaDrainInversion => "Convert the creature's mana drain into mana restoration.".into(),
    }
}
pub(super) fn page(source: &CanonicalCharmCatalogue, after: u8) -> Option<Page> {
    if after > 32 {
        return None;
    }
    let mut cards = Vec::new();
    for (index, definition) in source
        .catalogue()
        .definitions()
        .enumerate()
        .skip(usize::from(after))
        .take(PAGE_SIZE + 1)
    {
        let effect = source.charm(definition.key.as_str())?;
        let mut detail = description(effect.effect());
        let (category, currency) = match definition.category {
            CharmCategory::Major => ("Major", "Charm Points"),
            CharmCategory::Minor => ("Minor", "Minor Charm Echoes"),
        };
        let value = match effect.effect().stage_value() {
            CharmStageValue::EffectPercent => "Effect bonus",
            CharmStageValue::TriggerChancePercent => "Trigger chance",
        };
        for stage in 1..=3 {
            let _ = write!(
                detail,
                "\r\nStage {stage}: {value} {}; cost {} {currency}",
                percent(effect.stage(stage)?),
                definition.stage_costs[usize::from(stage - 1)]
            );
        }
        cards.push(Card {
            race: (index + 1).try_into().ok()?,
            name: definition
                .key
                .as_str()
                .rsplit('.')
                .next()?
                .replace('_', " "),
            category: category.into(),
            detail,
            ..Default::default()
        });
    }
    let more = cards.len() > PAGE_SIZE;
    cards.truncate(PAGE_SIZE);
    let next = more.then(|| cards.last().map(|c| c.race)).flatten();
    Some(Page {
        next,
        cards,
        categories: vec!["Major".into(), "Minor".into()],
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t081_charm_descriptions_follow_canonical_progression_indices_and_effect_values()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = CanonicalCharmCatalogue::embedded()?;
        let first = page(&source, 0).ok_or("page")?;
        assert_eq!(first.cards.len(), 20);
        let next = first.next.ok_or("next")?;
        let mut cards = first.cards;
        cards.extend(page(&source, next.try_into()?).ok_or("next page")?.cards);
        assert_eq!(cards.len(), source.catalogue().definitions().count());
        for (card, definition) in cards.iter().zip(source.catalogue().definitions()) {
            let effect = source.charm(definition.key.as_str()).ok_or("effect")?;
            assert_eq!(
                card.name,
                definition
                    .key
                    .as_str()
                    .rsplit('.')
                    .next()
                    .ok_or("name")?
                    .replace('_', " ")
            );
            assert!(
                card.detail
                    .contains(&percent(effect.stage(1).ok_or("stage")?))
            );
            assert!(card.detail.contains(&definition.stage_costs[2].to_string()));
        }
        assert!(page(&source, 33).is_none());
        Ok(())
    }
}
