//! Verify the production callback selectors against the node's pinned artifact.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Position};
use oteryn_world_bundle::bundle;
use std::path::{Path, PathBuf};

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
            let path = std::env::temp_dir().join(format!("qc-pin-{}", std::process::id()));
            if !path.join(&filename).is_file() {
                let status = std::process::Command::new("cargo")
                    .current_dir(root)
                    .args([
                        "run",
                        "--locked",
                        "--release",
                        "-p",
                        "oteryn-world-bundle-compiler",
                        "--",
                        "pin-check",
                        ".",
                    ])
                    .arg(&path)
                    .status()?;
                if !status.success() {
                    return Err("world pin-check failed".into());
                }
            }
            path
        }
    };
    let bytes = std::fs::read(output.join(filename))?;
    Ok((bytes, pin))
}

pub(super) const PILE: &str = "qc-twenty-miles-pile-1";

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    check_floors: bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    let (bytes, pin) = artifact()?;
    let field = |key: &str| -> TestResult<&str> { Ok(pin[key].as_str().ok_or("world pin field")?) };
    let hex = field("digest")?;
    let mut digest = [0; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * index..2 * index + 2], 16)?;
    }
    if check_floors {
        verify_selectors(&bytes, digest)?;
    }
    let entry = &pin["entry_start"];
    let pins = crate::map::boot::BootPins {
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
            floor: i8::try_from(entry["floor"].as_i64().ok_or("entry floor")?)?,
        },
    };
    crate::map::boot::boot(
        &bytes,
        &pins,
        crate::foundation::WorldId::decode(&super::fixture::id(42))
            .map_err(super::fixture::debug)?,
        crate::foundation::ChannelId::decode(&super::fixture::id(43))
            .map_err(super::fixture::debug)?,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(super::fixture::debug)
}

fn verify_selectors(bytes: &[u8], digest: [u8; 32]) -> TestResult {
    let callbacks: Vec<_> = world_callbacks::load()?
        .into_iter()
        .filter(|callback| {
            callback
                .key
                .starts_with("oteryn:quest-callback/qc-twenty-miles-")
                || callback
                    .key
                    .starts_with("oteryn:quest-callback/qc-lizard-sanctuary-altar-")
        })
        .collect();
    assert_eq!(callbacks.len(), 27);
    let mut matched = std::collections::BTreeSet::new();
    let visited = bundle::visit(bytes, |manifest, sector| {
        for callback in &callbacks {
            let Some(p) = callback.selector.position else {
                continue;
            };
            if p.floor != sector.floor || p.x / 32 != sector.sx || p.y / 32 != sector.sy {
                continue;
            }
            let Some(appearance) = callback.selector.appearance else {
                continue;
            };
            if sector.tiles.iter().any(|tile| {
                callback.selector.matches_position(Position {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                }) && tile.items.iter().any(|item| {
                    item.depth == 0
                        && crate::map::palette_appearance(
                            &manifest.palette[item.palette as usize].key,
                        ) == Some(appearance)
                })
            }) {
                matched.insert(callback.key.as_str());
            }
        }
        Ok(())
    })?;
    assert_eq!(visited.digest, digest);
    let missing: Vec<_> = callbacks
        .iter()
        .map(|callback| callback.key.as_str())
        .filter(|key| !matched.contains(key))
        .collect();
    assert!(
        missing.is_empty(),
        "selectors missing in pinned world: {missing:?}"
    );
    Ok(())
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
    super::qengine_outfit_stories::seed_quest_prerequisites(
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
