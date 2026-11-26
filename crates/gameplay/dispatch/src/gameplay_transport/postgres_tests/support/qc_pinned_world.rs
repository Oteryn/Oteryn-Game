//! Verify the production callback selectors against the node's pinned artifact.
use super::TestResult;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub(super) fn artifact() -> TestResult<(Vec<u8>, serde_json::Value)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let pin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
    let digest = pin["digest"].as_str().ok_or("pinned world digest")?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("invalid world pin digest".into());
    }
    let filename = format!("oteryn-{digest}.otwb");
    let output = match std::env::var_os("OTERYN_QC_PIN_CHECK_OUTPUT") {
        Some(path) => PathBuf::from(path),
        None => {
            static BUILD: Mutex<()> = Mutex::new(());
            let _publish = BUILD.lock().map_err(|_| "world pin build lock poisoned")?;
            // Nextest runs every case in its own process. Serialize the build across
            // processes so concurrent cases neither race the toolchain install nor run
            // parallel release builds; later cases reuse this digest's artifact.
            let path = std::env::temp_dir().join(format!("qc-pin-{digest}"));
            std::fs::create_dir_all(&path)?;
            let build_lock = std::fs::File::create(path.join(".build.lock"))?;
            build_lock.lock()?;
            if !path.join(&filename).is_file() {
                // Build into a private directory and publish by rename, so an
                // interrupted build never leaves a partial artifact at the shared path.
                let staging =
                    std::env::temp_dir().join(format!("qc-pin-{digest}-{}", std::process::id()));
                // The fixture needs the exact published bytes once. Use the
                // compiler already linked into the test binary so the lock
                // never encloses a Cargo build or a second parity compile.
                use oteryn_world_bundle_compiler::{bundle, compile, project, source};
                let identity = serde_json::from_slice(&std::fs::read(
                    root.join("content/world/pins/oteryn.identity.json"),
                )?)?;
                let project = source::project(&root)?;
                let (registry, palette) = source::registry(&root, &project)?;
                let compiled = compile::compile(
                    &compile::Input {
                        regions: &project.regions,
                        palette: &palette,
                        identity,
                        world: source::world_extent(&root)?,
                        build_class: bundle::BuildClass::NonProduction,
                        draft_areas: project::draft_areas(&project.index)?,
                        families: &project.families,
                    },
                    &registry,
                )?;
                if oteryn_world_bundle::npc::hex(&compiled.digest) != digest {
                    return Err("compiled World differs from the published pin".into());
                }
                std::fs::create_dir_all(&staging)?;
                std::fs::write(staging.join(&filename), compiled.bytes)?;
                std::fs::rename(staging.join(&filename), path.join(&filename))?;
                let _ = std::fs::remove_dir_all(&staging);
            }
            path
        }
    };
    let bytes = std::fs::read(output.join(filename))?;
    Ok((bytes, pin))
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let (bytes, pin) = artifact()?;
    boot(native, &bytes, &boot_pins(&pin)?)
}

/// Retain the real source tiles, including their ordinals and attributes, and
/// the surrounding sectors needed by the live map view. Verify the original
/// digest before booting the smaller fixture bundle.
pub(super) fn world_near(
    native: &crate::content::native_gameplay::NativeGameplayState,
    positions: &[(u16, u16)],
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector::SECTOR_SIZE};

    let (bytes, pin) = artifact()?;
    let mut pins = boot_pins(&pin)?;
    let mut sectors = Vec::new();
    let visited = bundle::visit(&bytes, |_, sector| {
        if positions.iter().copied().chain([(pins.start.x, pins.start.y)]).any(|(x, y)| {
            sector.sx.abs_diff(x / SECTOR_SIZE) <= 1 && sector.sy.abs_diff(y / SECTOR_SIZE) <= 1
        }) {
            sectors.push(sector);
        }
        Ok(())
    })?;
    if visited.digest != pins.bundle.digest || visited.manifest.is_production() != pins.bundle.production {
        return Err("pinned World identity mismatch".into());
    }
    drop(bytes);

    let mut manifest = visited.manifest;
    manifest.build_class = bundle::BuildClass::NonProduction;
    manifest.spawns = Default::default();
    manifest.npcs = Default::default();
    manifest.dropped_teleports.retain(|&key| bundle::names_entry(&sectors, key));
    let projected = oteryn_world_bundle::writer::write(&manifest, &sectors, &Default::default())?;
    pins.bundle.digest = projected[projected.len() - bundle::DIGEST..].try_into()?;
    pins.bundle.production = false;
    pins.map_revision = format!("sha256:{}", oteryn_world_bundle::npc::hex(&pins.bundle.digest));
    boot(native, &projected, &pins)
}

