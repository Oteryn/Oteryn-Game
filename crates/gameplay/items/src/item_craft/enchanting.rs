//! Crystal 201189f enchanting.lua:13-41,76-102,126-133.
use super::*;

pub const GEMS: [u16; 4] = [3030, 3029, 3032, 3033];
pub const ENCHANTED: [u16; 4] = [676, 675, 677, 678];
const ALTARS: [[u16; 4]; 4] = [
    [146, 147, 148, 149],
    [150, 151, 152, 153],
    [158, 159, 160, 161],
    [154, 155, 156, 157],
];
const WEAPONS: [(u16, [u16; 4]); 17] = [
    (3271, [660, 679, 779, 794]),
    (7383, [661, 680, 780, 795]),
    (7384, [662, 681, 781, 796]),
    (7406, [663, 682, 782, 797]),
    (7402, [664, 683, 783, 798]),
    (3317, [665, 684, 784, 801]),
    (3318, [666, 685, 785, 802]),
    (7389, [667, 686, 786, 803]),
    (7380, [668, 687, 787, 804]),
    (3342, [669, 688, 788, 805]),
    (3311, [670, 689, 789, 806]),
    (3333, [671, 690, 790, 807]),
    (7415, [672, 691, 791, 808]),
    (7392, [673, 692, 792, 809]),
    (3279, [674, 693, 793, 810]),
    (3447, [763, 762, 774, 761]),
    (8077, [8078, 8079, 8081, 8080]),
];

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
    if !(1..=100).contains(&source_count) || !(1..=100).contains(&target_count) {
        return None;
    }
    if let Some(element) = GEMS.iter().position(|id| *id == source) {
        return ALTARS[element].contains(&target).then_some(EnchantingRule {
            output: ENCHANTED[element],
            count: source_count,
            source_debit: source_count,
            mana: 300 * source_count,
            soul: 2 * source_count,
            charges: None,
        });
    }
    let element = ENCHANTED.iter().position(|id| *id == source)?;
    let (_, outputs) = WEAPONS.iter().find(|(id, _)| *id == target)?;
    let ammo = matches!(target, 3447 | 8077);
    if !ammo && target_count != 1 {
        return None;
    }
    Some(EnchantingRule {
        output: outputs[element],
        count: target_count,
        source_debit: 1,
        mana: 0,
        soul: 0,
        charges: (!ammo).then_some(1000),
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
    fn crystal_all_shrines_weapons_stacks_and_refusals() -> Result<(), Box<dyn std::error::Error>> {
        #[derive(serde::Deserialize)]
        struct Donor {
            gems: [u16; 4],
            enchanted: [u16; 4],
            altars: [[u16; 4]; 4],
            weapons: Vec<(u16, [u16; 4])>,
            mana: u32,
            soul: u32,
        }
        #[derive(serde::Deserialize)]
        struct Evidence {
            rules: Donor,
        }
        // Extracted by build_enchanting_item_profiles.py from Crystal 201189f's Lua tables.
        let evidence: Evidence = serde_json::from_slice(include_bytes!(
            "../../../../../tools/content-schema/native-gameplay/enchanting-item-profiles.json"
        ))?;
        let donor = evidence.rules;
        assert_eq!(donor.weapons.len(), 17);
        for (element, gem) in donor.gems.into_iter().enumerate() {
            for altar in donor.altars[element] {
                for count in [1, 2, 100] {
                    let actual = rule(gem, altar, count, 1);
                    assert_eq!(
                        actual,
                        Some(EnchantingRule {
                            output: donor.enchanted[element],
                            count,
                            source_debit: count,
                            mana: donor.mana * count,
                            soul: donor.soul * count,
                            charges: None
                        })
                    );
                }
            }
            for &(weapon, outputs) in &donor.weapons {
                let ammo = matches!(weapon, 3447 | 8077);
                let count = if ammo { 73 } else { 1 };
                assert_eq!(
                    rule(donor.enchanted[element], weapon, 2, count),
                    Some(EnchantingRule {
                        output: outputs[element],
                        count,
                        source_debit: 1,
                        mana: 0,
                        soul: 0,
                        charges: (!ammo).then_some(1000)
                    })
                );
            }
            assert_eq!(rule(gem, ALTARS[(element + 1) % 4][0], 1, 1), None);
            assert_eq!(rule(gem, 3271, 1, 1), None);
            assert_eq!(rule(ENCHANTED[element], 146, 1, 1), None);
        }
        for (source, target, n, m) in [
            (3030, 146, 0, 1),
            (3030, 146, 101, 1),
            (676, 3271, 1, 2),
            (1, 146, 1, 1),
            (676, 1, 1, 1),
        ] {
            assert_eq!(rule(source, target, n, m), None);
        }
        Ok(())
    }
}
