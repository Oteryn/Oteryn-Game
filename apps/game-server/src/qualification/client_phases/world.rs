//! Real-world journeys use the same pinned bundle and native content as `serve`.
use super::*;
use crate::map::boot::BundleWorld;
use crate::map::overlay::TilePos;
#[path = "creature_sweep.rs"]
mod creature_sweep;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

#[path = "mp/mount.rs"]
mod mp;

#[path = "general_stats.rs"]
mod general_stats;

include!("character_core.rs");
include!("character_release.rs");
include!("e2e_row_batch.rs");
include!("floor_journey.rs");
#[path = "full_e2ef4.rs"]
mod full_e2ef4;
#[path = "full_e2ef6.rs"]
mod full_e2ef6;
include!("full_e2ef7_mount.rs");
include!("full_gold_mount.rs");
include!("full_row_c03_mount.rs");
include!("full_row_c12_mount.rs");
include!("client_row_c01_mount.rs");
include!("client_row_c02_mount.rs");
include!("client_row_c03_mount.rs");
include!("client_row_c04_mount.rs");
include!("client_row_c05_mount.rs");
include!("client_row_c06_mount.rs");
include!("client_row_c07_mount.rs");
include!("client_row_c09_mount.rs");
include!("client_row_c10_mount.rs");
include!("client_row_c11_mount.rs");
include!("client_row_c12_mount.rs");
include!("client_row_c13_mount.rs");
include!("client_row_c14_mount.rs");
include!("client_row_c15_mount.rs");
include!("client_row_c16a_mount.rs");
include!("client_row_c16b_mount.rs");
include!("client_row_c16c_mount.rs");
include!("client_row_c16d_mount.rs");
include!("client_row_c17a_mount.rs");
include!("client_row_c17b_mount.rs");
include!("client_row_c17d_mount.rs");
include!("client_row_c18a_mount.rs");
include!("client_row_c18b_mount.rs");
include!("client_row_c20a_mount.rs");
include!("client_row_c20b_mount.rs");
include!("client_row_c20c_mount.rs");
include!("client_row_c20d_mount.rs");
include!("client_row_e01_mount.rs");
include!("client_row_i01_mount.rs");
include!("client_row_i03_mount.rs");
include!("client_row_i04_mount.rs");
include!("client_row_i05_mount.rs");
include!("client_row_i06_mount.rs");
include!("client_row_i07_mount.rs");
include!("client_row_i08_mount.rs");
include!("client_row_i09_mount.rs");
include!("client_row_i10_mount.rs");
include!("client_row_i11_mount.rs");
include!("client_row_i12_mount.rs");
include!("client_row_i13_mount.rs");
include!("client_row_i14_mount.rs");
include!("client_row_i15_mount.rs");
include!("client_row_i16_mount.rs");
include!("client_row_m01_mount.rs");
include!("client_row_m02_mount.rs");
include!("client_row_m03_mount.rs");
include!("client_row_m04_mount.rs");
include!("client_row_m05_mount.rs");
include!("client_row_m06_mount.rs");
include!("client_row_m07_mount.rs");
include!("client_row_m08_mount.rs");
include!("client_row_m09_mount.rs");
include!("client_row_m10_mount.rs");
include!("client_row_m11a_mount.rs");
include!("client_row_m11b_mount.rs");
include!("client_row_n01_mount.rs");
include!("client_row_n02_mount.rs");
include!("client_row_n03_mount.rs");
include!("client_row_n04_mount.rs");
include!("client_row_c08_mount.rs");
include!("client_row_c17c_mount.rs");
include!("client_row_c19a_mount.rs");
include!("client_row_e02_mount.rs");
include!("client_row_e03_mount.rs");
include!("client_row_e05_mount.rs");
include!("client_row_e06_mount.rs");
include!("client_row_e07_mount.rs");
include!("client_row_e08_mount.rs");
include!("client_row_e09_mount.rs");
include!("client_row_e10_mount.rs");
include!("client_row_e12_mount.rs");
include!("client_row_p01_mount.rs");
include!("client_row_p02_mount.rs");
include!("client_row_p03_mount.rs");
include!("client_row_p04_mount.rs");
include!("client_row_p05_mount.rs");
include!("client_row_p06_mount.rs");
include!("client_row_p07_mount.rs");
include!("client_row_p08_mount.rs");
include!("client_row_p09_mount.rs");
include!("client_row_p10_mount.rs");
include!("client_row_p11_mount.rs");
include!("client_row_p12a_mount.rs");
include!("client_row_p12c_mount.rs");
include!("client_row_p13_mount.rs");
include!("client_row_p14_mount.rs");
include!("client_row_p15_mount.rs");
include!("client_row_p16_mount.rs");
include!("client_row_p17_mount.rs");
include!("client_row_p18_mount.rs");
include!("client_row_p19_mount.rs");
include!("client_row_p20_mount.rs");
include!("client_row_p21_mount.rs");
include!("client_row_p23c_mount.rs");
include!("client_row_s01_mount.rs");
include!("client_row_s02_mount.rs");
include!("client_row_s03_mount.rs");
include!("client_row_s04a_mount.rs");
include!("client_row_s04b_mount.rs");
include!("client_row_s18_mount.rs");
include!("client_row_w01a_mount.rs");
include!("client_row_w01b_mount.rs");
include!("client_row_w01c_mount.rs");
include!("client_row_w02_mount.rs");
include!("client_row_w03_mount.rs");
include!("client_row_w04_mount.rs");
include!("client_row_w05a_mount.rs");
include!("client_row_w05c_mount.rs");
include!("client_row_w10_mount.rs");
include!("client_row_w11_mount.rs");
include!("client_row_w12_mount.rs");
include!("client_row_w13_mount.rs");
include!("client_row_s06_mount.rs");
include!("client_row_s08_mount.rs");
include!("client_row_s13c_mount.rs");
include!("client_row_s13b_mount.rs");
include!("client_row_s13a_mount.rs");
include!("client_row_s15b_mount.rs");
include!("client_row_s04c_mount.rs");
include!("client_row_s07_mount.rs");
include!("client_row_s16b_mount.rs");
include!("client_row_s09_mount.rs");
include!("client_row_s05_mount.rs");
include!("client_row_s10_mount.rs");
include!("client_row_s16a_mount.rs");
include!("client_row_s11_mount.rs");
include!("client_row_w15a_mount.rs");
include!("client_row_w15b_mount.rs");
include!("client_row_w15d_mount.rs");
include!("client_row_w16_mount.rs");
include!("client_row_w17_mount.rs");
include!("client_row_w19a_mount.rs");
include!("client_row_w20a_mount.rs");
include!("client_row_w20b_mount.rs");
include!("client_row_w22a_mount.rs");
include!("client_row_w22b_mount.rs");
include!("client_row_w23_mount.rs");
include!("client_row_w24_mount.rs");
include!("client_row_n07a_mount.rs");
include!("client_row_n07b_mount.rs");
include!("client_row_n07c_mount.rs");
include!("client_row_n07d_mount.rs");
include!("full_row_c09_mount.rs");
include!("full_row_c06_mount.rs");
include!("full_row_c02_mount.rs");
include!("full_row_c01_mount.rs");
include!("full_row_w22b_mount.rs");
include!("full_row_c05_mount.rs");
include!("full_row_c04_mount.rs");
include!("full_row_w18_mount.rs");
include!("full_row_c11_mount.rs");
include!("full_row_w19a_mount.rs");
include!("full_row_w01b_mount.rs");
include!("full_row_w23_mount.rs");
include!("full_row_w15d_mount.rs");
include!("full_row_w15a_mount.rs");
include!("full_row_w10_mount.rs");
include!("full_row_w08_mount.rs");
include!("full_row_w24_mount.rs");
include!("full_row_w22a_mount.rs");
include!("full_row_w15b_mount.rs");
include!("full_row_w11_mount.rs");
include!("full_row_w05a_mount.rs");
include!("full_row_w03_mount.rs");
include!("full_row_w01a_mount.rs");
include!("full_row_s16b_mount.rs");
include!("full_row_s15b_mount.rs");
include!("full_row_s10_mount.rs");
include!("full_row_s07_mount.rs");
include!("full_row_s06_mount.rs");
include!("full_row_s05_mount.rs");
include!("full_row_p07_mount.rs");
include!("full_row_w17_mount.rs");
include!("full_row_w16_mount.rs");
include!("full_row_w13_mount.rs");
include!("full_row_w12_mount.rs");
include!("full_row_w05c_mount.rs");
include!("full_row_w04_mount.rs");
include!("full_row_w02_mount.rs");
include!("full_row_w01c_mount.rs");
include!("full_row_s13c_mount.rs");
include!("full_row_s04c_mount.rs");
include!("full_row_s04a_mount.rs");
include!("full_row_s02_mount.rs");
include!("full_row_p23c_mount.rs");
include!("full_row_p20_mount.rs");
include!("full_row_p18_mount.rs");
include!("full_row_p17_mount.rs");
include!("full_row_p15_mount.rs");
include!("full_row_p14_mount.rs");
include!("full_row_p13_mount.rs");
include!("full_row_p12c_mount.rs");
include!("full_row_p12a_mount.rs");
include!("full_row_p11_mount.rs");
include!("full_row_p09_mount.rs");
include!("full_row_p08_mount.rs");
include!("full_row_p06_mount.rs");
include!("full_row_p05_mount.rs");
include!("full_row_p04_mount.rs");
include!("full_row_p03_mount.rs");
include!("full_row_p02_mount.rs");
include!("full_row_p01_mount.rs");
include!("full_row_q02_mount.rs");
include!("full_row_q01_mount.rs");
include!("full_row_e10_mount.rs");
include!("full_row_e09_mount.rs");
include!("full_row_e08_mount.rs");
include!("full_row_e06_mount.rs");
include!("full_row_i11_mount.rs");
include!("full_row_i10_mount.rs");
include!("full_row_i05_mount.rs");
include!("full_row_m11b_mount.rs");
include!("full_row_m11a_mount.rs");
include!("full_row_m08_mount.rs");
include!("full_row_c18b_mount.rs");
include!("full_row_c17b_mount.rs");
include!("full_row_c10_mount.rs");
include!("full_row_c07_mount.rs");
include!("full_row_e12_mount.rs");
include!("full_row_i14_mount.rs");
include!("full_row_i03_mount.rs");
include!("full_row_m10_mount.rs");
include!("full_row_m09_mount.rs");
include!("full_row_m06_mount.rs");
include!("full_row_c15_mount.rs");
include!("full_row_c08_mount.rs");
include!("full_row_c19b_mount.rs");
include!("full_row_g01_mount.rs");
include!("full_row_g02_mount.rs");
include!("full_row_g03_mount.rs");
include!("full_row_g04_mount.rs");
include!("full_row_g05_mount.rs");
include!("full_row_g06_mount.rs");
include!("full_row_g07_mount.rs");
include!("full_row_g08_mount.rs");
include!("full_row_g09_mount.rs");
include!("full_row_g10_mount.rs");
include!("full_row_g11_mount.rs");
include!("full_row_g12_mount.rs");
include!("full_row_g13_mount.rs");
include!("full_row_g14_mount.rs");
include!("full_row_g15_mount.rs");
include!("full_row_g16_mount.rs");
include!("full_row_g17_mount.rs");
include!("full_row_g18_mount.rs");
include!("full_row_g19_mount.rs");
include!("full_row_g20_mount.rs");
include!("full_row_g21_mount.rs");
include!("full_row_g22_mount.rs");
include!("full_row_g23_mount.rs");
include!("full_row_g24_mount.rs");
include!("full_row_g25_mount.rs");
include!("full_row_g26_mount.rs");
include!("full_row_g27_mount.rs");
include!("full_row_g28_mount.rs");
include!("full_row_g29_mount.rs");
include!("full_row_g30_mount.rs");
include!("full_row_g31_mount.rs");
include!("full_row_g32_mount.rs");
include!("full_row_g33_mount.rs");
include!("full_row_g34_mount.rs");
include!("full_row_g35_mount.rs");
include!("full_row_g36_mount.rs");
include!("full_row_g37_mount.rs");
include!("full_row_g38_mount.rs");
include!("full_row_i17a_mount.rs");
include!("full_row_i17b_mount.rs");
include!("full_row_i17c_mount.rs");
include!("full_row_i17d_mount.rs");
include!("full_row_i18a_mount.rs");
include!("full_row_i18b_mount.rs");
include!("full_row_i18c_mount.rs");
include!("full_row_i19_mount.rs");
include!("full_row_p12b_mount.rs");
include!("full_row_p22a_mount.rs");
include!("full_row_p22b_mount.rs");
include!("full_row_p22c_mount.rs");
include!("full_row_p22d_mount.rs");
include!("full_row_s14a_mount.rs");
include!("full_row_s14b_mount.rs");
include!("full_row_s14c_mount.rs");
include!("full_row_s15a_mount.rs");
include!("full_row_s16c_mount.rs");
include!("full_row_s16d_mount.rs");
include!("full_row_s17a_mount.rs");
include!("full_row_s17b_mount.rs");
include!("full_row_s17c_mount.rs");
include!("full_row_s19_mount.rs");
include!("full_row_w05b_mount.rs");
include!("full_row_w05d_mount.rs");
include!("full_row_w06_mount.rs");
include!("full_row_w07_mount.rs");
include!("full_row_w09_mount.rs");
include!("full_row_w15c_mount.rs");
include!("full_row_w19b_mount.rs");
include!("full_row_w19c_mount.rs");
include!("full_row_w19d_mount.rs");
include!("full_row_w19e_mount.rs");
include!("full_row_w21_mount.rs");
include!("full_e2ef19_mount.rs");
include!("full_e2ef18_mount.rs");
include!("full_e2ef20_mount.rs");
include!("full_e2ef5_mount.rs");
#[path = "map_journey.rs"]
mod map_journey;
include!("native_content.rs");
#[path = "full_e2ef1.rs"]
pub(super) mod provisioning;
include!("quest_journal.rs");
#[path = "spatial_basis.rs"]
mod spatial_basis;
#[path = "starter_inventory.rs"]
mod starter_inventory;
// E2E-FLR2 owns this planner here; travel.rs remains unchanged reference code.
mod travel {
    //! FLOOR-E2E: multi-floor route planning over the production bundle, travelled by real play.
    //!
    //! Every planned edge is a command a player sends:
    //! - a STEP, resolved by the production floor resolver, so stairs, ramps, holes and teleports
    //!   change floor by stepping;
    //! - a STEP onto a closed door, after USE on that door (level and quest doors answer `Locked`
    //!   below their predicate, which ends the journey with that exact refusal);
    //! - USE on an immovable ladder, and USE of a carried rope on a rope spot. Both land one floor up
    //!   on the first free tile in the server's order (`crates/gameplay/world/src/map/floor/rope.rs`,
    //!   `OFFSETS`).
    //! - sewer 435 USE lands directly below; a shovel on 593 opens ground 594, then STEP
    //!   enters it; the Naginata stone requires six pick USEs before stepping into its hole.
    //!   Tools come only from the fixture inventory; missing tools are reported as MISSING.
    //!
    //! - an NPC ship or carpet passage, as a player buys it: walk next to the captain, say "hi", the
    //!   destination keyword and "yes" through the NPC dialogue protocol (`npc_shop::talk`), pay the
    //!   fare and arrive at the destination. Captains, keywords, fares and destinations come from the
    //!   bundle's NPC placements joined with `travel_routes.json`, the content the server answers from.
    //!   A passage is one planner edge priced `TRAVEL_COST` plus its fare. A refusal (no money, no
    //!   premium, no such route) ends the journey with the NPC's own line.
    //!
    //! A refused door USE (a level door below its level, a quest or key door) bans that door and
    //! replans; only when no other route remains does the journey fail, naming every refusal.
    //!
    //! Levers: no bundle fact maps a lever to the passage its quest callback opens, and the row's
    //! world copy never sees that callback, so a lever cannot be discovered by the search. A scenario
    //! that knows its lever from the quest source calls `pull_lever` with the tiles it opens.
    use super::*;
    use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef, SessionEvent};
    use oteryn_protocol_oteryn::npc_service as service;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashMap, HashSet};

    const DIRECTIONS: [StepDirection; 4] = [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ];
    /// `rope.rs` `OFFSETS`: south, north, east, west, then the diagonals.
    const OFFSETS: [(i16, i16); 8] = [
        (0, 1),
        (0, -1),
        (1, 0),
        (-1, 0),
        (-1, 1),
        (1, 1),
        (-1, -1),
        (1, -1),
    ];
    /// `rope.rs` `ROPE_GROUND` (canary data/global.lua:83-84) and its top-item rope spot 12935.
    const ROPE_GROUND: &[u16] = &[
        386, 421, 7762, 12202, 12936, 14238, 17238, 21965, 21966, 21967, 21968, 23363,
    ];
    const ROPE_TOP: u16 = 12935;
    const ROPE_ITEM: u32 = 3003;
    /// Tiles around the start-to-target box that a detour may use.
    const MARGIN: u16 = 160;
    /// The search gives up after this many settled tiles instead of exhausting memory.
    const SEARCH_LIMIT: usize = 2_000_000;
    pub(super) const MAX_REPLANS: usize = 12;
    const STEP_COST: u32 = 10;
    /// A closed door costs a USE and may be refused, so an open way round is preferred.
    const DOOR_COST: u32 = 40;
    const CLIMB_COST: u32 = 20;
    /// A passage costs the conversation and the wait, plus its fare in gold.
    const TRAVEL_COST: u32 = 300;
    /// Talk range of the planned conversation, in tiles (Chebyshev) from the captain's placement.
    const TALK_RANGE: i16 = 2;
    /// Offers further than this from the target are not allowed to widen the search box.
    const OFFER_REACH: u16 = 1500;

    /// One NPC passage: the captain's placement, what to say, what it costs and where it lands.
    #[derive(Debug, PartialEq, Eq)]
    pub(super) struct Offer {
        npc: String,
        at: TilePos,
        keyword: String,
        price: u64,
        premium: bool,
        dest: TilePos,
    }

    /// Every unheld route of `travel_routes.json` whose captain stands in the bundle. Leaked: the
    /// planner lives for the whole run.
    fn offers(npcs: &oteryn_world_bundle::npc::Table) -> TestResult<Vec<&'static Offer>> {
        let routes: Vec<serde_json::Value> =
            serde_json::from_str(crate::content::npc_catalogue::travel::revision_material())?;
        let mut found = Vec::new();
        for npc in &npcs.npcs {
            let prefix = format!(
                "oteryn:service.travel.{}/",
                npc.key.trim_start_matches("oteryn:npc.")
            );
            for route in routes
                .iter()
                .filter(|route| {
                    route["key"]
                        .as_str()
                        .is_some_and(|key| key.starts_with(&prefix))
                })
                .filter(|route| route["held"].as_bool() == Some(false))
            {
                let coordinate =
                    |name: &str| route[name].as_u64().and_then(|v| u16::try_from(v).ok());
                let (Some(keyword), Some(price), Some(x), Some(y), Some(z)) = (
                    route["keyword"].as_str(),
                    route["price"].as_u64(),
                    coordinate("x"),
                    coordinate("y"),
                    route["floor"].as_i64().and_then(|v| i8::try_from(v).ok()),
                ) else {
                    continue;
                };
                for place in &npc.placements {
                    found.push(&*Box::leak(Box::new(Offer {
                        npc: npc.key.clone(),
                        at: TilePos {
                            x: place.x,
                            y: place.y,
                            floor: place.floor,
                        },
                        keyword: keyword.to_ascii_lowercase(),
                        price,
                        premium: route["premium"].as_bool().unwrap_or(false),
                        dest: TilePos { x, y, floor: -z },
                    })));
                }
            }
        }
        Ok(found)
    }

    /// One player action of a route.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) enum Leg {
        Step(StepDirection),
        Ladder {
            at: TilePos,
            ordinal: u8,
        },
        Rope {
            at: TilePos,
            ordinal: u8,
        },
        Travel(&'static Offer),
        Descent {
            at: TilePos,
            ordinal: u8,
            tool: Option<u32>,
            hits: u8,
            direction: Option<StepDirection>,
        },
    }

    /// Passage state learned during play: tiles a pulled lever opened, doors whose USE was refused.
    #[derive(Default)]
    pub(super) struct Passages {
        pub(super) open: HashSet<TilePos>,
        pub(super) refused: HashSet<TilePos>,
    }

    /// What one route action did.
    enum Acted {
        At(TilePos),
        DoorRefused(TilePos, String),
    }

    pub(super) struct Planner<'a> {
        pub(super) world: &'a BundleWorld,
        /// Door tile -> (placement key, authored action id).
        doors: HashMap<TilePos, (u64, u16)>,
        ladders: HashMap<TilePos, u8>,
        ropes: HashMap<TilePos, u8>,
        descents: HashMap<TilePos, (u8, Option<u32>, u8, u8)>,
        offers: Vec<&'static Offer>,
        /// Captain placement -> the passages he sells.
        offer_at: HashMap<TilePos, Vec<&'static Offer>>,
        /// Authored `unique` attribute -> its tile and top-level ordinal.
        pub(super) uniques: HashMap<u16, (TilePos, u8)>,
    }

    fn descent_clear(
        world: &BundleWorld,
        at: TilePos,
        tile: &oteryn_world_bundle::sector::Tile,
        ordinal: u8,
    ) -> bool {
        // A closed shovel pile has no ground until USE installs ground 594.
        // Preserve all other blockers; only the tool's own entry is replaced.
        world.facts().house_id(at).is_none()
            && tile
                .items
                .iter()
                .filter(|item| item.depth == 0)
                .enumerate()
                .all(|(index, item)| {
                    index == usize::from(ordinal) || !world.facts().blocks(item.palette)
                })
    }

    impl<'a> Planner<'a> {
        /// Door facts only; `scan` adds the bundle's ladders, rope spots and unique ids.
        pub(super) fn new(world: &'a BundleWorld) -> Self {
            let mut doors = HashMap::new();
            for key in world.facts().doors().keys() {
                if let Some((at, _)) = crate::map::view::placement_of(key) {
                    doors.entry(at).or_insert((key, 0));
                }
            }
            Self {
                world,
                doors,
                ladders: HashMap::new(),
                ropes: HashMap::new(),
                descents: HashMap::new(),
                offers: Vec::new(),
                offer_at: HashMap::new(),
                uniques: HashMap::new(),
            }
        }

        pub(super) fn load(world: &'a BundleWorld, bytes: &[u8]) -> TestResult<Self> {
            let mut planner = Self::new(world);
            let Self {
                doors,
                ladders,
                ropes,
                descents,
                uniques,
                ..
            } = &mut planner;
            let visited = oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
                verdict::Harness::global().progress("travel catalogue");
                for tile in sector.tiles {
                    let at = TilePos {
                        x: tile.x,
                        y: tile.y,
                        floor: sector.floor,
                    };
                    for (ordinal, item) in
                        tile.items.iter().filter(|item| item.depth == 0).enumerate()
                    {
                        let Ok(ordinal) = u8::try_from(ordinal) else {
                            continue;
                        };
                        if let Some(unique) = item.attrs.unique {
                            uniques.entry(unique).or_insert((at, ordinal));
                        }
                        if let Some(door) = doors.get_mut(&at)
                            && crate::map::view::placement_key(at, ordinal) == Some(door.0)
                        {
                            door.1 = item.attrs.action.unwrap_or(0);
                        }
                        let entry = &manifest.palette[item.palette as usize];
                        let Some(id) = crate::map::palette_appearance(&entry.key) else {
                            continue;
                        };
                        // Crystal 201189f8 ladder_up.lua:5-10,21: 435 uses the
                        // straight-down sewer action. Only the confirmed dig targets are added.
                        if id == 435 && world.facts().house_id(at).is_none() {
                            descents.insert(at, (ordinal, None, 1, 0));
                        } else if id == 593
                            && item.attrs.action.is_none()
                            && descent_clear(world, at, &tile, ordinal)
                        {
                            let mask = crate::map::floor::palette_mask(
                                entry.family,
                                "oteryn:item.tibia.i594",
                            );
                            descents.insert(at, (ordinal, Some(3457), 1, mask));
                        } else if id == 1791
                            && item.attrs.action == Some(40031)
                            && descent_clear(world, at, &tile, ordinal)
                            && at
                                == (TilePos {
                                    x: 32356,
                                    y: 32074,
                                    floor: -10,
                                })
                        {
                            descents.insert(at, (ordinal, Some(3456), 6, 1));
                        }
                        if crate::map::floor::is_ladder_appearance(id) {
                            ladders.entry(at).or_insert(ordinal);
                        }
                        let ground = entry.terrain.is_some_and(|terrain| {
                            terrain.kind == oteryn_world_bundle::bundle::TerrainKind::Ground
                        });
                        if (ground && ROPE_GROUND.contains(&id)) || (!ground && id == ROPE_TOP) {
                            ropes.entry(at).or_insert(ordinal);
                        }
                    }
                }
                Ok(())
            })?;
            if visited.digest != world.base().digest() {
                return Err("travel planner and boot world have different bundle digests".into());
            }
            planner.offers = offers(&visited.npcs)?;
            for &offer in &planner.offers {
                planner.offer_at.entry(offer.at).or_default().push(offer);
            }
            Ok(planner)
        }

        fn walkable(&self, at: TilePos, through_doors: bool, passages: &Passages) -> bool {
            self.world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| tile.walkable())
                && (self.world.enterable(at)
                    || passages.open.contains(&at)
                    || (through_doors
                        && self.doors.contains_key(&at)
                        && !passages.refused.contains(&at)))
        }

        /// The production floor resolver, with every door tile enterable: the walker opens a closed
        /// door with USE before it steps, as a player does.
        pub(super) fn step(
            &self,
            from: TilePos,
            direction: StepDirection,
            passages: &Passages,
        ) -> Option<TilePos> {
            use crate::map::floor::{Ground, resolve_landing};
            use crate::movement::CardinalStep;
            let direction = match direction {
                StepDirection::North => CardinalStep::North,
                StepDirection::East => CardinalStep::East,
                StepDirection::South => CardinalStep::South,
                StepDirection::West => CardinalStep::West,
                StepDirection::NorthEast => CardinalStep::NorthEast,
                StepDirection::SouthEast => CardinalStep::SouthEast,
                StepDirection::SouthWest => CardinalStep::SouthWest,
                StepDirection::NorthWest => CardinalStep::NorthWest,
            };
            let position = |(x, y, z): (i32, i32, i32)| {
                Some(TilePos {
                    x: u16::try_from(x).ok()?,
                    y: u16::try_from(y).ok()?,
                    floor: -i8::try_from(z).ok().filter(|z| (0..=15).contains(z))?,
                })
            };
            let ground = |cell| match position(cell) {
                Some(pos) if self.walkable(pos, true, passages) => Ground::Walkable,
                Some(pos) if self.world.base().tile(pos.x, pos.y, pos.floor).is_some() => {
                    Ground::Blocked
                }
                _ => Ground::Absent,
            };
            let next = resolve_landing(
                self.world.floors(),
                ground,
                |_| true,
                (i32::from(from.x), i32::from(from.y), -i32::from(from.floor)),
                direction,
            )
            .ok()?;
            position(next)
        }

        /// Ladder and rope landing: one floor up, the first walkable tile in `OFFSETS` order.
        fn climb(&self, spot: TilePos, passages: &Passages) -> Option<TilePos> {
            first_landing(spot, |at| self.walkable(at, false, passages))
        }

        fn edges(
            &self,
            at: TilePos,
            rope: bool,
            passages: &Passages,
            out: &mut Vec<(Leg, TilePos, u32)>,
        ) {
            for direction in DIRECTIONS.into_iter().chain([
                StepDirection::NorthEast,
                StepDirection::SouthEast,
                StepDirection::SouthWest,
                StepDirection::NorthWest,
            ]) {
                if let Some(spot) = neighbour(at, direction)
                    && let Some(&(ordinal, tool, hits, mask)) = self.descents.get(&spot)
                    && tool.is_some()
                {
                    let mut floors = self.world.floors().clone();
                    floors.add(spot, mask);
                    if let Ok((x, y, z)) = crate::map::floor::resolve_landing(
                        &floors,
                        |(x, y, z)| {
                            let Some(pos) = u16::try_from(x)
                                .ok()
                                .zip(u16::try_from(y).ok())
                                .zip(i8::try_from(z).ok().filter(|z| (0..=15).contains(z)))
                                .map(|((x, y), z)| TilePos { x, y, floor: -z })
                            else {
                                return crate::map::floor::Ground::Absent;
                            };
                            if pos == spot || self.walkable(pos, false, passages) {
                                crate::map::floor::Ground::Walkable
                            } else if self.world.base().tile(pos.x, pos.y, pos.floor).is_some() {
                                crate::map::floor::Ground::Blocked
                            } else {
                                crate::map::floor::Ground::Absent
                            }
                        },
                        |_| true,
                        (i32::from(at.x), i32::from(at.y), -i32::from(at.floor)),
                        match direction {
                            StepDirection::North => crate::movement::CardinalStep::North,
                            StepDirection::East => crate::movement::CardinalStep::East,
                            StepDirection::South => crate::movement::CardinalStep::South,
                            StepDirection::West => crate::movement::CardinalStep::West,
                            StepDirection::NorthEast => crate::movement::CardinalStep::NorthEast,
                            StepDirection::SouthEast => crate::movement::CardinalStep::SouthEast,
                            StepDirection::SouthWest => crate::movement::CardinalStep::SouthWest,
                            StepDirection::NorthWest => crate::movement::CardinalStep::NorthWest,
                        },
                    ) && let (Ok(x), Ok(y), Ok(z)) =
                        (u16::try_from(x), u16::try_from(y), i8::try_from(z))
                        && -z < at.floor
                    {
                        out.push((
                            Leg::Descent {
                                at: spot,
                                ordinal,
                                tool,
                                hits,
                                direction: Some(direction),
                            },
                            TilePos { x, y, floor: -z },
                            CLIMB_COST * u32::from(hits),
                        ));
                    }
                }
                if let Some(next) = self.step(at, direction, passages) {
                    let closed = neighbour(at, direction).is_some_and(|entered| {
                        self.doors.contains_key(&entered) && !self.world.enterable(entered)
                    });
                    let cost = if closed { DOOR_COST } else { STEP_COST };
                    out.push((Leg::Step(direction), next, cost));
                }
            }
            if !self.offer_at.is_empty() {
                for dy in -TALK_RANGE..=TALK_RANGE {
                    for dx in -TALK_RANGE..=TALK_RANGE {
                        let (Some(x), Some(y)) =
                            (at.x.checked_add_signed(dx), at.y.checked_add_signed(dy))
                        else {
                            continue;
                        };
                        for &offer in self
                            .offer_at
                            .get(&TilePos { x, y, ..at })
                            .into_iter()
                            .flatten()
                        {
                            let fare = u32::try_from(offer.price).unwrap_or(u32::MAX / 2);
                            out.push((
                                Leg::Travel(offer),
                                offer.dest,
                                TRAVEL_COST.saturating_add(fare),
                            ));
                        }
                    }
                }
            }
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (Some(x), Some(y)) =
                        (at.x.checked_add_signed(dx), at.y.checked_add_signed(dy))
                    else {
                        continue;
                    };
                    let spot = TilePos { x, y, ..at };
                    if let Some(&(ordinal, None, hits, _)) = self.descents.get(&spot)
                        && let Some(floor) = spot.floor.checked_sub(1).filter(|floor| *floor >= -15)
                        && self.walkable(TilePos { floor, ..spot }, false, passages)
                    {
                        out.push((
                            Leg::Descent {
                                at: spot,
                                ordinal,
                                tool: None,
                                hits,
                                direction: None,
                            },
                            TilePos { floor, ..spot },
                            CLIMB_COST,
                        ));
                    }
                    if let Some(&ordinal) = self.ladders.get(&spot)
                        && let Some(next) = self.climb(spot, passages)
                    {
                        out.push((Leg::Ladder { at: spot, ordinal }, next, CLIMB_COST));
                    }
                    if rope
                        && let Some(&ordinal) = self.ropes.get(&spot)
                        && let Some(next) = self.climb(spot, passages)
                    {
                        out.push((Leg::Rope { at: spot, ordinal }, next, CLIMB_COST));
                    }
                }
            }
        }

        /// A route of player actions from `from` to a tile satisfying `goal`, searched inside the
        /// start-to-`target` box plus `MARGIN`. Weighted toward `target`, so not always the shortest.
        /// The error names why the search ended and the tile nearest `target` it reached.
        pub(super) fn plan(
            &self,
            from: TilePos,
            target: TilePos,
            goal: &dyn Fn(TilePos) -> bool,
            rope: bool,
            passages: &Passages,
        ) -> Result<Vec<(Leg, TilePos)>, String> {
            complete_search(self.plan_serviced(from, target, goal, rope, passages, async || Ok(())))
        }

        pub(super) async fn plan_serviced(
            &self,
            from: TilePos,
            target: TilePos,
            goal: &dyn Fn(TilePos) -> bool,
            rope: bool,
            passages: &Passages,
            service: impl AsyncFnMut() -> Result<(), String>,
        ) -> Result<Vec<(Leg, TilePos)>, String> {
            // The box also covers every captain (and his landing) near the target, so a passage
            // whose port lies outside the plain start-to-target box can still be walked to.
            let mut points = vec![from, target];
            for offer in &self.offers {
                let gap = |at: TilePos| at.x.abs_diff(target.x).max(at.y.abs_diff(target.y));
                if gap(offer.at) <= OFFER_REACH && gap(offer.dest) < gap(from) {
                    points.extend([offer.at, offer.dest]);
                }
            }
            let span = |value: fn(&TilePos) -> u16| {
                let low = points.iter().map(value).min().unwrap_or(0);
                let high = points.iter().map(value).max().unwrap_or(0);
                (low.saturating_sub(MARGIN), high.saturating_add(MARGIN))
            };
            let (low_x, high_x) = span(|at| at.x);
            let (low_y, high_y) = span(|at| at.y);
            search_serviced(
                from,
                goal,
                |at| {
                    10 * (u32::from(at.x.abs_diff(target.x))
                        + u32::from(at.y.abs_diff(target.y))
                        + u32::from(at.floor.abs_diff(target.floor)))
                },
                |at, out| {
                    self.edges(at, rope, passages, out);
                    out.retain(|(_, next, _)| {
                        (low_x..=high_x).contains(&next.x) && (low_y..=high_y).contains(&next.y)
                    });
                },
                SEARCH_LIMIT,
                service,
            )
            .await
        }

        fn door(&self, at: TilePos) -> Option<(u64, u16)> {
            self.doors.get(&at).copied()
        }

        /// Doors of one authored action (1000 + level for a level door) on `near`'s floor within `reach` tiles of it.
        pub(super) fn level_doors(&self, near: TilePos, reach: u16, wanted: u16) -> Vec<TilePos> {
            let mut doors: Vec<_> = self
                .doors
                .iter()
                .filter(|(at, (_, action))| {
                    *action == wanted
                        && at.floor == near.floor
                        && at.x.abs_diff(near.x).max(at.y.abs_diff(near.y)) <= reach
                })
                .map(|(at, _)| *at)
                .collect();
            doors.sort_by_key(|at| (at.x, at.y));
            doors
        }

        /// The door's placement bytes for a USE, as `use_map_object` takes them.
        pub(super) fn door_placement(&self, at: TilePos) -> TestResult<Vec<u8>> {
            let (key, _) = self
                .door(at)
                .ok_or_else(|| format!("no door at {}", show(at)))?;
            let mut placement = self.world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            Ok(placement)
        }

        pub(super) fn tile_evidence(&self, at: TilePos) -> String {
            use crate::map::view::MapFacts;
            let Some(tile) = self.world.base().tile(at.x, at.y, at.floor) else {
                return format!("{} absent", show(at));
            };
            let appearances: Vec<_> = tile
                .ids()
                .iter()
                .zip(tile.depths())
                .filter(|(_, depth)| **depth == 0)
                .enumerate()
                .map(|(ordinal, (id, _))| {
                    u8::try_from(ordinal)
                        .ok()
                        .and_then(|ordinal| self.world.facts().base_entry(at, ordinal, *id))
                        .map(|entry| entry.appearance_id)
                })
                .collect();
            format!(
                "{} items={appearances:?} ground_walkable={} enterable={} floor_change={}",
                show(at),
                tile.walkable(),
                self.world.enterable(at),
                self.world.floors().contains(at)
            )
        }

        /// Reward objects are used from a free adjacent tile on their own floor.
        pub(super) fn reward_goal(&self, target: TilePos) -> impl Fn(TilePos) -> bool + '_ {
            move |at| {
                at.floor == target.floor
                    && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) == 1
                    && self.world.enterable(at)
                    && self
                        .world
                        .base()
                        .tile(at.x, at.y, at.floor)
                        .is_some_and(|tile| tile.walkable())
            }
        }

        /// Evidence: every passage landing within `reach` tiles, with captain and fare.
        pub(super) fn offers_near(&self, target: TilePos, reach: u16) -> Vec<String> {
            let near = self
                .offers
                .iter()
                .filter(|offer| {
                    offer
                        .dest
                        .x
                        .abs_diff(target.x)
                        .max(offer.dest.y.abs_diff(target.y))
                        <= reach
                })
                .map(|offer| describe_offer(offer))
                .collect::<Vec<_>>();
            if near.is_empty() {
                vec![format!(
                    "no placed captain lands within {reach} tiles of {} ({} passages of placed captains in all)",
                    show(target),
                    self.offers.len()
                )]
            } else {
                near
            }
        }

        /// Evidence lines naming every walk run, door, floor change, ladder and rope of `route`.
        pub(super) fn describe(&self, from: TilePos, route: &[(Leg, TilePos)]) -> Vec<String> {
            describe(from, route, |at| self.door(at).map(|(_, action)| action))
        }

        pub(super) fn placement(&self, at: TilePos, ordinal: u8) -> TestResult<Vec<u8>> {
            let key = crate::map::view::placement_key(at, ordinal)
                .ok_or_else(|| format!("no placement key for {} #{ordinal}", show(at)))?;
            let mut placement = self.world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            Ok(placement)
        }
    }

    /// The first tile of `OFFSETS` around `spot`, one floor up, that `free` accepts.
    pub(super) fn first_landing(spot: TilePos, free: impl Fn(TilePos) -> bool) -> Option<TilePos> {
        let floor = spot.floor.checked_add(1).filter(|floor| *floor <= 0)?;
        OFFSETS.iter().find_map(|&(dx, dy)| {
            let at = TilePos {
                x: spot.x.checked_add_signed(dx)?,
                y: spot.y.checked_add_signed(dy)?,
                floor,
            };
            free(at).then_some(at)
        })
    }

    pub(super) fn missing_tool_error(tool: u32, context: &str) -> Box<dyn std::error::Error> {
        verdict::HarnessError::boxed(
            verdict::Cause::Precondition {
                name: format!("fixture-tool-{tool}"),
            },
            format!("{} {context}", missing_fixture_tool(tool)),
        )
    }

    // The Naginata owner requires object revision to equal the channel hit count.
    fn remaining_pick_hits(hits: u8, revision: u64) -> u8 {
        hits.saturating_sub(u8::try_from(revision).unwrap_or(u8::MAX))
    }

    fn missing_fixture_tool(tool: u32) -> String {
        if tool == 3456 {
            "fixture has no pick 3456".into()
        } else {
            format!("fixture inventory has no tool {tool}")
        }
    }

    /// Weighted A* over player actions, each with its own cost.
    #[cfg(test)]
    fn search<E: Copy>(
        from: TilePos,
        goal: &dyn Fn(TilePos) -> bool,
        estimate: impl Fn(TilePos) -> u32,
        edges: impl FnMut(TilePos, &mut Vec<(E, TilePos, u32)>),
        limit: usize,
    ) -> Result<Vec<(E, TilePos)>, String> {
        complete_search(search_serviced(
            from,
            goal,
            estimate,
            edges,
            limit,
            async || Ok(()),
        ))
    }

    fn complete_search<T>(
        future: impl std::future::Future<Output = Result<T, String>>,
    ) -> Result<T, String> {
        let mut future = std::pin::pin!(future);
        match future
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
        {
            std::task::Poll::Ready(result) => result,
            std::task::Poll::Pending => {
                Err("synchronous search unexpectedly waited for session IO".into())
            }
        }
    }

    async fn search_serviced<E: Copy>(
        from: TilePos,
        goal: &dyn Fn(TilePos) -> bool,
        estimate: impl Fn(TilePos) -> u32,
        mut edges: impl FnMut(TilePos, &mut Vec<(E, TilePos, u32)>),
        limit: usize,
        mut service: impl AsyncFnMut() -> Result<(), String>,
    ) -> Result<Vec<(E, TilePos)>, String> {
        let mut open = BinaryHeap::from([Reverse((estimate(from), 0u32, from))]);
        let mut costs = HashMap::from([(from, 0u32)]);
        let mut parents: HashMap<TilePos, (TilePos, E)> = HashMap::new();
        let mut buffer = Vec::new();
        let mut closest = (estimate(from), from);
        let mut settled = 0usize;
        #[cfg(test)]
        let mut floor_diagnostic = std::collections::BTreeMap::new();
        while let Some(Reverse((_, cost, at))) = open.pop() {
            if costs.get(&at) != Some(&cost) {
                continue;
            }
            settled += 1;
            #[cfg(test)]
            {
                let entry = floor_diagnostic
                    .entry(at.floor)
                    .or_insert((0usize, (estimate(at), at)));
                entry.0 += 1;
                entry.1 = entry.1.min((estimate(at), at));
            }
            if settled.is_multiple_of(1024) {
                verdict::Harness::global().progress("travel search");
                service().await?;
            }
            closest = closest.min((estimate(at), at));
            if goal(at) {
                let mut path = Vec::new();
                let mut cursor = at;
                while cursor != from {
                    let &(previous, edge) = parents
                        .get(&cursor)
                        .ok_or("route reconstruction lost a parent")?;
                    path.push((edge, cursor));
                    cursor = previous;
                }
                path.reverse();
                return Ok(path);
            }
            if costs.len() > limit {
                return Err(format!(
                    "search limit of {limit} tiles hit after {settled} settled; closest tile reached {}",
                    show(closest.1)
                ));
            }
            buffer.clear();
            edges(at, &mut buffer);
            for &(edge, next, step) in &buffer {
                let candidate = cost.saturating_add(step);
                if costs.get(&next).is_none_or(|&old| candidate < old) {
                    costs.insert(next, candidate);
                    parents.insert(next, (at, edge));
                    open.push(Reverse((
                        candidate.saturating_add(estimate(next)),
                        candidate,
                        next,
                    )));
                }
            }
        }
        #[cfg(test)]
        eprintln!("FLR2_SETTLED {floor_diagnostic:?}");
        Err(format!(
            "every reachable tile of the search box exhausted ({settled} settled); closest tile reached {}",
            show(closest.1)
        ))
    }

    #[cfg(test)]
    #[test]
    fn qualification_route_search_services_live_session_between_batches() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let start = TilePos {
                    x: 0,
                    y: 0,
                    floor: -7,
                };
                let mut serviced = 0;
                let route = search_serviced(
                    start,
                    &|at| at.x == 4096,
                    |_| 0,
                    |at, out| {
                        if at.x < 4096 {
                            out.push(((), TilePos { x: at.x + 1, ..at }, 1));
                        }
                    },
                    5000,
                    async || {
                        serviced += 1;
                        Ok(())
                    },
                )
                .await?;
                assert_eq!(route.len(), 4096);
                assert!(serviced >= 4, "long route search starved session liveness");
                Ok(())
            })
    }

    fn describe_offer(offer: &Offer) -> String {
        format!(
            "{}@{} says '{}' for {} gold{} -> {}",
            offer.npc,
            show(offer.at),
            offer.keyword,
            offer.price,
            if offer.premium { " (premium)" } else { "" },
            show(offer.dest)
        )
    }

    fn show(at: TilePos) -> String {
        format!("{},{},{}", at.x, at.y, -i16::from(at.floor))
    }

    fn neighbour(at: TilePos, direction: StepDirection) -> Option<TilePos> {
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

    fn describe(
        from: TilePos,
        route: &[(Leg, TilePos)],
        door: impl Fn(TilePos) -> Option<u16>,
    ) -> Vec<String> {
        let mut segments = Vec::new();
        let mut run = 0usize;
        let mut at = from;
        for &(leg, landed) in route {
            let segment = match leg {
                Leg::Step(direction) => {
                    let entered = neighbour(at, direction);
                    match entered.and_then(|entered| door(entered).map(|action| (entered, action)))
                    {
                        Some((entered, action)) => {
                            Some(format!("door@{}(action={action})", show(entered)))
                        }
                        None if landed.floor != at.floor => {
                            Some(format!("floor-step@{}->{}", show(at), show(landed)))
                        }
                        None => None,
                    }
                }
                Leg::Ladder { at: spot, .. } => {
                    Some(format!("ladder-USE@{}->{}", show(spot), show(landed)))
                }
                Leg::Rope { at: spot, .. } => {
                    Some(format!("rope-USE@{}->{}", show(spot), show(landed)))
                }
                Leg::Descent {
                    at: spot,
                    tool,
                    hits,
                    ..
                } => Some(format!(
                    "{}@{}(uses={hits})->{}",
                    match tool {
                        None => "sewer-down",
                        Some(3457) => "shovel",
                        _ => "pick",
                    },
                    show(spot),
                    show(landed)
                )),
                Leg::Travel(offer) => Some(format!(
                    "passage[cost={}]({})",
                    TRAVEL_COST.saturating_add(u32::try_from(offer.price).unwrap_or(0)),
                    describe_offer(offer)
                )),
            };
            match segment {
                Some(segment) => {
                    if run != 0 {
                        segments.push(format!("walk({run})->{}", show(at)));
                        run = 0;
                    }
                    segments.push(segment);
                }
                None => run += 1,
            }
            at = landed;
        }
        if run != 0 {
            segments.push(format!("walk({run})->{}", show(at)));
        }
        segments.chunks(24).map(|chunk| chunk.join(" ")).collect()
    }

    fn base_ordinal_revision(
        tiles: &[oteryn_protocol_oteryn::world_map::MapTile],
        spot: TilePos,
        ordinal: u8,
    ) -> Option<u64> {
        use oteryn_protocol_oteryn::world_map::MapOrigin;
        let position = ActorPosition {
            x: i32::from(spot.x),
            y: i32::from(spot.y),
            floor: i16::from(spot.floor),
        };
        tiles
            .iter()
            .find(|tile| tile.position == position)?
            .items
            .iter()
            .find_map(|item| match item.origin {
                MapOrigin::BaseOrdinal {
                    ordinal: actual,
                    object_revision,
                } if actual == ordinal => Some(object_revision),
                _ => None,
            })
    }

    const NAGINATA_PICK_HITS: u64 = 6;

    fn initial_pick_revision(revision: Option<u64>, complete: bool) -> Option<u64> {
        revision.or(complete.then_some(NAGINATA_PICK_HITS))
    }

    fn pick_observation(revision: Option<u64>, complete: bool, before: u64) -> Option<u64> {
        match revision {
            Some(revision) if revision > before => Some(revision),
            None if complete => Some(NAGINATA_PICK_HITS),
            _ => None,
        }
    }

    fn projected_pick_revision(
        tile: &oteryn_protocol_oteryn::world_map::MapTile,
        spot: TilePos,
        ordinal: u8,
        before: u64,
    ) -> Option<u64> {
        pick_observation(
            base_ordinal_revision(std::slice::from_ref(tile), spot, ordinal),
            !tile.more,
            before,
        )
    }

    fn tool_revision(events: &[SessionEvent], spot: TilePos, ordinal: u8) -> Option<u64> {
        events.iter().rev().find_map(|event| {
            let tiles = match event {
                SessionEvent::WorldMap(delta) => &delta.value.tiles,
                SessionEvent::WorldMapSnapshot(snapshot) => &snapshot.value.tiles,
                _ => return None,
            };
            base_ordinal_revision(tiles, spot, ordinal)
        })
    }

    pub(super) fn observed_revision(
        client: &mut DevClientSession,
        placement: &[u8],
        at: TilePos,
        ordinal: u8,
    ) -> TestResult<u64> {
        let position = ActorPosition {
            x: i32::from(at.x),
            y: i32::from(at.y),
            floor: i16::from(at.floor),
        };
        let view_revision = client
            .session_mut()
            .world_map_tile(position)
            .and_then(|tile| base_ordinal_revision(&[tile], at, ordinal));
        let overlay = client
            .world_object_overlay()
            .iter()
            .find(|entry| entry.placement == placement);
        view_revision
            .into_iter()
            .chain(overlay.map(|entry| entry.revision))
            .max()
            .ok_or_else(|| {
                format!("no current map revision for {} ordinal {ordinal}", show(at)).into()
            })
    }

    fn overlay_revision(client: &DevClientSession, placement: &[u8]) -> u64 {
        client
            .world_object_overlay()
            .iter()
            .find(|entry| entry.placement == placement)
            .map_or(0, |entry| entry.revision)
    }

    /// USE on a map object at the revision this client last saw, as a player's click sends it.
    /// An `Exhausted` answer (the stair-hop cooldown) is retried a bounded number of times.
    pub(super) async fn use_map_object(
        client: &mut DevClientSession,
        placement: &[u8],
    ) -> TestResult<UseDisposition> {
        let mut disposition = UseDisposition::Exhausted;
        for _ in 0..10 {
            let revision = overlay_revision(client, placement);
            disposition = client.use_object(placement, revision).await?.disposition;
            if disposition != UseDisposition::Exhausted {
                break;
            }
            verdict::Harness::global().progress("travel wait");
            client.service_liveness(Duration::from_millis(300)).await?;
            client.take_events();
        }
        Ok(disposition)
    }

    /// One STEP with the town walker's bounded retry of a stationary `Rejected` (pacing).
    async fn paced_step(
        client: &mut DevClientSession,
        direction: StepDirection,
    ) -> TestResult<(StepDisposition, String)> {
        let before = client.world_spatial().actor_position;
        let mut outcome = client.step_retrying(direction).await?;
        for attempt in 1..=6 {
            let Some(delay) = town_route_retry_delay(
                outcome.disposition,
                client.world_spatial().actor_position == before,
                attempt,
            ) else {
                break;
            };
            verdict::Harness::global().progress("travel wait");
            tokio::time::sleep(delay).await;
            outcome = client.step_retrying(direction).await?;
        }
        // A `Moved` reply without a published position change is checked by the caller's landing.
        Ok((outcome.disposition, format!("{outcome:?}")))
    }

    impl Planner<'_> {
        async fn execute(
            &self,
            client: &mut DevClientSession,
            label: &str,
            at: TilePos,
            leg: Leg,
        ) -> TestResult<Acted> {
            match leg {
                Leg::Step(direction) => {
                    let (mut disposition, mut detail) = paced_step(client, direction).await?;
                    let door = neighbour(at, direction)
                        .and_then(|entered| self.door(entered).map(|door| (entered, door)));
                    if disposition == StepDisposition::Blocked
                        && let Some((entered, (key, action))) = door
                    {
                        let mut placement = self.world.base().digest().to_vec();
                        placement.extend_from_slice(&key.to_be_bytes());
                        let reply = use_map_object(client, &placement).await?;
                        evidence(&format!(
                            "TRAVEL_DOOR {label} door={} action={action} use={reply:?}",
                            show(entered)
                        ));
                        if reply != UseDisposition::Committed {
                            let level = if action > 1000 {
                                format!(" (level door, requires level {})", action - 1000)
                            } else {
                                String::new()
                            };
                            return Ok(Acted::DoorRefused(
                                entered,
                                format!(
                                    "door {} action={action} USE refused: {reply:?}{level}",
                                    show(entered)
                                ),
                            ));
                        }
                        (disposition, detail) = paced_step(client, direction).await?;
                    }
                    if disposition != StepDisposition::Moved {
                        return Err(format!(
                            "{label} STEP {direction:?} from {} refused: {detail}",
                            show(at)
                        )
                        .into());
                    }
                }
                Leg::Travel(offer) => travel_by_npc(client, label, offer).await?,
                Leg::Descent {
                    at: spot,
                    ordinal,
                    tool,
                    hits,
                    direction,
                } => {
                    let open_pick = tool == Some(3456) && {
                        let position = ActorPosition {
                            x: i32::from(spot.x),
                            y: i32::from(spot.y),
                            floor: i16::from(spot.floor),
                        };
                        client
                            .session_mut()
                            .world_map_tile(position)
                            .is_some_and(|tile| {
                                let complete = !tile.more;
                                initial_pick_revision(
                                    base_ordinal_revision(&[tile], spot, ordinal),
                                    complete,
                                ) == Some(NAGINATA_PICK_HITS)
                            })
                    };
                    let mut revision = if open_pick {
                        NAGINATA_PICK_HITS
                    } else if tool.is_some() {
                        observed_revision(client, &self.placement(spot, ordinal)?, spot, ordinal)?
                    } else {
                        0
                    };
                    let hits = if tool == Some(3456) {
                        remaining_pick_hits(hits, revision)
                    } else {
                        hits
                    };
                    for hit in 1..=hits {
                        verdict::Harness::global().progress("travel descent");
                        let reply = if let Some(tool) = tool {
                            self.use_tool(
                                client,
                                spot,
                                ordinal,
                                tool,
                                &mut revision,
                                tool == 3456 || hit < hits,
                            )
                            .await?
                        } else {
                            use_map_object(client, &self.placement(spot, ordinal)?).await?
                        };
                        evidence(&format!(
                            "TRAVEL_DESCENT {label} at={} tool={tool:?} hit={hit}/{hits} use={reply:?}",
                            show(spot)
                        ));
                        if tool.is_some()
                            && matches!(
                                reply,
                                UseDisposition::NothingToUse | UseDisposition::StaleState
                            )
                        {
                            // The subsequent STEP and published lower-floor landing must prove
                            // that a preceding player already opened this exact passage.
                            break;
                        }
                        if reply != UseDisposition::Committed {
                            return Err(format!(
                                "{label} descent USE at {} refused: {reply:?}",
                                show(spot)
                            )
                            .into());
                        }
                        if tool == Some(3456) && revision >= NAGINATA_PICK_HITS {
                            break;
                        }
                    }
                    if let Some(direction) = direction {
                        let (reply, detail) = paced_step(client, direction).await?;
                        if reply != StepDisposition::Moved {
                            return Err(format!(
                                "{label} opened hole STEP at {} refused: {detail}",
                                show(spot)
                            )
                            .into());
                        }
                    }
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                    while tile(client.world_spatial().actor_position)?.floor >= at.floor {
                        verdict::Harness::global().progress("travel descent landing");
                        if tokio::time::Instant::now() >= deadline {
                            return Err(format!(
                                "{label} descent at {} committed but no lower floor was published",
                                show(spot)
                            )
                            .into());
                        }
                        verdict::Harness::global().progress("travel wait");
                        client.service_liveness(Duration::from_millis(50)).await?;
                        client.take_events();
                    }
                }
                Leg::Ladder { at: spot, ordinal } | Leg::Rope { at: spot, ordinal } => {
                    let kind = if matches!(leg, Leg::Ladder { .. }) {
                        "ladder"
                    } else {
                        "rope"
                    };
                    let reply = if kind == "ladder" {
                        use_map_object(client, &self.placement(spot, ordinal)?).await?
                    } else {
                        let mut revision = observed_revision(
                            client,
                            &self.placement(spot, ordinal)?,
                            spot,
                            ordinal,
                        )?;
                        self.use_tool(client, spot, ordinal, ROPE_ITEM, &mut revision, false)
                            .await?
                    };
                    evidence(&format!(
                        "TRAVEL_CLIMB {label} {kind}={} use={reply:?}",
                        show(spot)
                    ));
                    if reply != UseDisposition::Committed {
                        return Err(format!(
                            "{label} {kind} USE at {} from {} refused: {reply:?}",
                            show(spot),
                            show(at)
                        )
                        .into());
                    }
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                    loop {
                        let landed = tile(client.world_spatial().actor_position)?;
                        if landed.floor == spot.floor + 1
                            && landed.x.abs_diff(spot.x).max(landed.y.abs_diff(spot.y)) <= 1
                        {
                            break;
                        }
                        if tokio::time::Instant::now() >= deadline {
                            return Err(format!(
                            "{label} {kind} USE at {} committed but no landing one floor up was published (at {})",
                            show(spot),
                            show(landed)
                        )
                        .into());
                        }
                        verdict::Harness::global().progress("travel wait");
                        client.service_liveness(Duration::from_millis(50)).await?;
                        client.take_events();
                    }
                }
            }
            client.take_events();
            verdict::Harness::global().progress("walk");
            Ok(Acted::At(tile(client.world_spatial().actor_position)?))
        }

        async fn use_tool(
            &self,
            client: &mut DevClientSession,
            spot: TilePos,
            ordinal: u8,
            tool: u32,
            revision: &mut u64,
            needs_revision: bool,
        ) -> TestResult<UseDisposition> {
            use oteryn_protocol_oteryn::world_object::*;
            let reference = provisioning::item_reference(&provisioning::item_index()?, tool)?;
            let placement = self.placement(spot, ordinal)?;
            for _ in 0..10 {
                let source = client
                    .inventory()
                    .and_then(|inventory| {
                        inventory
                            .entries
                            .iter()
                            .find(|entry| entry.item_definition_ref == reference)
                    })
                    .ok_or_else(|| {
                        missing_tool_error(tool, &format!("required at {}", show(spot)))
                    })?
                    .handle;
                let intent = UseIntent {
                    target: UseTarget::Item(source),
                    use_with: None,
                    use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                        expected_revision: *revision,
                        placement: placement.clone(),
                    })),
                    use_with_position: None,
                };
                let payload = encode_item_use_intent(&intent)
                    .map_err(|error| format!("tool {tool} USE encode: {error:?}"))?;
                client.take_events();
                let response = client.session_mut().world_context_use(&payload).await?;
                if response.status != CommandStatus::Accepted {
                    return Err(format!("tool {tool} USE command refused: {response:?}").into());
                }
                let disposition = decode_use_result(&response.payload)
                    .map_err(|error| format!("tool {tool} USE result: {error:?}"))?;
                let pick = tool == 3456;
                if pick
                    && disposition == UseDisposition::Committed
                    && *revision + 1 == NAGINATA_PICK_HITS
                {
                    *revision = NAGINATA_PICK_HITS;
                    return Ok(disposition);
                }
                if (disposition == UseDisposition::Committed && needs_revision)
                    || (pick && disposition == UseDisposition::StaleState)
                {
                    let before = *revision;
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                    loop {
                        verdict::Harness::global().progress("travel tool map update");
                        let events = client.take_events();
                        let observed = if pick {
                            let position = ActorPosition {
                                x: i32::from(spot.x),
                                y: i32::from(spot.y),
                                floor: i16::from(spot.floor),
                            };
                            client
                                .session_mut()
                                .world_map_tile(position)
                                .and_then(|tile| {
                                    projected_pick_revision(&tile, spot, ordinal, before)
                                })
                        } else {
                            tool_revision(&events, spot, ordinal)
                                .filter(|&revision| revision > before)
                        };
                        if let Some(observed) = observed {
                            *revision = observed;
                            if pick && observed >= NAGINATA_PICK_HITS {
                                return Ok(UseDisposition::NothingToUse);
                            }
                            break;
                        }
                        if tokio::time::Instant::now() >= deadline {
                            return Err(format!(
                                "tool {tool} USE at {} published no usable map update",
                                show(spot)
                            )
                            .into());
                        }
                        client.service_liveness(Duration::from_millis(50)).await?;
                    }
                    if disposition == UseDisposition::StaleState {
                        continue;
                    }
                    return Ok(disposition);
                }
                if disposition != UseDisposition::Exhausted {
                    return Ok(disposition);
                }
                verdict::Harness::global().progress("travel tool cooldown");
                verdict::Harness::global().progress("travel wait");
                client.service_liveness(Duration::from_millis(300)).await?;
                client.take_events();
            }
            Ok(UseDisposition::Exhausted)
        }
    }

    fn conversation_lines(
        delta: oteryn_dev_client::AppliedDelta<service::NpcConversation>,
        after: u64,
        npc: EntityRef,
        paid_confirmation: bool,
    ) -> Option<Vec<String>> {
        let named_reply = delta.value.open.is_some_and(|(actor, _)| actor == npc);
        // A successful paid passage closes the focus in the same publication.
        // Other anonymous closes do not carry a correlatable reply.
        let passage_committed = paid_confirmation
            && delta.value.open.is_none()
            && delta.value.lines == ["Set the sails!"];
        (delta.server_sequence > after && (named_reply || passage_committed))
            .then_some(delta.value.lines)
    }

    /// One line to the NPC through the production dialogue protocol, and the lines it answers.
    pub(super) async fn converse(
        client: &mut DevClientSession,
        npc: EntityRef,
        text: &str,
    ) -> TestResult<Vec<String>> {
        converse_with(client, npc, text, false).await
    }

    async fn converse_with(
        client: &mut DevClientSession,
        npc: EntityRef,
        text: &str,
        paid_confirmation: bool,
    ) -> TestResult<Vec<String>> {
        client.take_events();
        let payload = service::encode_npc_talk_intent(&service::NpcTalkIntent {
            npc_actor: npc,
            text: text.into(),
        })
        .map_err(|error| format!("NPC talk '{text}' encode: {error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(
                service::CAPABILITY_NPC_SERVICE_V1,
                service::COMMAND_TYPE_NPC_TALK_INTENT,
                &payload,
            )
            .await?;
        let disposition = service::decode_npc_intent_result(&reply.payload)
            .map_err(|error| format!("NPC talk '{text}' reply: {error:?}"))?;
        if reply.status != CommandStatus::Accepted
            || disposition != service::NpcIntentDisposition::Ok
        {
            return Err(format!(
                "NPC talk '{text}' status={:?} disposition={disposition:?}",
                reply.status
            )
            .into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
        loop {
            for event in client.take_events() {
                if let SessionEvent::NpcConversation(delta) = event
                    && let Some(lines) =
                        conversation_lines(delta, reply.server_sequence, npc, paid_confirmation)
                {
                    return Ok(lines);
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("NPC talk '{text}' got no conversation delta").into());
            }
            verdict::Harness::global().progress("travel wait");
            client.service_liveness(Duration::from_millis(25)).await?;
        }
    }

    /// Buy a passage as a player does: say "hi", the destination and "yes" to the captain next to
    /// whom the route stopped, then wait for the arrival at the destination port.
    async fn travel_by_npc(
        client: &mut DevClientSession,
        label: &str,
        offer: &Offer,
    ) -> TestResult {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let npc = loop {
            let found = client.world_entities().and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == EntityKind::Npc
                        && entity.position.x == i32::from(offer.at.x)
                        && entity.position.y == i32::from(offer.at.y)
                        && entity.position.floor == -i16::from(offer.at.floor)
                })
            });
            if let Some(entity) = found {
                break entity.entity;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "{label} captain {} is not visible at {}",
                    offer.npc,
                    show(offer.at)
                )
                .into());
            }
            verdict::Harness::global().progress("travel wait");
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        };
        let need = || {
            format!(
                "{label} passage {} needs {} gold{}; the node-boot fixture mints axe, rope and shovel only (qualification_fixture.rs prepare_items)",
                describe_offer(offer),
                offer.price,
                if offer.premium {
                    " and a premium account"
                } else {
                    ""
                }
            )
        };
        let mut quote = Vec::new();
        for text in ["hi", offer.keyword.as_str()] {
            quote = converse(client, npc, text).await?;
            evidence(&format!("TRAVEL_NPC {label} say={text:?} reply={quote:?}"));
        }
        if !quote.iter().any(|line| line.contains("passage")) {
            return Err(format!("{} refused: {quote:?}", need()).into());
        }
        let reply = converse_with(client, npc, "yes", true).await?;
        evidence(&format!("TRAVEL_NPC {label} say=\"yes\" reply={reply:?}"));
        if reply.is_empty() || reply.iter().any(|line| !line.contains("sails")) {
            return Err(format!("{} refused: {reply:?}", need()).into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        loop {
            let here = tile(client.world_spatial().actor_position)?;
            if here.floor == offer.dest.floor
                && here
                    .x
                    .abs_diff(offer.dest.x)
                    .max(here.y.abs_diff(offer.dest.y))
                    <= 4
            {
                evidence(&format!(
                    "TRAVEL_ARRIVED {label} {} landed={}",
                    describe_offer(offer),
                    show(here)
                ));
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "{label} paid passage {} never landed at the destination (at {})",
                    describe_offer(offer),
                    show(here)
                )
                .into());
            }
            verdict::Harness::global().progress("travel wait");
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        }
    }

    fn carries(client: &DevClientSession, item: u32) -> TestResult<bool> {
        let reference = provisioning::item_reference(&provisioning::item_index()?, item)?;
        Ok(client.inventory().is_some_and(|inventory| {
            inventory
                .entries
                .iter()
                .any(|entry| entry.item_definition_ref == reference)
        }))
    }

    /// Reach a tile satisfying `goal` near `target` by real STEP and USE commands only. The route is
    /// printed before it is walked. A divergent landing (an occupied ladder landing) replans from
    /// where the server put the character; a refused door is banned and the route replanned.
    pub(super) async fn travel_to(
        client: &mut DevClientSession,
        planner: &Planner<'_>,
        label: &str,
        target: TilePos,
        goal: &dyn Fn(TilePos) -> bool,
        passages: &mut Passages,
    ) -> TestResult {
        let rope = carries(client, ROPE_ITEM)?;
        let mut refusals = Vec::new();
        for attempt in 0..=MAX_REPLANS {
            let from = tile(client.world_spatial().actor_position)?;
            if goal(from) {
                return Ok(());
            }
            verdict::Harness::global().progress("walk");
            client.take_events();
            let mut service_error = None;
            let planned = planner
                .plan_serviced(from, target, goal, rope, passages, async || {
                    verdict::Harness::global().progress("travel session liveness");
                    match client.service_liveness(Duration::from_millis(25)).await {
                        Ok(()) => {
                            client.take_events();
                            Ok(())
                        }
                        Err(error) => {
                            let reason = error.to_string();
                            service_error = Some(error);
                            Err(reason)
                        }
                    }
                })
                .await;
            if let Some(error) = service_error {
                return Err(error.into());
            }
            let observed = tile(client.world_spatial().actor_position)?;
            if observed != from {
                evidence(&format!(
                    "TRAVEL_REPLAN {label} origin={} observed={}",
                    show(from),
                    show(observed)
                ));
                continue;
            }
            let route = planned.map_err(|reason| {
                format!(
                    "{label} no route by step, door, stairs, hole, ladder{} from {} to {}: {reason}; refused on the way: [{}]",
                    if rope { " or rope" } else { "" },
                    show(from),
                    show(target),
                    refusals.join("; ")
                )
            })?;
            verdict::Harness::global().progress("walk");
            evidence(&format!(
                "TRAVEL_ROUTE {label} attempt={attempt} from={} to={} actions={}",
                show(from),
                show(target),
                route.len()
            ));
            for line in planner.describe(from, &route) {
                evidence(&format!("TRAVEL_ROUTE {label} {line}"));
            }
            for &(leg, _) in &route {
                if let Leg::Descent {
                    at: spot,
                    tool: Some(tool),
                    ..
                } = leg
                    && !carries(client, tool)?
                {
                    return Err(missing_tool_error(
                        tool,
                        &format!("required for descent at {}", show(spot)),
                    ));
                }
            }
            let mut at = from;
            let mut diverged = false;
            for &(leg, expected) in &route {
                match planner.execute(client, label, at, leg).await? {
                    Acted::At(landed) if landed == expected => at = landed,
                    Acted::At(landed) => {
                        evidence(&format!(
                            "TRAVEL_REPLAN {label} leg={leg:?} expected={} landed={}",
                            show(expected),
                            show(landed)
                        ));
                        diverged = true;
                        break;
                    }
                    Acted::DoorRefused(door, reason) => {
                        evidence(&format!("TRAVEL_REPLAN {label} {reason}"));
                        refusals.push(reason);
                        passages.refused.insert(door);
                        diverged = true;
                        break;
                    }
                }
            }
            if !diverged {
                return Ok(());
            }
        }
        Err(format!(
            "{label} route still diverged after {MAX_REPLANS} replans; refused on the way: [{}]",
            refusals.join("; ")
        )
        .into())
    }

    /// Walk next to a lever the quest source names, pull it with USE and treat `opens` as passable
    /// from then on, as a player who saw the wall move would.
    #[allow(
        dead_code,
        reason = "scenario-facing: no current Q01 chest is behind a lever; quest rows with one call it"
    )]
    pub(super) async fn pull_lever(
        client: &mut DevClientSession,
        planner: &Planner<'_>,
        label: &str,
        (lever, ordinal): (TilePos, u8),
        opens: &[TilePos],
        passages: &mut Passages,
    ) -> TestResult {
        travel_to(client, planner, label, lever, &beside(lever), passages).await?;
        let reply = use_map_object(client, &planner.placement(lever, ordinal)?).await?;
        evidence(&format!(
            "TRAVEL_LEVER {label} lever={} use={reply:?}",
            show(lever)
        ));
        if reply != UseDisposition::Committed {
            return Err(format!("{label} lever USE at {} refused: {reply:?}", show(lever)).into());
        }
        passages.open.extend(opens.iter().copied());
        Ok(())
    }

    /// Next to `at` on its floor (Chebyshev distance at most one), where USE reaches an object.
    pub(super) fn beside(at: TilePos) -> impl Fn(TilePos) -> bool {
        move |here| here.floor == at.floor && here.x.abs_diff(at.x).max(here.y.abs_diff(at.y)) <= 1
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn qualification_dialogue_ignores_other_npcs_and_anonymous_closes() {
            let npc = EntityRef {
                identity: [1; 16],
                generation: 1,
            };
            let other = EntityRef {
                identity: [2; 16],
                generation: 1,
            };
            let delta = |server_sequence, actor: Option<EntityRef>, line: &str| {
                oteryn_dev_client::AppliedDelta {
                    server_sequence,
                    base_revision: 1,
                    new_revision: 2,
                    value: service::NpcConversation {
                        open: actor.map(|id| (id, service::NpcConversationState::Open)),
                        lines: vec![line.into()],
                    },
                }
            };
            assert_eq!(
                conversation_lines(delta(8, Some(other), "wrong"), 7, npc, false),
                None
            );
            assert_eq!(
                conversation_lines(delta(8, None, "farewell"), 7, npc, true),
                None
            );
            assert_eq!(
                conversation_lines(delta(7, Some(npc), "stale"), 7, npc, false),
                None
            );
            assert_eq!(
                conversation_lines(delta(8, Some(npc), "hello"), 7, npc, false),
                Some(vec!["hello".into()])
            );
            assert_eq!(
                conversation_lines(delta(8, None, "Set the sails!"), 7, npc, true),
                Some(vec!["Set the sails!".into()])
            );
        }

        #[test]
        fn qualification_tool_revision_requires_matching_ordinal() {
            use oteryn_protocol_oteryn::world_map::*;
            let spot = TilePos {
                x: 100,
                y: 100,
                floor: -8,
            };
            let position = ActorPosition {
                x: 100,
                y: 100,
                floor: -8,
            };
            let event = |ordinal, object_revision| {
                SessionEvent::WorldMapSnapshot(oteryn_dev_client::AppliedDelta {
                    server_sequence: 2,
                    base_revision: 1,
                    new_revision: 2,
                    value: WorldMapViewSnapshot {
                        header: MapViewHeader {
                            content_generation: [0; 32],
                            bundle_digest: [0; 32],
                            reset_epoch: 1,
                            origin: position,
                            first_visible_floor: -8,
                        },
                        tiles: vec![MapTile {
                            position,
                            items: vec![MapItem {
                                definition: MapDefinition::Terrain(std::num::NonZeroU32::MIN),
                                count: 1,
                                sub_type: 0,
                                appearance_id: 0,
                                origin: MapOrigin::BaseOrdinal {
                                    ordinal,
                                    object_revision,
                                },
                            }],
                            more: false,
                            ground_speed: 150,
                        }],
                    },
                })
            };
            assert_eq!(tool_revision(&[event(2, 9)], spot, 1), None);
            assert_eq!(tool_revision(&[event(1, 4), event(2, 9)], spot, 1), Some(4));
        }

        #[test]
        fn qualification_partial_pick_remaining_hits() {
            assert_eq!(remaining_pick_hits(6, 0), 6);
            assert_eq!(remaining_pick_hits(6, 1), 5);
            assert_eq!(remaining_pick_hits(6, 5), 1);
            assert_eq!(remaining_pick_hits(6, 6), 0);
        }

        #[test]
        fn shared_pick_missing_ordinal_can_mark_open_passage() {
            assert_eq!(pick_observation(None, false, 4), None);
            assert_eq!(pick_observation(Some(4), true, 4), None);
            assert_eq!(pick_observation(Some(5), true, 4), Some(5));
            assert_eq!(pick_observation(None, true, 4), Some(6));
        }

        #[test]
        fn qualification_initial_pick_accepts_only_complete_open_projection() {
            assert_eq!(initial_pick_revision(None, false), None);
            assert_eq!(initial_pick_revision(Some(4), true), Some(4));
            assert_eq!(initial_pick_revision(None, true), Some(6));
        }

        include!("travel_tests.rs");
    }
}
#[path = "verdict.rs"]
pub(crate) mod verdict;