pub(super) fn boot_pins(pin: &serde_json::Value) -> TestResult<crate::map::boot::BootPins> {
    let field = |key: &str| -> TestResult<&str> { Ok(pin[key].as_str().ok_or("world pin field")?) };
    let hex = field("digest")?;
    let mut digest = [0; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * index..2 * index + 2], 16)?;
    }
    let entry = &pin["entry_start"];
    Ok(crate::map::boot::BootPins {
        bundle: crate::map::BundlePins {
            digest,
            project_format_version: field("project_format_version")?.into(),
            world_schema_version: field("world_schema_version")?.into(),
            content_revision: field("content_revision")?.into(),
            production: pin["production"].as_bool().ok_or("world pin class")?,
        },
        map_revision: format!("sha256:{hex}"),
        start: crate::map::overlay::TilePos {
            x: u16::try_from(entry["x"].as_u64().ok_or("entry x")?)?,
            y: u16::try_from(entry["y"].as_u64().ok_or("entry y")?)?,
            floor: -i8::try_from(entry["floor"].as_u64().filter(|&z| z <= 15).ok_or("entry floor")?)?,
        },
    })
}

fn boot(
    native: &crate::content::native_gameplay::NativeGameplayState,
    bytes: &[u8],
    pins: &crate::map::boot::BootPins,
) -> TestResult<crate::map::boot::BundleWorld> {
    crate::map::boot::boot(
        bytes,
        pins,
        crate::foundation::WorldId::decode(&super::fixture::id(42))
            .map_err(super::fixture::debug)?,
        crate::foundation::ChannelId::decode(&super::fixture::id(43))
            .map_err(super::fixture::debug)?,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(super::fixture::debug)
}

pub(super) fn ordinal(
    tile: crate::map::TileView<'_>,
    appearance: u16,
    unique: Option<u16>,
) -> TestResult<u8> {
    (0..oteryn_world_bundle::bundle::MAX_TILE_BASE_ENTRIES)
        .filter_map(|index| u8::try_from(index).ok())
        .find(|&index| {
            tile.appearance(index) == Some(appearance)
                && unique
                    .is_none_or(|uid| tile.unique(index).is_some_and(|entry| entry.unique == uid))
        })
        .ok_or_else(|| format!("live object {appearance} UID {unique:?} missing").into())
}

// Crystal 201189f8, data-global/startup/tables/chest.lua:623-628, 1216-1232, 2057-2065.
pub(super) const COMPLETION_CHEST_UIDS: [u16; 4] = [6006, 6083, 6084, 6188];

pub(super) async fn seed_saved_progress(
    h: &super::fixture::Harness,
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
    quest: &str,
    hash: &str,
    track: &str,
    value: i64,
) -> TestResult {
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(track.to_owned(), value)],
        119,
    )
    .await?;
    let mut tx = h.pool.begin().await?;
    // Install the legacy row that would already exist before admission.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let installed = sqlx::query(
        "INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) SELECT character_id,$2,'content-1',decode($3,'hex'),character_revision,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(super::fixture::id(41).as_slice())
    .bind(quest)
    .bind(hash)
    .execute(&mut *tx)
    .await?;
    assert_eq!(installed.rows_affected(), 1);
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}
