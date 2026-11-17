use super::*;

// Pinned 15.30 appearances (SHA-256 2dfa943b...dae7e2): each of these six
// decorations has unmove=true and neither unpass nor unpath set. The source
// profile producer independently checks the complete flags and XML overrides.
const DECORATIONS: [u32; 6] = [1534, 1535, 1856, 1857, 1865, 1867];

fn on_route_thread(run: fn() -> TestResult) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || run().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NBROUTE worker panicked")??;
    Ok(())
}

#[test]
fn qualification_nbroute_native_decoration_profiles() -> TestResult {
    on_route_thread(|| {
        let world = WorldId::decode(&v7(1, 2))?;
        let input = native_input()?;
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
            .and_then(|active| active.native_gameplay());
        for id in DECORATIONS {
            let key = format!("oteryn:item.tibia.i{id}");
            let definition = crate::map::boot::native_item_definition(native, &key)
                .ok_or_else(|| format!("missing native Item {key}"))?;
            assert_eq!(definition.solid, Some(false), "donor collision for {key}");
            assert!(
                !definition.blocks_projectile,
                "donor projectile flag for {key}"
            );
            assert!(!definition.pickupable, "fixed donor decoration {key}");
        }
        let portal = crate::map::boot::native_item_definition(native, "oteryn:item.tibia.i37001");
        assert_eq!(
            portal.and_then(|definition| definition.solid),
            Some(false),
            "donor forcefield is not solid"
        );
        Ok(())
    })
}

fn endpoint(world: &BundleWorld, from: TilePos, path: &[WalkAction]) -> TestResult<TilePos> {
    path.iter().try_fold(from, |at, direction| {
        destination(world, at, *direction).ok_or_else(|| "route contains a refused step".into())
    })
}

type CompiledTown = (
    BundleWorld,
    oteryn_world_bundle::npc::Table,
    oteryn_world_bundle::spawn::Table,
);
fn compile_town(regions: &[(u16, u16)]) -> TestResult<CompiledTown> {
    use oteryn_world_bundle_compiler::{bundle::BuildClass, compile, source};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // The same compile-regions inputs and equivalence proof as node boot.
    let project = source::project_regions(&root, regions)?;
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
    compile::equivalence(&input, &registry, &compiled.bytes)?;
    let mut pin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
    pin["digest"] = compiled
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .into();
    let metadata = oteryn_world_bundle::bundle::visit(&compiled.bytes, |_, _| Ok(()))?;
    let world = boot_world(
        compiled.bytes,
        &pin,
        WorldId::decode(&v7(1, 2))?,
        ChannelId::decode(&v7(1, 3))?,
    )?;
    Ok((world, metadata.npcs, metadata.spawns))
}
fn compiled_town() -> TestResult<&'static CompiledTown> {
    static TOWN_WORLD: std::sync::OnceLock<Result<CompiledTown, String>> =
        std::sync::OnceLock::new();
    TOWN_WORLD
        .get_or_init(|| {
            compile_town(&[(126, 125), (126, 126), (127, 127)]).map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| error.clone().into())
}

