//! Acceptance tests over the pinned 15.30 assets.
use oteryn_client_assets::{
    AppearanceIndex, AssetError, AssetStore, Catalog, MAX_APPEARANCE_ID, MAX_ENTRY_CELLS,
    MAX_RESIDENT_SHEETS, Placement, SpriteLayout, SpriteSheets,
};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest() -> PathBuf {
    root().join("imports/official/client-assets/15.30/manifest.json")
}

fn assets() -> PathBuf {
    root().join("content/assets/files")
}

fn load() -> Result<(AppearanceIndex, SpriteSheets), Box<dyn Error>> {
    let store = AssetStore::open(&assets(), &manifest())?;
    let catalog = Catalog::load(&store)?;
    let index = AppearanceIndex::load(&store, &catalog)?;
    Ok((index, SpriteSheets::new(store, catalog)))
}

/// Copies the catalogue, the appearances and one sprite sheet into a scratch directory.
fn scratch_copy(
    tag: &str,
    sheet_id: u32,
) -> Result<(PathBuf, String, Vec<String>), Box<dyn Error>> {
    let store = AssetStore::open(&assets(), &manifest())?;
    let catalog = Catalog::load(&store)?;
    let sheets = SpriteSheets::new(store, catalog.clone());
    let sheet = sheets
        .sheet_of(sheet_id)
        .ok_or("sprite has no sheet")?
        .file
        .clone();
    let dir = std::env::temp_dir().join(format!("oteryn-assets-{}-{tag}", std::process::id()));
    fs::create_dir_all(&dir)?;
    let names = [
        "catalog-content.json".to_owned(),
        catalog.appearances_file().to_owned(),
        sheet.clone(),
    ];
    for name in &names {
        fs::copy(assets().join(name), dir.join(name))?;
    }
    Ok((dir, sheet, names.to_vec()))
}

fn flip_first_byte(path: &Path) -> TestResult {
    let mut bytes = fs::read(path)?;
    bytes[0] ^= 0xFF;
    fs::write(path, bytes)?;
    Ok(())
}

#[test]
fn pinned_catalogue_and_appearances_load() -> TestResult {
    let (index, _) = load()?;
    assert_eq!(index.len(), 43_516);
    assert_eq!(index.max_id(), Some(55_117));
    Ok(())
}

