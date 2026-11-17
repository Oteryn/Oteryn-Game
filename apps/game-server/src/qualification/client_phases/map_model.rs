// MAPBOT-1: immutable client oracle, using the production bundle reader and boot facts.
use super::*;
use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

const DIRECTIONS: [StepDirection; 4] = [
    StepDirection::North,
    StepDirection::East,
    StepDirection::South,
    StepDirection::West,
];
const RADIUS: u16 = 160;

#[derive(Debug)]
struct MapTile {
    walkable: bool,
    door: bool,
    protection_zone: bool,
    house: u32,
    field: Option<String>,
    ladder: bool,
    rope_spot: bool,
    rope_ordinal: Option<u8>,
    shovel_ordinal: Option<u8>,
    shovel_target: bool,
    hole: bool,
    teleport: Option<TilePos>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Sample {
    kind: &'static str,
    position: TilePos,
    content: Option<String>,
}

pub(super) struct MapModel<'a> {
    world: &'a BundleWorld,
    tiles: BTreeMap<TilePos, MapTile>,
    samples: Vec<Sample>,
}

impl<'a> MapModel<'a> {
    pub(super) fn load(world: &'a BundleWorld, bytes: &[u8], root: &Path) -> TestResult<Self> {
        // Classification supplements the bundle, never parses map geometry a second time.
        // UNKNOWN catalogue fields stay unknown; no appearance/name heuristics.
        let mut kinds = BTreeMap::new();
        for directory in ["content/world/objects", "content/world/terrain"] {
            let mut paths = std::fs::read_dir(root.join(directory))?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<Vec<_>, _>>()?;
            paths.sort();
            for path in paths {
                if path.extension().is_none_or(|extension| extension != "json") {
                    continue;
                }
                let shard: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
                for record in shard["records"].as_array().into_iter().flatten() {
                    if record["kind"]["state"] != "KNOWN" {
                        continue;
                    }
                    let key = record["provenance"]["item_pointer"]["key"]
                        .as_str()
                        .or_else(|| record["identity"]["key"].as_str());
                    if let (Some(key), Some(kind)) = (key, record["kind"]["value"].as_str()) {
                        kinds.insert(key.to_owned(), kind.to_owned());
                    }
                }
            }
        }
        let mut tiles = BTreeMap::new();
        let spells: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join("content/abilities/definitions/player-spells.json"),
        )?)?;
        let rope_ground =
            find_parameter(&spells, "rope_ground_items").ok_or("rope spell has no ground facts")?;
        let rope_top =
            find_parameter(&spells, "rope_top_items").ok_or("rope spell has no top facts")?;
        let shovel_bindings: Vec<_> = oteryn_gameplay_world::terrain_tools::crystal_core_bindings()
            .into_iter()
            .filter(|binding| {
                binding.tool == oteryn_gameplay_world::terrain_tools::TerrainTool::Shovel
            })
            .collect();
        let metadata = oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
            for tile in sector.tiles {
                let position = TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                };
                let mut field = None;
                let mut ladder = false;
                let mut rope_spot = false;
                let mut rope_ordinal = None;
                let mut shovel_ordinal = None;
                let mut shovel_target = false;
                let mut hole = false;
                let mut door = world.facts().doors().at(position);
                for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate()
                {
                    let entry = &manifest.palette[item.palette as usize];
                    let kind = kinds.get(&entry.key).map(String::as_str);
                    if kind == Some("field")
                        || entry.terrain.is_some_and(|terrain| {
                            terrain.kind == oteryn_world_bundle::bundle::TerrainKind::Field
                        })
                    {
                        field = Some(entry.key.clone());
                    }
                    ladder |= kind == Some("ladder");
                    door |= kind == Some("door");
                    if let Some(id) = crate::map::palette_appearance(&entry.key) {
                        let id = u32::from(id);
                        let ground = entry.terrain.is_some_and(|terrain| {
                            terrain.kind == oteryn_world_bundle::bundle::TerrainKind::Ground
                        });
                        if (ground && parameter_contains(rope_ground, id))
                            || (!ground && parameter_contains(rope_top, id))
                        {
                            rope_spot = true;
                            rope_ordinal = Some(u8::try_from(ordinal).map_err(|_| {
                                oteryn_world_bundle::Error::Limit("map tool ordinal".into())
                            })?);
                        }
                        if shovel_bindings.iter().any(|binding| {
                            binding.target == id && binding.required_action == item.attrs.action
                        }) {
                            shovel_target = true;
                            shovel_ordinal = Some(u8::try_from(ordinal).map_err(|_| {
                                oteryn_world_bundle::Error::Limit("map tool ordinal".into())
                            })?);
                        }
                        hole |= shovel_bindings
                            .iter()
                            .any(|binding| binding.transformed == id);
                    }
                }
                tiles.insert(
                    position,
                    MapTile {
                        walkable: world.enterable(position)
                            && world
                                .base()
                                .tile(position.x, position.y, position.floor)
                                .is_some_and(|tile| tile.walkable()),
                        door,
                        protection_zone: tile.flags & 1 != 0,
                        house: tile.house,
                        field,
                        ladder,
                        rope_spot,
                        rope_ordinal,
                        shovel_ordinal,
                        shovel_target,
                        hole,
                        teleport: world.floors().teleport(position),
                    },
                );
            }
            Ok(())
        })?;
        if metadata.digest != world.base().digest() {
            return Err("map oracle and boot world have different bundle digests".into());
        }
        let mut model = Self {
            world,
            tiles,
            samples: Vec::new(),
        };
        for (&position, facts) in &model.tiles {
            let mut add = |kind, content| {
                model.samples.push(Sample {
                    kind,
                    position,
                    content,
                })
            };
            if facts.door && !facts.walkable {
                add("closed_door", None);
            }
            if !facts.walkable && !facts.door {
                add("blocking_item", None);
            }
            if facts.ladder {
                add("ladder", None);
            }
            if facts.rope_spot {
                add("rope_spot", None);
            }
            if facts.shovel_target {
                add("shovel_hole", None);
            }
            if let Some(field) = &facts.field {
                add("field", Some(field.clone()));
            }
            if facts.teleport.is_some() {
                add("teleport", None);
            }
            if DIRECTIONS.iter().any(|&direction| {
                adjacent(position, direction)
                    .and_then(|at| model.tiles.get(&at))
                    .is_some_and(|other| other.protection_zone != facts.protection_zone)
            }) {
                add("pz_border", None);
            }
        }
        // A transition sample names the tile entered, not the remote landing tile.
        for &from in model.tiles.keys() {
            for direction in DIRECTIONS {
                if let (Some(entered), Some(landed)) = (
                    adjacent(from, direction),
                    model.destination(from, direction),
                ) && landed.floor != from.floor
                {
                    model.samples.push(Sample {
                        kind: if model.tiles.get(&entered).is_some_and(|facts| facts.hole) {
                            "hole_down"
                        } else if landed.floor > from.floor {
                            "stairs_up"
                        } else {
                            "stairs_down"
                        },
                        position: entered,
                        content: None,
                    });
                }
            }
        }
        for source in &metadata.spawns.sources {
            for point in &source.points {
                model.samples.push(Sample {
                    kind: "monster_spawn",
                    position: TilePos {
                        x: point.x,
                        y: point.y,
                        floor: source.floor,
                    },
                    content: Some(
                        metadata.spawns.creatures[point.creature as usize]
                            .key
                            .clone(),
                    ),
                });
            }
        }
        for npc in &metadata.npcs.npcs {
            for point in &npc.placements {
                model.samples.push(Sample {
                    kind: "npc",
                    position: TilePos {
                        x: point.x,
                        y: point.y,
                        floor: point.floor,
                    },
                    content: Some(npc.key.clone()),
                });
            }
        }
        model.samples.sort_by(|a, b| {
            (a.kind, a.position, &a.content).cmp(&(b.kind, b.position, &b.content))
        });
        model
            .samples
            .dedup_by(|a, b| a.kind == b.kind && a.position == b.position);
        Ok(model)
    }

    fn destination(&self, from: TilePos, direction: StepDirection) -> Option<TilePos> {
        // The same resolver as the movement owner, including automatic floor chains.
        // `destination`'s ground test includes live boot door state through enterable().
        destination(self.world, from, direction)
    }

    fn path(
        &self,
        from: TilePos,
        to: TilePos,
        bounds: Option<TilePos>,
    ) -> Option<Vec<StepDirection>> {
        search(
            from,
            to,
            |at, direction| {
                let destination = self.destination(at, direction)?;
                if bounds.is_some_and(|center| !in_town(destination, center)) {
                    return None;
                }
                Some(destination)
            },
            self.world.floors().count() == 0 && self.world.floors().teleport_count() == 0,
        )
    }

    fn samples(&self, start: TilePos, seed: u64) -> Vec<Sample> {
        let mut reachable = BTreeSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some(at) = queue.pop_front() {
            for direction in DIRECTIONS {
                if let Some(next) = self.destination(at, direction)
                    && in_town(next, start)
                    && reachable.insert(next)
                {
                    queue.push_back(next);
                }
            }
        }
        let mut groups: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for sample in &self.samples {
            if in_town(sample.position, start) {
                groups.entry(sample.kind).or_default().push(sample.clone());
            }
        }
        let mut output = Vec::new();
        let mut state = seed;
        for mut group in groups.into_values() {
            let approachable: Vec<_> = group
                .iter()
                .filter(|sample| {
                    DIRECTIONS.iter().any(|&direction| {
                        adjacent(sample.position, direction)
                            .is_some_and(|at| reachable.contains(&at))
                    })
                })
                .cloned()
                .collect();
            if !approachable.is_empty() {
                group = approachable;
            }
            let count = group.len().min(5);
            for at in 0..count {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let chosen = at + (state % (group.len() - at) as u64) as usize;
                group.swap(at, chosen);
            }
            group.truncate(count);
            group.sort_by_key(|sample| sample.position);
            output.extend(group);
        }
        output
    }
}