#[test]
fn qualification_nbroute_compiled_town() -> TestResult {
    on_route_thread(|| {
        let (world, npcs, spawns) = compiled_town()?;
        assert!(
            world.base().tile(TOWN.x, TOWN.y, -7).is_some(),
            "compiled floor is native -z"
        );
        assert!(
            world.base().tile(TOWN.x, TOWN.y, 7).is_none(),
            "positive source z is not a native bundle floor"
        );
        // The runner uses these exact regions and pristine temple coordinates.
        // Prove absence of candidates rather than claiming a failed walk search.
        for id in [3165, 3180, 3156, 3177, 3203, 3178, 3197, 2854] {
            let sellers = rune_seller_keys(id)?;
            assert!(
                !sellers.is_empty(),
                "missing trade eligibility facts for {id}"
            );
            let inventory =
                npc_route_failure(TOWN, &format!("oteryn:item.tibia.i{id}"), npcs, &sellers);
            assert!(
                inventory.placements.is_empty(),
                "compiled seller inventory changed for {id}: {inventory:?}"
            );
            assert_eq!(inventory.in_town_window, 0);
        }
        let inventory = npc_route_failure(
            TOWN,
            "oteryn:npc.oressa",
            npcs,
            &std::collections::BTreeSet::from(["oteryn:npc.oressa".to_owned()]),
        );
        assert!(
            inventory.placements.is_empty(),
            "compiled Oressa inventory changed: {inventory:?}"
        );
        let shrine = TilePos {
            x: 32376,
            y: 32239,
            floor: -7,
        };
        assert!(
            (0..=u8::MAX).any(|ordinal| world
                .base()
                .tile(shrine.x, shrine.y, shrine.floor)
                .and_then(|tile| tile.appearance(ordinal))
                == Some(25803)),
            "production shrine missing from compiled town"
        );
        // The donor shrine has unpass=true. P10 uses an adjacent PZ tile, never
        // walks onto the shrine itself or changes its collision to reach it.
        assert!(!world.enterable(shrine));
        let rats = [
            TilePos {
                x: 32368,
                y: 32240,
                floor: -8,
            },
            TilePos {
                x: 32368,
                y: 32242,
                floor: -8,
            },
        ];
        // A failed/finished preceding hunt can leave the next row downstairs.
        // Exercise PZ exit, shrine approach and cleanup from those saved positions too.
        for rat in rats {
            assert!(
                spawns.sources.iter().any(|source| source.floor == rat.floor
                    && source.points.iter().any(|point| point.x == rat.x
                        && point.y == rat.y
                        && spawns
                            .creatures
                            .get(point.creature as usize)
                            .is_some_and(|entry| entry.key == "oteryn:creature.rat"))),
                "production rat missing from compiled town at {rat:?}"
            );
        }
        for from in [
            TOWN,
            TilePos {
                x: 32369,
                y: 32241,
                floor: -7,
            },
            rats[0],
            rats[1],
            endpoint(
                world,
                TOWN,
                &spawn_route(world, TOWN, spawns, "oteryn:creature.rat").ok_or("rat route")?,
            )?,
            endpoint(
                world,
                TOWN,
                &spawn_route(world, TOWN, spawns, "oteryn:creature.poison_spider")
                    .ok_or("spider route")?,
            )?,
        ] {
            for target in rats {
                let path = route_toward(world, from, target, |at| at == target)
                    .ok_or_else(|| format!("no rat route: {from:?} -> {target:?}"))?;
                if from.floor != target.floor {
                    assert!(
                        path.iter()
                            .any(|action| matches!(action, WalkAction::Use { .. })),
                        "sewer descent must use the authored grate"
                    );
                }
                assert_eq!(endpoint(world, from, &path)?, target);
                let back = route_toward(world, target, from, |at| at == from)
                    .ok_or_else(|| format!("no post-hunt return: {target:?} -> {from:?}"))?;
                assert_eq!(endpoint(world, target, &back)?, from);
            }
            let path = route_toward(world, from, shrine, |at| {
                at.floor == shrine.floor
                    && at.x.abs_diff(shrine.x).max(at.y.abs_diff(shrine.y)) <= 1
                    && world
                        .base()
                        .tile(at.x, at.y, at.floor)
                        .is_some_and(|tile| tile.protection_zone())
            })
            .ok_or_else(|| format!("no shrine approach: {from:?} -> {shrine:?}"))?;
            let inside = endpoint(world, from, &path)?;
            let outside = route_toward(world, inside, "outside PZ", |at| {
                at.floor == inside.floor
                    && at.x.abs_diff(inside.x).max(at.y.abs_diff(inside.y)) >= 4
                    && world
                        .base()
                        .tile(at.x, at.y, at.floor)
                        .is_some_and(|tile| !tile.protection_zone())
            })
            .ok_or("no post-shrine route outside PZ")?;
            let outside = endpoint(world, inside, &outside)?;
            let back = route_toward(world, outside, from, |at| at == from)
                .ok_or("no outside-PZ return")?;
            assert_eq!(endpoint(world, outside, &back)?, from);
        }
        Ok(())
    })
}

#[path = "nbroute_transport_tests.rs"]
mod transport;

