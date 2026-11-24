//! Load the reviewed map coordinates for retained Crystal spawn declarations.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{hex, json, read};
use crate::Error;
use crate::project::{LegacyPosition, SpawnSource};

const CAPTURE: &str = "tools/world-bundle-compiler/spawn-map-slice-3.json";

#[derive(Deserialize)]
struct Pin {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
struct Capture {
    points: Vec<Point>,
}

#[derive(Deserialize)]
struct Point {
    source: String,
    creature: String,
    cell: LegacyPosition,
    move_to: Option<LegacyPosition>,
}

fn invalid(problem: &str) -> Error {
    Error::Format(format!("spawn map reconciliation: {problem}"))
}

pub(super) fn apply(root: &Path, spawns: &mut [SpawnSource]) -> Result<(), Error> {
    let index = root.join("content/world/spawns/index.json");
    if !index
        .try_exists()
        .map_err(|error| invalid(&error.to_string()))?
    {
        return Ok(());
    }
    let index = json(&read(&index)?, "spawn index")?;
    let Some(pin) = index.get("map_reconciliation") else {
        return Ok(());
    };
    let pin: Pin = serde_json::from_value(pin.clone()).map_err(|err| invalid(&err.to_string()))?;
    if pin.path != CAPTURE {
        return Err(invalid("unexpected capture path"));
    }
    let raw = read(&root.join(CAPTURE))?;
    if hex(&Sha256::digest(&raw)) != pin.sha256 {
        return Err(invalid("capture digest"));
    }
    let capture: Capture = serde_json::from_slice(&raw).map_err(|err| invalid(&err.to_string()))?;
    let mut anchors = BTreeMap::new();
    let mut destinations = BTreeSet::new();
    for row in &capture.points {
        if let Some((x, y, z)) = row.move_to
            && (z != row.cell.2
                || x.abs_diff(row.cell.0).max(y.abs_diff(row.cell.1)) > 8
                || !destinations.insert((x, y, z)))
        {
            return Err(invalid("invalid or duplicate destination"));
        }
        if anchors
            .insert(
                (row.source.as_str(), row.creature.as_str(), row.cell),
                row.move_to,
            )
            .is_some()
        {
            return Err(invalid("duplicate source anchor"));
        }
    }
    for source in spawns {
        for point in &mut source.points {
            if destinations.contains(&point.cell) {
                return Err(invalid("destination is already an authored spawn"));
            }
            if let Some(Some(cell)) =
                anchors.remove(&(source.key.as_str(), point.creature.as_str(), point.cell))
            {
                point.cell = cell;
            }
        }
    }
    if let Some(((source, creature, cell), _)) = anchors.first_key_value() {
        return Err(invalid(&format!(
            "missing source anchor {source} {creature} {cell:?}"
        )));
    }
    Ok(())
}