pub(crate) const TOWN: TilePos = TilePos {
    x: 32369,
    y: 32238,
    floor: -7,
};

/// One production world per test process: every journey shares it, so the
/// ~110 s bundle boot runs once instead of once per row.
pub(crate) fn load_world(
    data: Vec<u8>,
    pin: &serde_json::Value,
    world: WorldId,
    channel: ChannelId,
) -> TestResult<&'static BundleWorld> {
    static LOADED: std::sync::OnceLock<(WorldId, ChannelId, BundleWorld)> =
        std::sync::OnceLock::new();
    if LOADED.get().is_none() {
        let booted = boot_world(data, pin, world, channel)?;
        let _ = LOADED.set((world, channel, booted));
    }
    match LOADED.get() {
        Some((loaded_world, loaded_channel, booted))
            if *loaded_world == world && *loaded_channel == channel =>
        {
            Ok(booted)
        }
        Some(_) => Err("world loader asked for a different world or channel".into()),
        None => Err("world loader cache empty".into()),
    }
}

fn boot_world(
    data: Vec<u8>,
    pin: &serde_json::Value,
    world: WorldId,
    channel: ChannelId,
) -> TestResult<BundleWorld> {
    let digest = hex32(pin["digest"].as_str().ok_or("World digest")?)?;
    let checked = crate::map::boot::check(
        data,
        crate::map::boot::BootPins {
            bundle: crate::map::BundlePins {
                digest,
                project_format_version: pin["project_format_version"]
                    .as_str()
                    .ok_or("World project")?
                    .into(),
                world_schema_version: pin["world_schema_version"]
                    .as_str()
                    .ok_or("World schema")?
                    .into(),
                content_revision: pin["content_revision"]
                    .as_str()
                    .ok_or("World content")?
                    .into(),
                production: false,
            },
            map_revision: format!("sha256:{}", pin["digest"].as_str().ok_or("World digest")?),
            start: TOWN,
        },
    )?;
    let input = native_input()?;
    // Prewarm before the world pass admits its first client.
    native_spell_book()?;
    let room = crate::content::qualify_selected_native_gameplay_room(world, input)?;
    let issuance = crate::content::NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = crate::content::ContentActivationController::new();
    crate::content::activate_native_entry_room_with_gameplay(
        &mut controller,
        &crate::content::NodeBootQuiescence::before_channel_runtime(),
        world,
        &issuance,
        input,
    )?;
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("world has no native generation")?;
    let rat = native
        .reward_table()
        .row("oteryn:creature.rat")
        .map_err(|reason| format!("rat reward unavailable: {reason:?}"))?;
    if rat.loot_table.entries.is_empty() {
        return Err("town hunt has no production loot table".into());
    }
    crate::node::serve::boot_bundle_world(
        checked,
        world,
        channel,
        controller
            .active()
            .and_then(|active| active.native_gameplay()),
    )
    .map_err(|error| format!("production world loader: {error:?}").into())
}

