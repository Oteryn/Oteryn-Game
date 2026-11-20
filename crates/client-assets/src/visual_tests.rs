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
    #[test]
    fn map_phases_preserve_placement_and_reject_out_of_range_frames()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(crate::world::MANIFEST_PATH),
            crate::world::MANIFEST_SHA256,
        )?;
        let catalog = Catalog::load(&store)?;
        let index = AppearanceIndex::load(&store, &catalog)?;
        let sheets = SpriteSheets::new(store, catalog);
        let appearance = index
            .iter()
            .filter(|appearance| {
                appearance.phases > 1
                    && appearance.sprite_ids.first() != appearance.sprite_ids.get(1)
            })
            .min_by_key(|appearance| appearance.id)
            .ok_or("animated map item")?;
        let placement = Placement::at(100, 200, 7);
        let first = index.resolve(&sheets, appearance.id, placement)?;
        assert_eq!(
            first,
            index.resolve_phase(&sheets, appearance.id, placement, 0)?
        );
        assert_ne!(
            first.cells,
            index
                .resolve_phase(&sheets, appearance.id, placement, 1)?
                .cells
        );
        assert!(
            index
                .resolve_phase(&sheets, appearance.id, placement, appearance.phases)
                .is_err()
        );
        Ok(())
    }
}
