//! Additional closed source witnesses; the original CRAFT-1 snapshot stays frozen.
use super::ProjectV2ConversionSource;

pub(super) fn pinned(source: &ProjectV2ConversionSource) -> bool {
    source.repository == "opentibiabr/canary"
        && source.revision == "04b83b512114bfd888000d6e1433ed8ecaec7c5b"
        && matches!(
            (source.path.as_str(), source.sha256.as_str(), source.lines),
            (
                "data-otservbr-global/scripts/quests/hidden_threats/action-key_forge.lua",
                "de4ddfb2c7fc132479dd9dc83cde430981295c2c0716901968897443d21f02eb",
                38
            ) | (
                "data-otservbr-global/scripts/actions/tools/metal_file.lua",
                "b7ed2f97a9632aacd831c8dbd47a6e0be75440100b433ad90f2c006bffb5d402",
                21
            )
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forge_source_admission_requires_exact_repository_revision_path_hash_and_length()
    -> Result<(), Box<dyn std::error::Error>> {
        let rows: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../../../content/craft/recipes.json"
        ))?;
        let row = rows["records"]
            .as_array()
            .ok_or("rows")?
            .iter()
            .find(|row| {
                row["identity"]["key"] == "oteryn:item-conversion.hidden-threats-file-27261"
            })
            .ok_or("forge row")?;
        let mut source: ProjectV2ConversionSource =
            serde_json::from_value(row["recipe"]["source"].clone())?;
        assert!(pinned(&source));
        source.sha256 = "0".repeat(64);
        assert!(!pinned(&source));
        source = serde_json::from_value(row["recipe"]["source"].clone())?;
        source.repository = "unreviewed/source".into();
        assert!(!pinned(&source));
        source = serde_json::from_value(row["recipe"]["source"].clone())?;
        source.lines += 1;
        assert!(!pinned(&source));
        Ok(())
    }
}