fn tile(position: ActorPosition) -> TestResult<TilePos> {
    let basis = if position.floor < 0 {
        spatial_basis::Basis::Native
    } else {
        spatial_basis::Basis::Legacy
    };
    served_tile(position, basis)
}

fn actor_tile(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<TilePos> {
    let position = client.world_spatial().actor_position;
    served_tile(position, spatial_basis::current(client)?)
}

fn served_tile(position: ActorPosition, basis: spatial_basis::Basis) -> TestResult<TilePos> {
    let floor = match basis {
        spatial_basis::Basis::Native => position.floor,
        spatial_basis::Basis::Legacy => position
            .floor
            .checked_neg()
            .ok_or("legacy floor overflow")?,
    };
    if !(-15..=0).contains(&floor) {
        return Err("position outside the serving spatial basis".into());
    }
    Ok(TilePos {
        x: u16::try_from(position.x)?,
        y: u16::try_from(position.y)?,
        floor: i8::try_from(floor)?,
    })
}

#[cfg(test)]
#[test]
fn qualification_route_origin_projects_native_and_legacy_floor() -> TestResult {
    let tile = TilePos {
        x: 12,
        y: 34,
        floor: -7,
    };
    for basis in [spatial_basis::Basis::Native, spatial_basis::Basis::Legacy] {
        assert_eq!(served_tile(basis.position(tile), basis)?, tile);
    }
    Ok(())
}

/// Plan from immutable production map facts; only replies to real STEP commands prove travel.
fn route(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
) -> Option<Vec<StepDirection>> {
    route_with(world, from, goal, |_| true)
}

fn route_with(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
    traversable: impl Fn(TilePos) -> bool,
) -> Option<Vec<StepDirection>> {
    route_with_edges(from, goal, |at, direction| {
        destination(world, at, direction).filter(|next| traversable(*next))
    })
}

fn route_with_edges(
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
    edge: impl Fn(TilePos, StepDirection) -> Option<TilePos>,
) -> Option<Vec<StepDirection>> {
    let mut queue = VecDeque::from([from]);
    let mut parents = BTreeMap::from([(from, None)]);
    while let Some(at) = queue.pop_front() {
        if goal(at) {
            let mut path = Vec::new();
            let mut cursor = at;
            while let Some(Some((previous, direction))) = parents.get(&cursor) {
                path.push(*direction);
                cursor = *previous;
            }
            path.reverse();
            return Some(path);
        }
        for direction in [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ] {
            let Some(next) = edge(at, direction) else {
                continue;
            };
            if next.x.abs_diff(from.x) > 160
                || next.y.abs_diff(from.y) > 160
                || next.floor.abs_diff(from.floor) > 2
                || parents.contains_key(&next)
            {
                continue;
            }
            parents.insert(next, Some((at, direction)));
            queue.push_back(next);
        }
    }
    None
}

const REACH_SEARCH_NODES: usize = 3_000_000;
const REACH_CHUNK: usize = 20_000;

struct ReachBudget {
    remaining: usize,
}

impl ReachBudget {
    fn take(&mut self) -> bool {
        let available = self.remaining > 0;
        self.remaining = self.remaining.saturating_sub(1);
        available
    }
}

impl Default for ReachBudget {
    fn default() -> Self {
        Self {
            remaining: REACH_SEARCH_NODES,
        }
    }
}

#[test]
fn route_replans_share_one_expansion_budget() {
    let mut budget = ReachBudget { remaining: 2 };
    let mut plan = || budget.take();
    assert!(plan());
    assert!(plan());
    assert!(!plan());
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ReachFailure {
    searched: usize,
    component: (TilePos, TilePos),
}

impl std::fmt::Display for ReachFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "searched={} component={:?}..{:?}",
            self.searched, self.component.0, self.component.1
        )
    }
}

