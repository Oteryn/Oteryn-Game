//! Retain source-qualified tutorial citizenship independently of the current entry pin.
use super::{Error, Position, Source, Value, fail, native_floor, subarea};

pub(super) fn admit(source: &mut Source, towns: &[Value]) -> Result<(), Error> {
    for (name, expected_town, area_id) in [("Newhaven", 33, 514), ("Targuna", 34, 526)] {
        let mut matching = towns.iter().filter(|town| town["name"] == name);
        let town = matching
            .next()
            .ok_or_else(|| fail(format!("{name} source town")))?;
        if matching.next().is_some() {
            return Err(fail(format!("ambiguous {name} town")));
        }
        if town["town_id"].as_u64() != Some(expected_town) {
            return Err(fail(format!("{name} town id")));
        }
        let temple: Position = serde_json::from_value(town["temple"].clone()).map_err(fail)?;
        let area = source
            .table
            .areas
            .iter_mut()
            .find(|area| area.id == area_id && area.name == name)
            .ok_or_else(|| fail(format!("{name} source area")))?;
        area.town_id = Some(u32::try_from(expected_town).map_err(fail)?);
        area.hometown = true;
        area.temple = Some(subarea::Temple {
            x: temple.x,
            y: temple.y,
            floor: native_floor(temple.z)?,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m38_source_town_rejects_relabelled_and_ambiguous_targuna() -> Result<(), Error> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut source = Source::load(&root)?;
        let snapshot: Value = serde_json::from_slice(&super::super::read(
            &root.join(super::super::TOWN_SOURCE),
            super::super::MAX_INPUT,
        )?)
        .map_err(fail)?;
        let towns = snapshot["towns"]
            .as_array()
            .ok_or_else(|| fail("source towns"))?;
        let mut wrong_id = towns.clone();
        let town = wrong_id
            .iter_mut()
            .find(|town| town["name"] == "Targuna")
            .ok_or_else(|| fail("Targuna source town"))?;
        town["town_id"] = serde_json::json!(33);
        assert!(admit(&mut source, &wrong_id).is_err());
        let mut ambiguous = towns.clone();
        ambiguous.push(
            towns
                .iter()
                .find(|town| town["name"] == "Targuna")
                .ok_or_else(|| fail("Targuna source town"))?
                .clone(),
        );
        assert!(admit(&mut source, &ambiguous).is_err());
        assert_eq!(source.table.default_town, Some(33));
        Ok(())
    }
}