fn in_town(at: TilePos, start: TilePos) -> bool {
    at.x.abs_diff(start.x) <= RADIUS
        && at.y.abs_diff(start.y) <= RADIUS
        && at.floor.abs_diff(start.floor) <= 2
}

fn find_parameter<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    match value {
        serde_json::Value::Object(object) => object
            .get(key)
            .or_else(|| object.values().find_map(|value| find_parameter(value, key))),
        serde_json::Value::Array(array) => {
            array.iter().find_map(|value| find_parameter(value, key))
        }
        _ => None,
    }
}

fn parameter_contains(parameter: &serde_json::Value, id: u32) -> bool {
    parameter.as_array().is_some_and(|values| {
        values
            .iter()
            .any(|value| value.as_u64() == Some(u64::from(id)))
    })
}

fn adjacent(at: TilePos, direction: StepDirection) -> Option<TilePos> {
    let (dx, dy) = match direction {
        StepDirection::North => (0, -1),
        StepDirection::East => (1, 0),
        StepDirection::South => (0, 1),
        StepDirection::West => (-1, 0),
        StepDirection::NorthEast => (1, -1),
        StepDirection::SouthEast => (1, 1),
        StepDirection::SouthWest => (-1, 1),
        StepDirection::NorthWest => (-1, -1),
    };
    Some(TilePos {
        x: at.x.checked_add_signed(dx)?,
        y: at.y.checked_add_signed(dy)?,
        floor: at.floor,
    })
}

