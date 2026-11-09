use super::*;
#[test]
fn cyclopedia_spell_modifiers_keep_cast_units_and_merge_same_source() -> Result<(), &'static str> {
    assert_eq!(
        gem("base_damage_bonus", 1750).map_err(|_| "damage")?,
        Some((Effect::BaseDamage, 100))
    );
    assert_eq!(
        gem("base_healing_bonus", 750).map_err(|_| "healing")?,
        Some((Effect::BaseHealing, 0))
    );
    assert_eq!(
        gem("critical_extra_damage", 155).map_err(|_| "critical")?,
        Some((Effect::CriticalDamage, 15))
    );
    assert_eq!(
        gem("cooldown_reduction", 2500).map_err(|_| "cooldown")?,
        Some((Effect::Cooldown, 2500))
    );
    assert!(gem("cooldown_reduction", -1).is_err());
    assert_eq!(gem("absorb_fire", -100).map_err(|_| "unrelated")?, None);
    let mut out = Capture::default();
    out.data.known_sources = 1;
    let a = ItemSpellAugment {
        spell: "Test spell".into(),
        kind: K::BaseDamage,
        value: 1500,
    };
    out.native(
        SourceKind::Equipment,
        5,
        "oteryn:item/test",
        "Test spell",
        &a,
    )
    .map_err(|_| "item")?;
    out.native(
        SourceKind::Equipment,
        5,
        "oteryn:item/test",
        "Test spell",
        &a,
    )
    .map_err(|_| "second item modifier")?;
    let facts = out.finish().map_err(|_| "snapshot")?;
    assert_eq!(facts.sources.len(), 1);
    assert_eq!(facts.rows.len(), 1);
    assert_eq!(facts.rows[0].value, 3000);
    assert_eq!(facts.rows[0].effect, Effect::BaseDamage);
    Ok(())
}
