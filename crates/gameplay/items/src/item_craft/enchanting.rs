//! Crystal 201189f enchanting.lua:13-41,76-102,126-133.
use super::*;

pub const GEMS: [u16; 4] = [3030, 3029, 3032, 3033];
pub const ENCHANTED: [u16; 4] = [676, 675, 677, 678];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnchantingRule {
    pub output: u16,
    pub count: u32,
    pub source_debit: u32,
    pub mana: u32,
    pub soul: u32,
    pub charges: Option<u16>,
}

pub fn is_gem(key: &str) -> bool {
    source_id(key).is_some_and(|id| GEMS.contains(&id) || ENCHANTED.contains(&id))
}

pub fn source_id(key: &str) -> Option<u16> {
    key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
}

pub fn rule(
    source: u16,
    target: u16,
    source_count: u32,
    target_count: u32,
) -> Option<EnchantingRule> {
    if !(1..=100).contains(&source_count) || target_count != 1 {
        return None;
    }
    let output = match (source, target) {
        (676, 8077) => 8078,
        (675, 8077) => 8079,
        (677, 8077) => 8081,
        (678, 8077) => 8080,
        (676, 9020) => 9019,
        _ => return None,
    };
    Some(EnchantingRule {
        output,
        count: 1,
        source_debit: 1,
        mana: 0,
        soul: 0,
        charges: None,
    })
}

pub fn output(native: &NativeGameplayState, rule: EnchantingRule) -> Option<TypedDefinitionRef> {
    let record = native
        .item_policy_for_source_id(u32::from(rule.output))?
        .record();
    record.admission.materializable.then(|| TypedDefinitionRef {
        family: record.production_definition.family.clone(),
        production_key: record.production_definition.production_key.clone(),
        revision_ref: record.production_definition.revision_ref.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn global_enchanted_gems_only_work_on_rainbow_shield_and_worn_firewalkers()
    -> Result<(), Box<dyn std::error::Error>> {
        let evidence: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../../tools/content-schema/native-gameplay/enchanting-item-profiles.json"
        ))?;
        let altars = evidence["rules"]["altars"].as_array().ok_or("altars")?;
        let weapons = evidence["rules"]["weapons"].as_array().ok_or("weapons")?;
        assert_eq!(altars.len(), GEMS.len());
        assert_eq!(weapons.len(), 17);
        for (&gem, altars) in GEMS.iter().zip(altars) {
            for altar in altars.as_array().ok_or("element altars")? {
                let altar = u16::try_from(altar.as_u64().ok_or("altar")?)?;
                for count in [1, 2, 100] {
                    assert_eq!(rule(gem, altar, count, 1), None, "gem {gem}, altar {altar}");
                }
            }
        }
        for (gem, shield_output) in ENCHANTED.into_iter().zip([8078, 8079, 8081, 8080]) {
            for weapon in weapons {
                let target = u16::try_from(weapon[0].as_u64().ok_or("weapon")?)?;
                let actual = rule(gem, target, 2, 1);
                if target == 8077 {
                    assert_eq!(actual, Some(gem_use(shield_output)));
                } else {
                    assert_eq!(actual, None, "gem {gem}, target {target}");
                }
            }
            assert_eq!(rule(gem, 9020, 2, 1), (gem == 676).then(|| gem_use(9019)));
        }
        for (source, target, n, m) in [
            (676, 8077, 0, 1),
            (676, 8077, 101, 1),
            (676, 8077, 1, 2),
            (1, 8077, 1, 1),
            (676, 1, 1, 1),
        ] {
            assert_eq!(rule(source, target, n, m), None);
        }
        Ok(())
    }

    fn gem_use(output: u16) -> EnchantingRule {
        EnchantingRule {
            output,
            count: 1,
            source_debit: 1,
            mana: 0,
            soul: 0,
            charges: None,
        }
    }
}