// Authored PZ flags remain authoritative for a safe return, independently of
// which hometown is currently recorded for the Character.
fn return_protection_zone(world: &BundleWorld, at: TilePos) -> bool {
    world
        .base()
        .tile(at.x, at.y, at.floor)
        .is_some_and(|tile| tile.walkable() && tile.protection_zone())
        && world.enterable(at)
}

async fn return_to_protection_zone(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult<TilePos> {
    let from = actor_tile(client)?;
    let path = reach_route(client, world, from, |at| return_protection_zone(world, at))
        .await?
        .map_err(|failure| {
            format!(
                "profile return has no reachable protection-zone temple: from={from:?} {failure}"
            )
        })?;
    walk(client, world, &path).await?;
    let at = actor_tile(client)?;
    if !return_protection_zone(world, at) {
        return Err(
            format!("profile return did not reach a protection-zone temple: at={at:?}").into(),
        );
    }
    Ok(at)
}

// Initially usable closed doors from map/door_registry.rs. Key-door closed
// forms 1629/1632 start unlocked; quest/level doors are never accepted.
fn retry_door(appearance: u16, action: Option<u16>) -> bool {
    ([1638, 1640, 1656, 1658, 1685, 1694, 2177, 2179].contains(&appearance) && action.is_none())
        || ([1629, 1632].contains(&appearance) && !matches!(action, Some(101 | 1001)))
}

// Resolve the configured entry against authored hometown temples, independently
// of the production pin default. The qualification bundle must agree.
#[cfg(test)]
fn fixture_home_town(world: &BundleWorld) -> TestResult<&oteryn_world_bundle::subarea::Area> {
    let temples: BTreeMap<_, _> = world
        .base()
        .subareas
        .areas
        .iter()
        .filter(|area| area.hometown)
        .filter_map(|area| {
            let temple = area.temple?;
            let at = TilePos {
                x: temple.x,
                y: temple.y,
                floor: temple.floor,
            };
            return_protection_zone(world, at).then_some((at, area))
        })
        .collect();
    // The entry may be a few cells from the catalogue's temple (Thais is three
    // cells south). Resolve the nearest reachable temple from the fixture ENTRY,
    // never from the post-death position or the source pin's default town.
    let path = route(world, TOWN, |at| temples.contains_key(&at))
        .ok_or("configured fixture start has no reachable authored hometown temple")?;
    let mut at = TOWN;
    for direction in path {
        at = destination(world, at, direction).ok_or("fixture temple route")?;
    }
    temples
        .get(&at)
        .copied()
        .ok_or_else(|| "fixture hometown temple absent".into())
}

fn home_temple_return(at: TilePos, failure: ReachFailure) -> String {
    format!("profile_return goal=home_temple at={at:?} fixture_town=unreachable {failure}")
}

/// Like `route` without its 160-tile and two-floor window: whole bundle up to
/// REACH_SEARCH_NODES tiles, serving the connection every REACH_CHUNK expansions.
/// `Ok(Err(failure))` records the searched component when unreachable or capped.
pub(super) async fn reach_route(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
) -> TestResult<Result<Vec<StepDirection>, ReachFailure>> {
    reach_route_until(client, world, from, goal, None, &BTreeSet::new()).await
}

#[derive(Debug)]
pub(super) struct SpawnProbeExpired;

impl std::fmt::Display for SpawnProbeExpired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("spawn probe deadline")
    }
}

impl std::error::Error for SpawnProbeExpired {}

fn probe_expired(deadline: Option<tokio::time::Instant>) -> bool {
    deadline.is_some_and(|limit| tokio::time::Instant::now() >= limit)
}

fn retry_fits_probe(
    now: tokio::time::Instant,
    deadline: Option<tokio::time::Instant>,
    delay: Duration,
) -> bool {
    deadline.is_none_or(|limit| now + delay < limit)
}

async fn probe_liveness(
    client: &mut oteryn_dev_client::DevClientSession,
    duration: Duration,
    deadline: Option<tokio::time::Instant>,
) -> TestResult {
    if let Some(limit) = deadline {
        let remaining = limit.saturating_duration_since(tokio::time::Instant::now());
        if remaining <= Duration::from_millis(1) {
            return Err(SpawnProbeExpired.into());
        }
        approach_action(client, limit, "spawn-frame", async |client| {
            client.service_liveness(duration.min(remaining / 2)).await?;
            Ok(())
        })
        .await?;
    } else {
        client.service_liveness(duration).await?;
    }
    Ok(())
}

async fn reach_route_until(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
    deadline: Option<tokio::time::Instant>,
    blocked_edges: &BTreeSet<(TilePos, TilePos)>,
) -> TestResult<Result<Vec<StepDirection>, ReachFailure>> {
    let mut origin = from;
    let mut budget = ReachBudget::default();
    for attempt in 0..=travel::MAX_REPLANS {
        if probe_expired(deadline) {
            return Err(SpawnProbeExpired.into());
        }
        probe_liveness(client, Duration::from_millis(1), deadline).await?;
        client.take_events(); // Canonical session stores retain the applied observations.
        let observed = actor_tile(client)?;
        if observed != origin {
            origin = observed;
        }
        let path = reach_route_serviced(
            world,
            origin,
            &goal,
            blocked_edges,
            &mut budget,
            async || {
                if probe_expired(deadline) {
                    return Err(SpawnProbeExpired.into());
                }
                verdict::Harness::global().progress("route-search");
                probe_liveness(client, Duration::from_millis(1), deadline).await?;
                client.take_events();
                Ok(())
            },
        )
        .await?;
        probe_liveness(client, Duration::from_millis(1), deadline).await?;
        client.take_events();
        let observed = actor_tile(client)?;
        if observed == origin {
            return Ok(path);
        }
        if budget.remaining == 0 {
            return Err(format!("actor moved during final route search from={origin:?} observed={observed:?} search_cap={REACH_SEARCH_NODES}").into());
        }
        evidence(&format!(
            "route_search replan=true attempt={attempt} from={origin:?} observed={observed:?}"
        ));
        origin = observed;
    }
    Err(format!(
        "reachability origin kept changing after {} replans",
        travel::MAX_REPLANS
    )
    .into())
}

// Keep the search shared with the offline production-bundle regression. Live
// callers always supply the connection service above, including error propagation.
async fn reach_route_serviced(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
    blocked_edges: &BTreeSet<(TilePos, TilePos)>,
    budget: &mut ReachBudget,
    mut service: impl AsyncFnMut() -> TestResult,
) -> TestResult<Result<Vec<StepDirection>, ReachFailure>> {
    let mut queue = VecDeque::from([from]);
    let mut parents = BTreeMap::from([(from, None)]);
    let mut expanded = 0usize;
    while let Some(at) = queue.pop_front() {
        if !budget.take() {
            break;
        }
        if goal(at) {
            let mut path = Vec::new();
            let mut cursor = at;
            while let Some(Some((previous, direction))) = parents.get(&cursor) {
                path.push(*direction);
                cursor = *previous;
            }
            path.reverse();
            return Ok(Ok(path));
        }
        if parents.len() > REACH_SEARCH_NODES {
            break;
        }
        expanded += 1;
        if expanded.is_multiple_of(REACH_CHUNK) {
            service().await?;
        }
        for direction in [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ] {
            let Some(next) = destination(world, at, direction) else {
                continue;
            };
            if blocked_edges.contains(&(at, next)) || parents.contains_key(&next) {
                continue;
            }
            parents.insert(next, Some((at, direction)));
            queue.push_back(next);
        }
    }
    let mut low = from;
    let mut high = from;
    for at in parents.keys() {
        low.x = low.x.min(at.x);
        low.y = low.y.min(at.y);
        low.floor = low.floor.min(at.floor);
        high.x = high.x.max(at.x);
        high.y = high.y.max(at.y);
        high.floor = high.floor.max(at.floor);
    }
    Ok(Err(ReachFailure {
        searched: parents.len(),
        component: (low, high),
    }))
}

/// Use the production floor resolver over the production bundle's collision facts.
fn destination(world: &BundleWorld, from: TilePos, direction: StepDirection) -> Option<TilePos> {
    landing(world, from, direction).map(|(at, _)| at)
}

fn landing(
    world: &BundleWorld,
    from: TilePos,
    direction: StepDirection,
) -> Option<(TilePos, bool)> {
    use crate::map::floor::{Ground, resolve_landing};
    use crate::movement::CardinalStep;
    let direction = match direction {
        StepDirection::North => CardinalStep::North,
        StepDirection::East => CardinalStep::East,
        StepDirection::South => CardinalStep::South,
        StepDirection::West => CardinalStep::West,
        StepDirection::NorthEast => CardinalStep::NorthEast,
        StepDirection::SouthEast => CardinalStep::SouthEast,
        StepDirection::SouthWest => CardinalStep::SouthWest,
        StepDirection::NorthWest => CardinalStep::NorthWest,
    };
    let position = |(x, y, z): (i32, i32, i32)| {
        Some(TilePos {
            x: u16::try_from(x).ok()?,
            y: u16::try_from(y).ok()?,
            floor: -i8::try_from(z).ok().filter(|z| (0..=15).contains(z))?,
        })
    };
    let ground = |cell| match position(cell) {
        Some(pos) => match world.base().tile(pos.x, pos.y, pos.floor) {
            None => Ground::Absent,
            Some(tile) if tile.walkable() && world.enterable(pos) => Ground::Walkable,
            Some(_) => Ground::Blocked,
        },
        None => Ground::Absent,
    };
    let teleported = std::cell::Cell::new(false);
    let next = resolve_landing(
        world.floors(),
        ground,
        |_| {
            teleported.set(true);
            true
        },
        (i32::from(from.x), i32::from(from.y), -i32::from(from.floor)),
        direction,
    )
    .ok()?;
    Some((position(next)?, teleported.get()))
}

fn near_locker(world: &BundleWorld, at: TilePos) -> bool {
    (-1..=1).any(|dy| {
        (-1..=1).any(|dx| {
            let Some((x, y)) = at.x.checked_add_signed(dx).zip(at.y.checked_add_signed(dy)) else {
                return false;
            };
            let pos = TilePos {
                x,
                y,
                floor: at.floor,
            };
            world.base().tile(x, y, at.floor).is_some_and(|tile| {
                (0..tile.depths().iter().filter(|depth| **depth == 0).count()).any(|ordinal| {
                    u8::try_from(ordinal)
                        .ok()
                        .is_some_and(|ordinal| world.facts().depot_at(pos, ordinal).is_some())
                })
            })
        })
    })
}

fn town_route_retry_delay(
    disposition: StepDisposition,
    stationary: bool,
    attempt: u8,
) -> Option<Duration> {
    (disposition == StepDisposition::Rejected && stationary && attempt <= 5)
        .then_some(Duration::from_millis(500))
}

/// `spawn <session> <key>` through `provision.py --spawn`; Err carries the rejected token.
/// Shared by `creature_sweep` and MP09 (both reach it through `use super::*`).
async fn request_spawn(session: String, key: String) -> Result<String, String> {
    let socket = required("NODE_BOOT_CONTROL_SOCKET").map_err(|error| error.to_string())?;
    let mut command = fixture_command();
    command.args(["--socket", &socket, "--session", &session, "--spawn", &key]);
    let output = tokio::time::timeout(FIXTURE_COMMAND_DEADLINE, run_fixture_command(&mut command))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(match stderr.split_once("reason=") {
        Some((_, rest)) => rest
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect(),
        None => format!(
            "status={};stderr_tail={}",
            output.status,
            stderr
                .replace('\n', " | ")
                .chars()
                .rev()
                .take(400)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        ),
    })
}

async fn walk(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    walk_observing(client, world, path, |_, _| Ok(())).await
}

async fn walk_observing(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
    observe: impl FnMut(
        &oteryn_dev_client::DevClientSession,
        Vec<oteryn_dev_client::SessionEvent>,
    ) -> TestResult,
) -> TestResult {
    walk_until_observing(client, world, path, None, false, observe).await
}

async fn probe_step(
    client: &mut oteryn_dev_client::DevClientSession,
    direction: StepDirection,
    deadline: Option<tokio::time::Instant>,
) -> TestResult<oteryn_dev_client::StepOutcome> {
    let Some(deadline) = deadline else {
        return Ok(client.step(direction).await?);
    };
    approach_action(client, deadline, "spawn-step", async |client| {
        Ok(client.step(direction).await?)
    })
    .await
}

#[derive(Debug)]
struct BlockedSpawnEdge(TilePos, TilePos);
impl std::fmt::Display for BlockedSpawnEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "blocked spawn edge {:?} -> {:?}", self.0, self.1)
    }
}
impl std::error::Error for BlockedSpawnEdge {}

async fn walk_until(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
    deadline: Option<tokio::time::Instant>,
    tolerate_refusal: bool,
) -> TestResult {
    walk_until_observing(client, world, path, deadline, tolerate_refusal, |_, _| {
        Ok(())
    })
    .await
}