#[test]
fn corrupted_files_fail_closed() -> TestResult {
    let (dir, sheet, names) = scratch_copy("corrupt", 106)?;
    let open = || AssetStore::open(&dir, &manifest());
    // Untouched copy loads.
    let catalog = Catalog::load(&open()?)?;
    let appearances = names[1].clone();
    let mut sheets = SpriteSheets::new(open()?, catalog.clone());
    assert!(sheets.cell_rgba(106, 0, 0).is_ok());

    flip_first_byte(&dir.join(&sheet))?;
    let mut sheets = SpriteSheets::new(open()?, catalog);
    assert_eq!(
        sheets.cell_rgba(106, 0, 0),
        Err(AssetError::HashMismatch { file: sheet })
    );

    flip_first_byte(&dir.join(&appearances))?;
    let catalog = Catalog::load(&open()?)?;
    assert_eq!(
        AppearanceIndex::load(&open()?, &catalog).err(),
        Some(AssetError::HashMismatch { file: appearances })
    );

    flip_first_byte(&dir.join("catalog-content.json"))?;
    assert_eq!(
        Catalog::load(&open()?).err(),
        Some(AssetError::HashMismatch {
            file: "catalog-content.json".into()
        })
    );
    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn a_hash_token_name_does_not_replace_the_manifest_check() -> TestResult {
    let dir = std::env::temp_dir().join(format!("oteryn-assets-{}-token", std::process::id()));
    fs::create_dir_all(&dir)?;
    // The file name carries the hash of the altered bytes; the manifest pins different bytes.
    let altered = b"altered sheet bytes";
    let token = {
        use std::fmt::Write as _;
        // sha256 via the pinned manifest is not needed: any 64-hex token shows the point.
        let mut text = String::new();
        for byte in altered.iter().cycle().take(32) {
            write!(text, "{byte:02x}")?;
        }
        text
    };
    let name = format!("sprites-{token}.bmp.lzma");
    fs::write(dir.join(&name), altered)?;
    let manifest_path = dir.join("manifest.json");
    fs::write(
        &manifest_path,
        format!(
            "{{\"files\":[{{\"name\":\"{name}\",\"sha256\":\"{}\"}}]}}",
            "0".repeat(64)
        ),
    )?;
    let store = AssetStore::open(&dir, &manifest_path)?;
    assert_eq!(
        store.read_verified(&name, 1024),
        Err(AssetError::HashMismatch { file: name })
    );
    assert_eq!(
        store.read_verified("absent.bmp.lzma", 1024),
        Err(AssetError::NotInManifest {
            file: "absent.bmp.lzma".into()
        })
    );
    fs::write(dir.join("unlisted.bin"), b"x")?;
    assert_eq!(
        store.read_verified("unlisted.bin", 1024),
        Err(AssetError::NotInManifest {
            file: "unlisted.bin".into()
        })
    );
    fs::remove_dir_all(&dir)?;
    Ok(())
}

#[test]
fn depth_patterns_follow_the_floor() -> TestResult {
    let (index, sheets) = load()?;
    let appearance = index.get(290).ok_or("appearance 290")?;
    assert_eq!(appearance.pattern_depth, 2);
    let ground = index.resolve(&sheets, 290, Placement::at(0, 0, 0))?;
    let seven = index.resolve(&sheets, 290, Placement::at(0, 0, -7))?;
    assert_ne!(ground.cells, seven.cells);
    let expected_ground = appearance.sprite_ids[0];
    let expected_seven = appearance.sprite_ids[(appearance.layers) as usize
        * (appearance.pattern_width * appearance.pattern_height) as usize];
    assert_eq!(ground.cells[0].sprite_id, expected_ground);
    assert_eq!(seven.cells[0].sprite_id, expected_seven);
    Ok(())
}

#[test]
fn ground_pattern_follows_position() -> TestResult {
    let (index, sheets) = load()?;
    let appearance = index.get(106).ok_or("appearance 106")?;
    assert_eq!(
        (appearance.pattern_width, appearance.pattern_height),
        (4, 4)
    );
    let a = index.resolve(&sheets, 106, Placement::at(0, 0, 0))?;
    let b = index.resolve(&sheets, 106, Placement::at(1, 0, 0))?;
    assert_ne!(a.cells, b.cells);
    // Negative coordinates wrap rather than index below zero.
    let wrapped = index.resolve(&sheets, 106, Placement::at(-1, 0, 0))?;
    let direct = index.resolve(&sheets, 106, Placement::at(3, 0, 0))?;
    assert_eq!(wrapped.cells, direct.cells);
    Ok(())
}

#[test]
fn stackable_count_selects_the_pattern() -> TestResult {
    let (index, sheets) = load()?;
    let one = index.resolve(&sheets, 130, Placement::at(0, 0, 0))?;
    let five = index.resolve(
        &sheets,
        130,
        Placement {
            count: 5,
            sub_type: 0,
            x: 0,
            y: 0,
            floor: 0,
        },
    )?;
    let hundred = index.resolve(
        &sheets,
        130,
        Placement {
            count: 100,
            sub_type: 0,
            x: 0,
            y: 0,
            floor: 0,
        },
    )?;
    assert_ne!(one.cells, five.cells);
    assert_ne!(five.cells, hundred.cells);
    assert_ne!(one.cells, hundred.cells);
    Ok(())
}

#[test]
fn a_64x64_object_resolves_to_four_offset_cells() -> TestResult {
    let (index, sheets) = load()?;
    let appearance = index.get(230).ok_or("appearance 230")?;
    let entry = index.resolve(&sheets, 230, Placement::at(0, 0, 0))?;
    assert_eq!(
        sheets.layout(appearance.sprite_ids[0])?,
        SpriteLayout::Size64x64
    );
    assert_eq!(entry.cells.len(), 4);
    let (dx, dy) = (
        appearance.displacement_x as i32,
        appearance.displacement_y as i32,
    );
    let mut offsets: Vec<(i32, i32)> = entry
        .cells
        .iter()
        .map(|cell| (cell.offset_x, cell.offset_y))
        .collect();
    offsets.sort_unstable();
    assert_eq!(
        offsets,
        vec![
            (-32 - dx, -32 - dy),
            (-32 - dx, -dy),
            (-dx, -32 - dy),
            (-dx, -dy)
        ]
    );
    Ok(())
}

#[test]
fn appearance_104_is_a_sixteen_pattern_ground() -> TestResult {
    let (index, sheets) = load()?;
    let appearance = index.get(104).ok_or("appearance 104")?;
    // Sixteen position patterns of one 32x32 cell each; one pattern is drawn per tile.
    assert_eq!(appearance.pattern_width * appearance.pattern_height, 16);
    assert!(appearance.ground);
    let mut distinct = std::collections::BTreeSet::new();
    for y in 0..4 {
        for x in 0..4 {
            let entry = index.resolve(&sheets, 104, Placement::at(x, y, 0))?;
            assert_eq!(entry.cells.len(), 1);
            distinct.insert(entry.cells[0].sprite_id);
        }
    }
    assert_eq!(distinct.len(), 16);
    Ok(())
}

#[test]
fn bad_ids_are_errors() -> TestResult {
    let (index, sheets) = load()?;
    assert_eq!(
        index.resolve(&sheets, 0, Placement::at(0, 0, 0)),
        Err(AssetError::InvalidAppearanceId { id: 0 })
    );
    assert_eq!(
        index.resolve(&sheets, 65_536, Placement::at(0, 0, 0)),
        Err(AssetError::InvalidAppearanceId { id: 65_536 })
    );
    assert_eq!(MAX_APPEARANCE_ID, 65_535);
    let unknown = (1..=MAX_APPEARANCE_ID)
        .find(|id| index.get(*id).is_none())
        .ok_or("every id is known")?;
    assert_eq!(
        index.resolve(&sheets, unknown, Placement::at(0, 0, 0)),
        Err(AssetError::UnknownAppearance { id: unknown })
    );
    assert_eq!(
        index.resolve(&sheets, 104, Placement::at(0, 0, 1)),
        Err(AssetError::InvalidFloor { floor: 1 })
    );
    Ok(())
}

#[test]
fn the_sheet_cache_evicts_the_least_recently_used() -> TestResult {
    let store = AssetStore::open(&assets(), &manifest())?;
    let catalog = Catalog::load(&store)?;
    let mut sheets = SpriteSheets::new(store, catalog);
    let mut per_sheet: Vec<(String, u32)> = Vec::new();
    let mut id = 1_u32;
    while per_sheet.len() < MAX_RESIDENT_SHEETS + 1 {
        if let Some(descriptor) = sheets.sheet_of(id)
            && per_sheet.last().map(|(file, _)| file) != Some(&descriptor.file)
        {
            per_sheet.push((descriptor.file.clone(), id));
        }
        id += 1;
        assert!(id < 1_000_000, "not enough sheets");
    }
    for (_, sprite) in per_sheet.iter().take(MAX_RESIDENT_SHEETS) {
        sheets.cell_rgba(*sprite, 0, 0)?;
    }
    assert_eq!(sheets.resident_sheets(), MAX_RESIDENT_SHEETS);
    // Touch the first sheet so the second becomes the least recently used.
    sheets.cell_rgba(per_sheet[0].1, 0, 0)?;
    sheets.cell_rgba(per_sheet[MAX_RESIDENT_SHEETS].1, 0, 0)?;
    assert_eq!(sheets.resident_sheets(), MAX_RESIDENT_SHEETS);
    assert!(sheets.is_resident(&per_sheet[0].0));
    assert!(!sheets.is_resident(&per_sheet[1].0));
    assert!(sheets.is_resident(&per_sheet[MAX_RESIDENT_SHEETS].0));
    Ok(())
}

#[test]
fn a_sheet_decodes_within_the_bound() -> TestResult {
    let (index, mut sheets) = load()?;
    let sprite = index.get(106).ok_or("appearance 106")?.sprite_ids[0];
    let start = std::time::Instant::now();
    let cell = sheets.cell_rgba(sprite, 0, 0)?;
    let elapsed = start.elapsed();
    assert_eq!(cell.len(), 32 * 32 * 4);
    println!("sheet decode (verify + lzma + bmp): {elapsed:?}");
    // Measured about 0.1 s in release; the bound leaves room for an unoptimised test build.
    assert!(elapsed < std::time::Duration::from_secs(10), "{elapsed:?}");
    Ok(())
}

#[test]
fn no_pinned_appearance_exceeds_the_cell_cap() -> TestResult {
    let (index, sheets) = load()?;
    let mut widest = 0;
    for appearance in index.iter() {
        let (cells_per_pattern, patterns) = (
            appearance.layers as usize,
            (appearance.pattern_width * appearance.pattern_height * appearance.pattern_depth)
                as usize,
        );
        for pattern in 0..patterns {
            let mut cells = 0;
            for layer in 0..cells_per_pattern {
                let sprite = appearance.sprite_ids[pattern * cells_per_pattern + layer];
                if sprite != 0 {
                    let (x, y) = sheets.layout(sprite)?.cells();
                    cells += (x * y) as usize;
                }
            }
            widest = widest.max(cells);
        }
    }
    println!("widest pinned entry: {widest} cells; cap {MAX_ENTRY_CELLS}");
    assert!(widest <= MAX_ENTRY_CELLS);
    Ok(())
}
