//! Reviewed Oteryn plain collectibles occupy their own source namespace.
use super::*;
use crate::content::ReferenceItemField as F;

include!("quest_collectible_pin.rs");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: u32,
    entries: Vec<Entry>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    profile: ItemProfileRecord,
    witnesses: Vec<String>,
    assumption: String,
}

fn packet() -> Result<Packet, ContentError> {
    let raw = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/quest-collectible-bindings.json"
    );
    if hex(sha256(raw)) != SHA256 {
        return Err(invalid("quest collectible proof digest"));
    }
    let packet: Packet =
        serde_json::from_slice(raw).map_err(|_| invalid("quest collectible proof"))?;
    // No fallback identities remain when ordinary native owners admit them all.
    if packet.schema != 1 || packet.entries.len() > 32 {
        return Err(invalid("quest collectible proof bounds"));
    }
    Ok(packet)
}

pub(super) fn extend(
    admitted: &mut BTreeMap<String, super::super::project::ProjectV2SourceIdentityBinding>,
    records: &[ItemProfileRecord],
) -> Result<(), ContentError> {
    extend_entries(admitted, records, packet()?.entries)
}

fn extend_entries(
    admitted: &mut BTreeMap<String, super::super::project::ProjectV2SourceIdentityBinding>,
    records: &[ItemProfileRecord],
    entries: Vec<Entry>,
) -> Result<(), ContentError> {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let expected = entry.profile;
        let binding = &expected.production_binding;
        let number = binding
            .external_id
            .parse::<u16>()
            .map_err(|_| invalid("quest collectible identity"))?;
        let (F::Known(physical), F::Known(stack)) =
            (&expected.semantics.physical, &expected.semantics.stack)
        else {
            return Err(invalid("quest collectible physical policy"));
        };
        if number == 0
            || number.to_string() != binding.external_id
            || binding.source_key != "oteryn:source.oteryn"
            || binding.source_revision != "quest-collectibles-v1"
            || binding.identity_namespace != "oteryn/quest_collectible_id"
            || binding.disposition
                != super::super::project::ProjectV2SourceIdentityDisposition::Exact
            || binding.target.key != format!("oteryn:item.tibia.i{number}")
            || !seen.insert(number)
            || !expected.admission.materializable
            || expected.admission.stack_class != NativeItemStackClass::NonStackable
            || expected.admission.legal_destinations
                != [
                    NativeItemDestination::CharacterInventory,
                    NativeItemDestination::Ground,
                ]
            || physical.movable != F::Known(true)
            || physical.pickupable != F::Known(true)
            || !matches!(physical.weight, F::Known(1..=100_000))
            || stack.stackable != F::Known(false)
            || expected.semantics.container != F::NotApplicable
            || expected.semantics.charges != F::NotApplicable
            || expected.semantics.temporal != F::NotApplicable
            || !matches!(expected.semantics.equipment, F::Unknown | F::NotApplicable)
            || !matches!(expected.semantics.weapon, F::Unknown | F::NotApplicable)
            || !matches!(expected.semantics.fluid, F::Unknown | F::NotApplicable)
            || !matches!(
                expected.semantics.use_transform,
                F::Unknown | F::NotApplicable
            )
            || !matches!(expected.semantics.imbuement, F::Unknown | F::NotApplicable)
            || expected.attributes.speed_bonus != Some(0)
            || expected.attributes.blocks_movement != Some(false)
            || expected.attributes.blocks_projectile != Some(false)
            || expected.attributes.immovable_block_solid != Some(false)
            || expected.attributes.field_condition.is_some()
            || expected.attributes.rune_consumption.is_some()
            || !expected.attributes.spell_augments.is_empty()
            || expected.attributes.field_replaceable.is_some()
            || expected.attributes.has_height != Some(false)
            || expected.production_binding_qualification.is_some()
            || !(3..=8).contains(&entry.witnesses.len())
            || entry
                .witnesses
                .iter()
                .any(|s| s.is_empty() || s.len() > 1024)
            || entry.assumption.is_empty()
            || entry.assumption.len() > 1024
        {
            return Err(invalid("quest collectible bounded policy"));
        }
        // Historical artifacts may omit this new collectible. A present one
        // must exactly equal the reviewed profile, including every Unknown.
        let mut matches = records
            .iter()
            .filter(|row| row.production_binding.external_id == binding.external_id);
        let Some(actual) = matches.next() else {
            continue;
        };
        if matches.next().is_some()
            || actual != &expected
            || admitted
                .insert(binding.external_id.clone(), binding.clone())
                .is_some()
        {
            return Err(invalid("quest collectible profile or binding conflict"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    mod fixture;
    #[test]
    fn reviewed_collectible_refuses_modified_profile_and_binding_collision()
    -> Result<(), ContentError> {
        let production_records: Vec<_> = packet()?
            .entries
            .into_iter()
            .map(|entry| entry.profile)
            .collect();
        let mut production = BTreeMap::new();
        extend(&mut production, &production_records)?;
        assert_eq!(production.len(), production_records.len());
        // Exercise every negative guard even after all live fallbacks retire.
        let entry = fixture::reviewed_entry()?;
        let records = vec![entry.profile.clone()];
        let extend = |admitted: &mut BTreeMap<_, _>, records: &[ItemProfileRecord]| {
            extend_entries(admitted, records, vec![entry.clone()])
        };
        let mut admitted = BTreeMap::new();
        extend(&mut admitted, &records)?;
        assert_eq!(admitted.len(), records.len());
        assert!(extend(&mut admitted, &records).is_err());
        let mut bad = records.clone();
        let first = bad
            .first_mut()
            .ok_or_else(|| invalid("collectible fixture"))?;
        first.admission.legal_destinations.clear();
        assert!(extend(&mut BTreeMap::new(), &bad).is_err());
        let mut bad = records.clone();
        let first = bad
            .first_mut()
            .ok_or_else(|| invalid("collectible fixture"))?;
        first.production_binding.source_key = "oteryn:source.crystalserver".into();
        assert!(extend(&mut BTreeMap::new(), &bad).is_err());
        let mut historical = BTreeMap::new();
        extend(&mut historical, &[])?;
        assert!(historical.is_empty());
        Ok(())
    }
}