async fn walk_until_observing(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
    deadline: Option<tokio::time::Instant>,
    tolerate_refusal: bool,
    mut observe: impl FnMut(
        &oteryn_dev_client::DevClientSession,
        Vec<oteryn_dev_client::SessionEvent>,
    ) -> TestResult,
) -> TestResult {
    for &direction in path {
        if probe_expired(deadline) {
            return Err(SpawnProbeExpired.into());
        }
        let before = client.world_spatial().actor_position;
        let basis = spatial_basis::current(client)?;
        let mut outcome = if deadline.is_some() {
            probe_step(client, direction, deadline).await?
        } else {
            client.step_retrying(direction).await?
        };
        for attempt in 1..=6 {
            if !matches!(
                outcome.disposition,
                StepDisposition::Rejected | StepDisposition::TooEarly
            ) || (deadline.is_none() && outcome.disposition == StepDisposition::TooEarly)
                || client.world_spatial().actor_position != before
            {
                break;
            }
            evidence(&format!(
                "town_route step_rejected attempt={attempt} direction={direction:?} from={before:?} expected={:?} actual={:?}",
                destination(world, served_tile(before, basis)?, direction),
                client.world_spatial().actor_position,
            ));
            let delay = if outcome.disposition == StepDisposition::TooEarly {
                Some(Duration::from_millis(50))
            } else {
                town_route_retry_delay(
                    outcome.disposition,
                    client.world_spatial().actor_position == before,
                    attempt,
                )
            };
            let Some(delay) = delay else { break };
            client.take_events();
            if !retry_fits_probe(tokio::time::Instant::now(), deadline, delay) {
                return Err(SpawnProbeExpired.into());
            }
            tokio::time::sleep(delay).await;
            if probe_expired(deadline) {
                return Err(SpawnProbeExpired.into());
            }
            outcome = if deadline.is_some() {
                probe_step(client, direction, deadline).await?
            } else {
                client.step_retrying(direction).await?
            };
        }
        if tolerate_refusal
            && matches!(
                outcome.disposition,
                StepDisposition::Rejected | StepDisposition::TooEarly | StepDisposition::Blocked
            )
            && client.world_spatial().actor_position == before
        {
            client.take_events();
            let from = served_tile(before, basis)?;
            let to = destination(world, from, direction).ok_or("invalid refused STEP edge")?;
            return Err(BlockedSpawnEdge(from, to).into());
        }
        if outcome.disposition != StepDisposition::Moved || outcome.world_spatial_delta.is_none() {
            return Err(format!(
                "town route step {direction:?} from={before:?} expected={:?} actual={:?}: {outcome:?}",
                destination(world, served_tile(before, basis)?, direction),
                client.world_spatial().actor_position,
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        let after = client.world_spatial().actor_position;
        if Some(actor_tile(client)?) != destination(world, served_tile(before, basis)?, direction) {
            return Err(format!(
                "town step changed position unexpectedly: {before:?} -> {after:?}"
            )
            .into());
        }
        let events = client.take_events();
        observe(client, events)?;
    }
    Ok(())
}

pub(super) async fn logout_retrying_busy(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<(
    Option<oteryn_protocol_oteryn::logout::LogoutResult>,
    String,
    usize,
    bool,
)> {
    logout_retrying_busy_observed(client, None).await
}

type LogoutObserver<'a> = dyn FnMut(usize, &str) + 'a;

pub(super) async fn logout_retrying_busy_observed(
    client: &mut oteryn_dev_client::DevClientSession,
    mut observe: Option<&mut LogoutObserver<'_>>,
) -> TestResult<(
    Option<oteryn_protocol_oteryn::logout::LogoutResult>,
    String,
    usize,
    bool,
)> {
    use oteryn_protocol_oteryn::logout::LogoutResult;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let mut attempts = 0;
    loop {
        attempts += 1;
        let outcome = logout_observation(
            client_readmission::within_retry_budget(deadline, async {
                Ok(client.session_mut().logout().await?)
            })
            .await,
            &mut observe,
            attempts,
            "command",
        )
        .map_err(|error| {
            verdict::context(
                format!("logout command attempt {attempts} failed: {error}"),
                error,
            )
        })?;
        if let Some(observe) = observe.as_mut() {
            observe(attempts, &format!("reply={outcome:?}"));
        }
        if outcome.result != Some(LogoutResult::Busy) {
            return Ok((outcome.result, format!("{outcome:?}"), attempts, false));
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok((outcome.result, format!("{outcome:?}"), attempts, true));
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        logout_observation(
            client_readmission::within_retry_budget(deadline, async {
                Ok(client
                    .service_liveness(remaining.min(Duration::from_millis(250)))
                    .await?)
            })
            .await,
            &mut observe,
            attempts,
            "busy_liveness",
        )
        .map_err(|error| {
            verdict::context(
                format!("logout busy_liveness attempt {attempts} failed: {error}"),
                error,
            )
        })?;
    }
}

fn logout_observation<T, E: std::fmt::Display>(
    result: Result<T, E>,
    observe: &mut Option<&mut LogoutObserver<'_>>,
    attempt: usize,
    phase: &str,
) -> Result<T, E> {
    result.inspect_err(|error| {
        if let Some(observe) = observe.as_mut() {
            observe(attempt, &format!("{phase}_error={error}"));
        }
    })
}

#[test]
fn qualification_logout_retains_server_close_for_indexed_classifier() -> TestResult {
    use oteryn_dev_client::SessionError;
    use verdict::Cause;

    let trace = [0xab; 16];
    let mut messages = Vec::new();
    let mut observe = |attempt, message: &str| messages.push((attempt, message.to_owned()));
    let error = logout_observation(
        Err::<(), _>(SessionError::ServerProtocolError {
            code: 1200,
            connection_trace: Some(trace),
        }),
        &mut Some(&mut observe),
        2,
        "command",
    )
    .err()
    .ok_or("expected transport error")?;
    assert_eq!(
        verdict::classify(&error),
        Cause::Transport {
            code: Some(1200),
            trace: Some("ab".repeat(16)),
            phase: None,
            last_frame: None,
        }
    );
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].0, 2);
    assert!(messages[0].1.contains("command_error="));
    Ok(())
}

pub(super) async fn logout_trade_character(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult {
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!(
            "trade logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}"
        )
        .into());
    }
    Ok(())
}

pub(super) async fn journeys(
    clients: &SeamClients<'_>,
    packet_clients: &SeamClients<'_>,
    path: &Path,
) -> TestResult {
    let started = std::time::Instant::now();
    let bytes = std::fs::read(path)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    evidence(&format!(
        "world_loader state=booted duration_ms={} start={TOWN:?}",
        started.elapsed().as_millis()
    ));
    let groups = verdict::Groups::from_env()?;
    // The required core keeps its ordinary pair; packet journeys own a separate pair.
    let mut client = connect_world(clients, 0x70).await?;
    let entered_at = tile(client.world_spatial().actor_position)?;
    evidence(&format!(
        "world_fixture role=core phase=first_entry position={entered_at:?}"
    ));
    if entered_at != TOWN {
        return Err(format!(
            "Platform-created character did not enter the configured town: expected={TOWN:?} actual={entered_at:?}"
        )
        .into());
    }
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("first town entry logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    drop(client);
    {
        let clients = packet_clients;
        // Default runs retain the ordinary packets alongside the indexed rows.
        // Explicit groups can isolate those packets from the row batch.
        include!("full_e2ef6_mount.rs");
        let batch = groups
            .runs("rows")
            .then(|| {
                let value = std::env::var("OTERYN_E2E_ROW_BATCH");
                match value.as_ref() {
                    Ok(value) => e2e_row_batch(Some(value)),
                    Err(std::env::VarError::NotPresent) => e2e_row_batch(None),
                    Err(std::env::VarError::NotUnicode(_)) => None,
                }
                .ok_or("invalid OTERYN_E2E_ROW_BATCH (expected all or start:positive-length)")
            })
            .transpose()?;
        let mut selected = false;
        let mut e2e_row_selected = |index: usize| {
            let matches = batch.is_some_and(|(start, len)| index >= start && index - start < len);
            selected |= matches;
            matches
        };
        if groups.runs("rows") {
            let before = journey_verdicts();
            release_row_characters(clients).await?;
            if e2e_row_selected(1) {
                verdict::indexed("C03.full", async {
                    full_row_c03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(2) {
                verdict::indexed("C12.full", async {
                    full_row_c12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(3) {
                verdict::indexed("C09.full", async {
                    full_row_c09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(4) {
                verdict::indexed("C06.full", async {
                    full_row_c06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(5) {
                verdict::indexed("C02.full", async {
                    full_row_c02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(6) {
                verdict::indexed("C01.full", async {
                    full_row_c01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(7) {
                verdict::indexed("W22b.full", async {
                    full_row_w22b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(8) {
                verdict::indexed("C05.full", async {
                    full_row_c05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(9) {
                verdict::indexed("C04.full", async {
                    full_row_c04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(10) {
                verdict::indexed("W18.full", async {
                    full_row_w18::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(11) {
                verdict::indexed("C11.full", async {
                    full_row_c11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(12) {
                verdict::indexed("W19a.full", async {
                    full_row_w19a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(13) {
                verdict::indexed("W01b.full", async {
                    full_row_w01b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(14) {
                verdict::indexed("W23.full", async {
                    full_row_w23::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(15) {
                verdict::indexed("W15d.full", async {
                    full_row_w15d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(16) {
                verdict::indexed("W15a.full", async {
                    full_row_w15a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(17) {
                verdict::indexed("W10.full", async {
                    full_row_w10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(18) {
                verdict::indexed("W08.full", async {
                    full_row_w08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(19) {
                verdict::indexed("W24.full", async {
                    full_row_w24::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(20) {
                verdict::indexed("W22a.full", async {
                    full_row_w22a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(21) {
                verdict::indexed("W15b.full", async {
                    full_row_w15b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(22) {
                verdict::indexed("W11.full", async {
                    full_row_w11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(23) {
                verdict::indexed("W05a.full", async {
                    full_row_w05a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(24) {
                verdict::indexed("W03.full", async {
                    full_row_w03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(25) {
                verdict::indexed("W01a.full", async {
                    full_row_w01a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(26) {
                verdict::indexed("S16b.full", async {
                    full_row_s16b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(27) {
                verdict::indexed("S15b.full", async {
                    full_row_s15b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(28) {
                verdict::indexed("S10.full", async {
                    full_row_s10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(29) {
                verdict::indexed("S07.full", async {
                    full_row_s07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(30) {
                verdict::indexed("S06.full", async {
                    full_row_s06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(31) {
                verdict::indexed("S05.full", async {
                    full_row_s05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(32) {
                verdict::indexed("P07.full", async {
                    full_row_p07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(33) {
                verdict::indexed("W17.full", async {
                    full_row_w17::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(34) {
                verdict::indexed("W16.full", async {
                    full_row_w16::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(35) {
                verdict::indexed("W13.full", async {
                    full_row_w13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(36) {
                verdict::indexed("W12.full", async {
                    full_row_w12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(37) {
                verdict::indexed("W05c.full", async {
                    full_row_w05c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(38) {
                verdict::indexed("W04.full", async {
                    full_row_w04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(39) {
                verdict::indexed("W02.full", async {
                    full_row_w02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(40) {
                verdict::indexed("W01c.full", async {
                    full_row_w01c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(41) {
                verdict::indexed("S13c.full", async {
                    full_row_s13c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(42) {
                verdict::indexed("S04c.full", async {
                    full_row_s04c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(43) {
                verdict::indexed("S04a.full", async {
                    full_row_s04a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(44) {
                verdict::indexed("S02.full", async {
                    full_row_s02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(45) {
                verdict::indexed("P23c.full", async {
                    full_row_p23c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(46) {
                verdict::indexed("P20.full", async {
                    full_row_p20::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(47) {
                verdict::indexed("P18.full", async {
                    full_row_p18::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(48) {
                verdict::indexed("P17.full", async {
                    full_row_p17::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(49) {
                verdict::indexed("P15.full", async {
                    full_row_p15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(50) {
                verdict::indexed("P14.full", async {
                    full_row_p14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(51) {
                verdict::indexed("P13.full", async {
                    full_row_p13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(52) {
                verdict::indexed("P12c.full", async {
                    full_row_p12c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(53) {
                verdict::indexed("P12a.full", async {
                    full_row_p12a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(54) {
                verdict::indexed("P11.full", async {
                    full_row_p11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(55) {
                verdict::indexed("P09.full", async {
                    full_row_p09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(56) {
                verdict::indexed("P08.full", async {
                    full_row_p08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(57) {
                verdict::indexed("P06.full", async {
                    full_row_p06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(58) {
                verdict::indexed("P05.full", async {
                    full_row_p05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(59) {
                verdict::indexed("P04.full", async {
                    full_row_p04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(60) {
                verdict::indexed("P03.full", async {
                    full_row_p03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(61) {
                verdict::indexed("P02.full", async {
                    full_row_p02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(62) {
                verdict::indexed("P01.full", async {
                    full_row_p01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(63) {
                verdict::indexed("E10.full", async {
                    full_row_e10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(64) {
                verdict::indexed("E09.full", async {
                    full_row_e09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(65) {
                verdict::indexed("E08.full", async {
                    full_row_e08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(66) {
                verdict::indexed("E06.full", async {
                    full_row_e06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(67) {
                verdict::indexed("I11.full", async {
                    full_row_i11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(68) {
                verdict::indexed("I10.full", async {
                    full_row_i10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(69) {
                verdict::indexed("I05.full", async {
                    full_row_i05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(70) {
                verdict::indexed("M11b.full", async {
                    full_row_m11b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(71) {
                verdict::indexed("M11a.full", async {
                    full_row_m11a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(72) {
                verdict::indexed("M08.full", async {
                    full_row_m08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(73) {
                verdict::indexed("C18b.full", async {
                    full_row_c18b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(74) {
                verdict::indexed("C17b.full", async {
                    full_row_c17b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(75) {
                verdict::indexed("C10.full", async {
                    full_row_c10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(76) {
                verdict::indexed("C07.full", async {
                    full_row_c07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(77) {
                verdict::indexed("E12.full", async {
                    full_row_e12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(78) {
                verdict::indexed("I14.full", async {
                    full_row_i14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(79) {
                verdict::indexed("I03.full", async {
                    full_row_i03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(80) {
                verdict::indexed("M10.full", async {
                    full_row_m10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(81) {
                verdict::indexed("M09.full", async {
                    full_row_m09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(82) {
                verdict::indexed("M06.full", async {
                    full_row_m06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(83) {
                verdict::indexed("C15.full", async {
                    full_row_c15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(84) {
                verdict::indexed("C08.full", async {
                    full_row_c08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(85) {
                full_e2ef19::run_packet(clients).await?;
            }
            if e2e_row_selected(86) {
                full_e2ef18::run_packet(clients).await?;
            }
            if e2e_row_selected(87) {
                full_e2ef20::run_packet(clients).await?;
            }
            if e2e_row_selected(88) {
                verdict::indexed("C01.client", async {
                    client_row_c01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(89) {
                verdict::indexed("C02.client", async {
                    client_row_c02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(90) {
                verdict::indexed("C03.client", async {
                    client_row_c03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(91) {
                verdict::indexed("C04.client", async {
                    client_row_c04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(92) {
                verdict::indexed("C05.client", async {
                    client_row_c05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(93) {
                verdict::indexed("C06.client", async {
                    client_row_c06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(227) {
                client_row_c07::run_client_packet(clients).await;
                release_row_characters(clients).await?;
            }
            if e2e_row_selected(94) {
                verdict::indexed("C09.client", async {
                    client_row_c09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(95) {
                verdict::indexed("C10.client", async {
                    client_row_c10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(96) {
                verdict::indexed("C11.client", async {
                    client_row_c11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(97) {
                verdict::indexed("C12.client", async {
                    client_row_c12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(98) {
                verdict::indexed("C13.client", async {
                    client_row_c13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(99) {
                verdict::indexed("C14.client", async {
                    client_row_c14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(100) {
                verdict::indexed("C15.client", async {
                    client_row_c15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(101) {
                verdict::indexed("C16a.client", async {
                    client_row_c16a::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(102) {
                verdict::indexed("C16b.client", async {
                    client_row_c16b::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(103) {
                verdict::indexed("C16c.client", async {
                    client_row_c16c::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(104) {
                verdict::indexed("C16d.client", async {
                    client_row_c16d::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(105) {
                verdict::indexed("C17a.client", async {
                    client_row_c17a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(106) {
                verdict::indexed("C17b.client", async {
                    client_row_c17b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(107) {
                verdict::indexed("C17d.client", async {
                    client_row_c17d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(108) {
                verdict::indexed("C18a.client", async {
                    client_row_c18a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(109) {
                verdict::indexed("C18b.client", async {
                    client_row_c18b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(228) {
                verdict::indexed("C20a.client", async {
                    client_row_c20a::run_packet(clients, world, &metadata.spawns).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(110) {
                verdict::indexed("C20b.client", async {
                    client_row_c20b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(111) {
                verdict::indexed("C20c.client", async {
                    client_row_c20c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(229) {
                verdict::indexed("C20d.client", async {
                    client_row_c20d::run_packet(clients, world, &metadata.spawns).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(112) {
                verdict::indexed("E01.client", async {
                    client_row_e01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(113) {
                verdict::indexed("I01.client", async {
                    client_row_i01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(114) {
                verdict::indexed("I03.client", async {
                    client_row_i03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(115) {
                verdict::indexed("I04.client", async {
                    client_row_i04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(116) {
                verdict::indexed("I05.client", async {
                    client_row_i05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(117) {
                verdict::indexed("I06.client", async {
                    client_row_i06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(118) {
                verdict::indexed("I07.client", async {
                    client_row_i07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(119) {
                verdict::indexed("I08.client", async {
                    client_row_i08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(120) {
                verdict::indexed("I09.client", async {
                    client_row_i09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(121) {
                verdict::indexed("I10.client", async {
                    client_row_i10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(122) {
                verdict::indexed("I11.client", async {
                    client_row_i11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(123) {
                verdict::indexed("I12.client", async {
                    client_row_i12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(124) {
                verdict::indexed("I13.client", async {
                    client_row_i13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(125) {
                verdict::indexed("I14.client", async {
                    client_row_i14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(126) {
                verdict::indexed("I15.client", async {
                    client_row_i15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(127) {
                verdict::indexed("I16.client", async {
                    client_row_i16::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(128) {
                verdict::indexed("M01.client", async {
                    client_row_m01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(129) {
                verdict::indexed("M02.client", async {
                    client_row_m02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(130) {
                verdict::indexed("M03.client", async {
                    client_row_m03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(131) {
                verdict::indexed("M04.client", async {
                    client_row_m04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(132) {
                verdict::indexed("M05.client", async {
                    client_row_m05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(133) {
                verdict::indexed("M06.client", async {
                    client_row_m06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(134) {
                verdict::indexed("M07.client", async {
                    client_row_m07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(135) {
                verdict::indexed("M08.client", async {
                    client_row_m08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(136) {
                verdict::indexed("M09.client", async {
                    client_row_m09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(137) {
                verdict::indexed("M10.client", async {
                    client_row_m10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(138) {
                verdict::indexed("M11a.client", async {
                    client_row_m11a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(139) {
                verdict::indexed("M11b.client", async {
                    client_row_m11b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(140) {
                verdict::indexed("N01.client", async {
                    client_row_n01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(141) {
                verdict::indexed("N02.client", async {
                    client_row_n02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(142) {
                verdict::indexed("N03.client", async {
                    client_row_n03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(143) {
                verdict::indexed("N04.client", async {
                    client_row_n04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(144) {
                verdict::indexed("C08.client", async {
                    client_row_c08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(145) {
                verdict::indexed("C17c.client", async {
                    client_row_c17c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(146) {
                verdict::indexed("C19a.client", async {
                    client_row_c19a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(147) {
                verdict::indexed("E02.client", async {
                    client_row_e02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(148) {
                verdict::indexed("E03.client", async {
                    client_row_e03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(149) {
                verdict::indexed("E05.client", async {
                    client_row_e05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(150) {
                verdict::indexed("E06.client", async {
                    client_row_e06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(151) {
                verdict::indexed("E07.client", async {
                    client_row_e07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(152) {
                verdict::indexed("E08.client", async {
                    client_row_e08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(153) {
                verdict::indexed("E09.client", async {
                    client_row_e09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(154) {
                verdict::indexed("E10.client", async {
                    client_row_e10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(155) {
                verdict::indexed("E12.client", async {
                    client_row_e12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(156) {
                verdict::indexed("P01.client", async {
                    client_row_p01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(157) {
                verdict::indexed("P02.client", async {
                    client_row_p02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(158) {
                verdict::indexed("P03.client", async {
                    client_row_p03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(159) {
                verdict::indexed("P04.client", async {
                    client_row_p04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(160) {
                verdict::indexed("P05.client", async {
                    client_row_p05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(161) {
                verdict::indexed("P06.client", async {
                    client_row_p06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(162) {
                verdict::indexed("P07.client", async {
                    client_row_p07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(163) {
                verdict::indexed("P08.client", async {
                    client_row_p08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(164) {
                verdict::indexed("P09.client", async {
                    client_row_p09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(165) {
                verdict::indexed("P10.client", async {
                    client_row_p10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(166) {
                verdict::indexed("P11.client", async {
                    client_row_p11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(167) {
                verdict::indexed("P12a.client", async {
                    client_row_p12a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(168) {
                verdict::indexed("P12c.client", async {
                    client_row_p12c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(169) {
                verdict::indexed("P13.client", async {
                    client_row_p13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(170) {
                verdict::indexed("P14.client", async {
                    client_row_p14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(171) {
                verdict::indexed("P15.client", async {
                    client_row_p15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(172) {
                verdict::indexed("P16.client", async {
                    client_row_p16::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(173) {
                verdict::indexed("P17.client", async {
                    client_row_p17::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(174) {
                verdict::indexed("P18.client", async {
                    client_row_p18::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(175) {
                verdict::indexed("P19.client", async {
                    client_row_p19::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(176) {
                verdict::indexed("P20.client", async {
                    client_row_p20::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(177) {
                verdict::indexed("P21.client", async {
                    client_row_p21::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(178) {
                verdict::indexed("P23c.client", async {
                    client_row_p23c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(179) {
                verdict::indexed("S01.client", async {
                    client_row_s01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(180) {
                verdict::indexed("S02.client", async {
                    client_row_s02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(181) {
                verdict::indexed("S03.client", async {
                    client_row_s03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(182) {
                verdict::indexed("S04a.client", async {
                    client_row_s04a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(183) {
                verdict::indexed("S04b.client", async {
                    client_row_s04b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(184) {
                verdict::indexed("S18.client", async {
                    client_row_s18::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(185) {
                verdict::indexed("W01a.client", async {
                    client_row_w01a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(186) {
                verdict::indexed("W01b.client", async {
                    client_row_w01b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(187) {
                verdict::indexed("W01c.client", async {
                    client_row_w01c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(188) {
                verdict::indexed("W02.client", async {
                    client_row_w02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(189) {
                verdict::indexed("W03.client", async {
                    client_row_w03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(190) {
                verdict::indexed("W04.client", async {
                    client_row_w04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(191) {
                verdict::indexed("W05a.client", async {
                    client_row_w05a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(192) {
                verdict::indexed("W05c.client", async {
                    client_row_w05c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(193) {
                verdict::indexed("W10.client", async {
                    client_row_w10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(194) {
                verdict::indexed("W11.client", async {
                    client_row_w11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(195) {
                verdict::indexed("W12.client", async {
                    client_row_w12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(196) {
                verdict::indexed("W13.client", async {
                    client_row_w13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(197) {
                verdict::indexed("S06.client", async {
                    client_row_s06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(198) {
                verdict::indexed("S08.client", async {
                    client_row_s08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(199) {
                verdict::indexed("S13c.client", async {
                    client_row_s13c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(200) {
                verdict::indexed("S13b.client", async {
                    client_row_s13b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(201) {
                verdict::indexed("S13a.client", async {
                    client_row_s13a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(202) {
                verdict::indexed("S15b.client", async {
                    client_row_s15b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(203) {
                verdict::indexed("S04c.client", async {
                    client_row_s04c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(204) {
                verdict::indexed("S07.client", async {
                    client_row_s07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(205) {
                verdict::indexed("S16b.client", async {
                    client_row_s16b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(206) {
                verdict::indexed("S09.client", async {
                    client_row_s09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(207) {
                verdict::indexed("S05.client", async {
                    client_row_s05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(208) {
                verdict::indexed("S10.client", async {
                    client_row_s10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(209) {
                verdict::indexed("S16a.client", async {
                    client_row_s16a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(210) {
                verdict::indexed("S11.client", async {
                    client_row_s11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(211) {
                verdict::indexed("W15a.client", async {
                    client_row_w15a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(212) {
                verdict::indexed("W15b.client", async {
                    client_row_w15b::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(213) {
                verdict::indexed("W15d.client", async {
                    client_row_w15d::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(214) {
                verdict::indexed("W16.client", async {
                    client_row_w16::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(215) {
                verdict::indexed("W17.client", async {
                    client_row_w17::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(216) {
                verdict::indexed("W19a.client", async {
                    client_row_w19a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(217) {
                verdict::indexed("W20a.client", async {
                    client_row_w20a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(218) {
                verdict::indexed("W20b.client", async {
                    client_row_w20b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(219) {
                verdict::indexed("W22a.client", async {
                    client_row_w22a::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(220) {
                verdict::indexed("W22b.client", async {
                    client_row_w22b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(221) {
                verdict::indexed("W23.client", async {
                    client_row_w23::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(222) {
                verdict::indexed("W24.client", async {
                    client_row_w24::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(223) {
                verdict::indexed("N07a.client", async {
                    client_row_n07a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(224) {
                verdict::indexed("N07b.client", async {
                    client_row_n07b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(225) {
                verdict::indexed("N07c.client", async {
                    client_row_n07c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(226) {
                verdict::indexed("N07d.client", async {
                    client_row_n07d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            journey_group_completed("rows_early", before);
        }
        if groups.runs("f5") {
            let before = journey_verdicts();
            full_e2ef5::run(clients, world).await;
            journey_group_completed("f5", before);
        }
        for index in 0..clients.accounts.len() {
            wait_character_released(clients, index).await?;
        }
        if groups.runs("f4") {
            let before = journey_verdicts();
            include!("full_e2ef4_mount.rs");
            journey_group_completed("f4", before);
        }
        // F4 can drop its socket; require terminal release before the next journey.
        for index in 0..clients.accounts.len() {
            wait_character_released(clients, index).await?;
        }
        // Keep the low-level speech witness and Adventurer victim eligible
        // until their rows finish, then let that player choose a caster vocation.
        if groups.runs("f5") {
            full_e2ef5::support_rows(clients, world).await;
        }
        if groups.runs("f7add") {
            full_e2ef7::run_light(clients).await;
        }
        if groups.runs("rows") {
            let before = journey_verdicts();
            if e2e_row_selected(233) {
                verdict::indexed("G01.full", async {
                    full_row_g01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(234) {
                verdict::indexed("G02.full", async {
                    full_row_g02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(235) {
                verdict::indexed("G03.full", async {
                    full_row_g03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(236) {
                verdict::indexed("G04.full", async {
                    full_row_g04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(237) {
                verdict::indexed("G05.full", async {
                    full_row_g05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(238) {
                verdict::indexed("G06.full", async {
                    full_row_g06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(239) {
                verdict::indexed("G07.full", async {
                    full_row_g07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(240) {
                verdict::indexed("G08.full", async {
                    full_row_g08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(241) {
                verdict::indexed("G09.full", async {
                    full_row_g09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(242) {
                verdict::indexed("G10.full", async {
                    full_row_g10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(243) {
                verdict::indexed("G11.full", async {
                    full_row_g11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(244) {
                verdict::indexed("G12.full", async {
                    full_row_g12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(245) {
                verdict::indexed("G13.full", async {
                    full_row_g13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(246) {
                verdict::indexed("G14.full", async {
                    full_row_g14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(247) {
                verdict::indexed("G15.full", async {
                    full_row_g15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(248) {
                verdict::indexed("G16.full", async {
                    full_row_g16::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(249) {
                verdict::indexed("G17.full", async {
                    full_row_g17::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(250) {
                verdict::indexed("G18.full", async {
                    full_row_g18::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(251) {
                verdict::indexed("G19.full", async {
                    full_row_g19::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(252) {
                verdict::indexed("G20.full", async {
                    full_row_g20::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(253) {
                verdict::indexed("G21.full", async {
                    full_row_g21::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(254) {
                verdict::indexed("G22.full", async {
                    full_row_g22::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(255) {
                verdict::indexed("G23.full", async {
                    full_row_g23::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(256) {
                verdict::indexed("G24.full", async {
                    full_row_g24::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(257) {
                verdict::indexed("G25.full", async {
                    full_row_g25::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(258) {
                verdict::indexed("G26.full", async {
                    full_row_g26::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(259) {
                verdict::indexed("G27.full", async {
                    full_row_g27::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(260) {
                verdict::indexed("G28.full", async {
                    full_row_g28::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(261) {
                verdict::indexed("G29.full", async {
                    full_row_g29::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(262) {
                verdict::indexed("G30.full", async {
                    full_row_g30::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(263) {
                verdict::indexed("G31.full", async {
                    full_row_g31::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(264) {
                verdict::indexed("G32.full", async {
                    full_row_g32::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(265) {
                verdict::indexed("G33.full", async {
                    full_row_g33::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(266) {
                verdict::indexed("G34.full", async {
                    full_row_g34::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(267) {
                verdict::indexed("G35.full", async {
                    full_row_g35::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(268) {
                verdict::indexed("G36.full", async {
                    full_row_g36::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(269) {
                verdict::indexed("C19b.full", async {
                    full_row_c19b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(270) {
                verdict::indexed("I17a.full", async {
                    full_row_i17a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(271) {
                verdict::indexed("I17b.full", async {
                    full_row_i17b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(272) {
                verdict::indexed("I17c.full", async {
                    full_row_i17c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(273) {
                verdict::indexed("I17d.full", async {
                    full_row_i17d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(274) {
                verdict::indexed("I18a.full", async {
                    full_row_i18a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(275) {
                verdict::indexed("I18b.full", async {
                    full_row_i18b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(276) {
                verdict::indexed("I18c.full", async {
                    full_row_i18c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(277) {
                verdict::indexed("I19.full", async {
                    full_row_i19::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(278) {
                verdict::indexed("P12b.full", async {
                    full_row_p12b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(279) {
                verdict::indexed("P22a.full", async {
                    full_row_p22a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(280) {
                verdict::indexed("P22b.full", async {
                    full_row_p22b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(281) {
                verdict::indexed("P22c.full", async {
                    full_row_p22c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(282) {
                verdict::indexed("P22d.full", async {
                    full_row_p22d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(283) {
                verdict::indexed("S14a.full", async {
                    full_row_s14a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(284) {
                verdict::indexed("S14b.full", async {
                    full_row_s14b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(285) {
                verdict::indexed("S14c.full", async {
                    full_row_s14c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(286) {
                verdict::indexed("S15a.full", async {
                    full_row_s15a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(287) {
                verdict::indexed("S16c.full", async {
                    full_row_s16c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(288) {
                verdict::indexed("S16d.full", async {
                    full_row_s16d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(289) {
                verdict::indexed("S17a.full", async {
                    full_row_s17a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(290) {
                verdict::indexed("S17b.full", async {
                    full_row_s17b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(291) {
                verdict::indexed("S17c.full", async {
                    full_row_s17c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(292) {
                verdict::indexed("S19.full", async {
                    full_row_s19::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(293) {
                verdict::indexed("W05b.full", async {
                    full_row_w05b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(294) {
                verdict::indexed("W05d.full", async {
                    full_row_w05d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(295) {
                verdict::indexed("W06.full", async {
                    full_row_w06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(296) {
                verdict::indexed("W07.full", async {
                    full_row_w07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(297) {
                verdict::indexed("W09.full", async {
                    full_row_w09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(298) {
                verdict::indexed("W15c.full", async {
                    full_row_w15c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(299) {
                verdict::indexed("W19b.full", async {
                    full_row_w19b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(300) {
                verdict::indexed("W19c.full", async {
                    full_row_w19c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(301) {
                verdict::indexed("W19d.full", async {
                    full_row_w19d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(302) {
                verdict::indexed("W19e.full", async {
                    full_row_w19e::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(303) {
                verdict::indexed("W21.full", async {
                    full_row_w21::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(304) {
                verdict::indexed("G37.full", async {
                    full_row_g37::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(305) {
                verdict::indexed("G38.full", async {
                    full_row_g38::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(306) {
                verdict::indexed("Q01a.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[0]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(307) {
                verdict::indexed("Q01b.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[1]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(308) {
                verdict::indexed("Q01c.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[2]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(309) {
                verdict::indexed("Q01d.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[3]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(310) {
                verdict::indexed("Q01e.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[4]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(311) {
                verdict::indexed("Q02.full", async {
                    full_row_q02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(312) {
                verdict::indexed("Q03a.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[5]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(313) {
                verdict::indexed("Q03b.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[6]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(314) {
                verdict::indexed("Q03c.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[7]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(315) {
                verdict::indexed("Q03d.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[8]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(316) {
                verdict::indexed("Q03e.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[9]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(317) {
                verdict::indexed("Q03f.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[10]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(318) {
                verdict::indexed("Q03g.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[11]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(319) {
                verdict::indexed("Q03h.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[12]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(320) {
                verdict::indexed("Q03i.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[13]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(321) {
                verdict::indexed("Q03j.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[14]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(322) {
                verdict::indexed("Q03k.full", async {
                    full_row_q01::run_packet(clients, &full_row_q01::CHESTS[15]).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            journey_group_completed("rows_late", before);
        }
        if let Some((start, len)) = batch
            && !selected
        {
            return Err(format!("OTERYN_E2E_ROW_BATCH={start}:{len} selects no live rows").into());
        }
        for index in 0..clients.accounts.len() {
            wait_character_released(clients, index).await?;
        }
        evidence(&format!(
            "world_fixture role=packets phase=terminal_release accounts={}",
            clients.accounts.len()
        ));
    }
    if groups.runs("creature_sweep") {
        let before = journey_verdicts();
        creature_sweep::run(clients, world, &metadata.spawns).await?;
        if groups.explicitly_selected() {
            journey_group_completed("creature_sweep", before);
        }
    }
    if groups.runs("mp") {
        let before = journey_verdicts();
        mp::run(clients, packet_clients, world, &metadata.manifest.subareas).await;
        journey_group_completed("mp", before);
    }
    // The core/profile pair has not inherited packet movement, loot or death.
    if !groups.runs("core") {
        evidence("e2e_groups core=skipped");
        return Ok(());
    }
    let before = journey_verdicts();
    let profile_result = profile_journey(clients, world, &bytes, &metadata.spawns).await;
    if let Err(error) = &profile_result {
        evidence(&format!("world_profile state=failed error={error}"));
    }
    let mut client = connect_world(clients, 0x71).await?;
    for capability in [17, 18, 151, 152, 153, 179, 230] {
        if !client.selected_capabilities().contains(&capability) {
            return Err(
                format!("qualified World did not offer journey capability {capability}").into(),
            );
        }
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("town join has no production bundle map snapshot")?;
    let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(|error| format!("town map snapshot: {error:?}"))?;
    if map.header.bundle_digest != world.base().digest() || map.tiles.is_empty() {
        return Err("town join served a different or empty World bundle".into());
    }
    // M04 can leave this Character at its Newhaven home temple after death.
    // First reach the nearest authored safe tile in the current component.
    let at = return_to_protection_zone(&mut client, world).await?;
    match reach_route(&mut client, world, at, |at| at == TOWN).await? {
        Ok(back) => {
            evidence(&format!("profile_return goal=fixture_town at={at:?}"));
            walk(&mut client, world, &back).await?;
        }
        Err(failure) => {
            let detail = home_temple_return(at, failure);
            evidence(&detail);
            // character_core's step/relogin phase plans from TOWN. Do not run it
            // (or its dependent town phases) against a different island.
            evidence(&format!(
                "e2e_all scenario=M01.step-relogin status=FAIL seconds=0 evidence={detail} cause=precondition root=profile_return/fixture-town"
            ));
            provisioning::e2ef1_logout_for_return(&mut client).await?;
            return Err(detail.into());
        }
    }
    client = character_core(client, clients, world).await?;
    client = include!("full_e2ef3_registration.rs");
    client = floor_journey(client, clients, world).await?;
    client_guild::guild(&mut client).await?;
    client_party::party(&mut client).await?;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let buddy: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_one(&mut connection)
    .await?;
    let mut coverage = client_vip::add(&mut client, &buddy).await;
    if let Err(error) = client_player_trade::pair(&mut client, clients).await {
        let logout = logout_trade_character(&mut client).await;
        drop(client);
        return finish_after_terminal_release(
            Err::<(), _>(error).and(logout),
            wait_character_released(clients, 1),
        )
        .await;
    }
    coverage = coverage.and(client_prey::prey(&mut client).await);
    coverage = coverage.and(client_task_board::task_board(&mut client).await);
    coverage = coverage.and(client_imbue::imbue(&mut client).await);
    coverage = coverage.and(client_hireling::hireling(&mut client).await);
    client_browse_field::browse_field(&mut client).await?;
    client_daily_boost::daily_boost(&mut client).await?;
    let path = route(world, TOWN, |at| near_locker(world, at))
        .ok_or("town has no reachable authored depot locker")?;
    walk(&mut client, world, &path).await?;
    client_depot_search::depot_search(&mut client).await?;
    coverage = coverage.and(client_stash::stash(&mut client).await);
    let market_funding: TestResult = async {
        let required = crate::durability::market::placement_fee(1, 1)
            .map_err(|error| format!("market fee: {error:?}"))?
            + 1;
        if client_market::market_bank_balance(clients).await? < required {
            full_e2ef20::fund_market_account(&mut client, world, &metadata.npcs).await?;
            let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
                near_locker(world, at)
            })
            .ok_or("market funding has no return route to the depot")?;
            walk(&mut client, world, &path).await?;
        }
        if client_market::market_bank_balance(clients).await? < required {
            return Err("core market deposit did not fund fee and escrow".into());
        }
        client_market::market(&mut client, clients).await
    }
    .await;
    coverage = coverage.and(market_funding);
    coverage = coverage.and(shop_journey(&mut client, world, &metadata.npcs, clients).await);
    client_hotkey_equip::hotkey_equip(&mut client).await?;
    coverage = coverage.and(client_item_forge::item_forge(&mut client).await);
    coverage = coverage.and(client_store::store(&mut client).await);
    let after = destination(
        world,
        tile(client.world_spatial().actor_position)?,
        StepDirection::West,
    )
    .map(|at| ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    });
    client = client_disconnect_resume::recover(client, clients, after).await?;
    let loot_deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    client_readmission::within_retry_budget(loot_deadline, async {
        loop {
            let rat =
                travel_to_spawn_target(&mut client, world, &metadata.spawns, "oteryn:creature.rat")
                    .await?;
            match client_hunt::hunt_target(&mut client, Some(rat)).await {
                Err(error) if error.is::<client_hunt::EmptyLoot>() => {}
                result => break result,
            }
        }
    })
    .await?;
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.poison_spider",
    )
    .await?;
    client_death::death_by(&mut client, clients, Some("oteryn:creature.poison_spider")).await?;
    let respawn = client.world_spatial().actor_position;
    let old_session = client.join_snapshot().game_session_id;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let logout = client_readmission::within_retry_budget(deadline, async {
            Ok(client.session_mut().logout().await?)
        })
        .await?;
        if logout.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("death relog logout was not accepted: {logout:?}").into());
        }
        client_readmission::within_retry_budget(deadline, async {
            Ok(client.service_liveness(Duration::from_millis(100)).await?)
        })
        .await?;
        client.take_events();
    }
    drop(client);
    client = connect_world(clients, 0x73).await?;
    let snapshot_result: TestResult = (|| {
        if client.join_snapshot().game_session_id == old_session
            || client.world_spatial().actor_position != respawn
            || client
                .actor_vitals()
                .is_none_or(|vitals| vitals.health != vitals.max_health)
        {
            return Err(
                "death relog did not retain the saved respawn position and full vitals".into(),
            );
        }
        evidence(&format!(
            "mechanic=death state=relogged position={respawn:?} health={}",
            client.actor_vitals().ok_or("death relog vitals")?.health
        ));

        Ok(())
    })();
    let logout_result: TestResult = async {
        let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(&mut client).await?;
        if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("final core logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
        }
        Ok(())
    }.await;
    drop(client);
    finish_after_terminal_release(
        profile_result
            .and(coverage)
            .and(snapshot_result)
            .and(logout_result),
        wait_character_released(clients, 1),
    )
    .await?;
    journey_group_completed("core", before);
    Ok(())
}

fn session_unexpected_eof(error: &(dyn std::error::Error + 'static)) -> bool {
    std::iter::successors(Some(error), |cause| cause.source()).any(|cause| {
        matches!(cause.downcast_ref::<oteryn_dev_client::SessionError>(),
            Some(oteryn_dev_client::SessionError::Io(io)) if io.kind() == io::ErrorKind::UnexpectedEof)
    })
}

#[test]
fn idle_removal_recognizes_contextual_session_eof() {
    use oteryn_dev_client::{DevClientError, SessionError};
    let wrapped = |kind| DevClientError::Context {
        phase: "liveness",
        elapsed: Duration::from_secs(960),
        last_frame: String::new(),
        source: Box::new(DevClientError::Session(SessionError::Io(io::Error::from(
            kind,
        )))),
    };
    assert!(session_unexpected_eof(&wrapped(
        io::ErrorKind::UnexpectedEof
    )));
    assert!(!session_unexpected_eof(&wrapped(
        io::ErrorKind::ConnectionReset
    )));
}

fn idle_removal_proven(elapsed: Duration, eof: bool, row: Option<&SessionLossRow>) -> bool {
    eof && (955..=1000).contains(&elapsed.as_secs())
        && row
            .is_some_and(|(state, epoch, grace, _)| *state == 3 && epoch.is_empty() && *grace == -1)
}

#[test]
fn idle_removal_requires_on_time_owner_release_without_a_loss_epoch() {
    let terminal = (3, String::new(), -1, None);
    let disconnect = (3, String::from("1"), 60, Some(0));
    assert!(!idle_removal_proven(
        Duration::from_secs(902),
        true,
        Some(&terminal)
    ));
    assert!(!idle_removal_proven(
        Duration::from_secs(962),
        false,
        Some(&terminal)
    ));
    assert!(!idle_removal_proven(
        Duration::from_secs(962),
        true,
        Some(&disconnect)
    ));
    assert!(idle_removal_proven(
        Duration::from_secs(962),
        true,
        Some(&terminal)
    ));
}

async fn profile_journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    bytes: &[u8],
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    // Catalogue classification takes several seconds in a debug build. Prepare it
    // before admission so the peer can keep acknowledging probes during the journey.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let model = map_journey::MapModel::load(world, bytes, &root)?;
    let client = connect_character(clients, 0x6e, 0).await?;
    let mut client = provisioning::provision(client, clients, 0).await?;
    use client_spell_journey::ProtectionProbe;
    let (words, target_intent) = client_spell_journey::fixture_protection_spell()?;
    let pz = if world
        .base()
        .tile(TOWN.x, TOWN.y, TOWN.floor)
        .is_some_and(|tile| tile.protection_zone())
    {
        client_spell_journey::protection_zone_probe(
            &mut client,
            ProtectionProbe::Inside,
            words,
            target_intent,
        )
        .await
    } else {
        Err("configured town start is not a bundle PZ".into())
    };
    let map = map_journey::run(&mut client, world, &model).await;
    let combat =
        client_combat_journey::combat_journey(&mut client, world, spawns, clients.url).await;
    let outside: TestResult = async {
        let from = tile(client.world_spatial().actor_position)?;
        let path = reach_route(&mut client, world, from, |at| {
            outside_protection_zone(world, at)
        })
        .await?
        .map_err(|error| format!("profile has no route outside protection: {error}"))?;
        walk(&mut client, world, &path).await?;
        if !outside_protection_zone(world, tile(client.world_spatial().actor_position)?) {
            return Err("profile protection comparison did not reach a non-PZ tile".into());
        }
        client_spell_journey::protection_zone_probe(
            &mut client,
            ProtectionProbe::Outside,
            words,
            target_intent,
        )
        .await
    }
    .await;
    let pz = pz.and(outside);
    let spells = client_spell_journey::spell_journey(&mut client, pz).await;
    // Combat may still hold the logout/PZ lock. Settle it outside the temple
    // before restoring the peer that main's trade/party phases use.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let logout = client_readmission::within_retry_budget(deadline, async {
            Ok(client.session_mut().logout().await?)
        })
        .await?;
        if logout.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("profile combat lock did not settle before logout: {logout:?}").into(),
            );
        }
        client_readmission::within_retry_budget(deadline, async {
            Ok(client.service_liveness(Duration::from_millis(100)).await?)
        })
        .await?;
        client.take_events();
    }
    drop(client);
    client = connect_character(clients, 0x6f, 0).await?;
    let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("profile journey has no return route")?;
    walk(&mut client, world, &back).await?;
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("profile journey logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    map?;
    combat?;
    spells
}

async fn connect_world(
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    connect_character(clients, tag, 1).await
}

fn fresh_grant_material(tag: u8) -> TestResult<([u8; 32], [u8; 16])> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_CONNECTION: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT_CONNECTION
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| "qualification connection counter exhausted")?;
    let mut nonce = [tag; 32];
    nonce[24..].copy_from_slice(&sequence.to_be_bytes());
    let mut attempt = v7(tag, 0x05);
    attempt[8..].copy_from_slice(&sequence.to_be_bytes());
    // Preserve all 64 counter bits around the UUIDv7 version and RFC variant.
    attempt[6] |= (sequence >> 62) as u8;
    attempt[8] = (attempt[8] & 0x3f) | 0x80;
    Ok((nonce, attempt))
}

const FIXTURE_COMMAND_DEADLINE: Duration = Duration::from_secs(30);

fn fixture_command() -> tokio::process::Command {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/node_boot/provision.py");
    let mut command = tokio::process::Command::new("timeout");
    command.kill_on_drop(true).stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        // Run the inner timeout with the same credentials as Python so its
        // signal reaches the privileged child even if the outer probe drops.
        .args(["--kill-after=2s", "30s", "sudo", "-n", "timeout", "--kill-after=2s", "25s", "python3"])
        .arg(script).arg("--watch-stdin");
    command
}

async fn run_fixture_command(
    command: &mut tokio::process::Command,
) -> io::Result<std::process::Output> {
    let mut child = command.spawn()?;
    let parent_pipe = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("fixture cancellation pipe missing"))?;
    if FIXTURE_WORKERS.try_with(|_| ()).is_err() {
        let output = child.wait_with_output().await;
        drop(parent_pipe);
        return output;
    }
    let (done, worker_done) = tokio::sync::oneshot::channel();
    FIXTURE_WORKERS.with(|workers| workers.borrow_mut().push(worker_done));
    // A cancelled row closes the child's parent pipe while its cleanup still
    // waits for process exit through the registered completion receiver.
    let task = tokio::spawn(async move {
        let output = child.wait_with_output().await;
        let _ = done.send(());
        output
    });
    let output = task.await?;
    drop(parent_pipe);
    output
}

pub(super) async fn observed_recovery_replay(session: [u8; 16], epoch: u64) -> TestResult<bool> {
    let mut command = fixture_command();
    command.args([
        "--socket",
        &required("NODE_BOOT_CONTROL_SOCKET")?,
        "--session",
        &uuid_text(&session),
        "--recovery-epoch",
        &epoch.to_string(),
    ]);
    let output =
        tokio::time::timeout(FIXTURE_COMMAND_DEADLINE, run_fixture_command(&mut command)).await??;
    Ok(output.status.success())
}

pub(super) async fn observed_item_instance(
    session: [u8; 16],
    handle: oteryn_protocol_oteryn::item_view::ItemHandle,
) -> TestResult<Option<[u8; 16]>> {
    let mut command = fixture_command();
    command.args([
        "--socket",
        &required("NODE_BOOT_CONTROL_SOCKET")?,
        "--session",
        &uuid_text(&session),
        "--item-handle",
        &handle.to_string(),
    ]);
    let output =
        tokio::time::timeout(FIXTURE_COMMAND_DEADLINE, run_fixture_command(&mut command)).await??;
    if !output.status.success() {
        return Ok(None);
    }
    let id = std::str::from_utf8(&output.stdout)?
        .trim()
        .strip_prefix("instance ")
        .ok_or("invalid fixture item observation")?;
    crate::node::uuid_bytes(id)
        .map(Some)
        .ok_or_else(|| "invalid observed item UUID".into())
}

tokio::task_local! {
    pub(super) static FIXTURE_WORKERS: std::cell::RefCell<Vec<tokio::sync::oneshot::Receiver<()>>>;
}

pub(super) async fn await_fixture_workers() {
    std::future::poll_fn(|cx| {
        FIXTURE_WORKERS.with(|workers| {
            let mut workers = workers.borrow_mut();
            while let Some(worker) = workers.last_mut() {
                if std::future::Future::poll(std::pin::Pin::new(worker), cx).is_pending() {
                    return std::task::Poll::Pending;
                }
                workers.pop();
            }
            std::task::Poll::Ready(())
        })
    })
    .await;
}

async fn fixture_timeout<T>(
    limit: Duration,
    operation: impl std::future::Future<Output = T>,
) -> Result<T, tokio::time::error::Elapsed> {
    let run = async {
        let result = tokio::time::timeout(limit, operation).await;
        await_fixture_workers().await;
        result
    };
    if FIXTURE_WORKERS.try_with(|_| ()).is_ok() {
        run.await
    } else {
        FIXTURE_WORKERS.scope(Default::default(), run).await
    }
}

#[test]
fn qualification_encounter_timeout_waits_for_fixture_completion() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let (release, gate) = std::sync::mpsc::sync_channel(1);
            let mut encounter = Box::pin(fixture_timeout(Duration::from_secs(1), async {
                let _task = FixtureCommand::start(move || {
                    gate.recv_timeout(Duration::from_secs(2)).map_err(|error| error.to_string())
                })?;
                std::future::pending::<TestResult>().await
            }));
            tokio::select! {
                biased;
                _ = &mut encounter => return Err("encounter completed before deadline".into()),
                () = tokio::task::yield_now() => {}
            }
            tokio::time::advance(Duration::from_secs(2)).await;
            tokio::select! {
                biased;
                _ = &mut encounter => return Err("encounter released a pending fixture worker".into()),
                () = tokio::task::yield_now() => {}
            }
            release.send(())?;
            assert!(encounter.await.is_err());
            Ok(())
        })
}

#[cfg(test)]
pub(super) struct FixtureCommand<T> {
    result: tokio::sync::oneshot::Receiver<Result<T, String>>,
}

#[cfg(test)]
impl<T: Send + 'static> FixtureCommand<T> {
    pub(super) fn start(
        command: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> std::io::Result<Self> {
        let (send, result) = tokio::sync::oneshot::channel();
        let (done, worker_done) = tokio::sync::oneshot::channel();
        std::thread::Builder::new().spawn(move || {
            let _ = send.send(command());
            let _ = done.send(());
        })?;
        let _ = FIXTURE_WORKERS.try_with(|workers| workers.borrow_mut().push(worker_done));
        Ok(Self { result })
    }
}

async fn wait_fixture_command<T>(
    task: impl Future<Output = TestResult<T>>,
    mut service: impl AsyncFnMut() -> TestResult,
) -> TestResult<T> {
    tokio::time::timeout(FIXTURE_COMMAND_DEADLINE, async {
        let mut task = pin!(task);
        loop {
            // Poll the process between complete liveness operations. A process
            // exit cannot cancel a frame that the liveness read already started.
            let result = poll_fn(|context| {
                Poll::Ready(match task.as_mut().poll(context) {
                    Poll::Ready(result) => Some(result),
                    Poll::Pending => None,
                })
            })
            .await;
            if let Some(result) = result {
                return result;
            }
            if let Err(error) = service().await {
                // Wait for the fenced request while the same total timer still runs.
                task.await?;
                return Err(error);
            }
        }
    })
    .await?
}

#[test]
fn qualification_fixture_command_stops_waiting_for_a_hung_child_after_loss() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let task = async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(())
            };
            let result = tokio::time::timeout(
                Duration::from_secs(31),
                wait_fixture_command(task, async || Err("transport lost".into())),
            )
            .await?;
            assert!(result.is_err());
            Ok(())
        })
}

#[cfg(test)]
#[test]
fn qualification_fixture_command_services_liveness_and_joins_after_loss() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let writer_finished = finished.clone();
            let (release, gate) = std::sync::mpsc::sync_channel(1);
            let task = FixtureCommand::start(move || -> Result<(), String> {
                gate.recv().map_err(|error| error.to_string())?;
                writer_finished.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })?;
            let mut serviced = 0;
            let result = wait_fixture_command(async { Ok(task.result.await??) }, async || {
                serviced += 1;
                release.send(())?;
                Err("transport lost".into())
            })
            .await;
            assert!(result.is_err());
            assert_eq!(serviced, 1);
            assert!(
                finished.load(std::sync::atomic::Ordering::SeqCst),
                "fixture writer detached after transport loss"
            );
            Ok(())
        })
}

pub(super) async fn connect_character(
    clients: &SeamClients<'_>,
    tag: u8,
    index: usize,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    wait_character_released(clients, index).await?;
    verdict::Harness::global().progress("admission");
    let generation = platform_generation(clients.descriptor, &clients.accounts[index]).await?;
    let (nonce, attempt) = fresh_grant_material(tag)?;
    let grant = Grant {
        signing: clients.signing,
        key_id: clients.key_id,
        account_id: &clients.accounts[index],
        character_id: clients.characters[index],
        world_id: clients.world,
        channel_id: clients.channel,
        security_generation: generation,
        scope_generation: clients.scope_generation,
        nonce,
        attempt,
    };
    let client = oteryn_dev_client::connect_session_with_admission(
        oteryn_dev_client::JoinRequest {
            address: clients.address,
            server_name: "localhost",
            root_certificate: clients.certificate,
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&clients.characters[index])?,
            admission_material: &[],
            client_build_id: "oteryn-dev-client/world-qualification",
            deadline: Duration::from_secs(20),
        },
        || {
            let issued = now_seconds().map_err(|error| {
                oteryn_dev_client::DevClientError::Io(std::io::Error::other(error.to_string()))
            })?;
            Ok(sign_grant(&grant, issued).into_bytes())
        },
    )
    .await?;
    verdict::Harness::global().progress("admission");
    Ok(client)
}

pub(super) fn has_display_name(
    entity: &oteryn_dev_client::WorldSpatialEntity,
    expected: &str,
) -> bool {
    entity
        .display_name
        .as_ref()
        .and_then(|name| name.as_str().ok())
        == Some(expected)
}

fn nearest_spawn<'a>(
    entities: impl Iterator<Item = &'a oteryn_dev_client::WorldSpatialEntity>,
    here: ActorPosition,
    name: &str,
) -> Option<&'a oteryn_dev_client::WorldSpatialEntity> {
    entities
        .filter(|entry| {
            entry.position.floor == here.floor && full_gold::creature_matches(entry, name)
        })
        .min_by_key(|entry| {
            entry
                .position
                .x
                .abs_diff(here.x)
                .max(entry.position.y.abs_diff(here.y))
        })
}

#[test]
fn qualification_spawn_prefers_requested_species_over_nearer_creature() -> TestResult {
    use oteryn_dev_client::{EntityDetail, EntityKind, EntityRef, WorldSpatialEntity};
    use oteryn_protocol_oteryn::entity_names::EntityDisplayName;
    let here = ActorPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let entity = |id, name, x| -> TestResult<WorldSpatialEntity> {
        Ok(WorldSpatialEntity {
            entity: EntityRef {
                identity: [id; 16],
                generation: 1,
            },
            kind: EntityKind::Creature,
            display_name: Some(
                EntityDisplayName::new(name).map_err(|error| format!("test name: {error:?}"))?,
            ),
            position: ActorPosition { x, ..here },
            detail: EntityDetail::Actor {
                direction: StepDirection::North,
                appearance_ref: 1,
                health_percent: 100,
            },
        })
    };
    let other = entity(1, "Poison Spider", 101)?;
    let rat = entity(2, "Rat", 102)?;
    assert_eq!(
        nearest_spawn([&other, &rat].into_iter(), here, "Rat"),
        Some(&rat)
    );
    assert_eq!(nearest_spawn([&other].into_iter(), here, "Rat"), None);
    Ok(())
}

async fn travel_to_spawn(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    creature: &str,
) -> TestResult {
    travel_to_spawn_target(client, world, spawns, creature)
        .await
        .map(|_| ())
}

#[cfg(test)]
fn spawn_adjacent(at: TilePos, point: TilePos) -> bool {
    at.floor == point.floor && at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) <= 1
}

fn outside_protection_zone(world: &BundleWorld, at: TilePos) -> bool {
    world
        .base()
        .tile(at.x, at.y, at.floor)
        .is_some_and(|tile| tile.walkable() && !tile.protection_zone())
        && world.enterable(at)
}

#[cfg(test)]
async fn seek_live_spawn<T>(
    mut remaining: BTreeSet<TilePos>,
    mut observe: impl AsyncFnMut(&BTreeSet<TilePos>) -> TestResult<Option<(TilePos, Option<T>)>>,
) -> TestResult<Option<T>> {
    while !remaining.is_empty() {
        let Some((at, live)) = observe(&remaining).await? else {
            return Ok(None);
        };
        if live.is_some() {
            return Ok(live);
        }
        remaining.retain(|point| !spawn_adjacent(at, *point));
    }
    Ok(None)
}

#[tokio::test]
async fn lethal_modes_do_not_wait_on_an_exhausted_authored_spawn() -> TestResult {
    let depleted = TilePos {
        x: 100,
        y: 100,
        floor: -7,
    };
    let available = TilePos { x: 110, ..depleted };
    let mut visits = Vec::new();
    let live = seek_live_spawn(BTreeSet::from([depleted, available]), async |remaining| {
        let at = *remaining.first().ok_or("missing available point")?;
        visits.push(at);
        Ok(Some((at, (at == available).then_some(42))))
    })
    .await?;
    assert_eq!(live, Some(42));
    assert_eq!(visits, [depleted, available]);
    Ok(())
}

pub(super) async fn travel_to_spawn_target(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    creature: &str,
) -> TestResult<oteryn_dev_client::EntityRef> {
    client_readmission::within_retry_budget(
        tokio::time::Instant::now() + Duration::from_secs(120),
        async {
            approach_spawn_target(client, world, spawns, creature, SpawnRespawn::Wait, None)
                .await?
                .map(|target| target.entity)
                .ok_or_else(|| format!("no reachable live {creature}").into())
        },
    )
    .await
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SpawnRespawn {
    Skip,
    Wait,
}

async fn approach_spawn_target(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    creature: &str,
    respawn_wait: SpawnRespawn,
    probe_deadline: Option<tokio::time::Instant>,
) -> TestResult<Option<oteryn_dev_client::WorldSpatialEntity>> {
    let (name, _) = full_gold::creature_profile(creature)?;
    let authored: BTreeMap<_, _> = spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|entry| entry.key == creature)
        })
        .map(|(floor, point)| {
            (
                TilePos {
                    x: point.x,
                    y: point.y,
                    floor,
                },
                point.respawn_ms,
            )
        })
        .collect();
    if authored.is_empty() {
        return Ok(None);
    }
    let outside = |at| outside_protection_zone(world, at);
    let near = |at: TilePos, target: TilePos| {
        at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) == 1
    };
    let mut remaining_lanes = authored;
    let mut blocked_edges = BTreeSet::new();
    let mut last_respawn = None;
    let mut search = true;
    let mut chase_steps = 0;
    let mut deadline = tokio::time::Instant::now();
    loop {
        if probe_expired(probe_deadline) {
            return Err(SpawnProbeExpired.into());
        }
        if search {
            chase_steps = 0;
            let from = actor_tile(client)?;
            let path = reach_route_until(
                client,
                world,
                from,
                |at| {
                    outside(at)
                        && (-1i16..=1).any(|dx| {
                            (-1i16..=1).any(|dy| {
                                (dx != 0 || dy != 0)
                                    && at
                                        .x
                                        .checked_add_signed(dx)
                                        .zip(at.y.checked_add_signed(dy))
                                        .is_some_and(|(x, y)| {
                                            remaining_lanes.contains_key(&TilePos {
                                                x,
                                                y,
                                                floor: at.floor,
                                            })
                                        })
                            })
                        })
                },
                probe_deadline,
                &blocked_edges,
            )
            .await?;
            if let Ok(path) = path {
                match walk_until(
                    client,
                    world,
                    &path,
                    probe_deadline,
                    probe_deadline.is_some(),
                )
                .await
                {
                    Err(error) if error.is::<BlockedSpawnEdge>() => {
                        let edge = error.downcast::<BlockedSpawnEdge>()?;
                        blocked_edges.insert((edge.0, edge.1));
                        continue;
                    }
                    result => result?,
                }
                let here = actor_tile(client)?;
                let visited: Vec<_> = remaining_lanes
                    .keys()
                    .copied()
                    .filter(|lane| near(here, *lane))
                    .collect();
                if visited.is_empty() {
                    return Err("spawn route did not arrive beside an authored lane".into());
                }
                last_respawn = visited
                    .into_iter()
                    .filter_map(|lane| remaining_lanes.remove(&lane))
                    .max();
                deadline = tokio::time::Instant::now() + Duration::from_millis(500);
                search = false;
            } else if respawn_wait == SpawnRespawn::Wait
                && let Some(respawn) = last_respawn.take()
            {
                remaining_lanes.clear();
                deadline = named_respawn_deadline(
                    spawn_observation_budget(std::iter::once(respawn)),
                    |until| verdict::Harness::global().bounded_wait(until, "named-spawn-respawn"),
                );
                search = false;
            } else {
                return Ok(None);
            }
        }
        let basis = spatial_basis::current(client)?;
        let here = actor_tile(client)?;
        let target = client
            .world_entities()
            .and_then(|entities| {
                nearest_spawn(entities.iter(), client.world_spatial().actor_position, name)
            })
            .copied();
        let target = target
            .map(|target| served_tile(target.position, basis).map(|at| (target, at)))
            .transpose()?;
        if let Some((target, at)) = target
            && near(here, at)
            && outside(here)
        {
            return Ok(Some(target));
        }
        if chase_steps == 32 && !remaining_lanes.is_empty()
            || tokio::time::Instant::now() >= deadline
        {
            if !remaining_lanes.is_empty() {
                search = true;
                continue;
            }
            if respawn_wait == SpawnRespawn::Wait
                && let Some(respawn) = last_respawn.take()
            {
                deadline = named_respawn_deadline(
                    spawn_observation_budget(std::iter::once(respawn)),
                    |until| verdict::Harness::global().bounded_wait(until, "named-spawn-respawn"),
                );
                chase_steps = 0;
            } else {
                return Ok(None);
            }
        }
        if chase_steps < 32
            && let Some((_, at)) = target
            && let Some(direction) = route(world, here, |position| {
                outside(position) && near(position, at)
            })
            .and_then(|path| path.first().copied())
        {
            chase_steps += 1;
            let deadline = probe_deadline.map_or(deadline, |limit| deadline.min(limit));
            match walk_until(client, world, &[direction], Some(deadline), true).await {
                Err(error) if error.is::<BlockedSpawnEdge>() || error.is::<SpawnProbeExpired>() => {
                    chase_steps = 32;
                }
                result => result?,
            }
            continue;
        }
        let observation_deadline = probe_deadline.map_or(deadline, |limit| deadline.min(limit));
        match wait_for_spawn_observation(
            verdict::Harness::global(),
            observation_deadline,
            probe_liveness(client, Duration::from_millis(100), probe_deadline),
        )
        .await
        {
            Err(error) if error.is::<SpawnProbeExpired>() => return Err(error),
            result => result?,
        }
        client.take_events();
    }
}

async fn approach_visible_spawn_target(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    name: &str,
    deadline: tokio::time::Instant,
    mut work_deadline: Option<&mut tokio::time::Instant>,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let mut steps = 0;
    let mut phase = "spawn-observation";
    loop {
        action_deadline(deadline, phase)?;
        let here = client.world_spatial().actor_position;
        let Some(target) = client
            .world_entities()
            .and_then(|entities| nearest_spawn(entities.iter(), here, name))
        else {
            if steps == 32 {
                break;
            }
            phase = "spawn-observation";
            let started = tokio::time::Instant::now();
            wait_for_spawn_observation(
                verdict::Harness::global(),
                deadline,
                approach_action(client, deadline, phase, async |client| {
                    client.service_liveness(Duration::from_millis(100)).await?;
                    Ok(())
                }),
            )
            .await?;
            if let Some(work_deadline) = work_deadline.as_deref_mut() {
                account_for_spawn_wait(work_deadline, started);
            }
            client.take_events();
            continue;
        };
        let entity = target.entity;
        let target = tile(target.position)?;
        let path = route(world, tile(here)?, |at| {
            at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 1
        })
        .ok_or("visible creature unreachable")?;
        let Some(direction) = path.first() else {
            verdict::Harness::global().progress("spawn target ready");
            return Ok(entity);
        };
        if steps == 32 {
            break;
        }
        let step_deadline = work_deadline
            .as_deref()
            .map_or(deadline, |work| deadline.min(*work));
        phase = "spawn-approach-step";
        verdict::Harness::global().progress(phase);
        approach_action(client, step_deadline, phase, async |client| {
            walk(client, world, &[*direction]).await
        })
        .await?;
        steps += 1;
    }
    Err("could not approach a live creature within the route budget".into())
}

fn named_respawn_deadline(
    timeout: Duration,
    register: impl FnOnce(tokio::time::Instant),
) -> tokio::time::Instant {
    let deadline = tokio::time::Instant::now() + timeout;
    register(deadline);
    deadline
}

fn spawn_observation_budget(respawn_ms: impl Iterator<Item = u32>) -> Duration {
    let authored = Duration::from_millis(u64::from(respawn_ms.max().unwrap_or_default()));
    // Leave room for spawn admission and the next visibility update.
    Duration::from_secs(75).max(authored + Duration::from_secs(5))
}

struct PendingExchange<'a, T, F: FnOnce(&mut T)> {
    client: &'a mut T,
    invalidate: Option<F>,
}

impl<T, F: FnOnce(&mut T)> Drop for PendingExchange<'_, T, F> {
    fn drop(&mut self) {
        if let Some(invalidate) = self.invalidate.take() {
            invalidate(self.client);
        }
    }
}

fn action_deadline(deadline: tokio::time::Instant, phase: &'static str) -> TestResult {
    if tokio::time::Instant::now() >= deadline {
        return Err(verdict::HarnessError::boxed(
            verdict::Cause::Deadline { phase },
            format!("{phase} deadline expired"),
        ));
    }
    Ok(())
}

async fn timed_exchange<T, R>(
    client: &mut T,
    deadline: tokio::time::Instant,
    phase: &'static str,
    invalidate: impl FnOnce(&mut T),
    operation: impl AsyncFnOnce(&mut T) -> R,
) -> TestResult<R> {
    action_deadline(deadline, phase)?;
    let mut pending = PendingExchange {
        client,
        invalidate: Some(invalidate),
    };
    let result = tokio::time::timeout_at(deadline, operation(&mut *pending.client)).await;
    if result.is_ok() {
        pending.invalidate = None;
    }
    // The operation can complete on the same poll as the timer. In that case the
    // exchange is complete, but its result arrived too late for qualification.
    action_deadline(deadline, phase)?;
    Ok(result?)
}

async fn approach_action<R>(
    client: &mut oteryn_dev_client::DevClientSession,
    deadline: tokio::time::Instant,
    phase: &'static str,
    operation: impl AsyncFnOnce(&mut oteryn_dev_client::DevClientSession) -> TestResult<R>,
) -> TestResult<R> {
    timed_exchange(
        client,
        deadline,
        phase,
        |client| client.session_mut().invalidate_interrupted_exchange(),
        operation,
    )
    .await?
}

fn account_for_spawn_wait(work_deadline: &mut tokio::time::Instant, started: tokio::time::Instant) {
    *work_deadline += started.elapsed();
}

async fn wait_for_spawn_observation(
    harness: &verdict::Harness,
    deadline: tokio::time::Instant,
    operation: impl std::future::Future<Output = TestResult>,
) -> TestResult {
    harness.bounded_wait(deadline, "spawn observation");
    operation.await
}

fn npc_display_name(key: &str) -> TestResult<String> {
    static NAMES: std::sync::OnceLock<Result<BTreeMap<String, String>, String>> =
        std::sync::OnceLock::new();
    let names = NAMES
        .get_or_init(|| {
            let load = || -> TestResult<BTreeMap<String, String>> {
                let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
                let index: serde_json::Value =
                    serde_json::from_slice(&fs::read(root.join("npcs/definitions/index.json"))?)?;
                let mut names = BTreeMap::new();
                for shard in index["shards"]
                    .as_array()
                    .ok_or("NPC definition index has no shards")?
                {
                    let path = shard
                        .as_str()
                        .ok_or("invalid NPC shard")?
                        .strip_prefix("content/")
                        .ok_or("NPC shard leaves content")?;
                    let shard: serde_json::Value =
                        serde_json::from_slice(&fs::read(root.join(path))?)?;
                    for record in shard["records"]
                        .as_array()
                        .ok_or("NPC shard has no records")?
                    {
                        let declaration = &record["declaration"];
                        let Some(key) = declaration["identity"]["key"].as_str() else {
                            continue;
                        };
                        let Some(fields) = declaration["fields"].as_array() else {
                            continue;
                        };
                        if let Some(name) = fields
                            .iter()
                            .filter(|field| field["field_path"] == "oteryn:source.npc.display_name")
                            .find_map(|field| field["value"]["value"].as_str())
                        {
                            names.insert(key.to_owned(), name.to_owned());
                        }
                    }
                }
                Ok(names)
            };
            load().map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| error.clone())?;
    names
        .get(key)
        .cloned()
        .ok_or_else(|| format!("{key} has no authored display name").into())
}

pub(super) fn creature_display_name(key: &str) -> TestResult<String> {
    static NAMES: std::sync::OnceLock<Result<BTreeMap<String, String>, String>> =
        std::sync::OnceLock::new();
    let names = NAMES
        .get_or_init(|| {
            let load = || -> TestResult<BTreeMap<String, String>> {
                let bytes = &native_input()?.creature_profiles.bytes;
                let content: serde_json::Value = serde_json::from_slice(bytes)?;
                let records = content["records"]
                    .as_array()
                    .ok_or("no pinned creature profile records")?;
                Ok(records
                    .iter()
                    .filter_map(|record| {
                        let profile = &record["profile"];
                        let key = profile["target"]["key"].as_str()?;
                        let name =
                            profile["data"]["profile"]["details"]["display_name"].as_str()?;
                        Some((key.to_owned(), name.to_owned()))
                    })
                    .collect())
            };
            load().map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| error.clone())?;
    names
        .get(key)
        .cloned()
        .ok_or_else(|| format!("{key} has no pinned display name").into())
}

pub(crate) fn smoke() -> TestResult {
    smoke_with(Some(&[(126, 125), (126, 126), (127, 127)]), |_| Ok(()))
}

fn smoke_with(
    regions: Option<&[(u16, u16)]>,
    check: impl FnOnce(&BundleWorld) -> TestResult,
) -> TestResult {
    use oteryn_world_bundle_compiler::{bundle::BuildClass, compile, source};
    let started = std::time::Instant::now();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut project = match regions {
        Some(regions) => source::project_regions(&root, regions)?,
        None => source::project(&root)?,
    };
    if regions.is_some() {
        project.families.subareas.select_default_town(8, regions)?;
        project
            .families
            .subareas
            .validate_default_town_tile(&project.regions)?;
    }
    let (registry, palette) = source::registry(&root, &project)?;
    let identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let input = compile::Input {
        regions: &project.regions,
        palette: &palette,
        identity,
        world: source::world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: oteryn_world_bundle_compiler::project::draft_areas(&project.index)?,
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    compile::equivalence(&input, &registry, &compiled.bytes)?;
    let compiled_ms = started.elapsed().as_millis();
    let boot_started = std::time::Instant::now();
    let mut pin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
    pin["digest"] = compiled
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .into();
    // This smoke may select a different bundle from other tests in this process.
    let world = boot_world(
        compiled.bytes,
        &pin,
        WorldId::decode(&v7(1, 2))?,
        ChannelId::decode(&v7(1, 3))?,
    )?;
    let world = &world;
    assert!(
        world
            .base()
            .tile(TOWN.x, TOWN.y, TOWN.floor)
            .is_some_and(|tile| tile.protection_zone()),
        "town temple must be a protection zone"
    );
    assert!(
        route(world, TOWN, |at| near_locker(world, at)).is_some(),
        "town depot must be reachable"
    );
    let floor_path = route(world, TOWN, |at| at.floor != TOWN.floor)
        .ok_or("town floor-change route must be reachable")?;
    let mut landing = TOWN;
    for direction in floor_path {
        landing = destination(world, landing, direction).ok_or("floor route landing")?;
    }
    assert_ne!(landing.floor, TOWN.floor);
    assert!(
        route(world, landing, |at| at == TOWN).is_some(),
        "town floor-change route must have a return path"
    );
    check(world)?;
    let rats: Vec<_> = project
        .families
        .spawns
        .iter()
        .flat_map(|source| &source.points)
        .filter(|point| point.creature == "oteryn:creature.rat")
        .map(|point| point.cell)
        .collect();
    assert!(
        route(world, TOWN, |at| rats.iter().any(|&(x, y, z)| i16::from(
            at.floor
        )
            == -i16::from(z)
            && at.x.abs_diff(x).max(at.y.abs_diff(y)) <= 1))
        .is_some(),
        "town hunt must reach an authored rat with an admitted corpse and loot table"
    );
    evidence(&format!(
        "world_loader_smoke state=booted compile_ms={compiled_ms} boot_ms={} duration_ms={}",
        boot_started.elapsed().as_millis(),
        started.elapsed().as_millis()
    ));
    Ok(())
}

async fn shop_journey(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    clients: &SeamClients<'_>,
) -> TestResult<()> {
    let basis = spatial_basis::current(client)?;
    let Some(npc) = npcs.npcs.iter().find(|npc| npc.key == "oteryn:npc.sam") else {
        evidence(
            "mechanic=npc_shop status=FAIL unproven=authored_shop_npc_missing content_id=oteryn:npc.sam source=content/npcs/definitions/index.json:1",
        );
        return Err("mechanic=npc_shop reason=authored_shop_npc_missing content_id=oteryn:npc.sam source=content/npcs/definitions/index.json:1".into());
    };
    let from = tile(client.world_spatial().actor_position)?;
    let Some(path) = route(world, from, |at| {
        npc.placements.iter().any(|placement| {
            placement.floor == at.floor
                && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
        })
    }) else {
        evidence(
            "mechanic=npc_shop status=FAIL unproven=no_route_to_authored_shop_within_town_budget content_id=oteryn:npc.sam source=apps/game-server/src/qualification/client_phases/world.rs:126",
        );
        return Err("mechanic=npc_shop reason=no_route_to_authored_shop_within_town_budget content_id=oteryn:npc.sam source=apps/game-server/src/qualification/client_phases/world.rs:126".into());
    };
    walk(client, world, &path).await?;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|placement| {
                        entity.position.x == i32::from(placement.x)
                            && entity.position.y == i32::from(placement.y)
                            && entity.position.floor == basis.floor(placement.floor)
                    })
            })
        })
        .copied();
    let Some(target) = target else {
        evidence(
            "mechanic=npc_shop status=FAIL unproven=authored_shop_has_no_visible_actor_at_placement content_id=oteryn:npc.sam source=crates/protocol-oteryn/src/world_spatial_entities.rs:105",
        );
        return Err("mechanic=npc_shop reason=authored_shop_has_no_visible_actor_at_placement content_id=oteryn:npc.sam source=crates/protocol-oteryn/src/world_spatial_entities.rs:105".into());
    };
    client_npc_shop::transact(client, target.entity, clients).await
}

#[cfg(test)]
#[test]
fn qualification_nbtown_checks_first_entry_before_mutating_packets() -> TestResult {
    let source = include_str!("world.rs");
    let (_, journey) = source
        .split_once("pub(super) async fn journeys(")
        .ok_or("missing world journey")?;
    let (journey, _) = journey
        .split_once("async fn profile_journey(")
        .ok_or("missing profile journey boundary")?;
    let entry_check = journey
        .find("if entered_at != TOWN {")
        .ok_or("missing strict Platform-created town assertion")?;
    for packet in [
        "include!(\"full_e2ef6_mount.rs\")",
        "full_e2ef5::run(clients",
        "include!(\"full_e2ef4_mount.rs\")",
        "profile_journey(clients",
    ] {
        let packet_start = journey.find(packet).ok_or("missing mutating packet")?;
        assert!(
            entry_check < packet_start,
            "town entry checked after {packet}"
        );
    }
    let mount = include_str!("full_e2ef6_mount.rs");
    let starter = mount
        .find("full_e2ef6::run_starter(clients)")
        .ok_or("missing pristine starter row")?;
    let packet = mount
        .find("full_e2ef7::run_packet(clients,")
        .ok_or("missing F7 packet")?;
    let remaining = mount
        .find("full_e2ef6::run(clients")
        .ok_or("missing remaining F6 rows")?;
    assert!(starter < packet && packet < remaining);
    Ok(())
}

#[cfg(test)]
#[test]
fn qualification_nbfix_fresh_grants_for_reconnects_and_colliding_tags() -> TestResult {
    let mut nonces = std::collections::BTreeSet::new();
    let mut attempts = std::collections::BTreeSet::new();
    for tag in [0xe0, 0xa1, 0xa0, 0xe0, 0xa1, 0xa0] {
        let (nonce, attempt) = fresh_grant_material(tag)?;
        assert!(nonces.insert(nonce), "reused grant nonce for tag {tag}");
        assert!(attempts.insert(attempt), "reused attempt for tag {tag}");
        assert_eq!(nonce[0], tag);
        assert_eq!(attempt[7], tag);
        assert_eq!(attempt[6] >> 4, 7);
        assert_eq!(attempt[8] >> 6, 2);
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn qualification_pursuit_retries_stop_before_lane_expiry() {
    let now = tokio::time::Instant::now();
    let delay = Duration::from_millis(50);
    let limit = now + Duration::from_millis(500);
    assert!(retry_fits_probe(now, Some(limit), delay));
    assert!(!retry_fits_probe(limit - delay, Some(limit), delay));
    assert!(!retry_fits_probe(limit, Some(limit), delay));
    assert!(retry_fits_probe(limit, None, delay));
}

#[test]
fn qualification_nbroute_retries_only_stationary_rejections_with_a_bound() {
    for attempt in 1..=6 {
        assert_eq!(
            town_route_retry_delay(StepDisposition::Rejected, true, attempt),
            (attempt <= 5).then_some(Duration::from_millis(500)),
        );
        for disposition in [StepDisposition::Moved, StepDisposition::Blocked] {
            assert_eq!(town_route_retry_delay(disposition, true, attempt), None);
        }
        for disposition in [
            StepDisposition::Moved,
            StepDisposition::Blocked,
            StepDisposition::Rejected,
        ] {
            assert_eq!(town_route_retry_delay(disposition, false, attempt), None);
        }
    }
}

#[cfg(test)]
#[test]
fn qualification_profile_return_reaches_home_temple_after_m04_respawn() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            smoke_with(None, |world| {
                let from = TilePos {
                    x: 32536,
                    y: 32514,
                    floor: -7,
                };
                assert!(route(world, from, |at| at == TOWN).is_none());
                let runtime = tokio::runtime::Builder::new_current_thread().build()?;
                let path = runtime
                    .block_on(reach_route_serviced(
                        world,
                        from,
                        |at| return_protection_zone(world, at),
                        &BTreeSet::new(),
                        &mut ReachBudget::default(),
                        async || {
                            tokio::task::yield_now().await;
                            Ok(())
                        },
                    ))?
                    .map_err(|failure| format!("profile return unreachable: {failure}"))?;
                let mut at = from;
                for direction in &path {
                    at = destination(world, at, *direction).ok_or("invalid profile return step")?;
                }
                assert!(return_protection_zone(world, at));
                assert!(
                    world
                        .base()
                        .tile(at.x, at.y, at.floor)
                        .is_some_and(|tile| tile.protection_zone())
                );
                // BFS must stop at the nearest PZ, including an already safe respawn.
                assert!(path.is_empty() || !return_protection_zone(world, from));
                let disconnected = runtime
                    .block_on(reach_route_serviced(
                        world,
                        at,
                        |at| at == TOWN,
                        &BTreeSet::new(),
                        &mut ReachBudget::default(),
                        async || Ok(()),
                    ))?
                    .err()
                    .ok_or("Newhaven unexpectedly has a walking route to the fixture town")?;
                let detail = home_temple_return(at, disconnected);
                assert!(detail.contains("profile_return goal=home_temple"));
                assert!(detail.contains("fixture_town=unreachable"));
                assert!(detail.contains("searched=") && detail.contains("component="));
                evidence(&format!("{detail} path_length={}", path.len()));
                let town = runtime
                    .block_on(reach_route_serviced(
                        world,
                        TOWN,
                        |at| return_protection_zone(world, at),
                        &BTreeSet::new(),
                        &mut ReachBudget::default(),
                        async || Ok(()),
                    ))?
                    .map_err(|failure| {
                        format!("fixture town lost its protection zone: {failure}")
                    })?;
                assert!(town.is_empty());
                let first = runtime
                    .block_on(reach_route_serviced(
                        world,
                        TOWN,
                        |at| at != TOWN,
                        &BTreeSet::new(),
                        &mut ReachBudget::default(),
                        async || Ok(()),
                    ))?
                    .map_err(|failure| failure.to_string())?;
                let blocked = destination(world, TOWN, first[0]).ok_or("missing first edge")?;
                let alternative = runtime
                    .block_on(reach_route_serviced(
                        world,
                        TOWN,
                        |at| at != TOWN,
                        &BTreeSet::from([(TOWN, blocked)]),
                        &mut ReachBudget::default(),
                        async || Ok(()),
                    ))?
                    .map_err(|failure| failure.to_string())?;
                assert_ne!(destination(world, TOWN, alternative[0]), Some(blocked));
                // A search that lasts a chunk must propagate a failed liveness service.
                let mut services = 0;
                let refused = runtime.block_on(reach_route_serviced(
                    world,
                    TOWN,
                    |_| false,
                    &BTreeSet::new(),
                    &mut ReachBudget::default(),
                    async || {
                        services += 1;
                        Err("route service failed".into())
                    },
                ));
                assert_eq!(services, 1);
                assert_eq!(
                    refused
                        .err()
                        .ok_or("liveness error was swallowed")?
                        .to_string(),
                    "route service failed"
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "profile return test thread failed")??;
    Ok(())
}

#[cfg(test)]
#[test]
fn qualification_fixture_hometown_follows_fixture_start() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            smoke_with(Some(&[(126, 125), (126, 126), (127, 127)]), |world| {
                let temple = fixture_home_town(world)?
                    .temple
                    .ok_or("fixture temple absent")?;
                assert_eq!(fixture_home_town(world)?.town_id, Some(8));
                assert_eq!(
                    fixture_home_town(world)?.town_id,
                    world.base().subareas.default_town
                );
                let temple = TilePos {
                    x: temple.x,
                    y: temple.y,
                    floor: temple.floor,
                };
                assert!(return_protection_zone(world, temple));
                assert!(route(world, temple, |at| at == TOWN).is_some());
                let target = world
                    .facts()
                    .doors()
                    .keys()
                    .find_map(|key| {
                        let (at, ordinal) = crate::map::view::placement_of(key)?;
                        let appearance = world
                            .base()
                            .tile(at.x, at.y, at.floor)?
                            .appearance(ordinal)?;
                        if !retry_door(appearance, world.facts().action_at(key)) {
                            return None;
                        }
                        let path = route(world, TOWN, |next| {
                            next.floor == at.floor
                                && next.x.abs_diff(at.x).max(next.y.abs_diff(at.y)) == 3
                        })?;
                        Some((at, appearance, path.len()))
                    })
                    .ok_or("fixture bundle has no reachable plain far-USE door")?;
                evidence(&format!(
                    "fixture_world_targets far_use={target:?} default_town={:?}",
                    world.base().subareas.default_town
                ));
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "fixture hometown test thread failed")??;
    Ok(())
}

#[cfg(test)]
#[test]
fn qualification_retry_door_excludes_quest_storage() {
    for appearance in [1642, 1644, 1660, 1662, 1674, 1676, 1689, 1698] {
        assert!(!retry_door(appearance, Some(41719)));
        assert!(!retry_door(appearance, None));
    }
    assert!(retry_door(2177, None));
    assert!(retry_door(2179, None));
    assert!(!retry_door(2177, Some(101)));
}

#[cfg(test)]
#[test]
fn qualification_q01_bundle_route_reaches_noble_armor_door() -> TestResult {
    flr2_reward_route(
        "Q01a",
        TilePos {
            x: 32451,
            y: 32048,
            floor: -8,
        },
    )
}

#[cfg(test)]
#[test]
fn qualification_flr2_q03a_town_route_currently_exhausts_search() -> TestResult {
    flr2_reward_route(
        "Q03a",
        TilePos {
            x: 32305,
            y: 32254,
            floor: -9,
        },
    )
}

#[cfg(test)]
#[test]
fn qualification_flr2_q03i_descends_to_adjacent_reward() -> TestResult {
    flr2_reward_route(
        "Q03i",
        TilePos {
            x: 32346,
            y: 32063,
            floor: -12,
        },
    )
}

#[cfg(test)]
#[test]
fn qualification_flr2_q03k_descends_to_adjacent_reward() -> TestResult {
    flr2_reward_route(
        "Q03k",
        TilePos {
            x: 32371,
            y: 32262,
            floor: -12,
        },
    )
}

#[cfg(test)]
fn flr2_reward_route(label: &'static str, target: TilePos) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            static FIXTURE: std::sync::OnceLock<Result<(BundleWorld, Vec<u8>), String>> =
                std::sync::OnceLock::new();
            let (world, bytes) = FIXTURE
                .get_or_init(|| flr2_bundle_diagnostic().map_err(|error| error.to_string()))
                .as_ref()
                .map_err(Clone::clone)?;
            let planner = travel::Planner::load(world, bytes).map_err(|error| error.to_string())?;
            let planned = planner.plan(
                TOWN,
                target,
                &planner.reward_goal(target),
                true,
                &travel::Passages::default(),
            );
            if label == "Q03a" {
                let reason = planned.err().ok_or("Q03a unexpectedly has a route")?;
                assert_eq!(reason, "every reachable tile of the search box exhausted (114052 settled); closest tile reached 32305,32254,7");
                let result = full_row_q01::q03a_route::<()>(label, Err(reason)).map(|_| ());
                assert_eq!(full_row_q01::row_status(&result), "FAIL");
                return Ok(());
            }
            let route = planned?;
            let description = planner.describe(TOWN, &route);
            if label == "Q01a" {
                let description = description.join(" ");
                assert!(description.contains("rope-USE@32453,32039,9"));
                assert!(description.contains("door@32448,32042,8(action=1035)"));
            }
            for line in description {
                eprintln!("FLR2_ROUTE {label} {line}");
            }
            let landing = route
                .last()
                .map(|(_, at)| *at)
                .ok_or("reward route is empty")?;
            assert_eq!(landing.floor, target.floor);
            assert_eq!(
                landing
                    .x
                    .abs_diff(target.x)
                    .max(landing.y.abs_diff(target.y)),
                1
            );
            assert!(
                world.enterable(landing),
                "{label} reward landing must be walkable"
            );
            if label == "Q03k" {
                for (from, to) in [
                    ((32357, 32171, -7), (32356, 32171, -8)),
                    ((32371, 32241, -9), (32371, 32242, -10)),
                    ((32362, 32264, -11), (32361, 32265, -12)),
                ] {
                    let pos = |(x, y, floor)| TilePos { x, y, floor };
                    let mut previous = TOWN;
                    assert!(route.iter().any(|(leg, next)| {
                        let found = previous == pos(from) && *next == pos(to)
                            && matches!(leg, travel::Leg::Step(_));
                        previous = *next;
                        found
                    }), "missing observed floor step {from:?}->{to:?}");
                }
            } else {
                assert!(route.iter().any(|(leg, _)| matches!(leg, travel::Leg::Descent { .. })),
                    "{label} route must include a real descent action");
            }
            assert!(
                route.iter().any(|(_, at)| at.floor < TOWN.floor),
                "{label} route must go below z7"
            );
            Ok(())
        })?
        .join()
        .map_err(|_| "FLR2 regression thread failed")??;
    Ok(())
}

#[cfg(test)]
fn flr2_bundle_diagnostic() -> TestResult<(BundleWorld, Vec<u8>)> {
    use oteryn_world_bundle_compiler::{bundle::BuildClass, compile, source};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    verdict::Harness::global().progress("flr2 compile");
    let project = source::project(&root)?;
    let (registry, palette) = source::registry(&root, &project)?;
    let input = compile::Input {
        regions: &project.regions,
        palette: &palette,
        identity: serde_json::from_slice(&std::fs::read(
            root.join("content/world/pins/oteryn.identity.json"),
        )?)?,
        world: source::world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: oteryn_world_bundle_compiler::project::draft_areas(&project.index)?,
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    let mut pin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
    pin["digest"] = compiled
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .into();
    let booted = boot_world(
        compiled.bytes.clone(),
        &pin,
        WorldId::decode(&v7(1, 2))?,
        ChannelId::decode(&v7(1, 3))?,
    )?;
    let world = &booted;
    let columns = [
        ("Q01a closest", 32446, 32048, 8),
        ("Q01a target", 32451, 32048, 8),
        ("Q01c closest", 33191, 31867, 12),
        ("Q01c target", 33178, 31870, 12),
        ("Q01d closest", 33143, 31773, 13),
        ("Q01d target", 33109, 31679, 13),
        ("Q03a sewer", 32310, 32248, 9),
        ("Q03k sewer", 32366, 32252, 12),
        ("Q03i shovel", 32347, 32123, 12),
        ("Q03i pick", 32356, 32074, 12),
        ("Q03a", 32305, 32254, 9),
        ("Q03i", 32346, 32063, 12),
        ("Q03k", 32371, 32262, 12),
        ("Q02 boat closest", 32392, 32414, 7),
        ("Q02 boat target", 32393, 32513, 7),
        ("Q02 Kesar closest", 32365, 32423, 7),
        ("Q02 Kesar target", 32365, 32478, 2),
        ("Q02 wall closest", 33479, 31985, 7),
        ("Q02 wall target", 33834, 32139, 4),
    ];
    oteryn_world_bundle::bundle::visit(&compiled.bytes, |manifest, sector| {
        verdict::Harness::global().progress("flr2 inspect");
        for tile in sector.tiles {
            for &(label, x, y, z) in &columns {
                if tile.x.abs_diff(x) <= 2
                    && tile.y.abs_diff(y) <= 2
                    && (7.min(z)..=7.max(z)).contains(&-sector.floor)
                {
                    let items: Vec<_> = tile
                        .items
                        .iter()
                        .map(|item| {
                            let entry = &manifest.palette[item.palette as usize];
                            format!(
                                "{} depth={} attrs={:?} floor_mask={} blocked={} terrain={:?}",
                                entry.key,
                                item.depth,
                                item.attrs,
                                crate::map::floor::palette_mask(entry.family, &entry.key),
                                world.facts().blocks(item.palette),
                                entry.terrain
                            )
                        })
                        .collect();
                    eprintln!(
                        "FLR2_TILE {label} {},{},{} flags={} items=[{}]",
                        tile.x,
                        tile.y,
                        -sector.floor,
                        tile.flags,
                        items.join("; ")
                    );
                }
            }
        }
        Ok(())
    })?;
    for &(label, x, y, z) in &columns {
        for z in 7.min(z)..=7.max(z) {
            if world.base().tile(x, y, -z).is_none() {
                eprintln!("FLR2_TILE {label} {x},{y},{z} absent");
            }
        }
    }
    Ok((booted, compiled.bytes))
}

#[cfg(test)]
#[test]
fn qualification_flr2_catalogue_admission_without_native_profile() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| flr2_catalogue_admission().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "FLR2 admission worker failed")??;
    Ok(())
}

#[cfg(test)]
fn flr2_catalogue_admission() -> TestResult {
    verdict::Harness::global().progress("flr2 catalogue admission");
    let world_id = WorldId::decode(&v7(1, 2))?;
    let channel = ChannelId::decode(&v7(1, 3))?;
    let room = crate::content::qualify_selected_native_gameplay_room(world_id, native_input()?)?;
    let issuance = crate::content::NativeEntryActivationIssuance {
        world_id,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = crate::content::ContentActivationController::new();
    crate::content::activate_native_entry_room_with_gameplay(
        &mut controller,
        &crate::content::NodeBootQuiescence::before_channel_runtime(),
        world_id,
        &issuance,
        native_input()?,
    )?;
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native gameplay")?;
    let definition = |id| -> TestResult<_> {
        let key = format!("oteryn:item.tibia.i{id}");
        let index = native.item_index().ok_or("item index")?;
        let revision = index.revision(&key).ok_or("catalogue revision")?;
        assert!(
            native.item_policy(&key, revision).is_none(),
            "fixture must lack a native profile: {id}"
        );
        let reference = index
            .definition_ref("Item", &key, revision)
            .ok_or("catalogue reference")?;
        Ok((key, reference.get() - 1))
    };
    let archway = definition(1530)?;
    let wall = definition(1863)?;
    let (bytes, bundle) =
        crate::map::boot::tests::bundle_with_item_ids((&archway.0, archway.1), (&wall.0, wall.1));
    let pins = crate::map::boot::BootPins {
        map_revision: format!(
            "sha256:{}",
            bundle
                .digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        bundle,
        start: TilePos {
            x: 2,
            y: 0,
            floor: -7,
        },
    };
    let world = crate::map::boot::boot(&bytes, &pins, world_id, channel, |key| {
        crate::map::boot::native_item_definition(Some(native), key)
    })?;
    assert!(
        world.enterable(TilePos {
            x: 6,
            y: 0,
            floor: -7
        }),
        "catalogue archway must admit movement"
    );
    assert!(
        !world.enterable(TilePos {
            x: 7,
            y: 0,
            floor: -7
        }),
        "catalogue blocking stone must block movement"
    );
    Ok(())
}

#[cfg(test)]
mod eligibility_review {
    use super::*;

    #[test]
    fn named_respawn_registers_its_full_watchdog_window() {
        let mut registered = None;
        let deadline =
            named_respawn_deadline(Duration::from_secs(92), |until| registered = Some(until));
        assert_eq!(registered, Some(deadline));
    }
}

async fn final_logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    drive_final_logout(
        client,
        async |client| Ok(client.session_mut().logout().await?.result),
        async |client| {
            client.service_liveness(Duration::from_millis(250)).await?;
            client.take_events();
            Ok(())
        },
    )
    .await
}

async fn drive_final_logout<C>(
    client: &mut C,
    mut send: impl AsyncFnMut(
        &mut C,
    ) -> TestResult<Option<oteryn_protocol_oteryn::logout::LogoutResult>>,
    mut service: impl AsyncFnMut(&mut C) -> TestResult,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let result = send(client).await?;
        if !final_logout_retry(result) || tokio::time::Instant::now() >= deadline {
            return require_final_logout_result(result);
        }
        service(client).await?;
    }
}

fn require_final_logout_result(
    result: Option<oteryn_protocol_oteryn::logout::LogoutResult>,
) -> TestResult {
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("final logout requires Accepted, observed {result:?}").into());
    }
    Ok(())
}

#[test]
fn round2_final_logout_requires_accepted_release() {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    assert!(require_final_logout_result(Some(LogoutResult::Accepted)).is_ok());
    assert!(require_final_logout_result(Some(LogoutResult::Busy)).is_err());
    assert!(
        require_final_logout_result(Some(LogoutResult::InFight { retry_after_ms: 1 })).is_err()
    );
    assert!(require_final_logout_result(None).is_err());
}

fn final_logout_retry(result: Option<oteryn_protocol_oteryn::logout::LogoutResult>) -> bool {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    matches!(
        result,
        Some(LogoutResult::Busy | LogoutResult::InFight { .. })
    )
}

#[tokio::test]
async fn round3_final_logout_retry() -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let mut state = (
        std::collections::VecDeque::from([
            LogoutResult::Busy,
            LogoutResult::InFight { retry_after_ms: 1 },
            LogoutResult::Accepted,
        ]),
        0,
    );
    drive_final_logout(
        &mut state,
        async |state| Ok(state.0.pop_front()),
        async |state| {
            state.1 += 1;
            Ok(())
        },
    )
    .await?;
    assert!(state.0.is_empty());
    assert_eq!(state.1, 2);
    Ok(())
}

#[test]
fn qualification_core_shop_requires_all_preconditions() -> TestResult {
    let world = include_str!("world.rs");
    let between = |start, end| -> TestResult<&str> {
        Ok(world
            .split_once(start)
            .ok_or("start")?
            .1
            .split_once(end)
            .ok_or("end")?
            .0)
    };
    let shop = between("async fn shop_journey(", "fn qualification_nbtown_checks")?;
    assert_eq!(shop.matches("return Err(").count(), 3, "shop preconditions");
    Ok(())
}

#[test]
fn qualification_paid_confirmation_propagates_failure() {
    let world = include_str!("world.rs")
        .split_once("fn qualification_paid_confirmation_propagates_failure")
        .map_or("", |(implementation, _)| implementation);
    let reference = include_str!("travel.rs");
    assert!(world.contains("converse_with(client, npc, \"yes\", true).await?"));
    assert!(reference.contains("converse(client, npc, \"yes\").await?"));
}

#[test]
fn qualification_map_tool_uses_command_owning_capability() {
    for source in [include_str!("map_journey.rs"), include_str!("travel.rs")] {
        assert!(source.contains(".world_context_use(&payload)"));
    }
}

#[test]
fn qualification_q03a_successful_preflight_admits_client() -> TestResult {
    let source = include_str!("full_row_q01.rs");
    let after_preflight = source
        .split_once("from_town(planner, chest, &label)?;")
        .ok_or("preflight")?
        .1;
    assert!(
        after_preflight
            .trim_start()
            .starts_with("let mut client = connect_character(")
    );
    Ok(())
}

#[test]
fn qualification_travel_replans_before_executing_stale_route() -> TestResult {
    let source = include_str!("world.rs");
    let after_search = source
        .split_once("if let Some(error) = service_error {")
        .ok_or("search result")?
        .1;
    let before_actions = after_search
        .split_once("planner.execute(client")
        .ok_or("route execution")?
        .0;
    assert!(before_actions.contains("if observed != from {"));
    Ok(())
}

#[test]
fn qualification_reachability_replans_are_bounded() -> TestResult {
    let source = include_str!("world.rs");
    let search = source
        .split_once("async fn reach_route_until(")
        .ok_or("reachability entry")?
        .1;
    let search = search
        .split_once("async fn reach_route_serviced(")
        .ok_or("offline search")?
        .0;
    assert!(search.contains("for attempt in 0..=travel::MAX_REPLANS"));
    Ok(())
}

#[test]
fn qualification_death_relog_awaits_fixture_release_on_mismatch() -> TestResult {
    let source = include_str!("world.rs");
    let relog = source
        .split_once("client = connect_world(clients, 0x73).await?;")
        .ok_or("death relog")?
        .1;
    let relog = relog
        .split_once("async fn profile_journey")
        .ok_or("journey end")?
        .0;
    assert!(relog.contains("wait_character_released(clients, 1)"));
    assert!(relog.contains(".and(snapshot_result)"));
    assert!(relog.contains("finish_after_terminal_release("));
    Ok(())
}

#[test]
fn qualification_serviced_search_and_provisioning_drain_observations() -> TestResult {
    let travel = include_str!("world.rs")
        .split_once(".plan_serviced(from, target, goal, rope, passages, async ||")
        .ok_or("search callback")?
        .1;
    assert!(
        travel
            .split_once("if let Some(error) = service_error")
            .ok_or("search callback end")?
            .0
            .contains("client.take_events()")
    );
    for source in [
        include_str!("full_row_q01.rs"),
        include_str!("provision.rs"),
    ] {
        let callback = source
            .split_once("wait_fixture_command(")
            .ok_or("fixture wait")?
            .1;
        assert!(
            callback
                .split_once("Ok(())")
                .ok_or("fixture service end")?
                .0
                .contains("client.take_events()")
        );
    }
    Ok(())
}

#[test]
fn qualification_fixture_command_joins_writer_when_wait_is_cancelled() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(FIXTURE_WORKERS.scope(Default::default(), async {
            let (release, gate) = std::sync::mpsc::sync_channel(1);
            let task = FixtureCommand::start(move || {
                gate.recv_timeout(Duration::from_secs(2))
                    .map_err(|error| error.to_string())?;
                Ok(())
            })?;
            drop(task);
            let mut cleanup = Box::pin(await_fixture_workers());
            tokio::select! {
                biased;
                () = &mut cleanup => return Err("cleanup completed before writer release".into()),
                () = tokio::task::yield_now() => {}
            }
            release.send(())?;
            cleanup.await;
            Ok(())
        }))
}

#[test]
fn qualification_missing_pick_is_classified_as_fixture_precondition() {
    let error = travel::missing_tool_error(3456, "required for descent");
    assert_eq!(
        verdict::classify(error.as_ref()),
        verdict::Cause::Precondition {
            name: "fixture-tool-3456".into()
        }
    );
    let result = Err(error);
    assert_eq!(
        e2e_result_line(&format!(
            "e2e_all scenario=Q03i.full status={} seconds=0",
            full_row_q01::row_status(&result)
        )),
        "e2e_all scenario=Q03i.full status=FAIL seconds=0",
    );
}

#[test]
fn qualification_spawn_and_fixture_command_have_outer_process_deadlines() -> TestResult {
    let command = fixture_command();
    let command = command.as_std();
    let args: Vec<_> = command.get_args().collect();
    assert_eq!(command.get_program(), "timeout");
    assert_eq!(
        args[..9],
        [
            "--kill-after=2s",
            "30s",
            "sudo",
            "-n",
            "timeout",
            "--kill-after=2s",
            "25s",
            "python3",
        ]
    );
    assert_eq!(args.len(), 11);
    assert!(Path::new(args[9]).ends_with("tools/qualification/node_boot/provision.py"));
    assert_eq!(args[10], "--watch-stdin");
    let source = include_str!("world.rs");
    let spawn = source
        .split_once("async fn request_spawn(")
        .ok_or("spawn")?
        .1;
    let spawn = spawn.split_once("async fn walk(").ok_or("spawn end")?.0;
    assert!(spawn.contains("fixture_command()"));
    for source in [
        include_str!("full_row_q01.rs"),
        include_str!("provision.rs"),
    ] {
        assert!(source.contains("fixture_command()"));
    }
    Ok(())
}

#[test]
fn qualification_round_two_caller_contracts() {
    let checks = [
        (include_str!("npc_shop.rs"), "require_buy_funds(before_gold"),
        (include_str!("full_row_p03.rs"), "hunt_named_target("),
        (include_str!("full_gold.rs"), "hunt_observed_target("),
        (include_str!("world.rs"), "world_map_tile(position)"),
        (
            include_str!("world.rs"),
            "disposition == UseDisposition::Committed && needs_revision",
        ),
        (
            include_str!("indexed_verdict.rs"),
            "await_fixture_workers().await",
        ),
        (
            include_str!("../../../../../tools/qualification/node_boot/provision.py"),
            "CONTROL_REPLY_TIMEOUT = 40",
        ),
    ];
    let implementation = |source: &'static str| {
        source
            .split("#[test]\nfn qualification_round_two_caller_contracts")
            .next()
            .unwrap_or_default()
    };
    let missing: Vec<_> = checks
        .into_iter()
        .filter_map(|(source, needle)| (!implementation(source).contains(needle)).then_some(needle))
        .collect();
    assert!(missing.is_empty(), "missing contracts: {missing:?}");
}

#[test]
fn qualification_retries_and_encounter_cleanup_keep_their_owners() {
    let q02 = include_str!("full_row_q02.rs");
    let s10 = include_str!("full_row_s10.rs");
    let c09 = include_str!("client_row_c09.rs");
    let mp09 = include_str!("mp/mp09_party_xp.rs");
    let combat = c09
        .split_once("for _ in 0..3 {")
        .map_or("", |(_, combat)| combat);
    let missing: Vec<_> = [
        ("Q02", q02.contains("hunt_target(client, Some(target))")),
        ("S10", s10.contains("hunt_target(client, Some(target))")),
        (
            "C09",
            combat.contains("let target = travel_to_spawn_target("),
        ),
        ("MP09", mp09.contains("fixture_timeout(")),
    ]
    .into_iter()
    .filter_map(|(name, ok)| (!ok).then_some(name))
    .collect();
    assert!(missing.is_empty(), "missing caller behavior: {missing:?}");
}

#[test]
fn qualification_cancelled_fixture_drain_keeps_pending_worker() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(FIXTURE_WORKERS.scope(Default::default(), async {
            let (release, gate) = std::sync::mpsc::sync_channel(1);
            let _worker = FixtureCommand::start(move || {
                gate.recv_timeout(Duration::from_secs(2))
                    .map_err(|error| error.to_string())
            })?;
            for _ in 0..2 {
                let mut drain = Box::pin(await_fixture_workers());
                tokio::select! {
                    biased;
                    () = &mut drain => return Err("pending worker escaped fixture cleanup".into()),
                    () = tokio::task::yield_now() => {}
                }
                drop(drain);
            }
            release.send(())?;
            await_fixture_workers().await;
            Ok(())
        }))
}

#[test]
fn qualification_routed_hunts_service_empty_spawns_and_approach_live_targets() {
    let q02 = include_str!("full_row_q02.rs");
    let s10 = include_str!("full_row_s10.rs");
    let world = include_str!("world.rs");
    let spawn = world
        .split_once("async fn travel_to_spawn_target")
        .map_or("", |(_, body)| body);
    let spawn = spawn
        .split_once("pub(crate) fn smoke")
        .map_or("", |(body, _)| body);
    let missing: Vec<_> = [
        (
            "Q02 approach",
            q02.contains("approach_visible_spawn_target("),
        ),
        (
            "S10 approach and wait",
            s10.contains("approach_visible_spawn_target("),
        ),
        (
            "C09 approach and wait",
            spawn.contains("approach_spawn_target("),
        ),
    ]
    .into_iter()
    .filter_map(|(name, ok)| (!ok).then_some(name))
    .collect();
    assert!(missing.is_empty(), "missing caller behavior: {missing:?}");
}

#[test]
fn qualification_timeout_invalidates_interrupted_exchange() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let start = tokio::time::Instant::now();
            let budget = Duration::from_secs(1);
            let mut usable = true;
            let result = timed_exchange(
                &mut usable,
                start + budget,
                "spawn-approach-step",
                |usable| *usable = false,
                async |_| {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                },
            )
            .await;
            assert!(result.is_err());
            assert!(!usable);
            assert_eq!(start.elapsed(), budget);
            usable = true;
            let outer = tokio::time::timeout(
                budget,
                timed_exchange(
                    &mut usable,
                    start + Duration::from_secs(30),
                    "spawn-approach-step",
                    |usable| *usable = false,
                    async |_| std::future::pending::<()>().await,
                ),
            )
            .await;
            assert!(outer.is_err());
            assert!(!usable);
            Ok(())
        })
}

#[test]
fn qualification_ready_exchange_at_deadline_is_rejected() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let mut usable = true;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            let phase = "earn-fare-combat-and-loot";
            let result = timed_exchange(
                &mut usable,
                deadline,
                phase,
                |usable| *usable = false,
                async |_| tokio::time::sleep_until(deadline).await,
            )
            .await;
            let Err(error) = result else {
                return Err("exchange completed at its deadline".into());
            };
            assert_eq!(
                verdict::classify(error.as_ref()),
                verdict::Cause::Deadline { phase }
            );
            assert!(
                usable,
                "a completed exchange leaves the session synchronized"
            );
            Ok(())
        })
}

#[test]
fn qualification_spawn_deadline_keeps_last_action() -> TestResult {
    for phase in ["spawn-observation", "spawn-approach-step", "earn-fare-work"] {
        let Err(error) = action_deadline(tokio::time::Instant::now(), phase) else {
            return Err(format!("{phase} accepted an expired deadline").into());
        };
        assert_eq!(
            verdict::classify(error.as_ref()),
            verdict::Cause::Deadline { phase }
        );
    }
    Ok(())
}

#[test]
fn qualification_two_respawns_preserve_fare_work_budget() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let start = tokio::time::Instant::now();
            let (mut work_deadline, hard_deadline) =
                full_row_s10::fare_deadlines(start, [90_000].into_iter());
            let work = Duration::from_secs(180);
            for _ in 0..2 {
                let waiting = tokio::time::Instant::now();
                tokio::time::sleep(Duration::from_secs(90)).await;
                account_for_spawn_wait(&mut work_deadline, waiting);
            }
            assert_eq!(work_deadline - tokio::time::Instant::now(), work);
            let mut usable = true;
            let combat = timed_exchange(
                &mut usable,
                work_deadline.min(hard_deadline),
                "earn-fare-combat-and-loot",
                |usable| *usable = false,
                async |_| tokio::time::sleep(Duration::from_secs(120)).await,
            )
            .await;
            assert!(combat.is_ok());
            assert_eq!(
                tokio::time::Instant::now() - start,
                Duration::from_secs(300)
            );
            assert!(hard_deadline >= work_deadline);
            Ok(())
        })
}

#[test]
fn qualification_spawn_budget_covers_authored_ninety_seconds() {
    assert!(spawn_observation_budget([90_000].into_iter()) > Duration::from_secs(90));
}