#[test]
fn qualification_nbroute_compiled_near_temple_returns() -> TestResult {
    on_route_thread(|| {
        let (world, _, spawns) = compiled_town()?;
        for creature in ["oteryn:creature.rat", "oteryn:creature.poison_spider"] {
            let path = spawn_route(world, TOWN, spawns, creature)
                .ok_or_else(|| format!("missing {creature} path"))?;
            let landed = endpoint(world, TOWN, &path)?;
            evidence(&format!(
                "nbroute_spawn creature={creature} landed={landed:?}"
            ));
            assert!(
                route_toward(world, landed, TOWN, |p| p == TOWN).is_some(),
                "no post-encounter return from {landed:?}"
            );
            assert!(
                spawn_route(world, landed, spawns, "oteryn:creature.rat").is_some(),
                "no post-encounter rat route from {landed:?}"
            );
        }
        let mut queue = VecDeque::from([TOWN]);
        let mut seen = std::collections::BTreeSet::from([TOWN]);
        while let Some(at) = queue.pop_front() {
            let actions = [
                StepDirection::North,
                StepDirection::East,
                StepDirection::South,
                StepDirection::West,
            ]
            .into_iter()
            .map(WalkAction::Step)
            .chain(walk_uses(world, at));
            for action in actions {
                if let Some(next) = destination(world, at, action)
                    && next.x.abs_diff(TOWN.x) <= 10
                    && next.y.abs_diff(TOWN.y) <= 10
                    && next.floor.abs_diff(TOWN.floor) <= 2
                    && seen.insert(next)
                {
                    queue.push_back(next);
                }
            }
        }
        for at in seen {
            assert!(
                route_toward(world, at, TOWN, |p| p == TOWN).is_some(),
                "no temple return from {at:?}"
            );
        }
        Ok(())
    })
}

#[test]
fn qualification_nbroute_preflight_shared_respawn_return() -> TestResult {
    on_route_thread(|| {
        let (world, _, _) = compiled_town()?;
        let towns = &world.base().subareas;
        let home = towns
            .default_town
            .and_then(|id| towns.town(id))
            .and_then(|town| town.temple)
            .ok_or("home temple")?;
        let home = TilePos {
            x: home.x,
            y: home.y,
            floor: home.floor,
        };
        assert_eq!(
            home,
            TilePos {
                x: 32536,
                y: 32514,
                floor: -7
            }
        );
        // Crystal's scripted Newhaven forcefield is present in the bundle.
        // Both the native forcefield collision and the teleport edge are admitted.
        let exit = TilePos {
            x: 32538,
            y: 32514,
            floor: -7,
        };
        assert_eq!(
            world.floors().teleport(exit),
            Some(TilePos {
                x: 32544,
                y: 32515,
                floor: -7
            })
        );
        assert!(
            world.enterable(exit),
            "forcefield native collision is passable"
        );
        let arrival = world
            .floors()
            .teleport(exit)
            .ok_or("forcefield destination")?;
        assert!(
            route_toward(world, home, arrival, |at| at == arrival).is_some(),
            "the authored forcefield exit is reachable"
        );
        assert!(
            world.base().tile(32561, 32485, -7).is_none(),
            "tutorial hunting cell is outside selected regions"
        );
        assert!(home.y.abs_diff(TOWN.y) > 160);
        assert!(route_toward(world, home, TOWN, |at| at == TOWN).is_none());
        assert!(
            route_toward(world, home, "outside Newhaven PZ", |at| world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone()))
            .is_none()
        );
        assert!(
            respawn_return_plan(world, TOWN).is_err(),
            "must refuse before deliberately stranding the shared Character"
        );
        // Staying at the real native home requires no fake relocation.
        assert_eq!(respawn_return_plan(world, home)?, home);
        Ok(())
    })
}

#[test]
fn qualification_nbroute_newhaven_exit_requires_existing_northern_region() -> TestResult {
    on_route_thread(|| {
        // Same source and native facts; include the existing B3 region cut off
        // by node boot's three-region compile. No content or profile is authored.
        let (world, _, _) = compile_town(&[(126, 125), (126, 126), (127, 126), (127, 127)])?;
        let home = TilePos {
            x: 32536,
            y: 32514,
            floor: -7,
        };
        assert!(
            world
                .base()
                .tile(32561, 32485, -7)
                .is_some_and(|tile| !tile.protection_zone())
        );
        let path = route_toward(
            &world,
            home,
            "Newhaven outside PZ with northern region",
            |at| {
                world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| !tile.protection_zone())
            },
        )
        .ok_or("northern region did not open an ordinary player route outside Newhaven PZ")?;
        let landed = endpoint(&world, home, &path)?;
        evidence(&format!(
            "nbroute_newhaven_exit additional_region=127,126 endpoint={landed:?} steps={}",
            path.len()
        ));
        Ok(())
    })
}
