#[cfg(test)]
mod tests {
    use crate::{AppearanceIndex, AssetStore, Catalog, SpriteSheets};
    #[test]
    fn walking_outfits_resolve_distinct_time_selected_frames()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(crate::world::MANIFEST_PATH),
            crate::world::MANIFEST_SHA256,
        )?;
        let catalog = Catalog::load(&store)?;
        let index = AppearanceIndex::load(&store, &catalog)?;
        let sheets = SpriteSheets::new(store, catalog);
        let standing = index.resolve_outfit(&sheets, 128, 2)?;
        let frames = (0..2000)
            .step_by(50)
            .map(|elapsed| index.resolve_outfit_at(&sheets, 128, 2, true, elapsed))
            .collect::<Result<Vec<_>, _>>()?;
        assert!(frames.iter().any(|frame| frame.cells != standing.cells));
        assert!(frames.windows(2).any(|pair| pair[0].cells != pair[1].cells));
        assert_eq!(
            frames[0],
            index.resolve_outfit_at(&sheets, 128, 2, true, 0)?
        );
        Ok(())
    }
}