/// A* over real STEP edges. Cardinal steps cost 10, as in movement/path_search.rs.
/// Cross-floor/teleport searches use h=0: Manhattan distance would overestimate shortcuts.
fn search(
    from: TilePos,
    to: TilePos,
    next: impl Fn(TilePos, StepDirection) -> Option<TilePos>,
    same_floor: bool,
) -> Option<Vec<StepDirection>> {
    let estimate = |at: TilePos| {
        if same_floor {
            10 * (u32::from(at.x.abs_diff(to.x)) + u32::from(at.y.abs_diff(to.y)))
        } else {
            0
        }
    };
    let mut open = BinaryHeap::from([Reverse((estimate(from), 0u32, from))]);
    let mut costs = BTreeMap::from([(from, 0u32)]);
    let mut parents = BTreeMap::new();
    while let Some(Reverse((_, cost, at))) = open.pop() {
        if costs.get(&at) != Some(&cost) {
            continue;
        }
        if at == to {
            let mut path = Vec::new();
            let mut cursor = at;
            while cursor != from {
                let &(previous, direction) = parents.get(&cursor)?;
                path.push(direction);
                cursor = previous;
            }
            path.reverse();
            return Some(path);
        }
        for direction in DIRECTIONS {
            let Some(destination) = next(at, direction) else {
                continue;
            };
            let candidate = cost + 10;
            if costs.get(&destination).is_none_or(|&old| candidate < old) {
                costs.insert(destination, candidate);
                parents.insert(destination, (at, direction));
                open.push(Reverse((
                    candidate + estimate(destination),
                    candidate,
                    destination,
                )));
            }
        }
    }
    None
}
