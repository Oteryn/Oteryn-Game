#[cfg(test)]
mod visual_tests {
    use super::*;
    #[test]
    fn t012_verified_effects_animate_once_and_missiles_keep_direction()
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
        let effect = index.visual(1, false).ok_or("effect 1 missing")?;
        assert_eq!(effect.phase_at(0), Some(0));
        assert_eq!(effect.phase_at(100), Some(1));
        assert_eq!(effect.phase_at(600), None);
        let first = index.resolve_visual(&sheets, 1, false, 0, [0, 0])?;
        let next = index.resolve_visual(&sheets, 1, false, 1, [0, 0])?;
        assert_ne!(first.cells, next.cells);
        assert!(!first.cells.is_empty());
        let nw = index.resolve_visual(&sheets, 1, true, 0, [-1, -1])?;
        let se = index.resolve_visual(&sheets, 1, true, 0, [1, 1])?;
        assert_ne!(nw.cells, se.cells);
        assert!(
            index
                .resolve_visual(&sheets, 65535, false, 0, [0, 0])
                .is_err()
        );
        Ok(())
    }
}
